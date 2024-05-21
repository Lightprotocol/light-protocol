use crate::constants::SERVER_URL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

pub struct Config {
    pub server_url: String,
    pub nullifier_queue_pubkey: Pubkey,
    pub merkle_tree_pubkey: Pubkey,
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
            merkle_tree_pubkey: self.merkle_tree_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            concurrency_limit: self.concurrency_limit,
            batch_size: self.batch_size,
            max_retries: self.max_retries,
        }
    }
}

impl Config {
    pub fn test() -> Self {
        let merkle_tree =
            "3wBL7d5qoWiYAV2bHMsmjKFr3u8SWa4Aps9mAcanfhRQMdFrtJtASwB5ZSvYeoAgD3SZsiYtnZVrrXpHKDpxkgZ2";
        let nullifier_queue =
            "5T2Fg9GVnZjGJetLnt2HF1CpYMM9fAzxodvmqJzh8dgjs96hqkwtcXkYrg7wT2ZCGj6syhAYtg5EEpeDBTQDJGY5";
        let payer = [
            46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249, 108, 80, 144, 32, 168, 245, 161, 146,
            92, 180, 79, 231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130, 60, 106, 251, 24, 224,
            192, 108, 70, 59, 111, 251, 186, 50, 23, 103, 106, 233, 113, 148, 57, 190, 158, 111,
            163, 28, 157, 47, 201, 41, 249, 59,
        ];

        Self {
            server_url: SERVER_URL.to_string(),
            nullifier_queue_pubkey: Keypair::from_base58_string(nullifier_queue).pubkey(),
            merkle_tree_pubkey: Keypair::from_base58_string(merkle_tree).pubkey(),
            payer_keypair: Keypair::from_bytes(&payer).unwrap(),
            concurrency_limit: 20,
            batch_size: 1000,
            max_retries: 5,
        }
    }
}
