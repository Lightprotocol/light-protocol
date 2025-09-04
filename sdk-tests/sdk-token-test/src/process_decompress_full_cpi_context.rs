use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::decompress_full::{
    decompress_full_ctoken_accounts_with_indices, DecompressFullIndices,
};
use light_sdk_types::CpiAccountsSmall;

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
    system_accounts_offset: u8,
    params: Option<MintCompressedTokensCpiWriteParams>,
) -> Result<()> {
    // Parse CPI accounts following the established pattern
    let config = if params.is_some() {
        light_sdk_types::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER)
    } else {
        light_sdk_types::CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER)
    };
    let (token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_offset as usize);

    let cpi_accounts = CpiAccountsSmall::new_with_config(
        ctx.accounts.signer.as_ref(),
        system_account_infos,
        config,
    );
    let cpi_context = if params.is_some() {
        Some(*cpi_accounts.cpi_context().unwrap().key)
    } else {
        None
    };
    // If minting params are provided, mint first (optional)
    if let Some(params) = params {
        process_mint_compressed_tokens_cpi_write(
            &ctx,
            params,
            &token_account_infos[1], // ctoken cpi authority at index 1
            &cpi_accounts,
        )?;
    }

    // Get the tree accounts (packed accounts) from CPI accounts
    let packed_accounts = cpi_accounts
        .tree_accounts()
        .map_err(|e| ProgramError::Custom(e.into()))?;

    let instruction = decompress_full_ctoken_accounts_with_indices(
        *ctx.accounts.signer.key,
        validity_proof,
        cpi_context,
        &indices,
        packed_accounts,
    )
    .map_err(ProgramError::from)?;

    // Execute the single instruction that handles full decompression
    let account_infos = [
        &[cpi_accounts.fee_payer().clone()][..],
        ctx.remaining_accounts,
    ]
    .concat();

    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
