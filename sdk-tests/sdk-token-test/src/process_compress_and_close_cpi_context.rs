use crate::{
    mint_compressed_tokens_cpi_write::{
        process_mint_compressed_tokens_cpi_write, MintCompressedTokensCpiWriteParams,
    },
    Generic,
};
use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::compress_and_close::{
    compress_and_close_ctoken_accounts_with_indices, CompressAndCloseIndices,
};
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};

/// Process compress_and_close operation using the new CompressAndClose mode with manual indices
/// This combines token compression and account closure in a single atomic transaction
pub fn process_compress_and_close_cpi_context<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    indices: Vec<CompressAndCloseIndices>,
    system_accounts_offset: u8,
    params: MintCompressedTokensCpiWriteParams,
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

    process_mint_compressed_tokens_cpi_write(
        &ctx,
        params,
        &_token_account_infos[1], // ctoken cpi authority
        &cpi_accounts,
    )?;

    // Get the tree accounts (packed accounts) from CPI accounts
    let packed_accounts = cpi_accounts
        .tree_accounts()
        .map_err(|e| ProgramError::Custom(e.into()))?;
    msg!(
        "packed_accounts {:?}",
        packed_accounts
            .iter()
            .map(|x| { x.key })
            .collect::<Vec<_>>()
    );
    // Use the SDK's compress_and_close function with the provided indices
    let instruction = compress_and_close_ctoken_accounts_with_indices(
        *ctx.accounts.signer.key,
        Some(*cpi_accounts.cpi_context().unwrap().key), // Use the CPI context from params
        &indices,
        &packed_accounts[1..], // skip cpi context account
    )
    .map_err(ProgramError::from)?;

    // Execute the single instruction that handles both compression and closure
    let account_infos = [
        &[cpi_accounts.fee_payer().clone()][..],
        ctx.remaining_accounts,
    ]
    .concat();

    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
