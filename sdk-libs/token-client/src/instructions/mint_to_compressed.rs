use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::{indexer::Indexer, rpc::RpcError};
use light_compressed_token_sdk::instructions::{
    create_mint_to_compressed_instruction, DecompressedMintConfig, MintToCompressedInputs,
};
use light_ctoken_types::{
    instructions::mint_to_compressed::{CompressedMintInputs, Recipient},
    state::CompressedMint,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair};

/// Creates a mint_to_compressed instruction that mints compressed tokens to recipients
pub async fn create_mint_to_compressed_instruction_helper<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
    recipients: Vec<(Pubkey, u64)>, // (recipient, amount) pairs
    mint_authority: Pubkey,
    payer: Pubkey,
    lamports: Option<u64>,
    decompressed_mint_config: Option<DecompressedMintConfig>,
) -> Result<Instruction, RpcError> {
    // Get the compressed mint account
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value;

    // Deserialize the compressed mint
    let compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut compressed_mint_account
            .data
            .unwrap()
            .data
            .as_slice(),
    )
    .map_err(|e| RpcError::CustomError(format!("Failed to deserialize compressed mint: {}", e)))?;

    // Get state tree info for outputs
    let state_tree_info = rpc.get_random_state_tree_info()
        .ok_or_else(|| RpcError::CustomError("No state tree available".to_string()))?;
    
    // Prepare compressed mint inputs
    let compressed_mint_inputs = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0,
            queue_pubkey_index: 1,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: 0,
        address: compressed_mint_address,
        compressed_mint_input: compressed_mint,
        output_merkle_tree_index: 3,
    };

    // Convert recipients to the expected format
    let recipients: Vec<Recipient> = recipients
        .into_iter()
        .map(|(recipient, amount)| Recipient {
            recipient: recipient.into(),
            amount,
        })
        .collect();

    // Create the instruction
    create_mint_to_compressed_instruction(MintToCompressedInputs {
        compressed_mint_inputs,
        lamports,
        recipients,
        mint_authority,
        payer,
        state_merkle_tree: compressed_mint_account.tree_info.tree,
        output_queue: compressed_mint_account.tree_info.queue,
        state_tree_pubkey: state_tree_info.tree,
        decompressed_mint_config,
    })
    .map_err(|e| RpcError::CustomError(format!("Failed to create mint_to_compressed instruction: {:?}", e)))
}