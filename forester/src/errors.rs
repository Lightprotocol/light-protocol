use account_compression::initialize_address_merkle_tree::Error as AccountCompressionError;
use light_hash_set::HashSetError;
use light_test_utils::indexer::IndexerError;
use light_test_utils::rpc::errors::RpcError;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use solana_client::pubsub_client::PubsubClientError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum ForesterError {
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
    #[error("error: {0:?}")]
    Custom(String),
    #[error("unknown error")]
    Unknown,
}
impl ForesterError {
    pub fn to_owned(&self) -> Self {
        match self {
            ForesterError::RpcError(e) => ForesterError::Custom(format!("RPC Error: {:?}", e)),
            ForesterError::DeserializeError(e) => {
                ForesterError::Custom(format!("Deserialize Error: {:?}", e))
            }
            ForesterError::CopyMerkleTreeError(e) => {
                ForesterError::Custom(format!("Copy Merkle Tree Error: {:?}", e))
            }
            ForesterError::AccountCompressionError(e) => {
                ForesterError::Custom(format!("Account Compression Error: {:?}", e))
            }
            ForesterError::HashSetError(e) => {
                ForesterError::Custom(format!("HashSet Error: {:?}", e))
            }
            ForesterError::PhotonApiError(e) => {
                ForesterError::Custom(format!("Photon API Error: {:?}", e))
            }
            ForesterError::BincodeError(e) => {
                ForesterError::Custom(format!("Bincode Error: {:?}", e))
            }
            ForesterError::SendError(e) => ForesterError::SendError(e.clone()),
            ForesterError::IndexerError(e) => ForesterError::IndexerError(e.clone()),
            ForesterError::RecvError(e) => ForesterError::RecvError(e.clone()),
            ForesterError::JoinError(e) => ForesterError::IndexerError(e.clone()),
            ForesterError::NoProofsFound => ForesterError::NoProofsFound,
            ForesterError::MaxRetriesReached => ForesterError::MaxRetriesReached,

            ForesterError::Custom(s) => ForesterError::Custom(s.clone()),
            ForesterError::Unknown => ForesterError::Unknown,
            ForesterError::PubsubClientError(e) => {
                ForesterError::Custom(format!("PubsubClientError: {:?}", e))
            }
            ForesterError::ChannelDisconnected => ForesterError::ChannelDisconnected,
            ForesterError::SubscriptionTimeout => ForesterError::SubscriptionTimeout,
            ForesterError::UnexpectedMessage(e) => ForesterError::UnexpectedMessage(e.clone()),
        }
    }
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
