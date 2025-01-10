use std::{fmt::Debug, io};

use solana_banks_client::BanksClientError;
use solana_client::client_error::ClientError;
use solana_program::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("BanksError: {0}")]
    BanksError(#[from] Box<BanksClientError>),

    #[error("TransactionError: {0}")]
    TransactionError(#[from] Box<TransactionError>),

    #[error("ClientError: {0}")]
    ClientError(#[from] Box<ClientError>),

    #[error("IoError: {0}")]
    IoError(#[from] Box<io::Error>),

    #[error("Error: `{0}`")]
    CustomError(String),

    #[error("Assert Rpc Error: {0}")]
    AssertRpcError(String),

    /// The chosen warp slot is not in the future, so warp is not performed
    #[error("Warp slot not in the future")]
    InvalidWarpSlot,
}

impl From<BanksClientError> for RpcError {
    fn from(err: BanksClientError) -> Self {
        RpcError::BanksError(Box::new(err))
    }
}

impl From<TransactionError> for RpcError {
    fn from(err: TransactionError) -> Self {
        RpcError::TransactionError(Box::new(err))
    }
}

impl From<ClientError> for RpcError {
    fn from(err: ClientError) -> Self {
        RpcError::ClientError(Box::new(err))
    }
}

impl From<io::Error> for RpcError {
    fn from(err: io::Error) -> Self {
        RpcError::IoError(Box::new(err))
    }
}

pub fn assert_rpc_error<T: Debug>(
    result: Result<T, RpcError>,
    i: u8,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    match result {
        Err(RpcError::TransactionError(ref box_err))
            if matches!(
                **box_err,
                TransactionError::InstructionError(
                    index,
                    InstructionError::Custom(_)
                ) if index != i
            ) =>
        {
            let TransactionError::InstructionError(_, InstructionError::Custom(actual_error_code)) =
                **box_err
            else {
                unreachable!()
            };
            Err(RpcError::AssertRpcError(format!(
                "Expected error code: {}, got: {} error: {:?}",
                expected_error_code, actual_error_code, result
            )))
        }

        Err(RpcError::TransactionError(ref box_err))
            if matches!(
                **box_err,
                TransactionError::InstructionError(
                    index,
                    InstructionError::Custom(error_code)
                ) if index == i && error_code == expected_error_code
            ) =>
        {
            Ok(())
        }

        Err(RpcError::TransactionError(ref box_err))
            if matches!(
                **box_err,
                TransactionError::InstructionError(0, InstructionError::ProgramFailedToComplete)
            ) =>
        {
            Ok(())
        }

        Err(e) => Err(RpcError::AssertRpcError(format!(
            "Unexpected error type: {:?}",
            e
        ))),
        _ => Err(RpcError::AssertRpcError(String::from(
            "Unexpected error type",
        ))),
    }
}
