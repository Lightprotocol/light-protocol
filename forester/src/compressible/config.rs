use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleConfig {
    /// Enable compressible mode
    pub enabled: bool,
    /// WebSocket URL for account subscriptions
    pub ws_url: String,
    /// Batch size for compression operations
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_batch_size() -> usize {
    10
}

impl CompressibleConfig {
    pub fn new(enabled: bool, ws_url: String) -> Self {
        Self {
            enabled,
            ws_url,
            batch_size: default_batch_size(),
        }
    }
}
