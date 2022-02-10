#![deny(missing_docs, unsafe_code)]
//! # Protocol
//!
//! 协议模块，包括前后端共用的协议、接口等
//!

mod error;
mod protocol;

pub use error::*;
pub use protocol::*;
