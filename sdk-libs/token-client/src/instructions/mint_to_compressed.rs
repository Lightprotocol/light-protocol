use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::{
    instructions::{
        create_mint_to_compressed_instruction, derive_compressed_mint_from_spl_mint,
        DecompressedMintConfig, MintToCompressedInputs,
    },
    token_pool::find_token_pool_pda_with_index,
};
use light_ctoken_types::{
    instructions::mint_to_compressed::{CompressedMintInputs, Recipient},
    state::CompressedMint,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Creates a mint_to_compressed instruction that mints compressed tokens to recipients
pub async fn mint_to_compressed_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipients: Vec<Recipient>,
    mint_authority: Pubkey,
    payer: Pubkey,
    lamports: Option<u64>,
) -> Result<Instruction, RpcError> {
    // Derive compressed mint address from SPL mint PDA
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_from_spl_mint(&spl_mint_pda, &address_tree_pubkey);

    // Get the compressed mint account
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value;

    // Deserialize the compressed mint
    let compressed_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .map_err(|e| {
            RpcError::CustomError(format!("Failed to deserialize compressed mint: {}", e))
        })?;

    // Get state tree info for outputs
    let state_tree_info = rpc.get_random_state_tree_info()?;

    // Create decompressed mint config if mint is decompressed
    let decompressed_mint_config = if compressed_mint.is_decompressed {
        let (token_pool_pda, _) = find_token_pool_pda_with_index(&spl_mint_pda, 0);
        Some(DecompressedMintConfig {
            mint_pda: spl_mint_pda,
            token_pool_pda,
            token_program: spl_token_2022::ID,
        })
    } else {
        None
    };

    // Prepare compressed mint inputs
    let compressed_mint_inputs = CompressedMintInputs {
        prove_by_index: true,
        leaf_index: compressed_mint_account.leaf_index,
        root_index: 0,
        address: compressed_mint_address,
        compressed_mint_input: compressed_mint,
    };

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
    .map_err(|e| {
        RpcError::CustomError(format!(
            "Failed to create mint_to_compressed instruction: {:?}",
            e
        ))
    })
}
