use anchor_lang::solana_program::instruction::InstructionError;
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

    #[error("Error: `{0}`")]
    CustomError(String),
}

pub fn assert_rpc_error(result: Result<(), RpcError>, i: u8, expected_error_code: u32) {
    match result {
        Err(RpcError::TransactionError(TransactionError::InstructionError(
            index,
            InstructionError::Custom(error_code),
        ))) if index == i => {
            assert_eq!(
                error_code, expected_error_code,
                "Error code does not match expected value"
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        _ => panic!("Expected an error, got a success or different error"),
    }
}
