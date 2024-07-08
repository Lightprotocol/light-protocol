use crate::external_services_config::ExternalServicesConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

#[derive(Debug)]
pub struct ForesterConfig {
    pub external_services: ExternalServicesConfig,
    pub nullifier_queue_pubkey: Pubkey,
    pub state_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_queue_pubkey: Pubkey,
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub cu_limit: u32,
    pub concurrency_limit: usize,
    pub batch_size: usize,
    pub max_retries: usize,
}

impl Clone for ForesterConfig {
    fn clone(&self) -> Self {
        Self {
            external_services: self.external_services.clone(),
            nullifier_queue_pubkey: self.nullifier_queue_pubkey,
            state_merkle_tree_pubkey: self.state_merkle_tree_pubkey,
            address_merkle_tree_pubkey: self.address_merkle_tree_pubkey,
            address_merkle_tree_queue_pubkey: self.address_merkle_tree_queue_pubkey,
            registry_pubkey: self.registry_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            cu_limit: self.cu_limit,
            concurrency_limit: self.concurrency_limit,
            batch_size: self.batch_size,
            max_retries: self.max_retries,
        }
    }
}
