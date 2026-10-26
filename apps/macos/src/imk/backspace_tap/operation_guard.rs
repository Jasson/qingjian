//! native 生命周期的重入门禁；状态撤销不受门禁阻挡。

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
    fn nested_startup_cannot_release_outer_guard_even_on_unwind() {
        let result = std::panic::catch_unwind(|| {
            let _outer = OperationGuard::enter().unwrap();
            assert!(OperationGuard::enter().is_none());
            assert!(super::super::BUSY.with(|busy| busy.get()));
            panic!("测试重入启动的 unwind 清理");
        });
        assert!(result.is_err());
        assert!(!super::super::BUSY.with(|busy| busy.get()));
        assert!(OperationGuard::enter().is_some());
    }
}
