use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::transfer2::{
    create_generic_transfer2_instruction, CompressAndCloseInput, Transfer2InstructionType,
};

/// Compress all tokens from a ctoken account and close it in a single transaction.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `solana_ctoken_account` - The compressible token account to compress from and close
/// * `authority` - Authority that can spend from and close the token account (owner or rent authority)
/// * `payer` - Transaction fee payer
/// * `destination` - Optional destination for compression incentive (defaults to authority)
///
/// # Returns
/// `Result<Signature, RpcError>` - Transaction signature
pub async fn compress_and_close<R: Rpc + Indexer>(
    rpc: &mut R,
    solana_ctoken_account: Pubkey,
    authority: &Keypair,
    payer: &Keypair,
    destination: Option<Pubkey>,
) -> Result<Signature, RpcError> {
    // Get output queue for compression
    let output_queue = rpc.get_random_state_tree_info()?.get_output_pubkey()?;

    // Create single compress_and_close instruction
    let compress_and_close_ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::CompressAndClose(
            CompressAndCloseInput {
                solana_ctoken_account,
                authority: authority.pubkey(),
                output_queue,
                destination,
                is_compressible: true, // This function is for compressible accounts
            },
        )],
        payer.pubkey(),
        false,
    )
    .await
    .map_err(|e| RpcError::CustomError(e.to_string()))?;

    // Prepare signers
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    // Send transaction with single instruction
    rpc.create_and_send_transaction(&[compress_and_close_ix], &payer.pubkey(), &signers)
        .await
}
