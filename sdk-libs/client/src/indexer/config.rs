#[derive(Debug, Clone, PartialEq, Default)]
pub struct IndexerRpcConfig {
    pub slot: u64,
    pub retry_config: RetryConfig,
}
impl IndexerRpcConfig {
    pub fn new(slot: u64) -> Self {
        Self {
            slot,
            retry_config: RetryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetryConfig {
    pub num_retries: u32,
    pub delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            num_retries: 10,
            delay_ms: 400,
            max_delay_ms: 8000,
        }
    }
}
