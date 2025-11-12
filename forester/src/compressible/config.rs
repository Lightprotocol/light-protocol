use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleConfig {
    /// Enable compressible mode
    pub enabled: bool,
    /// WebSocket URL for account subscriptions
    pub ws_url: String,
}

impl CompressibleConfig {
    pub fn new(enabled: bool, ws_url: String) -> Self {
        Self { enabled, ws_url }
    }
}
