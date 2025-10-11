pub use light_compressed_token_types::account_infos::mint_to_compressed::DecompressedMintConfig;
use light_compressed_token_types::CompressedProof;
use light_ctoken_types::instructions::mint_action::{
    CompressedMintWithContext, CpiContext, Recipient,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_action::{
        create_mint_action_cpi, MintActionInputs, MintActionType, MintToRecipient,
    },
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
    pub token_pool: Option<crate::instructions::mint_action::TokenPool>,
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
        output_queue_tokens,
        decompressed_mint_config: _,
        proof,
        token_account_version,
        cpi_context_pubkey,
        token_pool,
    } = inputs;

    // Convert Recipients to MintToRecipients
    let mint_to_recipients: Vec<MintToRecipient> = recipients
        .into_iter()
        .map(|recipient| MintToRecipient {
            recipient: solana_pubkey::Pubkey::from(recipient.recipient.to_bytes()),
            amount: recipient.amount,
        })
        .collect();

    // Create mint action inputs
    // For existing mint operations, we don't need a mint_seed since we can use the SPL mint directly
    // from the compressed_mint_inputs data. We use a dummy value that won't be used.
    let mint_action_inputs = MintActionInputs {
        compressed_mint_inputs,
        mint_seed: solana_pubkey::Pubkey::default(), // Dummy value, not used for existing mints
        create_mint: false,                          // Never creating mint in mint_to_compressed
        mint_bump: None,                             // No mint creation
        authority: mint_authority,
        payer,
        proof,
        actions: vec![MintActionType::MintTo {
            recipients: mint_to_recipients,
            token_account_version, // From inputs parameter
        }],
        address_tree_pubkey: state_merkle_tree, // State tree where compressed mint is stored
        input_queue: Some(input_queue),         // Input queue from compressed mint tree
        output_queue: output_queue_cmint,       // Output queue for updated compressed mint
        tokens_out_queue: Some(output_queue_tokens), // Output queue for new token accounts
        token_pool, // Required if the mint is decompressed for SPL operations
                    /*
                    cpi_context: cpi_context.map(|ctx| {
                        light_ctoken_types::instructions::mint_action::CpiContext {
                            set_context: ctx.set_context,
                            first_set_context: ctx.first_set_context,
                            in_tree_index: ctx.in_tree_index,
                            in_queue_index: ctx.in_queue_index,
                            out_queue_index: ctx.out_queue_index,
                            token_out_queue_index: ctx.token_out_queue_index,
                            assigned_account_index: 0, // Default value for mint operation
                        }
                    }),
                    cpi_context_pubkey,*/
    };

    // Use mint_action instruction internally
    create_mint_action_cpi(
        mint_action_inputs,
        cpi_context.map(|ctx| {
            light_ctoken_types::instructions::mint_action::CpiContext {
                set_context: ctx.set_context,
                first_set_context: ctx.first_set_context,
                in_tree_index: ctx.in_tree_index,
                in_queue_index: ctx.in_queue_index,
                out_queue_index: ctx.out_queue_index,
                token_out_queue_index: ctx.token_out_queue_index,
                assigned_account_index: 0, // Default value for mint operation
                ..Default::default()
            }
        }),
        cpi_context_pubkey,
    )
    .map_err(|e| {
        TokenSdkError::CpiError(format!("Failed to create mint_action instruction: {:?}", e))
    })
}
