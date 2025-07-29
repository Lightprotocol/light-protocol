use crate::{AnchorDeserialize, AnchorSerialize};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self, instructions::extensions::ExtensionInstructionData, COMPRESSED_MINT_SEED,
};
use light_sdk::constants::{ACCOUNT_COMPRESSION_AUTHORITY_PDA, REGISTERED_PROGRAM_PDA};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

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
}

/// Creates a compressed mint instruction with a pre-computed mint address
pub fn create_compressed_mint_cpi(
    input: CreateCompressedMintInputs,
    mint_address: [u8; 32],
) -> Instruction {
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
        version: 0,
    };

    let accounts = vec![
        // Static non-CPI accounts first
        AccountMeta::new_readonly(input.mint_signer, true), // 0: mint_signer (signer)
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID),
            false,
        ), // light system program
        // CPI accounts in exact order expected by execute_cpi_invoke
        AccountMeta::new(input.payer, true), // 1: fee_payer (signer, mutable)
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(light_ctoken_types::CPI_AUTHORITY),
            false,
        ), // 2: cpi_authority_pda
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(REGISTERED_PROGRAM_PDA),
            false,
        ), // 3: registered_program_pda
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(light_sdk::constants::NOOP_PROGRAM_ID),
            false,
        ), // 4: noop_program
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
            false,
        ), // 5: account_compression_authority
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(
                light_sdk::constants::ACCOUNT_COMPRESSION_PROGRAM_ID,
            ),
            false,
        ), // 6: account_compression_program
        AccountMeta::new_readonly(
            solana_pubkey::Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            false,
        ), // 7: invoking_program (self_program)
        AccountMeta::new_readonly(solana_pubkey::Pubkey::default(), false), // 10: system_program
        AccountMeta::new(input.address_tree_pubkey, false), // 12: address_merkle_tree (mutable)
        AccountMeta::new(input.output_queue, false), // 13: output_queue (mutable)
    ];

    Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
        ),
        accounts,
        data: [
            vec![CREATE_COMPRESSED_MINT_DISCRIMINATOR],
            instruction_data.try_to_vec().unwrap(),
        ]
        .concat(),
    }
}

/// Creates a compressed mint instruction with automatic mint address derivation
pub fn create_compressed_mint(input: CreateCompressedMintInputs) -> Instruction {
    let mint_address =
        derive_compressed_mint_address(&input.mint_signer, &input.address_tree_pubkey);
    create_compressed_mint_cpi(input, mint_address)
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
        &solana_pubkey::Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
    )
}
