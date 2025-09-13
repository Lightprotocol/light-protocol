use std::{sync::Arc, time::Duration};

use forester_utils::{rpc_pool::SolanaRpcPool, ParsedMerkleTreeData, ParsedQueueData};
use light_client::rpc::Rpc;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub enum BatchReadyState {
    NotReady,
    AddressReadyForAppend {
        merkle_tree_data: ParsedMerkleTreeData,
    },
    StateReadyForAppend {
        merkle_tree_data: ParsedMerkleTreeData,
        output_queue_data: ParsedQueueData,
    },
    StateReadyForNullify {
        merkle_tree_data: ParsedMerkleTreeData,
    },
    BothReady {
        merkle_tree_data: ParsedMerkleTreeData,
        output_queue_data: ParsedQueueData,
    },
}

#[derive(Debug, Clone)]
pub struct StateConfig<R: Rpc> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub merkle_tree_pubkey: Pubkey,
    #[allow(dead_code)]
    pub output_queue_pubkey: Pubkey,
    pub nullify_prover_url: String,
    pub append_prover_url: String,
    pub prover_api_key: Option<String>,
    pub polling_interval: Duration,
    pub max_wait_time: Duration,
}

#[derive(Debug, Clone)]
pub struct BatchProcessorConfig {
    pub prover_append_url: String,
    pub prover_update_url: String,
    pub prover_address_append_url: String,
    pub prover_api_key: Option<String>,
    pub prover_polling_interval: Duration,
    pub prover_max_wait_time: Duration,
}
