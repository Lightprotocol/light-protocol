#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
#![allow(deprecated)]

pub mod account_zero_copy;
pub mod address_merkle_tree_config;
pub mod error;
pub mod forester_epoch;
pub mod instructions;
pub mod rate_limiter;
pub mod registry;
pub mod rpc_pool;
pub mod utils;

/// Parsed merkle tree data extracted from account
#[derive(Debug, Clone)]
pub struct ParsedMerkleTreeData {
    pub next_index: u64,
    pub current_root: [u8; 32],
    pub root_history: Vec<[u8; 32]>,
    pub zkp_batch_size: u16,
    pub pending_batch_index: u32,
    pub num_inserted_zkps: u64,
    pub current_zkp_batch_index: u64,
    pub batch_start_index: u64,
    pub leaves_hash_chains: Vec<[u8; 32]>,
}

/// Parsed output queue data extracted from account
#[derive(Debug, Clone)]
pub struct ParsedQueueData {
    pub zkp_batch_size: u16,
    pub pending_batch_index: u32,
    pub num_inserted_zkps: u64,
    pub current_zkp_batch_index: u64,
    pub leaves_hash_chains: Vec<[u8; 32]>,
}
