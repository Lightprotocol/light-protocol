use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum IndexerError {
    #[error("Photon API error in {context}: {message}")]
    PhotonError { context: String, message: String },

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Failed to deserialize account data: {0}")]
    DeserializeError(#[from] solana_program_error::ProgramError),

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
    #[error("Invalid response data")]
    InvalidResponseData,

    #[error("Error: `{0}`")]
    CustomError(String),
    #[error(
        "Indexer not initialized. Set photon_url in LightClientConfig to enable indexer API calls."
    )]
    NotInitialized,
    #[error("Indexer slot has not reached the requested slot.")]
    IndexerNotSyncedToSlot,
    #[error("Address Merkle trees cannot be packed as output Merkle trees.")]
    InvalidPackTreeType,
    #[error("Cannot mix v1 and v2 trees in the same validity proof. State tree version: {state_version}, Address tree version: {address_version}")]
    MixedTreeVersions {
        state_version: String,
        address_version: String,
    },
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
            _ => IndexerError::ApiError(format!("Unknown API error {}", error)),
        }
    }
}

impl From<crate::rpc::RpcError> for IndexerError {
    fn from(error: crate::rpc::RpcError) -> Self {
        IndexerError::RpcError(error.to_string())
    }
}

impl Clone for IndexerError {
    fn clone(&self) -> Self {
        match self {
            IndexerError::PhotonError { context, message } => IndexerError::PhotonError {
                context: context.clone(),
                message: message.clone(),
            },
            IndexerError::RpcError(message) => IndexerError::RpcError(message.clone()),
            IndexerError::DeserializeError(err) => IndexerError::DeserializeError(err.clone()),
            IndexerError::ApiError(message) => IndexerError::ApiError(message.clone()),
            IndexerError::MissingResult { context, message } => IndexerError::MissingResult {
                context: context.clone(),
                message: message.clone(),
            },
            IndexerError::AccountNotFound => IndexerError::AccountNotFound,
            IndexerError::Base58DecodeError { field, message } => IndexerError::Base58DecodeError {
                field: field.clone(),
                message: message.clone(),
            },
            IndexerError::InvalidParameters(message) => {
                IndexerError::InvalidParameters(message.clone())
            }
            IndexerError::DataDecodeError { field, message } => IndexerError::DataDecodeError {
                field: field.clone(),
                message: message.clone(),
            },
            IndexerError::NotImplemented(message) => IndexerError::NotImplemented(message.clone()),
            IndexerError::Unknown(message) => IndexerError::Unknown(message.clone()),
            IndexerError::ReferenceIndexedMerkleTreeError(_) => {
                IndexerError::CustomError("ReferenceIndexedMerkleTreeError".to_string())
            }
            IndexerError::IndexedMerkleTreeError(_) => {
                IndexerError::CustomError("IndexedMerkleTreeError".to_string())
            }
            IndexerError::InvalidResponseData => IndexerError::InvalidResponseData,
            IndexerError::CustomError(_) => IndexerError::CustomError("IndexerError".to_string()),
            IndexerError::NotInitialized => IndexerError::NotInitialized,
            IndexerError::IndexerNotSyncedToSlot => IndexerError::IndexerNotSyncedToSlot,
            IndexerError::InvalidPackTreeType => IndexerError::InvalidPackTreeType,
            IndexerError::MixedTreeVersions {
                state_version,
                address_version,
            } => IndexerError::MixedTreeVersions {
                state_version: state_version.clone(),
                address_version: address_version.clone(),
            },
        }
    }
}
