//! Carbon 调用的重入门禁；状态失效仍立即发生，native 清理延后到外层返回。

pub(super) struct OperationGuard;

impl OperationGuard {
    pub fn enter() -> Option<Self> {
        (!super::BUSY.with(|busy| busy.replace(true))).then(|| Self)
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        super::BUSY.with(|busy| busy.set(false));
    }
}

#[cfg(test)]
mod tests {
    use super::OperationGuard;

    #[test]
    fn nested_native_operation_cannot_release_the_outer_reentry_guard() {
        let outer = OperationGuard::enter().unwrap();
        assert!(OperationGuard::enter().is_none());
        assert!(super::super::BUSY.with(|busy| busy.get()));
        drop(outer);
        assert!(!super::super::BUSY.with(|busy| busy.get()));
    }
}
