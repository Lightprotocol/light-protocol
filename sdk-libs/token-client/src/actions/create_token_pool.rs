use light_client::rpc::{Rpc, RpcError};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::create_token_pool::create_token_pool_instruction;

/// Creates a token pool PDA for a given mint and sends the transaction.
///
/// This action creates a token pool account that can hold SPL tokens for
/// compression/decompression operations. The token pool is owned by the
/// CPI authority PDA and can be used by the compressed token program.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The SPL mint for which to create the token pool
/// * `is_token_22` - Whether this is a Token-2022 mint (vs regular SPL Token)
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn create_token_pool<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
    is_token_22: bool,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Create the instruction
    let instruction = create_token_pool_instruction(&payer.pubkey(), mint, is_token_22)?;

    // Send the transaction (only payer needs to sign)
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

/// Creates a token pool PDA for a regular SPL Token mint and sends the transaction.
///
/// This is a convenience function for creating token pools for regular SPL Token mints.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The SPL mint for which to create the token pool
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn create_spl_token_pool<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    create_token_pool(rpc, mint, false, payer).await
}

/// Creates a token pool PDA for a Token-2022 mint and sends the transaction.
///
/// This is a convenience function for creating token pools for Token-2022 mints.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The Token-2022 mint for which to create the token pool
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn create_token_22_pool<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    create_token_pool(rpc, mint, true, payer).await
}
