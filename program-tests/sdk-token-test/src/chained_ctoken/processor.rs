use super::CreateCompressedMint;
use crate::chained_ctoken::create_mint::{
    create_compressed_mint, CreateCompressedMintInstructionData,
};
use crate::chained_ctoken::mint_to::{mint_to_compressed, MintToCompressedInstructionData};
use anchor_lang::prelude::*;
use light_compressed_token_sdk::CompressedProof;
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};

pub fn process_chained_ctoken<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: CreateCompressedMintInstructionData,
    mint_input: MintToCompressedInstructionData,
    _proof: CompressedProof,
) -> Result<()> {
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: true,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };

    let cpi_accounts = CpiAccountsSmall::new_with_config(
        ctx.accounts.payer.as_ref(),
        ctx.remaining_accounts,
        config,
    );

    // First CPI call: create compressed mint
    create_compressed_mint(&ctx, input, &cpi_accounts)?;

    // Second CPI call: mint to compressed tokens
    mint_to_compressed(&ctx, mint_input, &cpi_accounts)?;

    Ok(())
}
