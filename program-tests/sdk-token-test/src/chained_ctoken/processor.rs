use super::CreateCompressedMint;
use crate::chained_ctoken::create_pda::process_create_escrow_pda;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use light_compressed_token_sdk::instructions::mint_action::{
    MintActionCpiWriteAccounts, MintActionType, MintToRecipient,
};
use light_compressed_token_sdk::instructions::{mint_action_cpi_write, MintActionInputsCpiWrite};
use light_compressed_token_sdk::ValidityProof;
use light_ctoken_types::instructions::create_compressed_mint::{
    CompressedMintInstructionData, CompressedMintWithContext,
};
use light_ctoken_types::instructions::extensions::{
    ExtensionInstructionData, TokenMetadataInstructionData,
};
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct UpdateCompressedMintInstructionDataCpi {
    pub authority_type: CompressedMintAuthorityType,
    pub new_authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>, // Current mint authority (needed when updating freeze authority)
}
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CreateCompressedMintInstructionData {
    pub decimals: u8,
    pub freeze_authority: Option<Pubkey>,
    pub mint_bump: u8,
    pub address_merkle_tree_root_index: u16,
    pub version: u8,
    pub metadata: Option<TokenMetadataInstructionData>,
    pub compressed_mint_address: [u8; 32],
}

use light_ctoken_types::instructions::mint_to_compressed::Recipient;
use light_ctoken_types::instructions::update_compressed_mint::CompressedMintAuthorityType;
use light_ctoken_types::{COMPRESSED_MINT_SEED, COMPRESSED_TOKEN_PROGRAM_ID};
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintToCompressedInstructionData {
    pub recipients: Vec<Recipient>,
    pub lamports: Option<u64>,
    pub version: u8,
}
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

    // Single CPI call: consolidated mint action (create + mint + update authority)
    // Convert recipients from the mint input
    let recipients: Vec<MintToRecipient> = mint_input
        .recipients
        .iter()
        .map(|r| MintToRecipient {
            recipient: Pubkey::from(r.recipient.to_bytes()),
            amount: r.amount,
        })
        .collect();

    // Build actions for mint_action instruction
    let actions = vec![
        // 1. Mint tokens to recipients
        MintActionType::MintTo {
            recipients,
            lamports: None,
            token_account_version: mint_input.version,
        },
        // 2. Update mint authority (revoke if None)
        MintActionType::UpdateMintAuthority {
            new_authority: update_mint_input.new_authority,
        },
    ];

    // Create mint action CPI write inputs
    let mint_action_inputs = MintActionInputsCpiWrite {
        compressed_mint_inputs: compressed_mint_inputs.clone(),
        mint_seed: Some(ctx.accounts.mint_seed.key()), // Needed for creating mint and CreateSplMint action
        mint_bump: Some(input.mint_bump),              // Bump seed for creating SPL mint
        create_mint: true,                             // We are creating a new mint
        authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        actions,
        cpi_context: light_ctoken_types::instructions::mint_actions::CpiContext {
            set_context: false,
            first_set_context: true,
            in_tree_index: 0, // Used as address tree index if create mint
            in_queue_index: 1,
            out_queue_index: 1,
            token_out_queue_index: 1,
            assigned_account_index: 0, // Assign new address to the mint account (index 0)
        },
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
    };

    // Create the instruction using the SDK function
    let mint_action_instruction =
        mint_action_cpi_write(mint_action_inputs).map_err(ProgramError::from)?;
    msg!("mint_action_instruction {:?}", mint_action_instruction);
    // Prepare account infos following the same pattern as other CPI write functions
    let mint_action_account_infos = MintActionCpiWriteAccounts {
        light_system_program: cpi_accounts.system_program().unwrap(),
        mint_signer: Some(ctx.accounts.mint_seed.as_ref()),
        authority: ctx.accounts.mint_authority.as_ref(),
        fee_payer: ctx.accounts.payer.as_ref(),
        cpi_authority_pda: ctx.accounts.ctoken_cpi_authority.as_ref(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: crate::LIGHT_CPI_SIGNER,
    };

    // Execute the CPI call
    invoke(
        &mint_action_instruction,
        &mint_action_account_infos.to_account_infos(),
    )?;

    // Second CPI call: create compressed escrow PDA
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
