use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self,
    instructions::create_compressed_mint::UpdateCompressedMintInstructionData,
    instructions::update_compressed_mint::{
        CompressedMintAuthorityType, UpdateCompressedMintInstructionDataV2, UpdateMintCpiContext,
    },
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::update_compressed_mint::account_metas::{
        get_update_compressed_mint_instruction_account_metas, UpdateCompressedMintMetaConfig,
    },
    AnchorDeserialize, AnchorSerialize,
};

pub const UPDATE_COMPRESSED_MINT_DISCRIMINATOR: u8 = 105;

/// Input struct for updating a compressed mint instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct UpdateCompressedMintInputs {
    pub compressed_mint_inputs: UpdateCompressedMintInstructionData,
    pub authority_type: CompressedMintAuthorityType,
    pub new_authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>, // Current mint authority (needed when updating freeze authority)
    pub proof: CompressedProof,
    pub payer: Pubkey,
    pub authority: Pubkey,
    pub in_merkle_tree: Pubkey,
    pub in_output_queue: Pubkey,
    pub out_output_queue: Pubkey,
}

/// Creates an update compressed mint instruction with CPI context support
pub fn update_compressed_mint_cpi(
    input: UpdateCompressedMintInputs,
    cpi_context: Option<UpdateMintCpiContext>,
) -> Result<Instruction> {
    let with_cpi_context = cpi_context.is_some();

    let instruction_data = UpdateCompressedMintInstructionDataV2 {
        compressed_mint_inputs: input.compressed_mint_inputs,
        authority_type: input.authority_type.into(),
        new_authority: input.new_authority.map(|auth| auth.to_bytes().into()),
        mint_authority: input.mint_authority.map(|auth| auth.to_bytes().into()),
        cpi_context,
    };

    // Create account meta config for update_compressed_mint
    let meta_config = UpdateCompressedMintMetaConfig {
        fee_payer: Some(input.payer),
        authority: Some(input.authority),
        in_merkle_tree: input.in_merkle_tree,
        in_output_queue: input.in_output_queue,
        out_output_queue: input.out_output_queue,
        with_cpi_context,
    };

    // Get account metas
    let accounts = get_update_compressed_mint_instruction_account_metas(meta_config);

    // Serialize instruction data
    let data_vec = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: [vec![UPDATE_COMPRESSED_MINT_DISCRIMINATOR], data_vec].concat(),
    })
}

/// Creates an update compressed mint instruction without CPI context
pub fn update_compressed_mint(input: UpdateCompressedMintInputs) -> Result<Instruction> {
    update_compressed_mint_cpi(input, None)
}

/// Input struct for creating an update compressed mint instruction with CPI context write
#[derive(Debug, Clone)]
pub struct UpdateCompressedMintInputsCpiWrite {
    pub compressed_mint_inputs: UpdateCompressedMintInstructionData,
    pub authority_type: CompressedMintAuthorityType,
    pub new_authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>, // Current mint authority (needed when updating freeze authority)
    pub payer: Pubkey,
    pub authority: Pubkey,
    pub cpi_context: UpdateMintCpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Creates an update compressed mint instruction for CPI context writes
pub fn create_update_compressed_mint_cpi_write(
    inputs: UpdateCompressedMintInputsCpiWrite,
) -> Result<Instruction> {
    let UpdateCompressedMintInputsCpiWrite {
        compressed_mint_inputs,
        authority_type,
        new_authority,
        mint_authority,
        payer: _,
        authority: _,
        cpi_context,
        cpi_context_pubkey: _,
    } = inputs;

    if !cpi_context.first_set_context && !cpi_context.set_context {
        return Err(TokenSdkError::InvalidAccountData);
    }

    let instruction_data = UpdateCompressedMintInstructionDataV2 {
        compressed_mint_inputs,
        authority_type: authority_type.into(),
        new_authority: new_authority.map(|auth| auth.to_bytes().into()),
        mint_authority: mint_authority.map(|auth| auth.to_bytes().into()),
        cpi_context: Some(cpi_context),
    };

    // For CPI write mode, use the same pattern as mint_to_compressed
    let accounts = vec![
        solana_instruction::AccountMeta::new_readonly(
            Pubkey::new_from_array(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID),
            false,
        ), // light_system_program
        solana_instruction::AccountMeta::new_readonly(inputs.authority, true), // authority (signer)
        solana_instruction::AccountMeta::new(inputs.payer, true), // fee_payer
        solana_instruction::AccountMeta::new_readonly(
            crate::instructions::CTokenDefaultAccounts::default().cpi_authority_pda,
            false,
        ), // cpi_authority_pda
        solana_instruction::AccountMeta::new(inputs.cpi_context_pubkey, false), // cpi_context
    ];

    // Serialize instruction data
    let data_vec = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: [vec![UPDATE_COMPRESSED_MINT_DISCRIMINATOR], data_vec].concat(),
    })
}
