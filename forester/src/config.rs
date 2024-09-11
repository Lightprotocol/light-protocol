use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
use forester_utils::forester_epoch::{Epoch, TreeAccounts, TreeForesterSchedule};
use forester_utils::rpc::RetryConfig;
use light_registry::{EpochPda, ForesterEpochPda};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ForesterEpochInfo {
    pub epoch: Epoch,
    pub epoch_pda: EpochPda,
    pub forester_epoch_pda: ForesterEpochPda,
    pub trees: Vec<TreeForesterSchedule>,
}

impl ForesterEpochInfo {
    /// Internal function to init Epoch struct with registered account
    /// 1. calculate epoch phases
    /// 2. set current epoch state
    /// 3. derive tree schedule for all input trees
    pub fn add_trees_with_schedule(&mut self, trees: &[TreeAccounts], current_solana_slot: u64) {
        // let state = self.phases.get_current_epoch_state(current_solana_slot);
        // TODO: add epoch state to sync schedule
        for tree in trees {
            debug!("Adding tree schedule for {:?}", tree);
            debug!("Current slot: {}", current_solana_slot);
            debug!("Epoch: {:?}", self.epoch_pda);
            let tree_schedule = TreeForesterSchedule::new_with_schedule(
                tree,
                current_solana_slot,
                &self.forester_epoch_pda,
                &self.epoch_pda,
            );
            self.trees.push(tree_schedule);
        }
    }
}

#[derive(Debug)]
pub struct ForesterConfig {
    pub external_services: ExternalServicesConfig,
    pub retry_config: RetryConfig,
    pub queue_config: QueueConfig,
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub cu_limit: u32,
    pub indexer_batch_size: usize,
    pub indexer_max_concurrent_batches: usize,
    pub transaction_batch_size: usize,
    pub transaction_max_concurrent_batches: usize,
    pub rpc_pool_size: usize,
    pub slot_update_interval_seconds: u64,
    pub tree_discovery_interval_seconds: u64,
    pub address_tree_data: Vec<TreeAccounts>,
    pub state_tree_data: Vec<TreeAccounts>,
    pub enable_metrics: bool,
}

impl Clone for ForesterConfig {
    fn clone(&self) -> Self {
        Self {
            external_services: self.external_services.clone(),
            retry_config: self.retry_config,
            queue_config: self.queue_config,
            registry_pubkey: self.registry_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            cu_limit: self.cu_limit,
            indexer_batch_size: self.indexer_batch_size,
            indexer_max_concurrent_batches: self.indexer_max_concurrent_batches,
            transaction_batch_size: self.transaction_batch_size,
            transaction_max_concurrent_batches: self.transaction_max_concurrent_batches,
            rpc_pool_size: self.rpc_pool_size,
            state_tree_data: self.state_tree_data.clone(),
            address_tree_data: self.address_tree_data.clone(),
            slot_update_interval_seconds: self.slot_update_interval_seconds,
            tree_discovery_interval_seconds: self.tree_discovery_interval_seconds,
            enable_metrics: self.enable_metrics,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExternalServicesConfig {
    pub rpc_url: String,
    pub ws_rpc_url: String,
    pub indexer_url: String,
    pub prover_url: String,
    pub photon_api_key: Option<String>,
    pub derivation: String,
    pub pushgateway_url: String,
}

#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    pub state_queue_start_index: u16,
    pub state_queue_length: u16,
    pub address_queue_start_index: u16,
    pub address_queue_length: u16,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig {
            state_queue_start_index: 0,
            state_queue_length: STATE_NULLIFIER_QUEUE_VALUES,
            address_queue_start_index: 0,
            address_queue_length: ADDRESS_QUEUE_VALUES,
        }
    }
}
