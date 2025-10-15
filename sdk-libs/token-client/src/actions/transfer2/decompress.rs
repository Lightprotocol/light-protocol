use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::{Rpc, RpcError},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};

/// Decompress compressed tokens to SPL tokens and send the transaction.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `compressed_token_account` - Slice of compressed token accounts to decompress
/// * `decompress_amount` - Amount of tokens to decompress
/// * `solana_token_account` - The SPL token account to receive the decompressed tokens
/// * `authority` - Authority that can spend from the compressed token account
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn decompress<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_token_account: &[CompressedTokenAccount],
    decompress_amount: u64,
    solana_token_account: Pubkey,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: compressed_token_account.to_vec(),
            decompress_amount,
            solana_token_account,
            amount: decompress_amount,
            pool_index: None,
        })],
        payer.pubkey(),
        false,
    )
    .await
    .map_err(|e| RpcError::CustomError(e.to_string()))?;

    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await
}
