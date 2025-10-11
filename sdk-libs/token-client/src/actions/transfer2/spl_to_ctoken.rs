use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::{
    account2::create_spl_to_ctoken_transfer_instruction, token_pool::find_token_pool_pda_with_index,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

/// Transfer SPL tokens directly to compressed tokens in a single transaction.
///
/// This function wraps `create_spl_to_ctoken_transfer_instruction` to provide
/// a convenient action for transferring from SPL token accounts to compressed tokens.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `source_spl_token_account` - The SPL token account to transfer from
/// * `to` - Recipient pubkey for the compressed tokens
/// * `amount` - Amount of tokens to transfer
/// * `authority` - Authority that can spend from the SPL token account
/// * `payer` - Transaction fee payer
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn spl_to_ctoken_transfer<R: Rpc + Indexer>(
    rpc: &mut R,
    source_spl_token_account: Pubkey,
    to: Pubkey,
    amount: u64,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Get mint from SPL token account
    let token_account_info = rpc
        .get_account(source_spl_token_account)
        .await?
        .ok_or_else(|| RpcError::CustomError("SPL token account not found".to_string()))?;

    let pod_account = pod_from_bytes::<PodAccount>(&token_account_info.data)
        .map_err(|e| RpcError::CustomError(format!("Failed to parse SPL token account: {}", e)))?;

    let mint = pod_account.mint;

    // Derive token pool PDA
    let (token_pool_pda, bump) = find_token_pool_pda_with_index(&mint, 0);

    // Create the SPL to CToken transfer instruction
    let ix = create_spl_to_ctoken_transfer_instruction(
        source_spl_token_account,
        to,
        amount,
        authority.pubkey(),
        mint,
        payer.pubkey(),
        token_pool_pda,
        bump,
    )
    .map_err(|e| RpcError::CustomError(e.to_string()))?;

    // Prepare signers
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await
}
