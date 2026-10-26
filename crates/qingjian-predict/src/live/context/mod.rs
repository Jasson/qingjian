//! 编辑器上下文的能力证据与可信度核验，不依赖平台输入框实现。

mod confidence;
mod decision;
mod guard;
mod policy;
mod range;
mod snapshot;

pub use confidence::ContextConfidence;
pub use decision::ContextDecision;
pub use guard::ContextGuard;
pub use policy::ContextPolicy;
pub use range::ContextRange;
pub use snapshot::ContextSnapshot;
