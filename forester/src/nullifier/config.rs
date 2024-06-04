use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

pub struct Config {
    pub server_url: String,
    pub nullifier_queue_pubkey: Pubkey,
    pub state_merkle_tree_pubkey: Pubkey,
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub concurrency_limit: usize,
    pub batch_size: usize,
    pub max_retries: usize,
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Self {
            server_url: self.server_url.clone(),
            nullifier_queue_pubkey: self.nullifier_queue_pubkey,
            state_merkle_tree_pubkey: self.state_merkle_tree_pubkey,
            registry_pubkey: self.registry_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            concurrency_limit: self.concurrency_limit,
            batch_size: self.batch_size,
            max_retries: self.max_retries,
        }
    }
}
