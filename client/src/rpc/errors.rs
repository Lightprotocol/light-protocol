use std::io;

use solana_banks_client::BanksClientError;
use solana_client::client_error::ClientError;
use solana_program::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("BanksError: {0}")]
    BanksError(#[from] BanksClientError),

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
}

pub fn assert_rpc_error<T>(
    result: Result<T, RpcError>,
    i: u8,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    match result {
        Err(RpcError::TransactionError(TransactionError::InstructionError(
            index,
            InstructionError::Custom(error_code),
        ))) if index != i => Err(RpcError::AssertRpcError(
            format!(
                "Expected error code: {}, got: {} error: {}",
                expected_error_code,
                error_code,
                unsafe { result.unwrap_err_unchecked() }
            )
            .to_string(),
        )),
        Err(RpcError::TransactionError(TransactionError::InstructionError(
            index,
            InstructionError::Custom(error_code),
        ))) if index == i && error_code == expected_error_code => Ok(()),

        Err(RpcError::TransactionError(TransactionError::InstructionError(
            0,
            InstructionError::ProgramFailedToComplete,
        ))) => Ok(()),
        Err(e) => Err(RpcError::AssertRpcError(format!(
            "Unexpected error type: {:?}",
            e
        ))),
        _ => Err(RpcError::AssertRpcError(String::from(
            "Unexpected error type",
        ))),
    }
}
