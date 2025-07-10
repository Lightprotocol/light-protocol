use light_batched_merkle_tree::errors::BatchedMerkleTreeError;
use light_client::rpc::RpcError;
use light_hasher::HasherError;
use thiserror::Error;

use crate::rpc_pool::PoolError;

#[derive(Error, Debug)]
pub enum ForesterUtilsError {
    #[error("parse error: {0:?}")]
    Parse(String),
    #[error("prover error: {0:?}")]
    Prover(String),
    #[error("rpc error")]
    Rpc(#[from] RpcError),
    #[error("indexer error: {0:?}")]
    Indexer(String),
    #[error("invalid slot number")]
    InvalidSlotNumber,
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),

    #[error("Account zero-copy error: {0}")]
    AccountZeroCopy(String),

    #[error("batched merkle tree error: {0}")]
    BatchedMerkleTree(#[from] BatchedMerkleTreeError),

    #[error("pool error: {0}")]
    Pool(#[from] PoolError),
}
