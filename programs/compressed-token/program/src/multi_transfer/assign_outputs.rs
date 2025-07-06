use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;

use crate::{
    multi_transfer::{
        accounts::MultiTransferPackedAccounts,
        instruction_data::ZCompressedTokenInstructionDataMultiTransfer,
    },
    shared::{context::TokenContext, outputs::create_output_compressed_account},
};

/// Process output compressed accounts and return total output lamports
pub fn assign_output_compressed_accounts(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    context: &mut TokenContext,
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
) -> Result<u64, ProgramError> {
    let mut total_output_lamports = 0u64;

    for (i, output_data) in inputs.out_token_data.iter().enumerate() {
        let output_lamports = if let Some(lamports) = inputs.out_lamports.as_ref() {
            if let Some(lamports) = lamports.get(i) {
                lamports.get()
            } else {
                0
            }
        } else {
            0
        };

        total_output_lamports += output_lamports;

        let mint_index = output_data.mint;
        let mint_account = packed_accounts.get_u8(mint_index)?;
        let hashed_mint = context.get_or_hash_pubkey(mint_account.key);

        // Get owner account using owner index
        let owner_account = packed_accounts.get_u8(output_data.owner)?;
        let owner_pubkey = *owner_account.key;

        // Get delegate if present
        let delegate_pubkey = if output_data.delegate != 0 {
            let delegate_account = packed_accounts.get_u8(output_data.delegate)?;
            Some(*delegate_account.key)
        } else {
            None
        };

        create_output_compressed_account(
            cpi_instruction_struct
                .output_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            context,
            owner_pubkey.into(),
            delegate_pubkey.map(|d| d.into()),
            output_data.amount,
            if output_lamports > 0 {
                Some(output_lamports)
            } else {
                None
            },
            mint_account.key.into(),
            &hashed_mint,
            output_data.merkle_tree,
        )?;
    }

    Ok(total_output_lamports)
}
