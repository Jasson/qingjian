//! 独立主线程原生探针与真实帧预览；不初始化 Host、IMKServer 或连接翻译服务。

mod runner;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_predict::live::{FocusedDraftModel, Translation};
use qingjian_render::{FontLibrary, Layout, Renderer, Theme};

pub(crate) fn run() -> i32 {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .try_init();
    let Some(mtm) = MainThreadMarker::new() else {
        return 1;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    let mut engine = Engine::new(
        Dictionary::parse("你好\tni hao\t10000\n你号\tni hao\t9000\n你\tni\t8000\n好\thao\t7000\n")
            .expect("探针词库"),
    );
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "nihao", false, true);
    let Ok(fonts) = FontLibrary::system("zh-CN") else {
        return 1;
    };
    let mut renderer = Renderer::new(fonts);
    let samples = [
        (
            "candidate",
            "当前高亮候选参与预译；按空格选字，Enter 填入原文",
            false,
        ),
        ("ready", "译文已就绪；可填入译文或复制", true),
        (
            "confirm",
            "未填入。回到要填入的输入框，再按 Ctrl+Alt+E 明确确认当前新位置；也可复制译文",
            true,
        ),
    ];
    for (name, status, ready) in samples {
        if name == "ready" {
            model.choose(&mut engine, 0);
            model.text(&mut engine, "，今天很高兴见到你。", true, true);
            model.submitted(1);
            model.accept(Translation {
                id: 1,
                source: model.source(),
                result: Ok("Hello, I am very happy to see you today.".into()),
            });
        }
        let (frame, _) = super::render::build_frame(&model, status, ready, false);
        let path = format!("/tmp/qingjian-focused-draft-{name}.png");
        let rendered = match renderer.render(
            &crate::candidates::render_frame(&frame),
            Layout::Vertical,
            &Theme::light(),
            2.0,
        ) {
            Ok(rendered) => rendered,
            Err(error) => {
                eprintln!("草稿帧渲染失败：{error}");
                return 1;
            }
        };
        if let Err(error) = rendered.pixmap.save_png(&path) {
            eprintln!("预览保存失败：{error}");
            return 1;
        }
        println!("真实草稿帧：{path}");
    }
    runner::run(mtm, &app, model)
}

fn native_workflow(mtm: MainThreadMarker, app: &NSApplication, model: FocusedDraftModel) -> i32 {
    use super::owner::Owner;
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSBackingStoreType, NSEventModifierFlags, NSWindow, NSWindowStyleMask};
    use objc2_foundation::NSObject;
    let Some((_, origin)) = crate::imk::frontmost::identity() else {
        eprintln!("无法只读确认探针启动前的宿主");
        return 1;
    };
    let anchor = NSRect::new(NSPoint::new(300.0, 700.0), NSSize::ZERO);
    let mut config = qingjian_platform::Config::default();
    config.live_translation.enabled = true;
    Owner::Opening(origin).set("native_open");
    if !super::opening::present_prepared(mtm, None, origin, anchor, &config) {
        eprintln!(
            "focused-owner-v1 从Prohibited切换后未获焦，policy={:?}",
            app.activationPolicy()
        );
        stock_control(mtm, app);
        return 1;
    }
    super::STATE.with(|state| {
        let mut state = state.borrow_mut();
        let runtime = state.as_mut().unwrap();
        runtime.translator = None;
        runtime.model = model;
    });
    let focused =
        super::STATE.with(|state| state.borrow().as_ref().unwrap().window.smoke_contract());
    let mut legacy = crate::candidates::CandidateWindow::new(mtm);
    let frame = crate::candidates::Frame {
        preedit: crate::candidates::Preedit::plain("legacy", 0),
        ..Default::default()
    };
    legacy.show(frame.clone(), anchor);
    legacy.show_translation(frame.clone(), anchor, None);
    let suppressed = legacy.visible_frame().is_none() && Owner::controlled();
    let second: Retained<NSWindow> = unsafe {
        msg_send![mtm.alloc::<NSWindow>(),
        initWithContentRect: NSRect::new(NSPoint::new(100.0, 400.0), NSSize::new(100.0, 60.0)),
        styleMask: NSWindowStyleMask::Titled, backing: NSBackingStoreType::Buffered, defer: false]
    };
    unsafe {
        second.setReleasedWhenClosed(false);
    }
    second.makeKeyAndOrderFront(None);
    let hidden = super::STATE.with(|state| !state.borrow().as_ref().unwrap().window.is_visible());
    let restored =
        app.activationPolicy() == NSApplicationActivationPolicy::Prohibited && !Owner::controlled();
    second.orderOut(None);
    Owner::Opening(origin).set("native_reopen");
    let reopened = super::opening::present_prepared(mtm, None, origin, anchor, &config);
    super::output::request(false);
    for _ in 0..3 {
        super::output::attempt();
    }
    let client = NSObject::new();
    let y = local_event(
        "y",
        16,
        NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
    );
    let enter = local_event("\r", 36, NSEventModifierFlags::empty());
    let pending_y = super::intercept_host(&y, crate::imk::TextClient::new(&client)) == Some(true)
        && matches!(Owner::current(), Owner::Pending(_));
    let pending_enter = super::intercept_host(&enter, crate::imk::TextClient::new(&client))
        == Some(true)
        && matches!(Owner::current(), Owner::Pending(_));
    let letter = local_event("a", 0, NSEventModifierFlags::empty());
    let ordinary = super::intercept_host(&letter, crate::imk::TextClient::new(&client)).is_none()
        && !Owner::controlled();
    let preserved = super::STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .unwrap()
            .model
            .original_output()
            .is_some()
    });
    legacy.show(frame, anchor);
    let legacy_restored = legacy.visible_frame().is_some();
    legacy.hide();
    super::discard();
    println!(
        "focused-owner-v1 原生回放：production_focus={focused}，旧双窗抑制={suppressed}，失焦隐藏={hidden}，policy恢复={restored}，重开={reopened}，pending_Y={pending_y}，pending_Enter={pending_enter}，宿主文字撤确认={ordinary}，草稿保留={preserved}，旧候选恢复={legacy_restored}"
    );
    if focused
        && suppressed
        && hidden
        && restored
        && reopened
        && pending_y
        && pending_enter
        && ordinary
        && preserved
        && legacy_restored
    {
        0
    } else {
        1
    }
}

fn local_event(
    text: &str,
    code: u16,
    flags: objc2_app_kit::NSEventModifierFlags,
) -> objc2::rc::Retained<objc2_app_kit::NSEvent> {
    let text = objc2_foundation::NSString::from_str(text);
    objc2_app_kit::NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        objc2_app_kit::NSEventType::KeyDown, NSPoint::ZERO, flags, 0.0, 0, None, &text, &text, false, code).expect("原生本地事件")
}

fn stock_control(mtm: MainThreadMarker, app: &NSApplication) {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
    let previous = app.activationPolicy();
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let stock: Retained<NSWindow> = unsafe {
        msg_send![mtm.alloc::<NSWindow>(), initWithContentRect: NSRect::new(NSPoint::new(300.0, 500.0), NSSize::new(200.0, 100.0)), styleMask: NSWindowStyleMask::Titled, backing: NSBackingStoreType::Buffered, defer: false]
    };
    unsafe {
        stock.setReleasedWhenClosed(false);
    }
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    stock.makeKeyAndOrderFront(None);
    objc2_foundation::NSRunLoop::mainRunLoop()
        .runUntilDate(&objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(0.1));
    stock.makeKeyWindow();
    println!(
        "focused-owner-v1 stock NSWindow 正控制：key={} active={} running={} policy={:?}",
        stock.isKeyWindow(),
        app.isActive(),
        app.isRunning(),
        app.activationPolicy()
    );
    stock.orderOut(None);
    app.setActivationPolicy(previous);
}
