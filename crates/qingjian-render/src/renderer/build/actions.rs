//! 翻译条底部操作区：自绘按钮与独立语义点击区域。

use bamboo_slip::taffy::{
    AlignItems, Display, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style,
};
use bamboo_slip::{RenderError, TextStyle, Visual};

use super::Builder;
use crate::Action;

impl Builder<'_> {
    pub(super) fn actions(
        &mut self,
        content: NodeId,
        actions: &[Action],
    ) -> Result<NodeId, RenderError> {
        let spec = &self.theme.file().status;
        // 原候选背景的投影不能压到新增操作行；外层统一承载背景。
        self.scene.set_effects(content, Vec::new());
        let font = self.theme.font_ref(&spec.font);
        let scale = self.scale;
        let padding = Rect {
            left: LengthPercentage::length(10.0 * scale),
            right: LengthPercentage::length(10.0 * scale),
            top: LengthPercentage::length(7.0 * scale),
            bottom: LengthPercentage::length(7.0 * scale),
        };
        let mut buttons = Vec::with_capacity(actions.len());
        for action in actions {
            let color = self.theme.color(if action.enabled {
                &spec.emphasized
            } else {
                &spec.normal
            });
            let label = self.scene.node(
                Style::default(),
                Visual::Text {
                    text: action.target.label().into(),
                    style: TextStyle::new(
                        font.scaled(scale),
                        font.size,
                        color,
                        self.theme.text_gamma(),
                    ),
                },
                &[],
            )?;
            let button = self.scene.node(
                Style {
                    padding,
                    ..Style::default()
                },
                Visual::Group,
                &[label],
            )?;
            if action.enabled {
                self.action_nodes.push((action.target, button));
            } else {
                self.scene.set_opacity(button, 0.4);
            }
            buttons.push(button);
        }
        let toolbar = self.scene.node(
            Style {
                display: Display::Flex,
                align_items: Some(AlignItems::CENTER),
                gap: Size {
                    width: LengthPercentage::length(4.0 * scale),
                    height: LengthPercentage::length(0.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &buttons,
        )?;
        self.scene.node(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: Some(AlignItems::STRETCH),
                ..Style::default()
            },
            Visual::solid(self.theme.color(&spec.background), spec.radius * scale),
            &[content, toolbar],
        )
    }
}
