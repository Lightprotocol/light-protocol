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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default_values() {
        let config = RetryConfig::default();
        assert_eq!(config.num_retries, 10);
        assert_eq!(config.delay_ms, 400);
        assert_eq!(config.max_delay_ms, 8000);
    }

    #[test]
    fn test_indexer_rpc_config_new_sets_slot() {
        let slot = 42u64;
        let config = IndexerRpcConfig::new(slot);
        assert_eq!(config.slot, slot);
        assert_eq!(config.retry_config, RetryConfig::default());
    }
}
