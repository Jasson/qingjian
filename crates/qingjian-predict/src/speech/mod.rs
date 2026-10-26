//! 已确认句子的独立翻译朗读队列，不读取候选或应用上下文。
mod config;
mod coordinator;
mod planner;
mod worker;
pub use config::SpeechConfig;
pub use coordinator::SpeechCoordinator;

#[cfg(test)]
mod tests;
