use serde::{Deserialize, Serialize};

/// Pusher 的配置
#[derive(Debug, Deserialize, Serialize)]
pub struct PusherConfig {
    /// Comet 服务名
    pub comet_name: String,
}

impl Default for PusherConfig {
    fn default() -> Self {
        Self {
            comet_name: "comet".into(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::PusherConfig;

    #[test]
    fn default() {
        PusherConfig::default();
    }
}
