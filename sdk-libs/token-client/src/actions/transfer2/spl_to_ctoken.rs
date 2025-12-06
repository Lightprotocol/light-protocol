use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_ctoken_sdk::{
    ctoken::TransferSplToCtoken, spl_interface::find_spl_interface_pda_with_index,
    SPL_TOKEN_PROGRAM_ID,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

/// Transfer SPL tokens to compressed tokens
pub async fn spl_to_ctoken_transfer<R: Rpc + Indexer>(
    rpc: &mut R,
    source_spl_token_account: Pubkey,
    destination_ctoken_account: Pubkey,
    amount: u64,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let token_account_info = rpc
        .get_account(source_spl_token_account)
        .await?
        .ok_or_else(|| RpcError::CustomError("SPL token account not found".to_string()))?;

    let pod_account = pod_from_bytes::<PodAccount>(&token_account_info.data)
        .map_err(|e| RpcError::CustomError(format!("Failed to parse SPL token account: {}", e)))?;

    let mint = pod_account.mint;

    let (spl_interface_pda, spl_interface_pda_bump) = find_spl_interface_pda_with_index(&mint, 0);

    let ix = TransferSplToCtoken {
        amount,
        spl_interface_pda_bump,
        source_spl_token_account,
        destination_ctoken_account,
        authority: authority.pubkey(),
        mint,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID), // TODO: make dynamic
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(e.to_string()))?;

    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await
}
