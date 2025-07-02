use std::collections::HashSet;

use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use solana_keypair::Keypair;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::create_spl_mint::create_spl_mint_instruction;

/// Creates an SPL mint from a compressed mint and sends the transaction
///
/// This function:
/// - Creates the create_spl_mint instruction using the instruction helper
/// - Handles signer deduplication (payer and mint_authority may be the same)
/// - Builds and sends the transaction
/// - Returns the transaction signature
///
/// # Arguments
/// * `rpc` - RPC client with indexer access
/// * `compressed_mint_address` - Address of the compressed mint to convert to SPL mint
/// * `mint_seed` - Keypair used as seed for the SPL mint PDA
/// * `mint_authority` - Keypair that can mint tokens (must be able to sign)
/// * `payer` - Keypair for transaction fees (must be able to sign)
///
/// # Returns
/// Returns the transaction signature on success
pub async fn create_spl_mint<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
    mint_seed: &Keypair,
    mint_authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Create the instruction
    let instruction = create_spl_mint_instruction(
        rpc,
        compressed_mint_address,
        mint_seed,
        mint_authority.pubkey(),
        payer.pubkey(),
    )
    .await?;

    // Deduplicate signers (payer and mint_authority might be the same)
    let mut unique_signers = HashSet::new();
    let mut signers = Vec::new();

    // Always include payer
    if unique_signers.insert(payer.pubkey()) {
        signers.push(payer);
    }

    // Include mint_authority if different from payer
    if unique_signers.insert(mint_authority.pubkey()) {
        signers.push(mint_authority);
    }
    println!("unique_signers {:?}", unique_signers);

    // Create and send the transaction
    let signature = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await?;

    Ok(signature)
}
