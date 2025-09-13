use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    compress_and_close::{
        compress_and_close_ctoken_accounts_with_indices, CompressAndCloseIndices,
    },
    transfer2::Transfer2CpiAccounts,
};

use crate::{
    mint_compressed_tokens_cpi_write::{
        process_mint_compressed_tokens_cpi_write, MintCompressedTokensCpiWriteParams,
    },
    Generic,
};

/// Process compress_and_close operation using the new CompressAndClose mode with manual indices
/// This combines token compression and account closure in a single atomic transaction
pub fn process_compress_and_close_cpi_context<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    indices: Vec<CompressAndCloseIndices>,
    params: MintCompressedTokensCpiWriteParams,
) -> Result<()> {
    // Now use Transfer2CpiAccounts for compress_and_close
    let transfer2_accounts = Transfer2CpiAccounts::try_from_account_infos_cpi_context(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    process_mint_compressed_tokens_cpi_write(&ctx, params, &transfer2_accounts)?;

    // Get the packed accounts from Transfer2CpiAccounts
    let packed_accounts = transfer2_accounts.packed_accounts();

    // Use the SDK's compress_and_close function with the provided indices
    let instruction = compress_and_close_ctoken_accounts_with_indices(
        *ctx.accounts.signer.key,
        false,
        transfer2_accounts.cpi_context.map(|c| c.key()), // Use the CPI context from Transfer2CpiAccounts
        &indices,
        packed_accounts, // Pass complete packed accounts
    )
    .map_err(ProgramError::from)?;

    // Use Transfer2CpiAccounts to build account infos for invoke
    invoke(
        &instruction,
        transfer2_accounts.to_account_infos().as_slice(),
    )?;

    Ok(())
}
