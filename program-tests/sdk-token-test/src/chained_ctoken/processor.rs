use super::CreateCompressedMint;
use crate::chained_ctoken::create_pda::process_create_escrow_pda;
use crate::mint::process_mint_action;
use anchor_lang::prelude::*;
use light_compressed_token_sdk::instructions::mint_action::MintToRecipient;

use light_compressed_token_sdk::ValidityProof;
use light_ctoken_types::instructions::create_compressed_mint::CompressedMintWithContext;
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct ChainedCtokenInstructionData {
    pub compressed_mint_with_context: CompressedMintWithContext,
    pub mint_bump: u8,
    pub token_recipients: Vec<MintToRecipient>,
    pub lamports: Option<u64>,
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

use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};
pub fn process_chained_ctoken<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
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

    process_mint_action(&ctx, &input, &cpi_accounts)?;

    process_create_escrow_pda(
        input.pda_creation.proof,
        input.output_tree_index,
        input.pda_creation.amount,
        input.pda_creation.address,
        input.new_address_params,
        cpi_accounts,
    )?;

    Ok(())
}
