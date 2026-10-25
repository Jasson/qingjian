//! 本机模型实时翻译：维护句子预览，以可取消的后台请求更新整句译文。

mod archive;
mod candidate;
mod config;
pub mod context;
mod draft;
mod editor;
mod replacement;
mod sentence;
mod translation;
mod translator;

pub use archive::{DraftArchive, SavedDraft};
pub use config::LiveTranslationConfig;
pub use draft::{TerminalDraft, normalize_terminal_output};
pub use editor::FocusedDraftModel;
pub use replacement::ReplacementPlan;
pub use sentence::SentenceTracker;
pub use translation::Translation;
pub use translator::LiveTranslator;

#[cfg(test)]
mod replacement_tests;

pub use candidate::{MAX_REPLACEMENT_UNITS, ReplacementCandidate};
