use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_ctoken_types::{
    self,
    instructions::{
        extensions::ExtensionInstructionData,
        mint_action::{CompressedMintInstructionData, CompressedMintWithContext, CpiContext},
    },
    COMPRESSED_MINT_SEED,
};
use solana_instruction::Instruction;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_action::{
        get_mint_action_instruction_account_metas,
        get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
        MintActionMetaConfigCpiWrite,
    },
    AnchorDeserialize, AnchorSerialize,
};

pub const CREATE_COMPRESSED_MINT_DISCRIMINATOR: u8 = 100;

/// Input struct for creating a compressed mint instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CreateCompressedMintInputs {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub proof: CompressedProof,
    pub address_merkle_tree_root_index: u16,
    pub mint_signer: Pubkey,
    pub payer: Pubkey,
    pub address_tree_pubkey: Pubkey,
    pub output_queue: Pubkey,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
    pub version: u8,
}

/// Creates a compressed mint instruction with a pre-computed mint address (wrapper around mint_action)
pub fn create_compressed_mint_cpi(
    input: CreateCompressedMintInputs,
    mint_address: [u8; 32],
    cpi_context: Option<CpiContext>,
    cpi_context_pubkey: Option<Pubkey>,
) -> Result<Instruction> {
    // Build CompressedMintInstructionData from the input parameters
    let compressed_mint_instruction_data = CompressedMintInstructionData {
        supply: 0,
        decimals: input.decimals,
        metadata: light_ctoken_types::state::CompressedMintMetadata {
            version: input.version,
            mint: find_spl_mint_address(&input.mint_signer)
                .0
                .to_bytes()
                .into(),
            spl_mint_initialized: false,
        },
        mint_authority: Some(input.mint_authority.to_bytes().into()),
        freeze_authority: input.freeze_authority.map(|auth| auth.to_bytes().into()),
        extensions: input.extensions,
    };

    // Build CompressedMintWithContext
    let compressed_mint_with_context = CompressedMintWithContext {
        address: mint_address,
        mint: compressed_mint_instruction_data,
        leaf_index: 0,
        prove_by_index: false,
        root_index: input.address_merkle_tree_root_index,
    };

    // Build instruction data using builder pattern
    let mut instruction_data = light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
        mint_address,
        input.address_merkle_tree_root_index,
        input.proof,
        compressed_mint_with_context.mint.clone(),
    );

    // Add CPI context if provided
    if let Some(ctx) = cpi_context {
        instruction_data = instruction_data.with_cpi_context(ctx);
    }

    // Build account meta config
    let meta_config = if cpi_context_pubkey.is_some() {
        // CPI context mode
        MintActionMetaConfig::new_cpi_context(
            &instruction_data,
            input.mint_authority,
            input.payer,
            cpi_context_pubkey.unwrap(),
        )?
    } else {
        // Regular CPI mode
        MintActionMetaConfig::new_create_mint(
            &instruction_data,
            input.mint_authority,
            input.mint_signer,
            input.payer,
            input.address_tree_pubkey,
            input.output_queue,
        )?
    };

    // Get account metas
    let account_metas =
        get_mint_action_instruction_account_metas(meta_config, &compressed_mint_with_context);

    // Serialize instruction data with discriminator
    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Build instruction directly
    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}

/// Input struct for creating a compressed mint instruction in CPI write mode
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CreateCompressedMintInputsCpiWrite {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub address_merkle_tree_root_index: u16,
    pub mint_signer: Pubkey,
    pub payer: Pubkey,
    pub mint_address: [u8; 32],
    pub cpi_context: CpiContext,
    pub cpi_context_pubkey: Pubkey,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
    pub version: u8,
}

pub fn create_compressed_mint_cpi_write(
    input: CreateCompressedMintInputsCpiWrite,
) -> Result<Instruction> {
    if !input.cpi_context.first_set_context && !input.cpi_context.set_context {
        msg!(
            "Invalid CPI context first cpi set or set context must be true {:?}",
            input.cpi_context
        );
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Build CompressedMintInstructionData
    let compressed_mint_instruction_data = CompressedMintInstructionData {
        supply: 0,
        decimals: input.decimals,
        metadata: light_ctoken_types::state::CompressedMintMetadata {
            version: input.version,
            mint: find_spl_mint_address(&input.mint_signer)
                .0
                .to_bytes()
                .into(),
            spl_mint_initialized: false,
        },
        mint_authority: Some(input.mint_authority.to_bytes().into()),
        freeze_authority: input.freeze_authority.map(|auth| auth.to_bytes().into()),
        extensions: input.extensions,
    };

    // Build instruction data using builder pattern
    let instruction_data = light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
        input.mint_address,
        input.address_merkle_tree_root_index,
        light_compressed_account::instruction_data::compressed_proof::CompressedProof::default(), // Dummy proof for CPI write
        compressed_mint_instruction_data,
    ).with_cpi_context(input.cpi_context);

    // Build account meta config for CPI write
    let meta_config = MintActionMetaConfigCpiWrite {
        fee_payer: input.payer,
        mint_signer: Some(input.mint_signer),
        authority: input.mint_authority,
        cpi_context: input.cpi_context_pubkey,
        mint_needs_to_sign: true, // Always true for create mint
    };

    // Get account metas for CPI write
    let account_metas = get_mint_action_instruction_account_metas_cpi_write(meta_config);

    // Serialize instruction data with discriminator
    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Build instruction directly
    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}

/// Creates a compressed mint instruction with automatic mint address derivation
pub fn create_compressed_mint(input: CreateCompressedMintInputs) -> Result<Instruction> {
    let mint_address =
        derive_compressed_mint_address(&input.mint_signer, &input.address_tree_pubkey);
    create_compressed_mint_cpi(input, mint_address, None, None)
}

/// Derives the compressed mint address from the mint seed and address tree
pub fn derive_compressed_mint_address(
    mint_seed: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &find_spl_mint_address(mint_seed).0.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
    )
}

pub fn derive_cmint_from_spl_mint(mint: &Pubkey, address_tree_pubkey: &Pubkey) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &mint.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
    )
}

pub fn find_spl_mint_address(mint_seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
    )
}
