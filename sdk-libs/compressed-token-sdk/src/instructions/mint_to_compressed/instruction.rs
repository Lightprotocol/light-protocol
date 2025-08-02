use light_compressed_token_types::CompressedProof;
use light_ctoken_types::{
    instructions::{
        create_compressed_mint::CompressedMintWithContext,
        mint_to_compressed::{CpiContext, MintToCompressedInstructionData, Recipient},
    },
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_to_compressed::account_metas::{
        get_mint_to_compressed_instruction_account_metas,
        get_mint_to_compressed_instruction_account_metas_cpi_write, MintToCompressedMetaConfig,
        MintToCompressedMetaConfigCpiWrite,
    },
    AnchorSerialize,
};

pub use light_compressed_token_types::account_infos::mint_to_compressed::DecompressedMintConfig;

pub const MINT_TO_COMPRESSED_DISCRIMINATOR: u8 = 101;

/// Input parameters for creating a mint_to_compressed instruction
#[derive(Debug, Clone)]
pub struct MintToCompressedInputs {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub mint_authority: Pubkey,
    pub payer: Pubkey,
    pub state_merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub state_tree_pubkey: Pubkey,
    /// Required if the mint is decompressed
    pub decompressed_mint_config: Option<DecompressedMintConfig<Pubkey>>,
    pub proof: Option<CompressedProof>,
}

/// Create a mint_to_compressed instruction
pub fn create_mint_to_compressed_instruction(
    inputs: MintToCompressedInputs,
    cpi_context: Option<CpiContext>,
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
        proof,
    } = inputs;

    // Store decompressed flag before moving the compressed_mint_input
    let is_decompressed = compressed_mint_inputs.mint.is_decompressed;

    // Validate that decompressed_mint_config is provided when the mint is decompressed
    if is_decompressed && decompressed_mint_config.is_none() {
        return Err(TokenSdkError::DecompressedMintConfigRequired);
    }

    // Create mint_to_compressed instruction data
    let mint_to_instruction_data = MintToCompressedInstructionData {
        token_account_version: 2, // V2 for batched merkle trees
        compressed_mint_inputs,
        lamports,
        recipients,
        cpi_context,
        proof,
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

/// Input struct for creating a mint_to_compressed instruction with CPI context write
#[derive(Debug, Clone)]
pub struct MintToCompressedInputsCpiWrite {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub mint_authority: Pubkey,
    pub payer: Pubkey,
    pub cpi_context: CpiContext,
    pub cpi_context_pubkey: Pubkey,
    pub version: u8,
}

/// Create a mint_to_compressed instruction for CPI context writes
pub fn create_mint_to_compressed_cpi_write(
    inputs: MintToCompressedInputsCpiWrite,
) -> Result<Instruction> {
    let MintToCompressedInputsCpiWrite {
        compressed_mint_inputs,
        lamports,
        recipients,
        mint_authority,
        payer,
        cpi_context,
        cpi_context_pubkey: _,
        version,
    } = inputs;

    if !cpi_context.first_set_context && !cpi_context.set_context {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Create mint_to_compressed instruction data
    let mint_to_instruction_data = MintToCompressedInstructionData {
        token_account_version: version,
        compressed_mint_inputs,
        lamports,
        recipients,
        cpi_context: Some(cpi_context),
        proof: None,
    };

    // Create account meta config for CPI context write
    let meta_config = MintToCompressedMetaConfigCpiWrite {
        fee_payer: payer,
        mint_authority,
        cpi_context: inputs.cpi_context_pubkey,
    };

    // Get account metas
    let accounts = get_mint_to_compressed_instruction_account_metas_cpi_write(meta_config);

    // Serialize instruction data
    let data_vec = mint_to_instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: accounts.to_vec(),
        data: [vec![MINT_TO_COMPRESSED_DISCRIMINATOR], data_vec].concat(),
    })
}
