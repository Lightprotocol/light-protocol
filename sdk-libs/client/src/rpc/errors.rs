use std::io;

use solana_rpc_client_api::client_error::Error as ClientError;
use solana_transaction_error::TransactionError;
use thiserror::Error;

use crate::indexer::IndexerError;

#[derive(Error, Debug)]
pub enum RpcError {
    #[cfg(feature = "program-test")]
    #[error("BanksError: {0}")]
    BanksError(#[from] solana_banks_client::BanksClientError),

    #[error("TransactionError: {0}")]
    TransactionError(#[from] TransactionError),

    #[error("ClientError: {0}")]
    ClientError(#[from] ClientError),

    #[error("IoError: {0}")]
    IoError(#[from] io::Error),

    #[error("Error: `{0}`")]
    CustomError(String),

    #[error("Assert Rpc Error: {0}")]
    AssertRpcError(String),

    /// The chosen warp slot is not in the future, so warp is not performed
    #[error("Warp slot not in the future")]
    InvalidWarpSlot,

    #[error("Account {0} does not exist")]
    AccountDoesNotExist(String),

    #[error("Invalid response data.")]
    InvalidResponseData,

    #[error("Indexer not initialized.")]
    IndexerNotInitialized,

    #[error("Indexer error: {0}")]
    IndexerError(#[from] IndexerError),
}

// Convert light_compressed_account errors
impl From<light_compressed_account::indexer_event::error::ParseIndexerEventError> for RpcError {
    fn from(e: light_compressed_account::indexer_event::error::ParseIndexerEventError) -> Self {
        RpcError::CustomError(format!("ParseIndexerEventError: {}", e))
    }
}
