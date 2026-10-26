//! 独立编辑器会话隔离、待填入消费与物理按键周期的纯状态回放。

mod closing;
mod held_keys;
mod native_route;
mod owner;
mod paging;
mod pending;
mod scope;
mod session;

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

fn engine() -> Engine {
    Engine::new(
        Dictionary::parse(
            "你好\tni hao\t10000\n你\tni\t9000\n好\thao\t8000\n世界\tshi jie\t10000\n",
        )
        .unwrap(),
    )
}
