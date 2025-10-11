use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::{
    account2::create_ctoken_to_spl_transfer_instruction, token_pool::find_token_pool_pda_with_index,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Transfer tokens from a compressed token account to an SPL token account
pub async fn ctoken_to_spl_transfer<R: Rpc + Indexer>(
    rpc: &mut R,
    source_ctoken_account: Pubkey,
    destination_spl_token_account: Pubkey,
    amount: u64,
    authority: &Keypair,
    mint: Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Derive token pool PDA with bump
    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);

    // Create the transfer instruction
    let transfer_ix = create_ctoken_to_spl_transfer_instruction(
        source_ctoken_account,
        destination_spl_token_account,
        amount,
        authority.pubkey(),
        mint,
        payer.pubkey(),
        token_pool_pda,
        token_pool_pda_bump,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Build and send transaction
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    rpc.create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &signers)
        .await
}
