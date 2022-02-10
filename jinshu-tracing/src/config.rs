use byte_unit::Byte;
use rolling_file::RollingConditionBasic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;

/// 跟踪配置
#[derive(Debug, Deserialize, Serialize)]
pub struct TracingConfig {
    /// 跟踪级别
    #[serde(with = "serde_level")]
    pub level: tracing::Level,
    /// 文件保存地址
    pub path: PathBuf,
    /// 文件大小
    pub trigger_size: Byte,
    /// 文件个数
    pub archived_count: usize,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            level: tracing::Level::INFO,
            path: PathBuf::from("./log"),
            trigger_size: Byte::from_bytes(1024 * 1024),
            archived_count: 32,
        }
    }
}

mod serde_level {
    use serde::{Deserialize, Deserializer, Serializer};

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(level: &tracing::Level, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", level);
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<tracing::Level, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// 跟踪句柄
#[must_use]
pub struct TracingHandle {
    _guard: WorkerGuard,
}

impl TracingConfig {
    /// 初始化，确保全局执行一次
    pub fn init(&self, service: &str) -> crate::Result<TracingHandle> {
        let registry = tracing_subscriber::Registry::default();

        std::fs::create_dir_all(&self.path)?;

        let file_appender = rolling_file::BasicRollingFileAppender::new(
            &self.path.join(format!("{}.log", service)),
            RollingConditionBasic::new().max_size(self.trigger_size.get_bytes()),
            self.archived_count,
        )?;
        let (nonblocking, _guard) = tracing_appender::non_blocking(file_appender);
        let file = tracing_subscriber::fmt::layer()
            .with_writer(nonblocking.with_max_level(self.level))
            .json();

        let stdout = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout.with_max_level(self.level));

        // let tracer = opentelemetry_jaeger::new_pipeline()
        //     .with_service_name(service)
        //     .install_simple()?; //opentelemetry::runtime::Tokio
        //                         // let optl = tracing_opentelemetry::layer().with_tracer(tracer);

        let registry = registry.with(stdout).with(file); // .with(optl)

        tracing::subscriber::set_global_default(registry)?;

        Ok(TracingHandle { _guard })
    }
}
