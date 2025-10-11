use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    compress_and_close::{
        compress_and_close_ctoken_accounts_with_indices, CompressAndCloseIndices,
    },
    transfer2::Transfer2CpiAccounts,
};

use crate::Generic;

/// Process compress_and_close operation using the new CompressAndClose mode with manual indices
/// This combines token compression and account closure in a single atomic transaction
pub fn process_compress_and_close_cpi_indices<'info>(
    ctx: Context<'_, '_, 'info, 'info, Generic<'info>>,
    indices: Vec<CompressAndCloseIndices>,
    _system_accounts_offset: u8,
) -> Result<()> {
    let fee_payer = ctx.accounts.signer.to_account_info();
    // Use the new Transfer2CpiAccounts to parse accounts
    let transfer2_accounts =
        Transfer2CpiAccounts::try_from_account_infos(&fee_payer, ctx.remaining_accounts)
            .map_err(|_| ProgramError::InvalidAccountData)?;
    msg!("transfer2_accounts {:?}", transfer2_accounts);
    // Get the packed accounts from the parsed structure
    let packed_accounts = transfer2_accounts.packed_accounts();

    // Use the SDK's compress_and_close function with the provided indices
    // Use the signer from ctx.accounts as fee_payer since it's passed separately in the test
    let instruction = compress_and_close_ctoken_accounts_with_indices(
        *ctx.accounts.signer.key,
        false,
        None, // cpi_context_pubkey
        &indices,
        packed_accounts,
    )
    .map_err(ProgramError::from)?;

    invoke(
        &instruction,
        transfer2_accounts.to_account_infos().as_slice(),
    )?;

    Ok(())
}
