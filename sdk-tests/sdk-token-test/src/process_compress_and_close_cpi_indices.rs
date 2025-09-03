use crate::Generic;
use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Inputs,
    },
};
use light_sdk::cpi::{CpiAccounts, CpiAccountsExt};
use light_sdk_types::CpiAccountsConfig;

/// Process compress_and_close operation using the new CompressAndClose mode with manual indices
/// This combines token compression and account closure in a single atomic transaction
pub fn process_compress_and_close_cpi_indices<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    output_tree_index: u8,
    recipient_index: u8,
    mint_index: u8,
    source_index: u8,
    authority_index: u8,
    rent_recipient_index: u8,
    system_accounts_offset: u8,
) -> Result<()> {
    // Parse CPI accounts following the established pattern
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_offset as usize);

    let cpi_accounts =
        CpiAccounts::new_with_config(ctx.accounts.signer.as_ref(), system_account_infos, config);

    // Get the source token account info for amount extraction
    let token_account_info = cpi_accounts
        .get_tree_account_info(source_index as usize)
        .map_err(|_| {
            msg!(
                "Invalid source_index: {} not found in tree accounts",
                source_index
            );
            ProgramError::InvalidInstructionData
        })?;

    // Create CTokenAccount2 for CompressAndClose operation
    let mut token_account =
        CTokenAccount2::new_empty(recipient_index, mint_index, output_tree_index);

    // Use the new compress_and_close method
    // This will compress the full balance and mark the account for closure
    // The amount parameter is the full balance of the account (extracted by the SDK)
    let amount = {
        // Parse the token account to get its balance
        use light_ctoken_types::state::CompressedToken;
        use light_zero_copy::traits::ZeroCopyAt;

        let account_data = token_account_info.try_borrow_data().map_err(|_| {
            msg!("Failed to borrow account data");
            ProgramError::AccountBorrowFailed
        })?;

        let (compressed_token, _) = CompressedToken::zero_copy_at(&account_data).map_err(|e| {
            msg!("Failed to parse compressed token: {:?}", e);
            ProgramError::InvalidAccountData
        })?;
        u64::from(*compressed_token.amount)
    };

    token_account
        .compress_and_close(
            amount,               // Full balance to compress
            source_index,         // Source token account index
            authority_index,      // Authority (owner or rent authority)
            rent_recipient_index, // Rent recipient index
        )
        .map_err(ProgramError::from)?;

    // Get packed accounts from CpiAccounts directly
    let packed_accounts = cpi_accounts
        .get_packed_account_metas()
        .map_err(ProgramError::from)?;

    // Create the transfer2 instruction with CompressAndClose
    let inputs = Transfer2Inputs {
        meta_config: Transfer2AccountsMetaConfig::new(*ctx.accounts.signer.key, packed_accounts),
        token_accounts: vec![token_account],
        ..Default::default()
    };

    let instruction = create_transfer2_instruction(inputs).map_err(ProgramError::from)?;

    // Execute the single instruction that handles both compression and closure
    let account_infos = [
        &[cpi_accounts.fee_payer().clone()][..],
        ctx.remaining_accounts,
    ]
    .concat();

    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
