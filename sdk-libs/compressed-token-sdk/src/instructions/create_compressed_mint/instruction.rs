use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_compressed_token_types::CompressedCpiContext;
use light_ctoken_types::{
    self, instructions::extensions::ExtensionInstructionData, COMPRESSED_MINT_SEED,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::create_compressed_mint::account_metas::{
        get_create_compressed_mint_instruction_account_metas, CreateCompressedMintMetaConfig,
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

/// Creates a compressed mint instruction with a pre-computed mint address
pub fn create_compressed_mint_cpi(
    input: CreateCompressedMintInputs,
    mint_address: [u8; 32],
    cpi_context: Option<CompressedCpiContext>,
) -> Result<Instruction> {
    use light_ctoken_types::instructions::create_compressed_mint::CreateCompressedMintInstructionData;

    let instruction_data = CreateCompressedMintInstructionData {
        decimals: input.decimals,
        mint_authority: input.mint_authority.to_bytes().into(),
        freeze_authority: input.freeze_authority.map(|auth| auth.to_bytes().into()),
        proof: input.proof,
        mint_bump: input.mint_bump,
        address_merkle_tree_root_index: input.address_merkle_tree_root_index,
        extensions: input.extensions,
        mint_address,
        version: input.version,
        cpi_context,
    };

    // Create account meta config for create_compressed_mint
    let meta_config = CreateCompressedMintMetaConfig {
        fee_payer: Some(input.payer),
        mint_signer: Some(input.mint_signer),
        address_tree_pubkey: input.address_tree_pubkey,
        output_queue: input.output_queue,
    };

    // Get account metas
    let accounts = get_create_compressed_mint_instruction_account_metas(meta_config);

    // Serialize instruction data
    let data_vec = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: [vec![CREATE_COMPRESSED_MINT_DISCRIMINATOR], data_vec].concat(),
    })
}

/// Creates a compressed mint instruction with automatic mint address derivation
pub fn create_compressed_mint(input: CreateCompressedMintInputs) -> Result<Instruction> {
    let mint_address =
        derive_compressed_mint_address(&input.mint_signer, &input.address_tree_pubkey);
    create_compressed_mint_cpi(input, mint_address, None)
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
    spl_mint: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &spl_mint.to_bytes(),
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
