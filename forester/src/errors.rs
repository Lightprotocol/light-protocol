use crate::rpc_pool::PoolError;
use account_compression::initialize_address_merkle_tree::Error as AccountCompressionError;
use config::ConfigError;
use light_hash_set::HashSetError;
use light_test_utils::indexer::IndexerError;
use light_test_utils::rpc::errors::RpcError;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use prometheus::Error as PrometheusError;
use reqwest::Error as ReqwestError;
use solana_client::pubsub_client::PubsubClientError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum ForesterError {
    #[error("Element is not eligible for foresting")]
    NotEligible,
    #[error("RPC Error: {0}")]
    RpcError(#[from] RpcError),
    #[error("failed to deserialize account data")]
    DeserializeError(#[from] solana_sdk::program_error::ProgramError),
    #[error("failed to copy merkle tree")]
    CopyMerkleTreeError(#[from] std::io::Error),
    #[error(transparent)]
    AccountCompressionError(#[from] AccountCompressionError),
    #[error(transparent)]
    HashSetError(#[from] HashSetError),
    #[error(transparent)]
    PhotonApiError(PhotonApiErrorWrapper),
    #[error("bincode error")]
    BincodeError(#[from] Box<bincode::ErrorKind>),
    #[error("Indexer can't find any proofs")]
    NoProofsFound,
    #[error("Max retries reached")]
    MaxRetriesReached,
    #[error("error: {0:?}")]
    SendError(String),
    #[error("error: {0:?}")]
    IndexerError(String),
    #[error("Recv error: {0}")]
    RecvError(#[from] RecvError),
    #[error("error: {0:?}")]
    JoinError(String),
    #[error("Solana pubsub client error: {0}")]
    PubsubClientError(#[from] PubsubClientError),
    #[error("Channel disconnected")]
    ChannelDisconnected,
    #[error("Subscription timeout")]
    SubscriptionTimeout,
    #[error("Unexpected message: {0}")]
    UnexpectedMessage(String),
    #[error("Config error: {0:?}")]
    ConfigError(String),
    #[error("error: {0:?}")]
    PrometheusError(PrometheusError),
    #[error("error: {0:?}")]
    ReqwestError(ReqwestError),
    #[error("error: {0:?}")]
    Custom(String),
    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum PhotonApiErrorWrapper {
    #[error(transparent)]
    GetCompressedAccountProofPostError(#[from] PhotonApiError<GetCompressedAccountProofPostError>),
}

impl From<PhotonApiError<GetCompressedAccountProofPostError>> for ForesterError {
    fn from(err: PhotonApiError<GetCompressedAccountProofPostError>) -> Self {
        ForesterError::PhotonApiError(PhotonApiErrorWrapper::GetCompressedAccountProofPostError(
            err,
        ))
    }
}

impl From<IndexerError> for ForesterError {
    fn from(err: IndexerError) -> Self {
        ForesterError::IndexerError(err.to_string())
    }
}

impl<T> From<SendError<T>> for ForesterError {
    fn from(err: SendError<T>) -> Self {
        ForesterError::SendError(err.to_string())
    }
}

impl From<JoinError> for ForesterError {
    fn from(err: JoinError) -> Self {
        ForesterError::JoinError(err.to_string())
    }
}

impl From<PoolError> for ForesterError {
    fn from(err: PoolError) -> Self {
        ForesterError::Custom(err.to_string())
    }
}

impl From<ConfigError> for ForesterError {
    fn from(err: ConfigError) -> Self {
        ForesterError::Custom(err.to_string())
    }
}

impl From<PrometheusError> for ForesterError {
    fn from(err: PrometheusError) -> ForesterError {
        ForesterError::PrometheusError(err)
    }
}

impl From<ReqwestError> for ForesterError {
    fn from(err: ReqwestError) -> ForesterError {
        ForesterError::ReqwestError(err)
    }
}

impl From<String> for ForesterError {
    fn from(err: String) -> ForesterError {
        ForesterError::Custom(err)
    }
}
