#![deny(missing_docs, unsafe_code)]
//! # Redis
//!
//! Redis 相关

use std::fmt::Display;

/// 配置
pub mod config;
mod error;
/// 用户长链接会话存储
pub mod session;

pub use error::*;

/// 构造存储登录信息时使用的键
pub fn get_sign_in_key<D: Display>(user_id: D) -> String {
    format!("user:sign_in:{}", user_id)
}

#[cfg(test)]
mod tests {
    use super::get_sign_in_key;
    use uuid::Uuid;

    #[test]
    fn simple() {
        let uuid = Uuid::new_v4().simple();
        assert_eq!(get_sign_in_key(&uuid), get_sign_in_key(&uuid));
    }
}
