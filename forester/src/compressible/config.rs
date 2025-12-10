use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleConfig {
    /// WebSocket URL for account subscriptions
    pub ws_url: String,
    /// Batch size for compression operations
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Maximum number of concurrent compression batches
    #[serde(default = "default_max_concurrent_batches")]
    pub max_concurrent_batches: usize,
}

fn default_batch_size() -> usize {
    5
}

fn default_max_concurrent_batches() -> usize {
    10
}

impl CompressibleConfig {
    pub fn new(ws_url: String) -> Self {
        Self {
            ws_url,
            batch_size: default_batch_size(),
            max_concurrent_batches: default_max_concurrent_batches(),
        }
    }
}
