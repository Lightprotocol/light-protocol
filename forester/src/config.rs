use forester_utils::forester_epoch::{Epoch, TreeAccounts, TreeForesterSchedule};
use light_registry::{EpochPda, ForesterEpochPda};
use log::info;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

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
            info!("Adding tree schedule for {:?}", tree);
            info!("Current slot: {}", current_solana_slot);
            info!("Epoch: {:?}", self.epoch_pda);
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
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub cu_limit: u32,
    pub indexer_batch_size: usize,
    pub indexer_max_concurrent_batches: usize,
    pub transaction_batch_size: usize,
    pub transaction_max_concurrent_batches: usize,
    pub max_retries: usize,
    pub rpc_pool_size: usize,
    pub slot_update_interval_seconds: u64,
    pub address_tree_data: Vec<TreeAccounts>,
    pub state_tree_data: Vec<TreeAccounts>,
}

impl Clone for ForesterConfig {
    fn clone(&self) -> Self {
        Self {
            external_services: self.external_services.clone(),
            registry_pubkey: self.registry_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            cu_limit: self.cu_limit,
            indexer_batch_size: self.indexer_batch_size,
            indexer_max_concurrent_batches: self.indexer_max_concurrent_batches,
            transaction_batch_size: self.transaction_batch_size,
            transaction_max_concurrent_batches: self.transaction_max_concurrent_batches,
            max_retries: self.max_retries,
            rpc_pool_size: self.rpc_pool_size,
            state_tree_data: self.state_tree_data.clone(),
            address_tree_data: self.address_tree_data.clone(),
            slot_update_interval_seconds: self.slot_update_interval_seconds,
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
