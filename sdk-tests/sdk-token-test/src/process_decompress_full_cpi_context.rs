use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    decompress_full::{decompress_full_ctoken_accounts_with_indices, DecompressFullIndices},
    transfer2::Transfer2CpiAccounts,
};

use crate::{
    mint_compressed_tokens_cpi_write::{
        process_mint_compressed_tokens_cpi_write, MintCompressedTokensCpiWriteParams,
    },
    Generic,
};

/// Process decompress_full operation using the new DecompressFull mode with manual indices
/// This decompresses the full balance of compressed tokens to decompressed ctoken accounts
pub fn process_decompress_full_cpi_context<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    indices: Vec<DecompressFullIndices>,
    validity_proof: light_compressed_token_sdk::ValidityProof,
    params: Option<MintCompressedTokensCpiWriteParams>,
) -> Result<()> {
    // Parse CPI accounts following the established pattern
    let cpi_accounts = Transfer2CpiAccounts::try_from_account_infos_full(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        false,
        false,
        params.is_some(),
        false,
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    // If minting params are provided, mint first (optional)
    if let Some(params) = params {
        process_mint_compressed_tokens_cpi_write(&ctx, params, &cpi_accounts)?;
    }

    let instruction = decompress_full_ctoken_accounts_with_indices(
        *ctx.accounts.signer.key,
        validity_proof,
        cpi_accounts.cpi_context.map(|x| *x.key),
        &indices,
        cpi_accounts.packed_accounts(),
    )
    .map_err(ProgramError::from)?;

    invoke(&instruction, cpi_accounts.to_account_infos().as_slice())?;

    Ok(())
}
