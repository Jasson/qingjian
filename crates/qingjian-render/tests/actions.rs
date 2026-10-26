//! 翻译条按钮的自绘结果、语义点击与禁用状态；正文不冒充候选。

use qingjian_render::{
    Action, ActionTarget, FontLibrary, Frame, HitTarget, Layout, Renderer, Row, Theme,
};

#[test]
fn translation_actions_have_independent_hits_and_disabled_buttons_are_not_clickable() {
    let library = FontLibrary::system("zh-CN").unwrap();
    let mut renderer = Renderer::new(library);
    let full_translation =
        "这是很长的译文，显示区域可以截断但复制来源始终是完整成功译文。".repeat(20);
    let lines = qingjian_render::translation_lines(&full_translation, 48, 4);
    assert!(lines.concat().len() < full_translation.len());
    let frame = Frame {
        rows: lines
            .into_iter()
            .map(|text| {
                let mut row = Row::plain(0, text);
                row.index.clear();
                row
            })
            .collect(),
        actions: vec![
            Action {
                target: ActionTarget::CopyTranslation,
                enabled: true,
            },
            Action {
                target: ActionTarget::ReplaceOriginal,
                enabled: false,
            },
        ],
        ..Frame::default()
    };
    let rendered = renderer
        .render(&frame, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(rendered.hits.len(), 1);
    let copy = &rendered.hits[0];
    assert_eq!(
        copy.target,
        HitTarget::Action(ActionTarget::CopyTranslation)
    );
    assert!(copy.width > 30.0 && copy.height > 20.0);
    assert_eq!(
        rendered.hit_at(
            (copy.x + copy.width / 2.0) / 2.0,
            (copy.y + copy.height / 2.0) / 2.0
        ),
        Some(copy.target)
    );
    assert!(rendered.hits.iter().all(|hit| !matches!(
        hit.target,
        HitTarget::Candidate(_) | HitTarget::Translation { .. }
    )));
    rendered
        .pixmap
        .save_png("/tmp/qingjian-live-actions-disabled.png")
        .unwrap();

    let mut enabled = frame.clone();
    enabled.actions[1].enabled = true;
    renderer.forget();
    let rendered = renderer
        .render(&enabled, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(rendered.hits.len(), 2);
    assert!(
        rendered
            .hits
            .iter()
            .any(|hit| hit.target == HitTarget::Action(ActionTarget::ReplaceOriginal))
    );
    rendered
        .pixmap
        .save_png("/tmp/qingjian-live-actions-enabled.png")
        .unwrap();
}

#[test]
fn draft_toolbar_keeps_four_separate_actions_inside_the_image() {
    let mut renderer = Renderer::new(FontLibrary::system("zh-CN").unwrap());
    let targets = [
        ActionTarget::CopyTranslation,
        ActionTarget::InsertTranslation,
        ActionTarget::InsertOriginal,
        ActionTarget::CancelDraft,
    ];
    let frame = Frame {
        rows: ["原文：你好", "译文：Hello"]
            .into_iter()
            .map(|text| {
                let mut row = Row::plain(0, text);
                row.index.clear();
                row
            })
            .collect(),
        actions: targets
            .iter()
            .map(|target| Action {
                target: *target,
                enabled: true,
            })
            .collect(),
        ..Frame::default()
    };
    let output = renderer
        .render(&frame, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(output.hits.len(), targets.len());
    for (index, hit) in output.hits.iter().enumerate() {
        assert_eq!(hit.target, HitTarget::Action(targets[index]));
        assert!(hit.x >= 0.0 && hit.y >= 0.0);
        assert!(hit.x + hit.width <= output.pixmap.width() as f32);
        assert!(hit.y + hit.height <= output.pixmap.height() as f32);
        if index > 0 {
            assert!(output.hits[index - 1].x + output.hits[index - 1].width <= hit.x);
        }
        assert_eq!(
            output.hit_at(
                (hit.x + hit.width / 2.0) / 2.0,
                (hit.y + hit.height / 2.0) / 2.0
            ),
            Some(hit.target)
        );
    }
    output
        .pixmap
        .save_png("/tmp/qingjian-terminal-draft-toolbar.png")
        .unwrap();
    let mut pending = frame;
    pending.actions[1].enabled = false;
    renderer.forget();
    let output = renderer
        .render(&pending, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(output.hits.len(), 3);
    assert!(
        !output
            .hits
            .iter()
            .any(|hit| hit.target == HitTarget::Action(ActionTarget::InsertTranslation))
    );
}

#[test]
fn restore_draft_has_a_distinct_semantic_hit_and_disabled_restore_cannot_overwrite_active_input() {
    assert_eq!(ActionTarget::RestoreDraft.label(), "恢复草稿");
    let mut renderer = Renderer::new(FontLibrary::system("zh-CN").unwrap());
    let mut frame = Frame {
        rows: vec![Row::plain(0, "当前应用有草稿可恢复")],
        actions: vec![Action {
            target: ActionTarget::RestoreDraft,
            enabled: true,
        }],
        ..Frame::default()
    };
    let output = renderer
        .render(&frame, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(output.hits.len(), 1);
    let hit = &output.hits[0];
    assert_eq!(hit.target, HitTarget::Action(ActionTarget::RestoreDraft));
    assert_eq!(
        output.hit_at(
            (hit.x + hit.width / 2.0) / 2.0,
            (hit.y + hit.height / 2.0) / 2.0
        ),
        Some(hit.target)
    );
    frame.actions[0].enabled = false;
    renderer.forget();
    let disabled = renderer
        .render(&frame, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert!(disabled.hits.is_empty());
}

#[test]
fn opening_the_focused_draft_is_a_distinct_action_with_an_independent_hit() {
    assert_eq!(ActionTarget::OpenTranslationDraft.label(), "打开草稿窗");
    let mut renderer = Renderer::new(FontLibrary::system("zh-CN").unwrap());
    let frame = Frame {
        actions: vec![Action {
            target: ActionTarget::OpenTranslationDraft,
            enabled: true,
        }],
        ..Frame::default()
    };
    let output = renderer
        .render(&frame, Layout::Vertical, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(output.hits.len(), 1);
    assert_eq!(
        output.hits[0].target,
        HitTarget::Action(ActionTarget::OpenTranslationDraft)
    );
}
