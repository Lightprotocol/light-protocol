use clap::{Parser, Subcommand};

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

    #[arg(long, env = "FORESTER_PAYER")]
    pub payer: Option<String>,

    #[arg(long, env = "FORESTER_DERIVATION_PUBKEY")]
    pub derivation: Option<String>,

    #[arg(long, env = "FORESTER_PHOTON_API_KEY")]
    pub photon_api_key: Option<String>,

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

    #[arg(long, env = "FORESTER_BATCH_IXS_PER_TX", default_value = "1")]
    pub batch_ixs_per_tx: usize,

    #[arg(
        long,
        env = "FORESTER_TRANSACTION_MAX_CONCURRENT_BATCHES",
        default_value = "20"
    )]
    pub transaction_max_concurrent_batches: usize,

    #[arg(
        long,
        env = "FORESTER_TX_CACHE_TTL_SECONDS",
        default_value = "180",
        help = "Transaction cache TTL in seconds to prevent duplicate batch processing"
    )]
    pub tx_cache_ttl_seconds: u64,

    #[arg(long, env = "FORESTER_CU_LIMIT", default_value = "1000000")]
    pub cu_limit: u32,

    #[arg(long, env = "FORESTER_ENABLE_PRIORITY_FEES", default_value = "false")]
    pub enable_priority_fees: bool,

    #[arg(long, env = "FORESTER_RPC_POOL_SIZE", default_value = "50")]
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
