use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;

use crate::error::{LightSdkError, Result};

/// Transfers a specified amount of lamports from one account to another.
///
/// Attempts to transfer `lamports` from the `from` account to the `to`
/// account. It will update the lamport balances of both accounts if the
/// transfer is successful.
pub fn transfer_compressed_sol(
    from: &mut CompressedAccountInfo,
    to: &mut CompressedAccountInfo,
    lamports: u64,
) -> Result<()> {
    if let Some(output) = from.output.as_mut() {
        output.lamports = output
            .lamports
            .checked_sub(lamports)
            .ok_or(LightSdkError::TransferFromInsufficientLamports)?;
    }
    // Issue:
    // - we must not modify the balance of the input account since we need the correct value
    //  to verify the proof.
    // - If an account has no output compressed account have to transfer all lamports from it.
    // - However the account does not have an output balance to measure the difference.
    // - We could solve this by using the output balance anyway but skipping output values
    //      which only use lamports and no data (program owned compressed must have data).
    // else if let Some(input) = from.input.as_mut() {
    //     input.lamports = input
    //         .lamports
    //         .checked_sub(lamports)
    //         .ok_or(LightSdkError::TransferFromInsufficientLamports)?
    // }
    else {
        return Err(LightSdkError::TransferFromNoLamports);
    };

    if let Some(output) = to.output.as_mut() {
        output.lamports = output
            .lamports
            .checked_add(lamports)
            .ok_or(LightSdkError::TransferIntegerOverflow)?;
    } else {
        return Err(LightSdkError::TransferFromNoLamports);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use light_compressed_account::{
        compressed_account::PackedMerkleContext,
        instruction_data::with_account_info::{
            CompressedAccountInfo, InAccountInfo, OutAccountInfo,
        },
    };

    use super::*;
    use crate::Pubkey;

    /// Creates a mock account with the given input lamports.
    fn mock_account(_owner: &Pubkey, lamports: Option<u64>) -> CompressedAccountInfo {
        let input_lamports = lamports.unwrap_or(0);
        CompressedAccountInfo {
            input: Some(InAccountInfo {
                lamports: input_lamports,
                // None of the following values matter.
                data_hash: [0; 32],
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 0,
                    leaf_index: 0,
                    prove_by_index: false,
                },
                // None of the following values matter.
                discriminator: [0; 8],
                root_index: 0,
            }),
            output: Some(OutAccountInfo {
                lamports: input_lamports,
                // None of the following values matter.
                data_hash: [0; 32],
                data: Vec::new(),
                output_merkle_tree_index: 0,
                // None of the following values matter.
                discriminator: [0; 8],
            }),
            address: Some([1; 32]),
        }
    }

    /// Creates a mock account without input.
    fn mock_account_without_input(_owner: &Pubkey) -> CompressedAccountInfo {
        CompressedAccountInfo {
            input: None,
            output: Some(OutAccountInfo {
                lamports: 0,
                // None of the following values matter.
                data_hash: [0; 32],
                data: Vec::new(),
                output_merkle_tree_index: 0,
                // None of the following values matter.
                discriminator: [0; 8],
            }),
            address: Some([1; 32]),
        }
    }

    #[test]
    fn test_transfer_success() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(1000));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert!(result.is_ok());
        assert_eq!(from.output.as_ref().unwrap().lamports, 700);
        assert_eq!(to.output.as_ref().unwrap().lamports, 800);
    }

    #[test]
    fn test_transfer_from_no_input() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account_without_input(&from_pubkey);
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert_eq!(result, Err(LightSdkError::TransferFromInsufficientLamports));
    }

    #[test]
    fn test_transfer_from_no_lamports() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, None);
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert_eq!(result, Err(LightSdkError::TransferFromInsufficientLamports));
    }

    #[test]
    fn test_transfer_insufficient_lamports() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(200));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert_eq!(result, Err(LightSdkError::TransferFromInsufficientLamports));
    }

    #[test]
    fn test_transfer_integer_overflow() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(1000));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(u64::MAX - 500));

        let result = transfer_compressed_sol(&mut from, &mut to, 600);
        assert_eq!(result, Err(LightSdkError::TransferIntegerOverflow));
    }

    #[test]
    fn test_transfer_to_no_lamports() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(1000));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(0));
        to.output.as_mut().unwrap().lamports = 0;

        let result = transfer_compressed_sol(&mut from, &mut to, 500);
        assert!(result.is_ok());
        assert_eq!(from.output.as_ref().unwrap().lamports, 500);
        assert_eq!(to.output.as_ref().unwrap().lamports, 500);
    }
}
