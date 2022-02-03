use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PusherConfig {
    pub comet_name: String,
}
