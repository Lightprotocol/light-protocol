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
    pub rpc_url: String,

    #[arg(long, env = "FORESTER_PUSH_GATEWAY_URL")]
    pub push_gateway_url: Option<String>,

    #[arg(long, env = "FORESTER_WS_RPC_URL")]
    pub ws_rpc_url: String,

    #[arg(long, env = "FORESTER_INDEXER_URL")]
    pub indexer_url: String,

    #[arg(long, env = "FORESTER_PROVER_URL")]
    pub prover_url: String,

    #[arg(long, env = "FORESTER_PAYER")]
    pub payer: String,

    #[arg(long, env = "FORESTER_PHOTON_API_KEY")]
    pub photon_api_key: String,

    #[arg(long, env = "FORESTER_INDEXER_BATCH_SIZE", default_value = "50")]
    pub indexer_batch_size: usize,

    #[arg(
        long,
        env = "FORESTER_INDEXER_MAX_CONCURRENT_BATCHES",
        default_value = "10"
    )]
    pub indexer_max_concurrent_batches: usize,

    #[arg(long, env = "FORESTER_TRANSACTION_BATCH_SIZE", default_value = "1")]
    pub transaction_batch_size: usize,

    #[arg(
        long,
        env = "FORESTER_TRANSACTION_MAX_CONCURRENT_BATCHES",
        default_value = "20"
    )]
    pub transaction_max_concurrent_batches: usize,

    #[arg(long, env = "FORESTER_CU_LIMIT", default_value = "1000000")]
    pub cu_limit: u32,

    #[arg(long, env = "FORESTER_RPC_POOL_SIZE", default_value = "20")]
    pub rpc_pool_size: usize,

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

    #[arg(long, env = "FORESTER_STATE_QUEUE_LENGTH", default_value = "28807")]
    pub state_queue_length: u16,

    #[arg(long, env = "FORESTER_ADDRESS_QUEUE_START_INDEX", default_value = "0")]
    pub address_queue_start_index: u16,

    #[arg(long, env = "FORESTER_ADDRESS_QUEUE_LENGTH", default_value = "28807")]
    pub address_queue_length: u16,
}

#[derive(Parser, Clone, Debug)]
pub struct StatusArgs {
    #[arg(long, env = "FORESTER_RPC_URL")]
    pub rpc_url: String,

    #[arg(long, env = "FORESTER_PUSH_GATEWAY_URL")]
    pub push_gateway_url: Option<String>,
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
