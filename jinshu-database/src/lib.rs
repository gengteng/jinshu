#![deny(missing_docs, unsafe_code)]
//! 数据库相关，包括配置及模型定义
//!

/// 配置
pub mod config;
mod model;

pub use model::*;
