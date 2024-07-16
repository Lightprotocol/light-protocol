use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::external_services_config::ExternalServicesConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use crate::tree_sync::TreeData;

#[derive(Debug)]
pub struct ForesterConfig {
    pub external_services: ExternalServicesConfig,
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub cu_limit: u32,
    pub concurrency_limit: usize,
    pub batch_size: usize,
    pub max_retries: usize,
    pub rpc_pool_size: usize,
    pub address_tree_data: Vec<TreeData>,
    pub state_tree_data: Vec<TreeData>,
    pub last_sync_data: HashMap<Pubkey, TreeSyncMetadata>,
}
impl Clone for ForesterConfig {
    fn clone(&self) -> Self {
        Self {
            external_services: self.external_services.clone(),
            registry_pubkey: self.registry_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            cu_limit: self.cu_limit,
            concurrency_limit: self.concurrency_limit,
            batch_size: self.batch_size,
            max_retries: self.max_retries,
            rpc_pool_size: self.rpc_pool_size,
            state_tree_data: self.state_tree_data.clone(),
            address_tree_data: self.address_tree_data.clone(),
            last_sync_data: self.last_sync_data.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TreeSyncMetadata {
    pub last_synced_at: DateTime<Utc>,
    pub last_synced_root: [u8; 32],
}