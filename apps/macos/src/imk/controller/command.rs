//! 命令键（回车、退格、方向键、Tab、Esc 等）的处理。

use super::QingjianInputController;
use crate::host;
use crate::imk::TextClient;
use objc2::{runtime::Sel, sel};

impl QingjianInputController {
    /// 组句期间所有编辑动作都由我们接管；不认识的一律吞掉，否则应用会动光标、丢 marked text。
    pub(super) fn handle_command(
        &self,
        selector: Sel,
        client: TextClient<'_>,
        unmodified: bool,
    ) -> bool {
        tracing::debug!(selector = %selector, "didCommandBySelector");
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if client.stages_output() {
            if composing && selector == sel!(insertNewline:) {
                self.commit_highlighted(client);
                return true;
            }
            if !composing {
                if selector == sel!(deleteBackward:) {
                    host::with(|h| h.backspace_live_text());
                    self.render(client);
                } else if selector == sel!(insertNewline:) {
                    host::with(|h| {
                        h.live_action_notice("原文尚在青简草稿中，请点击输出译文或输出原文")
                    });
                } else if selector == sel!(cancelOperation:) || selector == sel!(complete:) {
                    host::with(|h| {
                        h.live_action_notice("草稿仍保留；请使用已配置的取消快捷键或取消按钮")
                    });
                } else {
                    host::with(|h| {
                        h.live_action_notice("草稿目前只支持末尾退格；请输出或取消后再操作终端")
                    });
                }
                return true;
            }
        }
        if !composing {
            if unmodified
                && (selector == sel!(cancelOperation:) || selector == sel!(complete:))
                && host::with(|h| {
                    if !h.live.active()
                        || h.draft_active()
                        || h.translation.is_some()
                        || h.engine.symbol_panel().is_some()
                        || h.live.window.visible_frame().is_none()
                    {
                        return false;
                    }
                    h.reset_live_translation("escape");
                    true
                })
                .unwrap_or(false)
            {
                return true;
            }
            // 删的是应用里的文字：刚上屏的词被整个删掉是「选错了」的信号，Engine 记着；
            // 按词 / 按行删的数不清删了几个字，撤销的账就不记了
            if selector == sel!(deleteBackward:) {
                host::with(|h| {
                    h.engine.note_backspace();
                    h.backspace_live_text();
                });
            } else if selector == sel!(deleteWordBackward:)
                || selector == sel!(deleteToBeginningOfLine:)
            {
                host::with(|h| {
                    h.engine.break_chain();
                    h.pause_live_translation("application_command");
                });
            } else if selector == sel!(insertNewline:) {
                // 回车交给应用：文本流里是一个段落边界
                host::with(|h| {
                    h.engine.note_passthrough('\n');
                    if host::context_policy(None, h.engine.application())
                        == qingjian_predict::live::context::ContextPolicy::EventOnly
                    {
                        h.pause_live_translation("application_submit");
                    } else {
                        h.commit_live_text("\n");
                    }
                });
            } else {
                host::with(|h| h.pause_live_translation("application_command"));
            }
            return false;
        }
        if selector == sel!(deleteBackward:) {
            host::with(|h| h.engine.backspace());
            self.refresh(client);
        } else if selector == sel!(deleteWordBackward:) {
            // ⌥⌫：删光标前一个音节
            host::with(|h| h.engine.delete_syllable_backward());
            self.refresh(client);
        } else if selector == sel!(deleteToBeginningOfLine:) {
            // ⌘⌫：删光标前的全部拼音
            host::with(|h| h.engine.delete_to_start());
            self.refresh(client);
        } else if selector != sel!(cancelOperation:)
            && selector != sel!(complete:)
            && self.restore_bare_question(client)
        {
            // 只有一个 ? 时按了回车：回车就是「把这个 ? 上屏」，吞掉，否则聊天框里会连消息一起发出去；
            // 方向键等其他键还原后交给应用
            return selector == sel!(insertNewline:);
        } else if selector == sel!(insertNewline:) {
            self.commit_raw(client);
        } else if selector == sel!(cancelOperation:) || selector == sel!(complete:) {
            // TextEdit 等应用把 Esc 绑成 complete:（自动补全），也当作取消。矩阵展开着时第一下 Esc 只收回单行
            if host::with(|h| h.session.collapse()).unwrap_or(false) {
                self.render(client);
                return true;
            }
            host::with(|h| {
                h.engine.clear();
                h.cancel_prediction();
            });
            self.refresh(client);
        } else if selector == sel!(insertTab:) {
            // 英文模式 Tab 选中高亮的词；中文模式有整句补全时接受它，否则翻页
            if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                self.commit_highlighted(client);
            } else if !self.accept_sentence(client) {
                self.turn_page(1, client);
            }
        } else if selector == sel!(deleteForward:) {
            host::with(|h| h.engine.delete_forward());
            self.refresh(client);
        } else if selector == sel!(moveDown:) {
            self.move_highlight(1, client);
        } else if selector == sel!(moveUp:) {
            self.move_highlight(-1, client);
        } else if selector == sel!(moveLeft:) {
            // 横排矩阵开着：左右键在候选之间移动高亮（单行、展开后都是）；否则照旧移动拼音光标
            if !self.move_cells(-1, client) {
                host::with(|h| h.engine.move_cursor_left());
                self.refresh(client);
            }
        } else if selector == sel!(moveRight:) {
            if !self.move_cells(1, client) {
                host::with(|h| h.engine.move_cursor_right());
                self.refresh(client);
            }
        } else if selector == sel!(moveWordLeft:) {
            // ⌥←：光标往左跳一个音节
            host::with(|h| h.engine.move_cursor_syllable_left());
            self.refresh(client);
        } else if selector == sel!(moveWordRight:) {
            // ⌥→：光标往右跳一个音节
            host::with(|h| h.engine.move_cursor_syllable_right());
            self.refresh(client);
        } else if selector == sel!(moveToBeginningOfLine:) || selector == sel!(moveToLeftEndOfLine:)
        {
            host::with(|h| h.engine.move_cursor_home());
            self.refresh(client);
        } else if selector == sel!(moveToEndOfLine:) || selector == sel!(moveToRightEndOfLine:) {
            host::with(|h| h.engine.move_cursor_end());
            self.refresh(client);
        } else if selector == sel!(pageDown:) || selector == sel!(scrollPageDown:) {
            self.turn_page(1, client);
        } else if selector == sel!(pageUp:)
            || selector == sel!(scrollPageUp:)
            || selector == sel!(insertBacktab:)
        {
            self.turn_page(-1, client);
        }
        true
    }
}
