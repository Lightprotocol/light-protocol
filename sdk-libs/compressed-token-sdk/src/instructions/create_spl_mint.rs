use crate::AnchorSerialize;
use light_compressed_token_types::{ValidityProof, CPI_AUTHORITY_PDA};
use light_ctoken_types::{
    instructions::{
        create_compressed_mint::UpdateCompressedMintInstructionData,
        create_spl_mint::CreateSplMintInstructionData, mint_to_compressed::CompressedMintInputs,
    },
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    NOOP_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

pub const POOL_SEED: &[u8] = b"pool";

pub struct CreateSplMintInputs {
    pub mint_signer: Pubkey,
    pub mint_bump: u8,
    pub compressed_mint_inputs: CompressedMintInputs,
    pub proof: ValidityProof,
    pub payer: Pubkey,
    pub input_merkle_tree: Pubkey,
    pub input_output_queue: Pubkey,
    pub output_queue: Pubkey,
    pub mint_authority: Pubkey,
}

pub fn create_spl_mint_instruction(inputs: CreateSplMintInputs) -> Instruction {
    // Extract values from compressed_mint_inputs
    let mint_pda: Pubkey = inputs
        .compressed_mint_inputs
        .compressed_mint_input
        .spl_mint
        .to_bytes()
        .into();
    // Find token pool PDA index 0
    let (token_pool_pda, _token_pool_bump) = Pubkey::find_program_address(
        &[POOL_SEED, &mint_pda.to_bytes()],
        &Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
    );
    create_spl_mint_instruction_with_bump(inputs, token_pool_pda)
}

pub fn create_spl_mint_instruction_with_bump(
    inputs: CreateSplMintInputs,
    token_pool_pda: Pubkey,
) -> Instruction {
    let CreateSplMintInputs {
        mint_signer,
        mint_bump,
        compressed_mint_inputs,
        proof,
        payer,
        input_merkle_tree,
        input_output_queue,
        output_queue,
        mint_authority,
    } = inputs;
    // Extract values from compressed_mint_inputs
    let mint_pda: Pubkey = compressed_mint_inputs
        .compressed_mint_input
        .spl_mint
        .to_bytes()
        .into();

    // Create UpdateCompressedMintInstructionData from the compressed mint inputs
    let update_mint_data = UpdateCompressedMintInstructionData {
        merkle_context: compressed_mint_inputs.merkle_context,
        root_index: compressed_mint_inputs.root_index,
        address: compressed_mint_inputs.address,
        proof: proof.into(),
        mint: compressed_mint_inputs.compressed_mint_input.into(),
    };

    // Create the create_spl_mint instruction data
    let create_spl_mint_instruction_data = CreateSplMintInstructionData {
        mint_bump,
        mint: update_mint_data,
    };

    // Create create_spl_mint accounts in the exact order expected by accounts.rs
    let create_spl_mint_accounts = vec![
        // Static non-CPI accounts first (in order from accounts.rs)
        AccountMeta::new(mint_authority, true), // authority (signer)
        AccountMeta::new(mint_pda, false),      // mint
        AccountMeta::new_readonly(mint_signer, false), // mint_signer
        AccountMeta::new(token_pool_pda, false), // token_pool_pda
        AccountMeta::new_readonly(spl_token_2022::ID, false), // token_program TODO: add constant
        AccountMeta::new_readonly(Pubkey::new_from_array(LIGHT_SYSTEM_PROGRAM_ID), false), // light_system_program
        // CPI accounts in exact order expected by light-system-program
        AccountMeta::new(payer, true), // fee_payer (signer, mutable)
        AccountMeta::new_readonly(Pubkey::new_from_array(CPI_AUTHORITY_PDA), false), // cpi_authority_pda
        AccountMeta::new_readonly(Pubkey::new_from_array(REGISTERED_PROGRAM_PDA), false), // registered_program_pda
        AccountMeta::new_readonly(Pubkey::new_from_array(NOOP_PROGRAM_ID), false), // noop_program
        AccountMeta::new_readonly(
            Pubkey::new_from_array(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
            false,
        ), // account_compression_authority
        AccountMeta::new_readonly(
            Pubkey::new_from_array(ACCOUNT_COMPRESSION_PROGRAM_ID),
            false,
        ), // account_compression_program
        AccountMeta::new_readonly(Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID), false), // self_program
        AccountMeta::new_readonly(Pubkey::default(), false), // system_program
        AccountMeta::new(input_merkle_tree, false),          // in_merkle_tree
        AccountMeta::new(input_output_queue, false),         // in_output_queue
        AccountMeta::new(output_queue, false),               // out_output_queue
    ];

    Instruction {
        program_id: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: create_spl_mint_accounts,
        data: [
            vec![102],                                              // CreateSplMint discriminator
            create_spl_mint_instruction_data.try_to_vec().unwrap(), // TODO: use manual serialization
        ]
        .concat(),
    }
}
