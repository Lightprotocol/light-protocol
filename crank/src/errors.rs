use thiserror::Error;
use account_compression::initialize_address_merkle_tree::Error as AccountCompressionError;
use light_hash_set::HashSetError;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};

#[derive(Error, Debug)]
pub enum CrankError {
    #[error("RPC Error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
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

impl From<PhotonApiError<GetCompressedAccountProofPostError>> for CrankError {
    fn from(err: PhotonApiError<GetCompressedAccountProofPostError>) -> Self {
        CrankError::PhotonApiError(PhotonApiErrorWrapper::GetCompressedAccountProofPostError(err))
    }
}