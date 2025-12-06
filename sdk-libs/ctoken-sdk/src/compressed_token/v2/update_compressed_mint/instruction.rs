use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_compressed_token_types::CompressedMintAuthorityType;
use light_ctoken_interface::{
    self,
    instructions::mint_action::{CompressedMintWithContext, CpiContext},
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    compressed_token::mint_action::{
        get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
        MintActionMetaConfigCpiWrite,
    },
    error::{Result, TokenSdkError},
    AnchorDeserialize, AnchorSerialize,
};

pub const UPDATE_COMPRESSED_MINT_DISCRIMINATOR: u8 = 105;

/// Input struct for updating a compressed mint instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct UpdateCompressedMintInputs {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub authority_type: CompressedMintAuthorityType,
    pub new_authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>, // Current mint authority (needed when updating freeze authority)
    pub proof: Option<CompressedProof>,
    pub payer: Pubkey,
    pub authority: Pubkey,
    pub in_merkle_tree: Pubkey,
    pub in_output_queue: Pubkey,
    pub out_output_queue: Pubkey,
}

/// Creates an update compressed mint instruction with CPI context support (now uses mint_action)
pub fn update_compressed_mint_cpi(
    input: UpdateCompressedMintInputs,
    cpi_context: Option<CpiContext>,
) -> Result<Instruction> {
    let mut instruction_data =
        light_ctoken_interface::instructions::mint_action::MintActionCompressedInstructionData::new(
            input.compressed_mint_inputs.clone(),
            input.proof,
        );

    let update_authority = light_ctoken_interface::instructions::mint_action::UpdateAuthority {
        new_authority: input.new_authority.map(|auth| auth.to_bytes().into()),
    };

    instruction_data = match input.authority_type {
        CompressedMintAuthorityType::MintTokens => {
            instruction_data.with_update_mint_authority(update_authority)
        }
        CompressedMintAuthorityType::FreezeAccount => {
            instruction_data.with_update_freeze_authority(update_authority)
        }
    };

    if let Some(ctx) = cpi_context {
        instruction_data = instruction_data.with_cpi_context(ctx);
    }

    let meta_config = MintActionMetaConfig::new(
        input.payer,
        input.authority,
        input.in_merkle_tree,
        input.in_output_queue,
        input.out_output_queue,
    );

    let account_metas = meta_config.to_account_metas();

    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}

/// Creates an update compressed mint instruction without CPI context
pub fn update_compressed_mint(input: UpdateCompressedMintInputs) -> Result<Instruction> {
    update_compressed_mint_cpi(input, None)
}

/// Input struct for creating an update compressed mint instruction with CPI context write
#[derive(Debug, Clone)]
pub struct UpdateCompressedMintInputsCpiWrite {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub authority_type: CompressedMintAuthorityType,
    pub new_authority: Option<Pubkey>,
    pub payer: Pubkey,
    pub authority: Pubkey,
    pub cpi_context: CpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Creates an update compressed mint instruction for CPI context writes (now uses mint_action)
pub fn create_update_compressed_mint_cpi_write(
    inputs: UpdateCompressedMintInputsCpiWrite,
) -> Result<Instruction> {
    if !inputs.cpi_context.first_set_context && !inputs.cpi_context.set_context {
        return Err(TokenSdkError::InvalidAccountData);
    }

    let mut instruction_data =
        light_ctoken_interface::instructions::mint_action::MintActionCompressedInstructionData::new(
            inputs.compressed_mint_inputs.clone(),
            None, // No proof for CPI write
        );

    let update_authority = light_ctoken_interface::instructions::mint_action::UpdateAuthority {
        new_authority: inputs.new_authority.map(|auth| auth.to_bytes().into()),
    };

    instruction_data = match inputs.authority_type {
        CompressedMintAuthorityType::MintTokens => {
            instruction_data.with_update_mint_authority(update_authority)
        }
        CompressedMintAuthorityType::FreezeAccount => {
            instruction_data.with_update_freeze_authority(update_authority)
        }
    };

    instruction_data = instruction_data.with_cpi_context(inputs.cpi_context);

    let meta_config = MintActionMetaConfigCpiWrite {
        fee_payer: inputs.payer,
        mint_signer: None, // Not needed for authority updates
        authority: inputs.authority,
        cpi_context: inputs.cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(meta_config);

    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}
