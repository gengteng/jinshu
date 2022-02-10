#![deny(missing_docs, unsafe_code)]
//! # Gateway
//!
//! 系统接入模块

mod api;
/// 配置
pub mod config;

pub use api::*;
