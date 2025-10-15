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

/// Transfer compressed tokens using delegated authority.
/// The delegate must be the signer, not the owner.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `compressed_token_account` - Slice of compressed token accounts with delegate set
/// * `to` - Recipient pubkey for the compressed tokens
/// * `amount` - Amount of tokens to transfer
/// * `delegate` - The delegate keypair that has authority to transfer
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn transfer_delegated<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_token_account: &[CompressedTokenAccount],
    to: Pubkey,
    amount: u64,
    delegate: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Verify that all accounts have the delegate set
    for account in compressed_token_account {
        if account.token.delegate != Some(delegate.pubkey()) {
            return Err(RpcError::CustomError(format!(
                "Account does not have delegate {} set. Found: {:?}",
                delegate.pubkey(),
                account.token.delegate
            )));
        }
    }

    let ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: compressed_token_account.to_vec(),
            to,
            amount,
            is_delegate_transfer: true, // Delegate transfer, delegate is signer
            mint: None,                 // Not needed when input accounts are provided
            change_amount: None,
        })],
        payer.pubkey(),
        false,
    )
    .await
    .map_err(|e| RpcError::CustomError(e.to_string()))?;

    let mut signers = vec![payer];
    if delegate.pubkey() != payer.pubkey() {
        signers.push(delegate);
    }

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await
}
