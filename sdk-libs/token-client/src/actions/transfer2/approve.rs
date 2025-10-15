use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::{Rpc, RpcError},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::transfer2::{
    create_generic_transfer2_instruction, ApproveInput, Transfer2InstructionType,
};

/// Approve a delegate for compressed tokens and send the transaction.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `compressed_token_account` - Slice of compressed token accounts to approve from
/// * `delegate` - The delegate pubkey to approve
/// * `delegate_amount` - Amount of tokens to delegate
/// * `authority` - Authority that owns the compressed token account
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn approve<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_token_account: &[CompressedTokenAccount],
    delegate: Pubkey,
    delegate_amount: u64,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Approve(ApproveInput {
            compressed_token_account: compressed_token_account.to_vec(),
            delegate,
            delegate_amount,
        })],
        payer.pubkey(),
        true,
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
