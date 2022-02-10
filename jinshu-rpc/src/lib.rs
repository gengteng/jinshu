#![deny(missing_docs)]
//! # Rpc
//!
//! RPC 相关，包括注册中心、服务端各模块间 RPC 调用的报文定义

/// 配置
pub mod config;
mod protocol;
/// 注册中心
pub mod registry;
mod status;

pub use status::*;

#[allow(missing_docs)]
pub mod domain {
    pub mod message {
        tonic::include_proto!("domain.message");
    }
}

#[allow(missing_docs)]
pub mod comet {
    tonic::include_proto!("comet");
}

#[allow(missing_docs)]
pub mod receiver {
    tonic::include_proto!("receiver");
}

#[allow(missing_docs)]
pub mod authorizer {
    tonic::include_proto!("authorizer");
}

#[allow(missing_docs)]
#[cfg(test)]
pub mod test {
    tonic::include_proto!("test");
}
