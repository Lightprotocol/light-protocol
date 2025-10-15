use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::{Rpc, RpcError},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::transfer2::{
    create_generic_transfer2_instruction, Transfer2InstructionType, TransferInput,
};

/// Transfer compressed tokens between compressed accounts and send the transaction.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `compressed_token_account` - Slice of compressed token accounts to transfer from
/// * `to` - Recipient pubkey for the compressed tokens
/// * `amount` - Amount of tokens to transfer
/// * `authority` - Authority that can spend from the compressed token account
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn transfer<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_token_account: &[CompressedTokenAccount],
    to: Pubkey,
    amount: u64,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: compressed_token_account.to_vec(),
            to,
            amount,
            is_delegate_transfer: false, // Regular transfer, owner is signer
            mint: None,                  // Not needed when input accounts are provided
            change_amount: None,
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
