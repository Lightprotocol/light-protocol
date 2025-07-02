use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_ctoken_types::instructions::extensions::TokenMetadataInstructionData;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::create_mint::create_compressed_mint_instruction;

/// Create a compressed mint and send the transaction.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `mint_seed` - Keypair used to derive the mint PDA (must sign the transaction)
/// * `decimals` - Number of decimal places for the token
/// * `mint_authority_keypair` - Authority keypair that can mint tokens (must sign the transaction)
/// * `freeze_authority` - Optional authority that can freeze tokens
/// * `payer` - Transaction fee payer keypair
/// * `metadata` - Optional metadata for the token
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn create_mint<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    decimals: u8,
    mint_authority_keypair: &Keypair,
    freeze_authority: Option<Pubkey>,
    metadata: Option<TokenMetadataInstructionData>,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Create the instruction
    let ix = create_compressed_mint_instruction(
        rpc,
        mint_seed,
        decimals,
        mint_authority_keypair.pubkey(),
        freeze_authority,
        payer.pubkey(),
        metadata,
    )
    .await?;

    // Determine signers (deduplicate if any keypairs are the same)
    let mut signers = vec![payer];
    if mint_seed.pubkey() != payer.pubkey() {
        signers.push(mint_seed);
    }
    if mint_authority_keypair.pubkey() != payer.pubkey()
        && mint_authority_keypair.pubkey() != mint_seed.pubkey()
    {
        signers.push(mint_authority_keypair);
    }

    // Send the transaction
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await
}
