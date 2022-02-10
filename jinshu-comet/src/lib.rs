#![deny(missing_docs, unsafe_code)]
//! # Comet
//!
//! 长链接保持，提供消息推送服务。
//!

/// 配置
pub mod config;

/// 长链接保持服务实现
pub mod comet;

/// 连接管理
pub mod connection;
