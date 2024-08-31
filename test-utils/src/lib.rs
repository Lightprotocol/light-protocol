use forester_utils::rpc::errors::RpcError;
use solana_sdk::{instruction::InstructionError, transaction};

pub mod address_tree_rollover;
pub mod assert_address_merkle_tree;
pub mod assert_compressed_tx;
pub mod assert_epoch;
pub mod assert_merkle_tree;
pub mod assert_queue;
pub mod assert_rollover;
pub mod assert_token_tx;
pub mod e2e_test_env;
#[allow(unused)]
pub mod indexer;
pub mod rpc;
pub mod spl;
pub mod state_tree_rollover;
pub mod system_program;
pub mod test_env;
#[allow(unused)]
pub mod test_forester;

/// Asserts that the given `BanksTransactionResultWithMetadata` is an error with a custom error code
/// or a program error.
/// Unfortunately BanksTransactionResultWithMetadata does not reliably expose the custom error code, so
/// we allow program error as well.
// TODO: unify with assert_rpc_error
pub fn assert_custom_error_or_program_error(
    result: Result<solana_sdk::signature::Signature, RpcError>,
    error_code: u32,
) -> Result<(), RpcError> {
    let accepted_errors = [
        (0, InstructionError::ProgramFailedToComplete),
        (0, InstructionError::Custom(error_code)),
    ];

    let is_accepted = accepted_errors.iter().any(|(index, error)| {
        matches!(result, Err(RpcError::TransactionError(transaction::TransactionError::InstructionError(i, ref e))) if i == (*index as u8) && e == error)
    });

    if !is_accepted {
        println!("result {:?}", result);
        println!("error_code {:?}", error_code);
        return Err(RpcError::AssertRpcError(format!(
            "Expected error code {} or program error, got {:?}",
            error_code, result
        )));
    }

    Ok(())
}
