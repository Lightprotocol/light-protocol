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

        // Handle built-in Solana errors (non-Custom) - TransactionError variants
        Err(RpcError::TransactionError(TransactionError::InstructionError(index, ref err)))
            if index == index_instruction =>
        {
            match (err, expected_error_code) {
                (InstructionError::GenericError, 0) => Ok(()),
                (InstructionError::InvalidArgument, 1) => Ok(()),
                (InstructionError::InvalidInstructionData, 2) => Ok(()),
                (InstructionError::InvalidAccountData, 3) => Ok(()),
                (InstructionError::AccountDataTooSmall, 5) => Ok(()),
                (InstructionError::InsufficientFunds, 6) => Ok(()),
                (InstructionError::IncorrectProgramId, 7) => Ok(()),
                (InstructionError::MissingRequiredSignature, 8) => Ok(()),
                (InstructionError::AccountAlreadyInitialized, 9) => Ok(()),
                (InstructionError::UninitializedAccount, 10) => Ok(()),
                (InstructionError::NotEnoughAccountKeys, 11) => Ok(()),
                (InstructionError::AccountBorrowFailed, 12) => Ok(()),
                (InstructionError::MaxSeedLengthExceeded, 13) => Ok(()),
                (InstructionError::InvalidSeeds, 14) => Ok(()),
                (InstructionError::BorshIoError(_), 15) => Ok(()),
                (InstructionError::AccountNotRentExempt, 16) => Ok(()),
                (InstructionError::InvalidRealloc, 17) => Ok(()),
                (InstructionError::ComputationalBudgetExceeded, 18) => Ok(()),
                (InstructionError::PrivilegeEscalation, 19) => Ok(()),
                (InstructionError::ProgramEnvironmentSetupFailure, 20) => Ok(()),
                (InstructionError::ProgramFailedToComplete, 21) => Ok(()),
                (InstructionError::ProgramFailedToCompile, 22) => Ok(()),
                (InstructionError::Immutable, 23) => Ok(()),
                (InstructionError::IncorrectAuthority, 24) => Ok(()),
                (InstructionError::AccountNotExecutable, 25) => Ok(()),
                (InstructionError::InvalidAccountOwner, 26) => Ok(()),
                (InstructionError::ArithmeticOverflow, 27) => Ok(()),
                (InstructionError::UnsupportedSysvar, 28) => Ok(()),
                (InstructionError::IllegalOwner, 29) => Ok(()),
                (InstructionError::MaxAccountsDataAllocationsExceeded, 30) => Ok(()),
                (InstructionError::MaxAccountsExceeded, 31) => Ok(()),
                (InstructionError::MaxInstructionTraceLengthExceeded, 32) => Ok(()),
                (InstructionError::BuiltinProgramsMustConsumeComputeUnits, 33) => Ok(()),
                _ => Err(RpcError::AssertRpcError(format!(
                    "Expected error code {}, but got {:?}",
                    expected_error_code, err
                ))),
            }
        }

        // Handle built-in Solana errors (non-Custom) - BanksClientError variants
        Err(RpcError::BanksError(BanksClientError::TransactionError(
            TransactionError::InstructionError(index, ref err),
        ))) if index == index_instruction => match (err, expected_error_code) {
            (InstructionError::GenericError, 0) => Ok(()),
            (InstructionError::InvalidArgument, 1) => Ok(()),
            (InstructionError::InvalidInstructionData, 2) => Ok(()),
            (InstructionError::InvalidAccountData, 3) => Ok(()),
            (InstructionError::AccountDataTooSmall, 5) => Ok(()),
            (InstructionError::InsufficientFunds, 6) => Ok(()),
            (InstructionError::IncorrectProgramId, 7) => Ok(()),
            (InstructionError::MissingRequiredSignature, 8) => Ok(()),
            (InstructionError::AccountAlreadyInitialized, 9) => Ok(()),
            (InstructionError::UninitializedAccount, 10) => Ok(()),
            (InstructionError::NotEnoughAccountKeys, 11) => Ok(()),
            (InstructionError::AccountBorrowFailed, 12) => Ok(()),
            (InstructionError::MaxSeedLengthExceeded, 13) => Ok(()),
            (InstructionError::InvalidSeeds, 14) => Ok(()),
            (InstructionError::BorshIoError(_), 15) => Ok(()),
            (InstructionError::AccountNotRentExempt, 16) => Ok(()),
            (InstructionError::InvalidRealloc, 17) => Ok(()),
            (InstructionError::ComputationalBudgetExceeded, 18) => Ok(()),
            (InstructionError::PrivilegeEscalation, 19) => Ok(()),
            (InstructionError::ProgramEnvironmentSetupFailure, 20) => Ok(()),
            (InstructionError::ProgramFailedToComplete, 21) => Ok(()),
            (InstructionError::ProgramFailedToCompile, 22) => Ok(()),
            (InstructionError::Immutable, 23) => Ok(()),
            (InstructionError::IncorrectAuthority, 24) => Ok(()),
            (InstructionError::AccountNotExecutable, 25) => Ok(()),
            (InstructionError::InvalidAccountOwner, 26) => Ok(()),
            (InstructionError::ArithmeticOverflow, 27) => Ok(()),
            (InstructionError::UnsupportedSysvar, 28) => Ok(()),
            (InstructionError::IllegalOwner, 29) => Ok(()),
            (InstructionError::MaxAccountsDataAllocationsExceeded, 30) => Ok(()),
            (InstructionError::MaxAccountsExceeded, 31) => Ok(()),
            (InstructionError::MaxInstructionTraceLengthExceeded, 32) => Ok(()),
            (InstructionError::BuiltinProgramsMustConsumeComputeUnits, 33) => Ok(()),
            _ => Err(RpcError::AssertRpcError(format!(
                "Expected error code {}, but got {:?}",
                expected_error_code, err
            ))),
        },

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
