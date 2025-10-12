use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::compress_and_close_ctoken_accounts;
use light_sdk_types::cpi_accounts::{v2::CpiAccounts as CpiAccountsSmall, CpiAccountsConfig};

use crate::OneCTokenAccount;

/// Process compress_and_close operation using the higher-level compress_and_close_ctoken_accounts function
/// This demonstrates using the SDK's abstraction for compress and close operations
pub fn process_compress_and_close_cpi<'info>(
    ctx: Context<'_, '_, '_, 'info, OneCTokenAccount<'info>>,
    with_compression_authority: bool,
    system_accounts_offset: u8,
) -> Result<()> {
    // Parse CPI accounts following the established pattern
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_offset as usize);

    let cpi_accounts = CpiAccountsSmall::new_with_config(
        ctx.accounts.signer.as_ref(),
        system_account_infos,
        config,
    );
    // Use the higher-level compress_and_close_ctoken_accounts function
    // This function handles:
    // - Deserializing the compressed token accounts
    // - Extracting rent authority from extensions if needed
    // - Finding all required indices
    // - Building the compress_and_close instruction
    let instruction = compress_and_close_ctoken_accounts(
        *ctx.accounts.signer.key,                          // fee_payer
        with_compression_authority, // whether to use rent authority from extension
        ctx.accounts.output_queue.to_account_info(), // output queue where compressed accounts will be stored
        &[&ctx.accounts.ctoken_account.to_account_info()], // slice of ctoken account infos
        cpi_accounts.tree_accounts().unwrap(),       // packed accounts for the instruction
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Build the account infos for the CPI call
    let account_infos = [
        &[
            cpi_accounts.fee_payer().clone(),
            ctx.accounts.output_queue.to_account_info(),
        ][..],
        ctx.remaining_accounts,
    ]
    .concat();

    // Execute the instruction
    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
