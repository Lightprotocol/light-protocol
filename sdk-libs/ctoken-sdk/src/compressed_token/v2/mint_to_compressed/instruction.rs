use light_compressed_account::instruction_data::traits::LightInstructionData;
pub use light_compressed_token_types::account_infos::mint_to_compressed::DecompressedMintConfig;
use light_compressed_token_types::CompressedProof;
use light_ctoken_interface::instructions::mint_action::{
    CompressedMintWithContext, CpiContext, Recipient,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    compressed_token::mint_action::MintActionMetaConfig,
    error::{CTokenSdkError, Result},
    spl_interface::SplInterfacePda,
};

pub const MINT_TO_COMPRESSED_DISCRIMINATOR: u8 = 101;

/// Input parameters for creating a mint_to_compressed instruction
#[derive(Debug, Clone)]
pub struct MintToCompressedInputs {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub recipients: Vec<Recipient>,
    pub mint_authority: Pubkey,
    pub payer: Pubkey,
    pub state_merkle_tree: Pubkey,
    pub input_queue: Pubkey,
    pub output_queue_cmint: Pubkey,
    pub output_queue_tokens: Pubkey,
    /// Required if the mint is decompressed
    pub decompressed_mint_config: Option<DecompressedMintConfig<Pubkey>>,
    pub proof: Option<CompressedProof>,
    pub token_account_version: u8,
    pub cpi_context_pubkey: Option<Pubkey>,
    /// Required if the mint is decompressed
    pub spl_interface_pda: Option<SplInterfacePda>,
}

/// Create a mint_to_compressed instruction (wrapper around mint_action)
pub fn create_mint_to_compressed_instruction(
    inputs: MintToCompressedInputs,
    cpi_context: Option<CpiContext>,
) -> Result<Instruction> {
    let MintToCompressedInputs {
        compressed_mint_inputs,
        recipients,
        mint_authority,
        payer,
        state_merkle_tree,
        input_queue,
        output_queue_cmint,
        output_queue_tokens: _,
        decompressed_mint_config: _,
        proof,
        token_account_version,
        cpi_context_pubkey,
        spl_interface_pda: _,
    } = inputs;

    let mint_to_action =
        light_ctoken_interface::instructions::mint_action::MintToCompressedAction {
            token_account_version,
            recipients,
        };

    let mut instruction_data =
        light_ctoken_interface::instructions::mint_action::MintActionCompressedInstructionData::new(
            compressed_mint_inputs.clone(),
            proof,
        )
        .with_mint_to_compressed(mint_to_action);

    if let Some(ctx) = cpi_context {
        instruction_data = instruction_data.with_cpi_context(ctx);
    }

    let meta_config = if cpi_context_pubkey.is_some() {
        MintActionMetaConfig::new_cpi_context(
            &instruction_data,
            payer,
            mint_authority,
            cpi_context_pubkey.unwrap(),
        )?
    } else {
        MintActionMetaConfig::new(
            payer,
            mint_authority,
            state_merkle_tree,
            input_queue,
            output_queue_cmint,
        )
        .with_mint_compressed_tokens()
    };

    let account_metas = meta_config.to_account_metas();

    let data = instruction_data
        .data()
        .map_err(|_| CTokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(
            light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID,
        ),
        accounts: account_metas,
        data,
    })
}
