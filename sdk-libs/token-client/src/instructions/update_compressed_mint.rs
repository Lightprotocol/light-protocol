use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::{
    instructions::update_compressed_mint::{update_compressed_mint, UpdateCompressedMintInputs},
    CompressedMintAuthorityType,
};
use light_ctoken_types::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMint,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Update a compressed mint authority instruction with automatic setup.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `authority_type` - Type of authority to update (mint or freeze)
/// * `current_authority` - Current authority keypair (signer)
/// * `new_authority` - New authority (None to revoke)
/// * `mint_authority` - Current mint authority (needed for freeze authority updates)
/// * `compressed_mint_hash` - Hash of the compressed mint to update
/// * `compressed_mint_leaf_index` - Leaf index of the compressed mint
/// * `payer` - Fee payer pubkey
///
/// # Returns
/// `Result<Instruction, RpcError>` - The update compressed mint instruction
#[allow(clippy::too_many_arguments)]
pub async fn update_compressed_mint_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    authority_type: CompressedMintAuthorityType,
    current_authority: &Keypair,
    new_authority: Option<Pubkey>,
    mint_authority: Option<Pubkey>,
    compressed_mint_hash: [u8; 32],
    compressed_mint_leaf_index: u32,
    compressed_mint_merkle_tree: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, RpcError> {
    // Get compressed account from indexer
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(
            &Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            None,
            None,
        )
        .await?;

    // Find the compressed mint account
    let compressed_mint_account = compressed_accounts
        .value
        .items
        .iter()
        .find(|account| {
            account.hash == compressed_mint_hash && account.leaf_index == compressed_mint_leaf_index
        })
        .ok_or_else(|| RpcError::CustomError("Compressed mint account not found".to_string()))?;

    // Get the compressed mint data
    let compressed_mint_data = compressed_mint_account
        .data
        .as_ref()
        .ok_or_else(|| RpcError::CustomError("Compressed mint data not found".to_string()))?;

    // Deserialize the compressed mint
    let compressed_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_mint_data.data.as_slice()).map_err(|e| {
            RpcError::CustomError(format!("Failed to deserialize compressed mint: {}", e))
        })?;

    // Convert to instruction data format
    let compressed_mint_instruction_data =
        CompressedMintInstructionData::try_from(compressed_mint.clone()).map_err(|e| {
            RpcError::CustomError(format!("Failed to convert compressed mint: {:?}", e))
        })?;

    // Get random state tree info for output queue
    let state_tree_info = rpc.get_random_state_tree_info()?;

    // Create the CompressedMintWithContext - using similar pattern to mint_to_compressed
    let compressed_mint_inputs = CompressedMintWithContext {
        leaf_index: compressed_mint_leaf_index,
        prove_by_index: true, // Use index-based proof like mint_to_compressed
        root_index: 0,        // Use 0 like mint_to_compressed
        address: compressed_mint_account.address.unwrap_or([0u8; 32]),
        mint: compressed_mint_instruction_data,
    };

    // Create instruction using the existing SDK function
    let inputs = UpdateCompressedMintInputs {
        compressed_mint_inputs,
        authority_type,
        new_authority,
        mint_authority,
        proof: None,
        payer,
        authority: current_authority.pubkey(),
        in_merkle_tree: compressed_mint_merkle_tree,
        in_output_queue: compressed_mint_account.tree_info.queue,
        out_output_queue: state_tree_info.queue, // Use same queue for output
    };

    update_compressed_mint(inputs)
        .map_err(|e| RpcError::CustomError(format!("Token SDK error: {:?}", e)))
}
