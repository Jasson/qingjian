//! ObjC 透传替身的计数与对象身份，仅用于测试。

use objc2::runtime::AnyObject;
use std::cell::Cell;

#[derive(Default)]
pub(super) struct MockState {
    pub counts: Cell<[usize; 4]>,

    pub sender: Cell<*const AnyObject>,

    pub result: Cell<*mut AnyObject>,
}
