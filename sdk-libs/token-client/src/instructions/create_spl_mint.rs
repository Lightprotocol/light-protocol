use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::instructions::{
    create_spl_mint_instruction as sdk_create_spl_mint_instruction, derive_token_pool,
    find_spl_mint_address, CreateSplMintInputs,
};
use light_ctoken_types::{
    instructions::mint_action::CompressedMintWithContext, state::CompressedMint,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Creates a create_spl_mint instruction with rpc.
///
/// This function automatically:
/// - Fetches the compressed mint account data
/// - Gets validity proof for the compressed mint
/// - Derives the necessary PDAs and tree information
/// - Constructs the complete instruction
///
/// # Arguments
/// * `rpc` - RPC client with indexer access
/// * `compressed_mint_address` - Address of the compressed mint to convert to SPL mint
/// * `mint_seed` - Keypair used as seed for the SPL mint PDA
/// * `mint_authority` - Authority that can mint tokens
/// * `payer` - Transaction fee payer
///
/// # Returns
/// Returns a configured `Instruction` ready for transaction execution
pub async fn create_spl_mint_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
    mint_seed: &Keypair,
    mint_authority: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, RpcError> {
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value;

    let compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut compressed_mint_account
            .data
            .as_ref()
            .ok_or_else(|| {
                RpcError::CustomError("Compressed mint account has no data".to_string())
            })?
            .data
            .as_slice(),
    )
    .map_err(|e| RpcError::CustomError(format!("Failed to deserialize compressed mint: {}", e)))?;

    let proof_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await?
        .value;

    let (spl_mint_pda, mint_bump) = find_spl_mint_address(&mint_seed.pubkey());

    let token_pool = derive_token_pool(&spl_mint_pda, 0);

    let input_tree = compressed_mint_account.tree_info.tree;
    let input_queue = compressed_mint_account.tree_info.queue;

    let output_tree_info = rpc.get_random_state_tree_info()?;
    let output_queue = output_tree_info.queue;

    let compressed_mint_inputs = CompressedMintWithContext {
        leaf_index: compressed_mint_account.leaf_index,
        prove_by_index: true,
        root_index: proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        mint: compressed_mint.try_into().map_err(|e| {
            RpcError::CustomError(format!("Failed to create SPL mint instruction: {}", e))
        })?,
    };

    let instruction = sdk_create_spl_mint_instruction(CreateSplMintInputs {
        mint_signer: mint_seed.pubkey(),
        mint_bump,
        compressed_mint_inputs,
        proof: proof_result.proof,
        payer,
        input_merkle_tree: input_tree,
        input_output_queue: input_queue,
        output_queue,
        mint_authority,
        token_pool,
    })
    .map_err(|e| RpcError::CustomError(format!("Failed to create SPL mint instruction: {}", e)))?;

    Ok(instruction)
}
