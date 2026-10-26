//! 有界的 IMK 入口取证；不依赖 staged，不改变输入事件路由或恢复行为。

mod kind;
mod limiter;
mod metadata;

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use objc2::MainThreadMarker;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventType};

use super::QingjianInputController;
use crate::host;
use crate::imk::secure_input;
use kind::Kind;
use limiter::Limiter;
use metadata::Metadata;

thread_local! {
    static LIMITER: RefCell<Limiter> = RefCell::new(Limiter::default());
    static CLOCK: Instant = Instant::now();
}

pub(super) fn arm(controller: &QingjianInputController) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let eligible = host::with(|h| h.draft_active()).unwrap_or(false);
        if !eligible {
            LIMITER.with(|limiter| limiter.borrow_mut().stop());
            return;
        }
        let owner = controller as *const QingjianInputController as usize;
        let session = LIMITER.with(|limiter| limiter.borrow_mut().arm(owner));
        tracing::info!(target: "live_translation", session, budget = 16, reason = "activation",
            "退格入口诊断已开启，本次激活限量记录；可输入后按普通退格，无时间限制");
    }));
}

pub(super) fn disarm(controller: &QingjianInputController) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let owner = controller as *const QingjianInputController as usize;
        LIMITER.with(|limiter| limiter.borrow_mut().disarm(owner));
    }));
}

pub(super) fn observe(
    controller: &QingjianInputController,
    event: Option<&NSEvent>,
    client: Option<&AnyObject>,
) {
    // 诊断 panic 只放弃该条记录，不触发 recover 或更改原按键返回值。
    let _ = catch_unwind(AssertUnwindSafe(|| {
        observe_inner(controller, event, client)
    }));
}

fn observe_inner(
    controller: &QingjianInputController,
    event: Option<&NSEvent>,
    client: Option<&AnyObject>,
) {
    let owner = controller as *const QingjianInputController as usize;
    if !LIMITER.with(|limiter| limiter.borrow().armed_for(owner)) {
        return;
    }
    let key_down = event.is_some_and(|event| event.r#type() == NSEventType::KeyDown);
    // 只检查单个 UTF-16 控制码，禁止将 NSString 转为待记录的字符串。
    let single_control = if key_down {
        event
            .and_then(|event| event.characters())
            .filter(|text| text.length() == 1)
            .map(|text| text.characterAtIndex(0))
            .filter(|unit| matches!(unit, 8 | 127))
    } else {
        None
    };
    let key = event.filter(|_| key_down).map(NSEvent::keyCode);
    let repeat = event
        .filter(|_| key_down)
        .map(NSEvent::isARepeat)
        .unwrap_or(false);
    let kind = Kind::classify(
        event.is_some(),
        client.is_some(),
        key_down,
        key,
        single_control,
    );
    let elapsed = CLOCK.with(Instant::elapsed);
    let selected = LIMITER.with(|limiter| {
        let mut limiter = limiter.borrow_mut();
        limiter
            .allow(kind, elapsed, repeat)
            .then(|| (limiter.ordinary_count(), limiter.session()))
    });
    let Some((ordinary_count, session)) = selected else {
        return;
    };
    let metadata = host::with(|h| Metadata::capture(h));
    let staged = metadata.as_ref().map(|_| host::stage_text_output());
    if kind == Kind::NonDeleteKeyDown {
        // 普通键只记一次聚合心跳；不包含键码、修饰位、按键长度或字符。
        tracing::info!(target: "live_translation", kind = ?kind, ordinary_count, session,
            event_present = event.is_some(), client_present = client.is_some(),
            host_available = metadata.is_some(), staged,
            current_controller = controller.is_current_controller(), "IMK诊断入口活跃");
        return;
    }
    let safe_key = kind
        .key_code()
        .or(key.filter(|key| matches!(key, 51 | 117)));
    let special = kind.special_slot().is_some() || safe_key.is_some();
    tracing::info!(target: "live_translation", kind = ?kind, session,
        event_present = event.is_some(), client_present = client.is_some(),
        event_type = event.map(|event| event.r#type().0), key_code = safe_key,
        flags = special.then(|| event.map(|event| event.modifierFlags().bits())).flatten(),
        repeat = event.filter(|_| key_down).map(NSEvent::isARepeat),
        control_form = special.then_some(single_control).flatten(),
        ordinary_count, host_available = metadata.is_some(), staged,
        main_thread = MainThreadMarker::new().is_some(), current_controller = controller.is_current_controller(),
        secure = secure_input::enabled(), live_enabled = metadata.as_ref().map(|value| value.live_enabled),
        draft_enabled = metadata.as_ref().map(|value| value.draft_enabled),
        application_known = metadata.as_ref().map(|value| value.application_known),
        terminal_application = metadata.as_ref().map(|value| value.terminal_application),
        pinyin_chars = metadata.as_ref().map(|value| value.pinyin_chars),
        draft_chars = metadata.as_ref().map(|value| value.draft_chars),
        epoch = metadata.as_ref().map(|value| value.epoch), "IMK退格入口诊断");
}
