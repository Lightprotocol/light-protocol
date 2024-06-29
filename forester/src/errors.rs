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
    #[error("Max retries reached")]
    MaxRetriesReached,
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
            ForesterError::NoProofsFound => ForesterError::NoProofsFound,
            ForesterError::MaxRetriesReached => ForesterError::MaxRetriesReached,
            ForesterError::Custom(s) => ForesterError::Custom(s.clone()),
            ForesterError::Unknown => ForesterError::Unknown,
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
