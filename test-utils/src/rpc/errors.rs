use solana_client::client_error::ClientError;
use solana_program_test::BanksClientError;
use solana_sdk::transaction::TransactionError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("BanksError: {0}")]
    BanksError(#[from] BanksClientError),

    #[error("ProgramTestError: {0}")]
    ProgramTestError(#[from] solana_program_test::ProgramTestError),

    #[error("TransactionError: {0}")]
    TransactionError(#[from] TransactionError),

    #[error("ClientError: {0}")]
    ClientError(#[from] ClientError),

    #[error("IoError: {0}")]
    IoError(#[from] io::Error),
}
