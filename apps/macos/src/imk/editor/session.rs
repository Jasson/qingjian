//! 短暂借用共享 Engine 的独立输入会话；异常退出也恢复普通 IMK 会话与隐私开关。

use qingjian_core::{Engine, EngineSession};
use std::ops::{Deref, DerefMut};

pub(super) struct SessionLease<'a> {
    engine: &'a mut Engine,

    parked: &'a mut EngineSession,

    private: bool,
}

impl<'a> SessionLease<'a> {
    pub fn new(engine: &'a mut Engine, session: &'a mut EngineSession) -> Self {
        let private = engine.is_private();
        engine.swap_session(session);
        engine.set_private(true);
        Self {
            engine,
            parked: session,
            private,
        }
    }
}

impl Deref for SessionLease<'_> {
    type Target = Engine;
    fn deref(&self) -> &Engine {
        self.engine
    }
}

impl DerefMut for SessionLease<'_> {
    fn deref_mut(&mut self) -> &mut Engine {
        self.engine
    }
}

impl Drop for SessionLease<'_> {
    fn drop(&mut self) {
        self.engine.swap_session(self.parked);
        self.engine.set_private(self.private);
    }
}
