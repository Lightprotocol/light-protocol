use super::CreateCompressedMint;
use crate::chained_ctoken::create_mint::{
    create_compressed_mint, CreateCompressedMintInstructionData,
};
use anchor_lang::prelude::*;
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};

pub fn process_chained_ctoken<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: CreateCompressedMintInstructionData,
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
    create_compressed_mint(&ctx, input, &cpi_accounts)?;
    Ok(())
}
