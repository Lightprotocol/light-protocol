use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::{
    instructions::{
        create_mint_to_compressed_instruction, derive_compressed_mint_from_spl_mint,
        derive_token_pool, DecompressedMintConfig, MintToCompressedInputs,
    },
    token_pool::find_token_pool_pda_with_index,
};
use light_ctoken_types::{
    instructions::mint_action::{CompressedMintWithContext, Recipient},
    state::{CompressedMint, TokenDataVersion},
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Creates a mint_to_compressed instruction that mints compressed tokens to recipients
pub async fn mint_to_compressed_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipients: Vec<Recipient>,
    token_account_version: TokenDataVersion,
    mint_authority: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, RpcError> {
    // Derive compressed mint address from SPL mint PDA
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_from_spl_mint(&spl_mint_pda, &address_tree_pubkey);

    // Get the compressed mint account
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "{:?}",
            compressed_mint_address
        )))?;

    // Deserialize the compressed mint
    let compressed_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .map_err(|e| {
            RpcError::CustomError(format!("Failed to deserialize compressed mint: {}", e))
        })?;

    let rpc_proof_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await?
        .value;

    // Get state tree info for outputs
    let state_tree_info = rpc.get_random_state_tree_info()?;

    // Create decompressed mint config and token pool if mint is decompressed
    let decompressed_mint_config = if compressed_mint.metadata.spl_mint_initialized {
        let (token_pool_pda, _) = find_token_pool_pda_with_index(&spl_mint_pda, 0);
        Some(DecompressedMintConfig {
            mint_pda: spl_mint_pda,
            token_pool_pda,
            token_program: spl_token_2022::ID,
        })
    } else {
        None
    };

    // Derive token pool if needed for decompressed mints
    let token_pool = if compressed_mint.metadata.spl_mint_initialized {
        Some(derive_token_pool(&spl_mint_pda, 0))
    } else {
        None
    };

    // Prepare compressed mint inputs
    let compressed_mint_inputs = CompressedMintWithContext {
        prove_by_index: rpc_proof_result.accounts[0].root_index.proof_by_index(),
        leaf_index: compressed_mint_account.leaf_index,
        root_index: rpc_proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        mint: compressed_mint.try_into().unwrap(),
    };

    // Create the instruction
    create_mint_to_compressed_instruction(
        MintToCompressedInputs {
            cpi_context_pubkey: None,
            compressed_mint_inputs,
            recipients,
            mint_authority,
            payer,
            state_merkle_tree: compressed_mint_account.tree_info.tree,
            input_queue: compressed_mint_account.tree_info.queue,
            output_queue_cmint: compressed_mint_account.tree_info.queue,
            output_queue_tokens: state_tree_info.queue,
            decompressed_mint_config,
            proof: rpc_proof_result.proof.into(),
            token_account_version: token_account_version as u8, // V2 for batched merkle trees
            token_pool,
        },
        None,
    )
    .map_err(|e| {
        RpcError::CustomError(format!(
            "Failed to create mint_to_compressed instruction: {:?}",
            e
        ))
    })
}
