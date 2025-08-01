use light_ctoken_types::{
    instructions::{
        create_compressed_mint::UpdateCompressedMintInstructionData,
        mint_to_compressed::{CompressedMintInputs, MintToCompressedInstructionData, Recipient},
    },
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_to_compressed::account_metas::{
        get_mint_to_compressed_instruction_account_metas, MintToCompressedMetaConfig,
    },
    AnchorSerialize,
};

pub use light_compressed_token_types::account_infos::mint_to_compressed::DecompressedMintConfig;

pub const MINT_TO_COMPRESSED_DISCRIMINATOR: u8 = 101;

/// Input parameters for creating a mint_to_compressed instruction
#[derive(Debug, Clone)]
pub struct MintToCompressedInputs {
    pub compressed_mint_inputs: CompressedMintInputs,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub mint_authority: Pubkey,
    pub payer: Pubkey,
    pub state_merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub state_tree_pubkey: Pubkey,
    /// Required if the mint is decompressed
    pub decompressed_mint_config: Option<DecompressedMintConfig<Pubkey>>,
}

/// Create a mint_to_compressed instruction
pub fn create_mint_to_compressed_instruction(
    inputs: MintToCompressedInputs,
) -> Result<Instruction> {
    let MintToCompressedInputs {
        compressed_mint_inputs,
        lamports,
        recipients,
        mint_authority,
        payer,
        state_merkle_tree,
        output_queue,
        state_tree_pubkey,
        decompressed_mint_config,
    } = inputs;

    // Store decompressed flag before moving the compressed_mint_input
    let is_decompressed = compressed_mint_inputs.compressed_mint_input.is_decompressed;
    
    // Validate that decompressed_mint_config is provided when the mint is decompressed
    if is_decompressed && decompressed_mint_config.is_none() {
        return Err(TokenSdkError::DecompressedMintConfigRequired);
    }

    // Create UpdateCompressedMintInstructionData from CompressedMintInputs
    let update_mint_data = UpdateCompressedMintInstructionData {
        leaf_index: compressed_mint_inputs.leaf_index.into(),
        prove_by_index: compressed_mint_inputs.prove_by_index.into(),
        root_index: compressed_mint_inputs.root_index,
        address: compressed_mint_inputs.address,
        proof: None, // No proof needed for this test
        mint: compressed_mint_inputs.compressed_mint_input.try_into()?,
    };

    // Create mint_to_compressed instruction data
    let mint_to_instruction_data = MintToCompressedInstructionData {
        token_account_version: 2, // V2 for batched merkle trees
        compressed_mint_inputs: update_mint_data,
        lamports,
        recipients,
        proof: None, // No proof needed for this test
    };

    // Create account meta config
    let has_sol_pool = lamports.is_some();
    
    let meta_config = if is_decompressed {
        let decompressed_config = decompressed_mint_config.unwrap();
        MintToCompressedMetaConfig::new_decompressed(
            mint_authority,
            payer,
            state_merkle_tree,
            output_queue,
            state_tree_pubkey,
            state_tree_pubkey, // compressed_mint_tree
            output_queue,      // compressed_mint_queue
            decompressed_config.mint_pda,
            decompressed_config.token_pool_pda,
            decompressed_config.token_program,
            has_sol_pool,
        )
    } else {
        MintToCompressedMetaConfig::new(
            mint_authority,
            payer,
            state_merkle_tree,
            output_queue,
            state_tree_pubkey,
            state_tree_pubkey, // compressed_mint_tree
            output_queue,      // compressed_mint_queue
            has_sol_pool,
        )
    };

    // Get account metas using the SDK function
    let accounts = get_mint_to_compressed_instruction_account_metas(meta_config);

    // Serialize instruction data
    let data_vec = mint_to_instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: [vec![MINT_TO_COMPRESSED_DISCRIMINATOR], data_vec].concat(),
    })
}