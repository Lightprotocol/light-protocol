use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self,
    instructions::{
        extensions::ExtensionInstructionData,
        mint_action::{CompressedMintWithContext, CpiContext},
    },
    COMPRESSED_MINT_SEED,
};
use solana_instruction::Instruction;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_action::{
        create_mint_action_cpi, mint_action_cpi_write, MintActionInputs, MintActionInputsCpiWrite,
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
    pub mint_bump: u8,
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
    // Build CompressedMintWithContext from the input parameters
    let compressed_mint_with_context = CompressedMintWithContext {
        address: mint_address,
        mint: light_ctoken_types::instructions::mint_action::CompressedMintInstructionData {
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
        },
        leaf_index: 0, // Default value for new mint
        prove_by_index: false,
        root_index: input.address_merkle_tree_root_index,
    };

    // Convert create_compressed_mint CpiContext to mint_actions CpiContext if present
    let mint_action_cpi_context = cpi_context.map(|ctx| {
        light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: ctx.set_context,
            first_set_context: ctx.first_set_context,
            in_tree_index: 0, // Default for create mint
            in_queue_index: 0,
            out_queue_index: 0,
            token_out_queue_index: 0,
            assigned_account_index: 0, // Default for create mint
            ..Default::default()
        }
    });

    // Create mint action inputs for compressed mint creation
    let mint_action_inputs = MintActionInputs {
        compressed_mint_inputs: compressed_mint_with_context,
        mint_seed: input.mint_signer,
        create_mint: true, // Key difference - we're creating a new compressed mint
        mint_bump: Some(input.mint_bump),
        authority: input.mint_authority,
        payer: input.payer,
        proof: Some(input.proof),
        actions: Vec::new(), // Empty - just creating mint, no additional actions
        address_tree_pubkey: input.address_tree_pubkey, // Address tree for new mint address
        input_queue: None,   // Not needed for create_mint: true
        output_queue: input.output_queue,
        tokens_out_queue: None, // No tokens being minted
        token_pool: None,       // Not needed for simple compressed mint creation
    };

    create_mint_action_cpi(
        mint_action_inputs,
        mint_action_cpi_context,
        cpi_context_pubkey,
    )
}

/// Input struct for creating a compressed mint instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CreateCompressedMintInputsCpiWrite {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub mint_bump: u8,
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

    // Build CompressedMintWithContext from the input parameters
    let compressed_mint_with_context = CompressedMintWithContext {
        address: input.mint_address,
        mint: light_ctoken_types::instructions::mint_action::CompressedMintInstructionData {
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
        },
        leaf_index: 0, // Default value for new mint
        prove_by_index: false,
        root_index: input.address_merkle_tree_root_index,
    };

    // Convert create_compressed_mint CpiContext to mint_actions CpiContext
    let mint_action_cpi_context = light_ctoken_types::instructions::mint_action::CpiContext {
        set_context: input.cpi_context.set_context,
        first_set_context: input.cpi_context.first_set_context,
        in_tree_index: 0, // Default for create mint
        in_queue_index: 0,
        out_queue_index: 0,
        token_out_queue_index: 0,
        assigned_account_index: 0, // Default for create mint
        ..Default::default()
    };

    // Create mint action inputs for compressed mint creation (CPI write mode)
    let mint_action_inputs = MintActionInputsCpiWrite {
        compressed_mint_inputs: compressed_mint_with_context,
        mint_seed: Some(input.mint_signer),
        mint_bump: Some(input.mint_bump),
        create_mint: true, // Key difference - we're creating a new compressed mint
        authority: input.mint_authority,
        payer: input.payer,
        actions: Vec::new(), // Empty - just creating mint, no additional actions
        cpi_context: mint_action_cpi_context,
        cpi_context_pubkey: input.cpi_context_pubkey,
    };

    mint_action_cpi_write(mint_action_inputs)
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

pub fn derive_compressed_mint_from_spl_mint(
    mint: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
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
