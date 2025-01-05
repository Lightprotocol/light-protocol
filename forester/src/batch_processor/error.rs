use forester_utils::forester_epoch::TreeType;
use light_client::rpc_pool::PoolError;
use solana_client::rpc_request::RpcError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, BatchProcessError>;

#[derive(Debug, Error)]
pub enum BatchProcessError {
    #[error("Failed to parse queue account: {0}")]
    QueueParsing(String),

    #[error("Failed to parse merkle tree account: {0}")]
    MerkleTreeParsing(String),

    #[error("Failed to create instruction data: {0}")]
    InstructionData(String),

    #[error("Transaction failed: {0}")]
    Transaction(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Pool error: {0}")]
    Pool(String),

    #[error("Indexer error: {0}")]
    Indexer(String),

    #[error("Unsupported tree type: {0:?}")]
    UnsupportedTreeType(TreeType),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<light_client::rpc::RpcError> for BatchProcessError {
    fn from(e: light_client::rpc::RpcError) -> Self {
        Self::Rpc(e.to_string())
    }
}

impl From<RpcError> for BatchProcessError {
    fn from(e: RpcError) -> Self {
        Self::Rpc(e.to_string())
    }
}

impl From<PoolError> for BatchProcessError {
    fn from(e: PoolError) -> Self {
        Self::Pool(e.to_string())
    }
}
