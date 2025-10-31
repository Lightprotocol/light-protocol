use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum Commands {
    Start(StartArgs),
    Status(StatusArgs),
    Health(HealthArgs),
}

#[derive(Parser, Clone, Debug)]
pub struct StartArgs {
    #[arg(long, env = "FORESTER_RPC_URL")]
    pub rpc_url: Option<String>,

    #[arg(long, env = "FORESTER_PUSH_GATEWAY_URL")]
    pub push_gateway_url: Option<String>,

    #[arg(long, env = "FORESTER_PAGERDUTY_ROUTING_KEY")]
    pub pagerduty_routing_key: Option<String>,

    #[arg(long, env = "FORESTER_WS_RPC_URL")]
    pub ws_rpc_url: Option<String>,

    #[arg(long, env = "FORESTER_INDEXER_URL")]
    pub indexer_url: Option<String>,

    #[arg(long, env = "FORESTER_PROVER_URL")]
    pub prover_url: Option<String>,

    #[arg(
        long,
        env = "FORESTER_PROVER_APPEND_URL",
        help = "Prover URL for append operations. If not specified, uses prover_url"
    )]
    pub prover_append_url: Option<String>,

    #[arg(
        long,
        env = "FORESTER_PROVER_UPDATE_URL",
        help = "Prover URL for update operations. If not specified, uses prover_url"
    )]
    pub prover_update_url: Option<String>,

    #[arg(
        long,
        env = "FORESTER_PROVER_ADDRESS_APPEND_URL",
        help = "Prover URL for address-append operations. If not specified, uses prover_url"
    )]
    pub prover_address_append_url: Option<String>,

    #[arg(long, env = "FORESTER_PROVER_API_KEY")]
    pub prover_api_key: Option<String>,

    #[arg(long, env = "FORESTER_PAYER")]
    pub payer: Option<String>,

    #[arg(long, env = "FORESTER_DERIVATION_PUBKEY")]
    pub derivation: Option<String>,

    #[arg(long, env = "FORESTER_PHOTON_API_KEY")]
    pub photon_api_key: Option<String>,

    #[arg(long, env = "FORESTER_PHOTON_GRPC_URL")]
    pub photon_grpc_url: Option<String>,

    #[arg(long, env = "FORESTER_INDEXER_BATCH_SIZE", default_value = "50")]
    pub indexer_batch_size: usize,

    #[arg(
        long,
        env = "FORESTER_INDEXER_MAX_CONCURRENT_BATCHES",
        default_value = "10"
    )]
    pub indexer_max_concurrent_batches: usize,

    #[arg(long, env = "FORESTER_LEGACY_XS_PER_TX", default_value = "1")]
    pub legacy_ixs_per_tx: usize,
    #[arg(
        long,
        env = "FORESTER_TRANSACTION_MAX_CONCURRENT_BATCHES",
        default_value = "20"
    )]
    pub transaction_max_concurrent_batches: usize,

    #[arg(
        long,
        env = "FORESTER_MAX_CONCURRENT_SENDS",
        default_value = "50",
        help = "Maximum number of concurrent transaction sends per batch"
    )]
    pub max_concurrent_sends: usize,

    #[arg(
        long,
        env = "FORESTER_TX_CACHE_TTL_SECONDS",
        default_value = "180",
        help = "TTL in seconds to prevent duplicate transaction processing"
    )]
    pub tx_cache_ttl_seconds: u64,

    #[arg(
        long,
        env = "FORESTER_OPS_CACHE_TTL_SECONDS",
        default_value = "180",
        help = "TTL in seconds to prevent duplicate batch operations processing"
    )]
    pub ops_cache_ttl_seconds: u64,

    #[arg(long, env = "FORESTER_CU_LIMIT", default_value = "1000000")]
    pub cu_limit: u32,

    #[arg(long, env = "FORESTER_ENABLE_PRIORITY_FEES", default_value = "false")]
    pub enable_priority_fees: bool,

    #[arg(long, env = "FORESTER_RPC_POOL_SIZE", default_value = "100")]
    pub rpc_pool_size: u32,

    #[arg(
        long,
        env = "FORESTER_RPC_POOL_CONNECTION_TIMEOUT_SECS",
        default_value = "15"
    )]
    pub rpc_pool_connection_timeout_secs: u64,

    #[arg(
        long,
        env = "FORESTER_RPC_POOL_IDLE_TIMEOUT_SECS",
        default_value = "300"
    )]
    pub rpc_pool_idle_timeout_secs: u64,

    #[arg(long, env = "FORESTER_RPC_POOL_MAX_RETRIES", default_value = "100")]
    pub rpc_pool_max_retries: u32,

    #[arg(
        long,
        env = "FORESTER_RPC_POOL_INITIAL_RETRY_DELAY_MS",
        default_value = "1000"
    )]
    pub rpc_pool_initial_retry_delay_ms: u64,

    #[arg(
        long,
        env = "FORESTER_RPC_POOL_MAX_RETRY_DELAY_MS",
        default_value = "16000"
    )]
    pub rpc_pool_max_retry_delay_ms: u64,

    #[arg(
        long,
        env = "FORESTER_SLOT_UPDATE_INTERVAL_SECONDS",
        default_value = "10"
    )]
    pub slot_update_interval_seconds: u64,

    #[arg(
        long,
        env = "FORESTER_TREE_DISCOVERY_INTERVAL_SECONDS",
        default_value = "5"
    )]
    pub tree_discovery_interval_seconds: u64,

    #[arg(long, env = "FORESTER_MAX_RETRIES", default_value = "3")]
    pub max_retries: u32,

    #[arg(long, env = "FORESTER_RETRY_DELAY", default_value = "1000")]
    pub retry_delay: u64,

    #[arg(long, env = "FORESTER_RETRY_TIMEOUT", default_value = "30000")]
    pub retry_timeout: u64,

    #[arg(long, env = "FORESTER_STATE_QUEUE_START_INDEX", default_value = "0")]
    pub state_queue_start_index: u16,

    #[arg(
        long,
        env = "FORESTER_STATE_PROCESSING_LENGTH",
        default_value = "28807"
    )]
    pub state_queue_processing_length: u16,

    #[arg(long, env = "FORESTER_ADDRESS_QUEUE_START_INDEX", default_value = "0")]
    pub address_queue_start_index: u16,

    #[arg(
        long,
        env = "FORESTER_ADDRESS_PROCESSING_LENGTH",
        default_value = "28807"
    )]
    pub address_queue_processing_length: u16,

    #[arg(long, env = "FORESTER_RPC_RATE_LIMIT")]
    pub rpc_rate_limit: Option<u32>,

    #[arg(long, env = "FORESTER_PHOTON_RATE_LIMIT")]
    pub photon_rate_limit: Option<u32>,

    #[arg(long, env = "FORESTER_SEND_TRANSACTION_RATE_LIMIT")]
    pub send_tx_rate_limit: Option<u32>,

    #[arg(
           long,
           env = "FORESTER_PROCESSOR_MODE",
           default_value_t = ProcessorMode::All,
           help = "Processor mode: v1 (process only v1 trees), v2 (process only v2 trees), all (process all trees)"
       )]
    pub processor_mode: ProcessorMode,

    #[arg(
        long,
        env = "FORESTER_TREE_ID",
        help = "Process only the specified tree (Pubkey). If specified, forester will process only this tree and ignore all others"
    )]
    pub tree_id: Option<String>,
}

#[derive(Parser, Clone, Debug)]
pub struct StatusArgs {
    #[arg(long, env = "FORESTER_RPC_URL")]
    pub rpc_url: String,

    #[arg(long, env = "FORESTER_PUSH_GATEWAY_URL")]
    pub push_gateway_url: Option<String>,
    #[arg(long, env = "FORESTER_PAGERDUTY_ROUTING_KEY")]
    pub pagerduty_routing_key: Option<String>,
    /// Select to run compressed token program tests.
    #[clap(long)]
    pub full: bool,
    #[clap(long)]
    pub protocol_config: bool,
    #[clap(long, default_value_t = true)]
    pub queue: bool,
}

impl StartArgs {
    pub fn enable_metrics(&self) -> bool {
        self.push_gateway_url.is_some()
    }
}

impl StatusArgs {
    pub fn enable_metrics(&self) -> bool {
        self.push_gateway_url.is_some()
    }
}

#[derive(Parser, Clone, Debug)]
pub struct HealthArgs {
    #[arg(long, help = "Check wallet balance")]
    pub check_balance: bool,

    #[arg(long, help = "Check forester registration for current epoch")]
    pub check_registration: bool,

    #[arg(long, env = "FORESTER_RPC_URL")]
    pub rpc_url: Option<String>,

    #[arg(long, env = "FORESTER_PAYER")]
    pub payer: Option<String>,

    #[arg(long, env = "FORESTER_DERIVATION_PUBKEY")]
    pub derivation: Option<String>,

    #[arg(
        long,
        help = "Minimum balance threshold in SOL",
        default_value = "0.01"
    )]
    pub min_balance: f64,

    #[arg(
        long,
        help = "Exit with non-zero code on failure",
        default_value = "true"
    )]
    pub exit_on_failure: bool,

    #[arg(long, help = "Output format: text or json", default_value = "text")]
    pub output: String,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProcessorMode {
    #[clap(name = "v1")]
    V1,
    #[clap(name = "v2")]
    V2,
    #[clap(name = "all")]
    #[default]
    All,
}

impl std::fmt::Display for ProcessorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessorMode::V1 => write!(f, "v1"),
            ProcessorMode::V2 => write!(f, "v2"),
            ProcessorMode::All => write!(f, "all"),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn test_processor_mode_parsing() {
        // Test v1-only
        let args = StartArgs::try_parse_from([
            "forester",
            "--processor-mode", "v1",
            "--rpc-url", "http://test.com",
            "--payer", "[1,2,3]",
            "--derivation", "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]"
        ]).unwrap();
        assert_eq!(args.processor_mode, ProcessorMode::V1);

        // Test v2-only
        let args = StartArgs::try_parse_from([
            "forester",
            "--processor-mode", "v2",
            "--rpc-url", "http://test.com",
            "--payer", "[1,2,3]",
            "--derivation", "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]"
        ]).unwrap();
        assert_eq!(args.processor_mode, ProcessorMode::V2);

        // Test all (default)
        let args = StartArgs::try_parse_from([
            "forester",
            "--rpc-url", "http://test.com",
            "--payer", "[1,2,3]",
            "--derivation", "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]"
        ]).unwrap();
        assert_eq!(args.processor_mode, ProcessorMode::All);

        // Test invalid mode should fail
        let result = StartArgs::try_parse_from([
            "forester",
            "--processor-mode", "invalid-mode",
            "--rpc-url", "http://test.com",
            "--payer", "[1,2,3]",
            "--derivation", "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]"
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_processor_mode_display() {
        assert_eq!(ProcessorMode::V1.to_string(), "v1");
        assert_eq!(ProcessorMode::V2.to_string(), "v2");
        assert_eq!(ProcessorMode::All.to_string(), "all");
    }
}
