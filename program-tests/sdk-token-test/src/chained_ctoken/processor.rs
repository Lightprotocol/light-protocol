use super::CreateCompressedMint;
use crate::chained_ctoken::create_mint::{
    create_compressed_mint, CreateCompressedMintInstructionData,
};
use crate::chained_ctoken::create_pda::process_create_escrow_pda;
use crate::chained_ctoken::mint_to::{mint_to_compressed, MintToCompressedInstructionData};
use crate::chained_ctoken::update_compressed_mint::{
    update_compressed_mint_cpi_write, UpdateCompressedMintInstructionDataCpi,
};
use anchor_lang::prelude::*;
use light_compressed_token_sdk::ValidityProof;
use light_ctoken_types::instructions::create_compressed_mint::{
    CompressedMintInstructionData, CompressedMintWithContext,
};
use light_ctoken_types::instructions::extensions::{
    ExtensionInstructionData, TokenMetadataInstructionData,
};

use light_ctoken_types::{COMPRESSED_MINT_SEED, COMPRESSED_TOKEN_PROGRAM_ID};
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};

pub fn process_chained_ctoken<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: CreateCompressedMintInstructionData,
    mint_input: MintToCompressedInstructionData,
    update_mint_input: UpdateCompressedMintInstructionDataCpi,
    pda_proof: ValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
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
    let spl_mint: Pubkey = Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            ctx.accounts.mint_seed.key().as_ref(),
            &[input.mint_bump],
        ],
        &COMPRESSED_TOKEN_PROGRAM_ID.into(),
    )
    .unwrap()
    .into();

    let compressed_mint_inputs = CompressedMintWithContext {
        leaf_index: 0, // The mint is created at index 1 in the CPI context
        prove_by_index: true,
        root_index: 0,
        address: input.compressed_mint_address,
        mint: CompressedMintInstructionData {
            version: input.version,
            mint_authority: Some(ctx.accounts.mint_authority.key().into()),
            spl_mint: spl_mint.into(),
            decimals: input.decimals,
            supply: 0,
            is_decompressed: false,
            freeze_authority: input.freeze_authority.map(|f| f.into()),
            extensions: input.metadata.as_ref().map(|metadata| {
                vec![ExtensionInstructionData::TokenMetadata(
                    TokenMetadataInstructionData {
                        update_authority: metadata.update_authority,
                        metadata: metadata.metadata.clone(),
                        additional_metadata: metadata.additional_metadata.clone(),
                        version: metadata.version,
                    },
                )]
            }),
        },
    };
    // First CPI call: create compressed mint
    create_compressed_mint(&ctx, input.clone(), &cpi_accounts)?;

    // Second CPI call: mint to compressed tokens
    mint_to_compressed(
        &ctx,
        mint_input.clone(),
        compressed_mint_inputs.clone(),
        &cpi_accounts,
    )?;

    // Third CPI call: update compressed mint (revoke mint authority)
    // Create updated mint data for the update operation (after minting)
    let updated_compressed_mint_inputs = light_ctoken_types::instructions::create_compressed_mint::CompressedMintWithContext {
        leaf_index: 1, // The mint is at index 1 after being created
        prove_by_index: true,
        root_index: 0,
        address: input.compressed_mint_address,
        mint: light_ctoken_types::instructions::create_compressed_mint::CompressedMintInstructionData {
            version: input.version,
            spl_mint: spl_mint.into(),
            supply: mint_input.recipients.iter().map(|r| r.amount).sum(), // Total supply after minting
            decimals: input.decimals,
            is_decompressed: false,
            mint_authority: Some(ctx.accounts.mint_authority.key().into()), // Current mint authority
            freeze_authority: input.freeze_authority.map(|f| f.into()),
            extensions: input.metadata.as_ref().map(|metadata| {
                vec![light_ctoken_types::instructions::extensions::ExtensionInstructionData::TokenMetadata(
                    light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
                        update_authority: metadata.update_authority,
                        metadata: metadata.metadata.clone(),
                        additional_metadata: metadata.additional_metadata.clone(),
                        version: metadata.version,
                    }
                )]
            }),
        },
    };

    update_compressed_mint_cpi_write(
        &ctx,
        update_mint_input,
        updated_compressed_mint_inputs,
        &cpi_accounts,
    )?;

    // Fourth CPI call: create compressed escrow PDA
    process_create_escrow_pda(
        pda_proof,
        output_tree_index,
        amount,
        address,
        new_address_params,
        cpi_accounts,
    )?;

    Ok(())
}
