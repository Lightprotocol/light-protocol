use crate::indexer::PhotonApiError;
use light_hash_set::HashSetError;
use photon_api::apis::default_api::GetCompressedAccountProofPostError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("RPC Error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
    #[error("failed to deserialize account data")]
    DeserializeError(#[from] solana_sdk::program_error::ProgramError),
    #[error("failed to copy merkle tree")]
    CopyMerkleTreeError(#[from] std::io::Error),
    #[error(transparent)]
    HashSetError(#[from] HashSetError),
    #[error(transparent)]
    PhotonApiError(PhotonApiErrorWrapper),
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

impl From<PhotonApiError<GetCompressedAccountProofPostError>> for IndexerError {
    fn from(err: PhotonApiError<GetCompressedAccountProofPostError>) -> Self {
        IndexerError::PhotonApiError(PhotonApiErrorWrapper::GetCompressedAccountProofPostError(
            err,
        ))
    }
}
