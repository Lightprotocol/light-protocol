use std::io;

use light_sdk::error::LightSdkError;
use solana_rpc_client_api::client_error::Error as ClientError;
use solana_transaction_error::TransactionError;
use thiserror::Error;

use crate::indexer::IndexerError;

#[derive(Error, Debug)]
pub enum RpcError {
    #[cfg(feature = "program-test")]
    #[error("BanksError: {0}")]
    BanksError(#[from] solana_banks_client::BanksClientError),

    #[error("State tree lookup table not found")]
    StateTreeLookupTableNotFound,

    #[error("State tree lookup table must have a multiple of 3 addresses")]
    InvalidStateTreeLookupTable,

    #[error("Nullify table not found")]
    NullifyTableNotFound,

    #[error("TransactionError: {0}")]
    TransactionError(#[from] TransactionError),

    #[error("ClientError: {0}")]
    ClientError(#[from] ClientError),

    #[error("IoError: {0}")]
    IoError(#[from] io::Error),

    #[error("Error: `{0}`")]
    CustomError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Assert Rpc Error: {0}")]
    AssertRpcError(String),

    /// The chosen warp slot is not in the future, so warp is not performed
    #[error("Warp slot not in the future")]
    InvalidWarpSlot,

    #[cfg(feature = "program-test")]
    #[error("LiteSVM Error: {0}")]
    LiteSvmError(String),

    #[error("Account {0} does not exist")]
    AccountDoesNotExist(String),

    #[error("Invalid response data.")]
    InvalidResponseData,

    #[error("Indexer not initialized.")]
    IndexerNotInitialized,

    #[error("Indexer error: {0}")]
    IndexerError(#[from] IndexerError),

    #[error(
        "No state trees available, use rpc.get_latest_active_state_trees() to fetch state trees"
    )]
    NoStateTreesAvailable,

    #[error("LightSdkError error: {0}")]
    LightSdkError(#[from] LightSdkError),
}

impl From<light_event::error::ParseIndexerEventError> for RpcError {
    fn from(e: light_event::error::ParseIndexerEventError) -> Self {
        RpcError::CustomError(format!("ParseIndexerEventError: {}", e))
    }
}

impl Clone for RpcError {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "program-test")]
            RpcError::BanksError(_) => RpcError::CustomError("BanksError".to_string()),
            RpcError::TransactionError(e) => RpcError::TransactionError(e.clone()),
            RpcError::ClientError(_) => RpcError::CustomError("ClientError".to_string()),
            RpcError::IoError(e) => RpcError::IoError(e.kind().into()),
            RpcError::CustomError(e) => RpcError::CustomError(e.clone()),
            RpcError::SigningError(e) => RpcError::SigningError(e.clone()),
            RpcError::AssertRpcError(e) => RpcError::AssertRpcError(e.clone()),
            RpcError::InvalidWarpSlot => RpcError::InvalidWarpSlot,
            RpcError::AccountDoesNotExist(e) => RpcError::AccountDoesNotExist(e.clone()),
            RpcError::InvalidResponseData => RpcError::InvalidResponseData,
            RpcError::IndexerNotInitialized => RpcError::IndexerNotInitialized,
            RpcError::IndexerError(e) => RpcError::IndexerError(e.clone()),
            RpcError::LightSdkError(e) => RpcError::CustomError(e.to_string()),
            RpcError::StateTreeLookupTableNotFound => RpcError::StateTreeLookupTableNotFound,
            RpcError::InvalidStateTreeLookupTable => RpcError::InvalidStateTreeLookupTable,
            RpcError::NullifyTableNotFound => RpcError::NullifyTableNotFound,
            RpcError::NoStateTreesAvailable => RpcError::NoStateTreesAvailable,
            #[cfg(feature = "program-test")]
            RpcError::LiteSvmError(e) => RpcError::LiteSvmError(e.clone()),
        }
    }
}

#[cfg(feature = "program-test")]
impl From<litesvm::error::LiteSVMError> for RpcError {
    fn from(e: litesvm::error::LiteSVMError) -> Self {
        RpcError::LiteSvmError(e.to_string())
    }
}
