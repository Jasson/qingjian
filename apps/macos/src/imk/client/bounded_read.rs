//! 有界精确读取的返回值；正文仅用于核验，日志只能记录 reader 与长度。

pub struct BoundedRead {
    pub text: Option<String>,

    pub reader: &'static str,

    pub attempted: bool,
}
