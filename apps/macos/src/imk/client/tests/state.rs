//! 可读与局部客户端的测试元数据，所有文本仅留在测试进程。

use objc2_foundation::NSRange;
use std::cell::{Cell, RefCell};

pub(super) struct ContextState {
    pub text: Option<String>,

    pub length: usize,

    pub selected: Cell<NSRange>,

    pub marked: Cell<NSRange>,

    pub requests: RefCell<Vec<NSRange>>,

    pub writes: RefCell<Vec<(String, NSRange)>>,

    pub document_access: Cell<bool>,

    pub support_queries: RefCell<Vec<u32>>,

    pub legacy_readable: Cell<bool>,

    pub maximum_read_units: Cell<usize>,

    pub adjust_actual: Cell<bool>,
}
