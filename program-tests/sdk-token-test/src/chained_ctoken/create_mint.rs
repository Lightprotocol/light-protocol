use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use light_compressed_token_sdk::instructions::instruction::{
    create_compressed_mint_cpi_write, CreateCompressedMintInputsCpiWrite,
};

use super::CreateCompressedMint;
use crate::LIGHT_CPI_SIGNER;
use light_compressed_token_sdk::instructions::create_compressed_mint::CpiContextWriteAccounts;
use light_compressed_token_sdk::{CompressedCpiContext, CompressedProof};
use light_ctoken_types::instructions::extensions::{
    ExtensionInstructionData, TokenMetadataInstructionData,
};
use light_sdk_types::CpiAccountsSmall;

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CreateCompressedMintInstructionData {
    pub decimals: u8,
    pub freeze_authority: Option<Pubkey>,
    pub proof: CompressedProof,
    pub mint_bump: u8,
    pub address_merkle_tree_root_index: u16,
    pub version: u8,
    pub metadata: Option<TokenMetadataInstructionData>,
    pub compressed_mint_address: [u8; 32],
}

pub fn create_compressed_mint<'a, 'b, 'c, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: CreateCompressedMintInstructionData,
    cpi_accounts: &CpiAccountsSmall<'a, AccountInfo<'info>>,
) -> Result<()> {
    let cpi_context_account_info = CpiContextWriteAccounts {
        mint_signer: ctx.accounts.mint_seed.as_ref(),
        light_system_program: cpi_accounts.system_program().unwrap(),
        fee_payer: ctx.accounts.payer.as_ref(),
        cpi_authority_pda: ctx.accounts.ctoken_cpi_authority.as_ref(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: LIGHT_CPI_SIGNER,
    };
    msg!("cpi_context_account_info {:?}", cpi_context_account_info);
    let create_mint_inputs = CreateCompressedMintInputsCpiWrite {
        mint_bump: input.mint_bump,
        address_merkle_tree_root_index: input.address_merkle_tree_root_index,
        version: input.version,
        decimals: input.decimals,
        extensions: input
            .metadata
            .map(|metadata| vec![ExtensionInstructionData::TokenMetadata(metadata)]),
        freeze_authority: input.freeze_authority,
        mint_authority: ctx.accounts.mint_authority.key(),
        proof: input.proof,
        mint_signer: *ctx.accounts.mint_seed.key,
        payer: ctx.accounts.payer.key(),
        mint_address: input.compressed_mint_address,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index: 0,
        },
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
    };

    let create_mint_instruction =
        create_compressed_mint_cpi_write(create_mint_inputs).map_err(ProgramError::from)?;
    msg!("create_mint_instruction: {:?}", create_mint_instruction);
    // Execute the CPI call to create the compressed mint
    invoke(
        &create_mint_instruction,
        &cpi_context_account_info.to_account_infos(),
    )?;

    Ok(())
}

#[error_code]
pub enum CreateCompressedMintErrorCode {
    #[msg("Token name cannot be empty")]
    InvalidTokenName,
    #[msg("Token symbol cannot be empty")]
    InvalidTokenSymbol,
    #[msg("Token URI cannot be empty")]
    InvalidTokenUri,
    #[msg("Decimals must be between 0 and 9")]
    InvalidDecimals,
    #[msg("Invalid proof provided")]
    InvalidProof,
}
