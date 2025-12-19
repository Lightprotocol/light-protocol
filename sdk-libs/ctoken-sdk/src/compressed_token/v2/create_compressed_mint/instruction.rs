use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_token_interface::{
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
    compressed_token::mint_action::{
        get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
        MintActionMetaConfigCpiWrite,
    },
    error::{Result, TokenSdkError},
    AnchorDeserialize, AnchorSerialize,
};

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
    let compressed_mint_instruction_data = CompressedMintInstructionData {
        supply: 0,
        decimals: input.decimals,
        metadata: light_token_interface::state::CompressedMintMetadata {
            version: input.version,
            mint: find_cmint_address(&input.mint_signer).0.to_bytes().into(),
            spl_mint_initialized: false,
        },
        mint_authority: Some(input.mint_authority.to_bytes().into()),
        freeze_authority: input.freeze_authority.map(|auth| auth.to_bytes().into()),
        extensions: input.extensions,
    };

    let compressed_mint_with_context = CompressedMintWithContext {
        address: mint_address,
        mint: compressed_mint_instruction_data,
        leaf_index: 0,
        prove_by_index: false,
        root_index: input.address_merkle_tree_root_index,
    };

    let mut instruction_data = light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
        mint_address,
        input.address_merkle_tree_root_index,
        input.proof,
        compressed_mint_with_context.mint.clone(),
    );

    if let Some(ctx) = cpi_context {
        instruction_data = instruction_data.with_cpi_context(ctx);
    }

    let meta_config = if cpi_context_pubkey.is_some() {
        MintActionMetaConfig::new_cpi_context(
            &instruction_data,
            input.payer,
            input.mint_authority,
            cpi_context_pubkey.unwrap(),
        )?
    } else {
        MintActionMetaConfig::new_create_mint(
            input.payer,
            input.mint_authority,
            input.mint_signer,
            input.address_tree_pubkey,
            input.output_queue,
        )
    };

    let account_metas = meta_config.to_account_metas();

    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}

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

    let compressed_mint_instruction_data = CompressedMintInstructionData {
        supply: 0,
        decimals: input.decimals,
        metadata: light_token_interface::state::CompressedMintMetadata {
            version: input.version,
            mint: find_cmint_address(&input.mint_signer).0.to_bytes().into(),
            spl_mint_initialized: false,
        },
        mint_authority: Some(input.mint_authority.to_bytes().into()),
        freeze_authority: input.freeze_authority.map(|auth| auth.to_bytes().into()),
        extensions: input.extensions,
    };

    let instruction_data = light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint_write_to_cpi_context(
        input.mint_address,
        input.address_merkle_tree_root_index,
        compressed_mint_instruction_data,input.cpi_context
    );

    let meta_config = MintActionMetaConfigCpiWrite {
        fee_payer: input.payer,
        mint_signer: Some(input.mint_signer),
        authority: input.mint_authority,
        cpi_context: input.cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(meta_config);

    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}

/// Creates a compressed mint instruction with automatic mint address derivation
pub fn create_compressed_mint(input: CreateCompressedMintInputs) -> Result<Instruction> {
    let mint_address =
        derive_cmint_compressed_address(&input.mint_signer, &input.address_tree_pubkey);
    create_compressed_mint_cpi(input, mint_address, None, None)
}

/// Derives the compressed mint address from the mint seed and address tree
pub fn derive_cmint_compressed_address(
    mint_seed: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &find_cmint_address(mint_seed).0.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
    )
}

pub fn derive_cmint_from_spl_mint(mint: &Pubkey, address_tree_pubkey: &Pubkey) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &mint.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
    )
}

pub fn find_cmint_address(mint_seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
    )
}
