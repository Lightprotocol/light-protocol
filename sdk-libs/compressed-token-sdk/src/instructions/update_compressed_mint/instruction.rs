use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_compressed_token_types::CompressedMintAuthorityType;
use light_ctoken_types::{
    self,
    instructions::mint_action::{CompressedMintWithContext, CpiContext},
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_action::instruction::{
        create_mint_action_cpi, mint_action_cpi_write, MintActionInputs, MintActionInputsCpiWrite,
        MintActionType,
    },
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
    // Convert UpdateMintCpiContext to mint_action CpiContext if needed
    let mint_action_cpi_context = cpi_context.map(|update_cpi_ctx| {
        CpiContext {
            set_context: update_cpi_ctx.set_context,
            first_set_context: update_cpi_ctx.first_set_context,
            in_tree_index: update_cpi_ctx.in_tree_index,
            in_queue_index: update_cpi_ctx.in_queue_index,
            out_queue_index: update_cpi_ctx.out_queue_index,
            token_out_queue_index: 0, // Default value - not used for authority updates
            assigned_account_index: 0, // Default value - mint account index for authority updates
            ..Default::default()
        }
    });

    // Create the appropriate action based on authority type
    let actions = match input.authority_type {
        CompressedMintAuthorityType::MintTokens => {
            vec![MintActionType::UpdateMintAuthority {
                new_authority: input.new_authority,
            }]
        }
        CompressedMintAuthorityType::FreezeAccount => {
            vec![MintActionType::UpdateFreezeAuthority {
                new_authority: input.new_authority,
            }]
        }
    };

    // Create mint action inputs for authority update
    let mint_action_inputs = MintActionInputs {
        compressed_mint_inputs: input.compressed_mint_inputs,
        mint_seed: Pubkey::default(), // Not needed for authority updates
        create_mint: false,           // We're updating an existing mint
        mint_bump: None,
        authority: input.authority,
        payer: input.payer,
        proof: input.proof,
        actions,
        address_tree_pubkey: input.in_merkle_tree, // Use in_merkle_tree as the state tree
        input_queue: Some(input.in_output_queue),
        output_queue: input.out_output_queue,
        tokens_out_queue: None, // Not needed for authority updates
        token_pool: None,       // Not needed for authority updates
    };

    create_mint_action_cpi(mint_action_inputs, mint_action_cpi_context, None)
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

    // Convert UpdateMintCpiContext to mint_action CpiContext
    let mint_action_cpi_context = light_ctoken_types::instructions::mint_action::CpiContext {
        set_context: inputs.cpi_context.set_context,
        first_set_context: inputs.cpi_context.first_set_context,
        in_tree_index: inputs.cpi_context.in_tree_index,
        in_queue_index: inputs.cpi_context.in_queue_index,
        out_queue_index: inputs.cpi_context.out_queue_index,
        token_out_queue_index: 0, // Default value - not used for authority updates
        assigned_account_index: 0, // Default value - mint account index for authority updates
        ..Default::default()
    };

    // Create the appropriate action based on authority type
    let actions = match inputs.authority_type {
        CompressedMintAuthorityType::MintTokens => {
            vec![MintActionType::UpdateMintAuthority {
                new_authority: inputs.new_authority,
            }]
        }
        CompressedMintAuthorityType::FreezeAccount => {
            vec![MintActionType::UpdateFreezeAuthority {
                new_authority: inputs.new_authority,
            }]
        }
    };

    // Create mint action inputs for CPI write
    let mint_action_inputs = MintActionInputsCpiWrite {
        compressed_mint_inputs: inputs.compressed_mint_inputs,
        mint_seed: None, // Not needed for authority updates
        mint_bump: None,
        create_mint: false, // We're updating an existing mint
        authority: inputs.authority,
        payer: inputs.payer,
        actions,
        cpi_context: mint_action_cpi_context,
        cpi_context_pubkey: inputs.cpi_context_pubkey,
    };

    mint_action_cpi_write(mint_action_inputs)
}
