use light_client::rpc::RpcError;
use solana_banks_client::BanksClientError;
use solana_instruction::error::InstructionError;
use solana_sdk::transaction::TransactionError;

#[allow(clippy::result_large_err)]
pub fn assert_rpc_error<T>(
    result: Result<T, RpcError>,
    index_instruction: u8,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    match result {
        Err(RpcError::TransactionError(TransactionError::InstructionError(
            index,
            InstructionError::Custom(error_code),
        ))) if index != index_instruction => Err(RpcError::AssertRpcError(
            format!(
                "Expected error code: {}, got: {} error: {}",
                expected_error_code,
                error_code,
                unsafe { result.unwrap_err_unchecked() }
            )
            .to_string(),
        )),
        Err(RpcError::BanksError(BanksClientError::TransactionError(
            TransactionError::InstructionError(index, InstructionError::Custom(error_code)),
        ))) if index != index_instruction => Err(RpcError::AssertRpcError(
            format!(
                "Expected error code: {}, got: {} error: {}",
                expected_error_code,
                error_code,
                unsafe { result.unwrap_err_unchecked() }
            )
            .to_string(),
        )),
        Err(RpcError::BanksError(BanksClientError::TransactionError(
            TransactionError::InstructionError(index, InstructionError::Custom(error_code)),
        ))) if index == index_instruction && error_code == expected_error_code => Ok(()),
        Err(RpcError::TransactionError(TransactionError::InstructionError(
            index,
            InstructionError::Custom(error_code),
        ))) if index == index_instruction && error_code == expected_error_code => Ok(()),

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
