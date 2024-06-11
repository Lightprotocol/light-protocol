use account_compression::initialize_address_merkle_tree::Error as AccountCompressionError;
use light_hash_set::HashSetError;
use light_test_utils::rpc::errors::RpcError;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use thiserror::Error;

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
