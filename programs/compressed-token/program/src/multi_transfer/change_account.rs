use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;

use crate::multi_transfer::{
    accounts::MultiTransferPackedAccounts,
    instruction_data::ZCompressedTokenInstructionDataMultiTransfer,
};

/// Create a change account for excess lamports (following anchor program pattern)
pub fn assign_change_account(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
    change_lamports: u64,
) -> Result<(), ProgramError> {
    // Find the next available output account slot
    let current_output_count = inputs.out_token_data.len();

    // Get the change account slot (should be pre-allocated by CPI config)
    let change_account = cpi_instruction_struct
        .output_compressed_accounts
        .get_mut(current_output_count)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Get merkle tree index - use specified index
    let merkle_tree_index = if inputs.with_lamports_change_account_merkle_tree_index != 0 {
        inputs.lamports_change_account_merkle_tree_index
    } else {
        return Err(ProgramError::InvalidInstructionData);
    };

    // Get the owner account using the specified index
    let owner_account = packed_accounts.get_u8(inputs.lamports_change_account_owner_index)?;
    let owner_pubkey = *owner_account.key();

    // Set up the change account as a lamports-only account (no token data)
    let compressed_account = &mut change_account.compressed_account;

    // Set owner from the specified account index
    compressed_account.owner = owner_pubkey.into();

    // Set lamports amount
    compressed_account.lamports.set(change_lamports);

    // No token data for change account

    if compressed_account.data.is_some() {
        unimplemented!("lamports change account shouldn't have data.")
    }

    // Set merkle tree index
    *change_account.merkle_tree_index = merkle_tree_index;

    Ok(())
}

pub fn process_change_lamports(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer<'_>,
    packed_accounts: &MultiTransferPackedAccounts<'_>,
    mut cpi_instruction_struct: ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    total_input_lamports: u64,
    total_output_lamports: u64,
) -> Result<(), ProgramError> {
    if total_input_lamports != total_output_lamports {
        let (change_lamports, is_compress) = if total_input_lamports > total_output_lamports {
            (
                total_input_lamports.saturating_sub(total_output_lamports),
                0,
            )
        } else {
            (
                total_output_lamports.saturating_sub(total_input_lamports),
                1,
            )
        };
        // Set CPI instruction fields for compression/decompression
        cpi_instruction_struct
            .compress_or_decompress_lamports
            .set(change_lamports);
        cpi_instruction_struct.is_compress = is_compress;
        // Create change account with the lamports difference
        assign_change_account(
            &mut cpi_instruction_struct,
            inputs,
            packed_accounts,
            change_lamports,
        )?;
    }

    Ok(())
}
