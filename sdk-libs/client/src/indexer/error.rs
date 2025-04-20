use light_merkle_tree_reference::indexed::IndexedReferenceMerkleTreeError as IndexedReferenceMerkleTreeErrorV2;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum IndexerError {
    #[error("Photon API error in {context}: {message}")]
    PhotonError { context: String, message: String },

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Failed to deserialize account data: {0}")]
    DeserializeError(#[from] solana_sdk::program_error::ProgramError),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Missing result from {context}: {message}")]
    MissingResult { context: String, message: String },

    #[error("Account not found")]
    AccountNotFound,

    #[error("Base58 decode error: {field} - {message}")]
    Base58DecodeError { field: String, message: String },

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Data decode error: {field} - {message}")]
    DataDecodeError { field: String, message: String },

    #[error("Method not implemented: {0}")]
    NotImplemented(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Indexed Merkle tree reference v1 error: {0}")]
    ReferenceIndexedMerkleTreeError(
        #[from] light_indexed_merkle_tree::reference::IndexedReferenceMerkleTreeError,
    ),
    #[error("Indexed Merkle tree v1 error: {0}")]
    IndexedMerkleTreeError(#[from] light_indexed_merkle_tree::errors::IndexedMerkleTreeError),
    #[error("Reference Merkle tree error: {0}")]
    ReferenceMerkleTreeError(#[from] light_merkle_tree_reference::ReferenceMerkleTreeError),
    #[error("Indexed Merkle tree v2 error: {0}")]
    IndexedMerkleTreeV2Error(#[from] IndexedReferenceMerkleTreeErrorV2),
    #[error("Light indexed array error: {0}")]
    LightIndexedArrayError(#[from] light_indexed_array::errors::IndexedArrayError),
}

impl IndexerError {
    pub fn missing_result(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::MissingResult {
            context: context.into(),
            message: message.into(),
        }
    }

    pub fn api_error(error: impl std::fmt::Display) -> Self {
        Self::ApiError(error.to_string())
    }

    pub fn decode_error(field: impl Into<String>, error: impl std::fmt::Display) -> Self {
        Self::DataDecodeError {
            field: field.into(),
            message: error.to_string(),
        }
    }

    pub fn base58_decode_error(field: impl Into<String>, error: impl std::fmt::Display) -> Self {
        Self::Base58DecodeError {
            field: field.into(),
            message: error.to_string(),
        }
    }
}

impl<T> From<photon_api::apis::Error<T>> for IndexerError {
    fn from(error: photon_api::apis::Error<T>) -> Self {
        match error {
            photon_api::apis::Error::Reqwest(e) => {
                IndexerError::ApiError(format!("Request error: {}", e))
            }
            photon_api::apis::Error::Serde(e) => {
                IndexerError::ApiError(format!("Serialization error: {}", e))
            }
            photon_api::apis::Error::Io(e) => IndexerError::ApiError(format!("IO error: {}", e)),
            _ => IndexerError::ApiError("Unknown API error".to_string()),
        }
    }
}
