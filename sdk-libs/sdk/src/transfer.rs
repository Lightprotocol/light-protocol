use anchor_lang::Result;

use crate::{account_info::LightAccountInfo, error::LightSdkError};

/// Transfers a specified amount of lamports from one account to another.
///
/// Attempts to transfer `lamports` from the `from` account to the `to`
/// account. It will update the lamport balances of both accounts if the
/// transfer is successful.
pub fn transfer_compressed_sol(
    from: &mut LightAccountInfo,
    to: &mut LightAccountInfo,
    lamports: u64,
) -> Result<()> {
    let output_from = from
        .input
        .as_ref()
        .ok_or(LightSdkError::TransferFromNoInput)?
        .lamports
        .ok_or(LightSdkError::TransferFromNoLamports)?
        .checked_sub(lamports)
        .ok_or(LightSdkError::TransferFromInsufficientLamports)?;
    let output_to = to
        .input
        .as_ref()
        .and_then(|input| input.lamports)
        .unwrap_or(0)
        .checked_add(lamports)
        .ok_or(LightSdkError::TransferIntegerOverflow)?;

    from.lamports = Some(output_from);
    to.lamports = Some(output_to);

    Ok(())
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;

    use super::*;
    use crate::{account_info::LightInputAccountInfo, merkle_context::PackedMerkleContext};

    /// Creates a mock account with the given input lamports.
    fn mock_account(owner: &Pubkey, lamports: Option<u64>) -> LightAccountInfo<'_> {
        LightAccountInfo {
            input: Some(LightInputAccountInfo {
                lamports,

                // None of the following values matter.
                address: Some([1; 32]),
                data: Some(b"ayy"),
                data_hash: Some([0; 32]),
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                    queue_index: false,
                },
                root_index: 0,
            }),
            owner,
            // None of the following values matter.
            lamports: None,
            discriminator: Some([0; 8]),
            data: None,
            data_hash: None,
            address: Some([1; 32]),
            output_merkle_tree_index: None,
            new_address_params: None,
        }
    }

    /// Creates a mock account without input.
    fn mock_account_without_input(owner: &Pubkey) -> LightAccountInfo<'_> {
        LightAccountInfo {
            input: None,
            owner,
            // None of the following values matter.
            lamports: None,
            discriminator: Some([0; 8]),
            data: None,
            data_hash: None,
            address: Some([1; 32]),
            output_merkle_tree_index: None,
            new_address_params: None,
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
        assert_eq!(from.lamports, Some(700));
        assert_eq!(to.lamports, Some(800));
    }

    #[test]
    fn test_transfer_from_no_input() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account_without_input(&from_pubkey);
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert_eq!(result, Err(LightSdkError::TransferFromNoInput.into()));
    }

    #[test]
    fn test_transfer_from_no_lamports() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, None);
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert_eq!(result, Err(LightSdkError::TransferFromNoLamports.into()));
    }

    #[test]
    fn test_transfer_insufficient_lamports() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(200));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(500));

        let result = transfer_compressed_sol(&mut from, &mut to, 300);
        assert_eq!(
            result,
            Err(LightSdkError::TransferFromInsufficientLamports.into())
        );
    }

    #[test]
    fn test_transfer_integer_overflow() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(1000));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, Some(u64::MAX - 500));

        let result = transfer_compressed_sol(&mut from, &mut to, 600);
        assert_eq!(result, Err(LightSdkError::TransferIntegerOverflow.into()));
    }

    #[test]
    fn test_transfer_to_no_lamports() {
        let from_pubkey = Pubkey::new_unique();
        let mut from = mock_account(&from_pubkey, Some(1000));
        let to_pubkey = Pubkey::new_unique();
        let mut to = mock_account(&to_pubkey, None);

        let result = transfer_compressed_sol(&mut from, &mut to, 500);
        assert!(result.is_ok());
        assert_eq!(from.lamports, Some(500));
        assert_eq!(to.lamports, Some(500));
    }
}
