use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::CompressedMintAuthorityType;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::update_compressed_mint::update_compressed_mint_instruction;

/// Update compressed mint authority action
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `authority_type` - Type of authority to update (mint or freeze)
/// * `current_authority` - Current authority keypair (signer)
/// * `new_authority` - New authority (None to revoke)
/// * `mint_authority` - Current mint authority (needed for freeze authority updates)
/// * `compressed_mint_hash` - Hash of the compressed mint to update
/// * `compressed_mint_leaf_index` - Leaf index of the compressed mint
/// * `compressed_mint_merkle_tree` - Merkle tree containing the compressed mint
/// * `payer` - Fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - Transaction signature
#[allow(clippy::too_many_arguments)]
pub async fn update_compressed_mint_authority<R: Rpc + Indexer>(
    rpc: &mut R,
    authority_type: CompressedMintAuthorityType,
    current_authority: &Keypair,
    new_authority: Option<Pubkey>,
    mint_authority: Option<Pubkey>,
    compressed_mint_hash: [u8; 32],
    compressed_mint_leaf_index: u32,
    compressed_mint_merkle_tree: Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Create the update instruction
    let instruction = update_compressed_mint_instruction(
        rpc,
        authority_type,
        current_authority,
        new_authority,
        mint_authority,
        compressed_mint_hash,
        compressed_mint_leaf_index,
        compressed_mint_merkle_tree,
        payer.pubkey(),
    )
    .await?;

    // Determine signers (current_authority must sign, and payer if different)
    let mut signers = vec![current_authority];
    if current_authority.pubkey() != payer.pubkey() {
        signers.push(payer);
    }

    // Send the transaction using RPC helper
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}

/// Convenience function to update mint authority
pub async fn update_mint_authority<R: Rpc + Indexer>(
    rpc: &mut R,
    current_mint_authority: &Keypair,
    new_mint_authority: Option<Pubkey>,
    compressed_mint_hash: [u8; 32],
    compressed_mint_leaf_index: u32,
    compressed_mint_merkle_tree: Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    update_compressed_mint_authority(
        rpc,
        CompressedMintAuthorityType::MintTokens,
        current_mint_authority,
        new_mint_authority,
        Some(compressed_mint_merkle_tree),
        compressed_mint_hash,
        compressed_mint_leaf_index,
        compressed_mint_merkle_tree,
        payer,
    )
    .await
}

/// Convenience function to update freeze authority
#[allow(clippy::too_many_arguments)]
pub async fn update_freeze_authority<R: Rpc + Indexer>(
    rpc: &mut R,
    current_freeze_authority: &Keypair,
    new_freeze_authority: Option<Pubkey>,
    mint_authority: Pubkey, // Required to preserve mint authority
    compressed_mint_hash: [u8; 32],
    compressed_mint_leaf_index: u32,
    compressed_mint_merkle_tree: Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    update_compressed_mint_authority(
        rpc,
        CompressedMintAuthorityType::FreezeAccount,
        current_freeze_authority,
        new_freeze_authority,
        Some(mint_authority),
        compressed_mint_hash,
        compressed_mint_leaf_index,
        compressed_mint_merkle_tree,
        payer,
    )
    .await
}
