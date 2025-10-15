use anchor_lang::prelude::*;
use light_compressed_token_sdk::{instructions::mint_action::MintToRecipient, ValidityProof};
use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;

use super::{
    create_pda::process_create_escrow_pda_with_cpi_context, mint::process_mint_action, PdaCToken,
};
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct ChainedCtokenInstructionData {
    pub compressed_mint_with_context: CompressedMintWithContext,
    pub mint_bump: u8,
    pub token_recipients: Vec<MintToRecipient>,
    pub final_mint_authority: Option<Pubkey>,
    pub pda_creation: PdaCreationData,
    pub output_tree_index: u8,
    pub new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PdaCreationData {
    pub amount: u64,
    pub address: [u8; 32],
    pub proof: ValidityProof,
}
// TODO: remove mint to compressed
// TODO: create a second ix which switches the cpis.
use light_sdk_types::cpi_accounts::{v2::CpiAccounts as CpiAccountsSmall, CpiAccountsConfig};
pub fn process_pda_ctoken<'info>(
    ctx: Context<'_, '_, '_, 'info, PdaCToken<'info>>,
    input: ChainedCtokenInstructionData,
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
    process_create_escrow_pda_with_cpi_context(
        input.pda_creation.amount,
        input.pda_creation.address,
        input.new_address_params,
        &cpi_accounts,
    )?;

    process_mint_action(&ctx, &input, &cpi_accounts)?;

    Ok(())
}
