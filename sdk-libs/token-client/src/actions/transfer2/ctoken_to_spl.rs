use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_ctoken_sdk::{
    constants::SPL_TOKEN_PROGRAM_ID, ctoken::TransferCTokenToSpl,
    spl_interface::find_spl_interface_pda,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Transfer tokens from a compressed token account to an SPL token account
#[allow(clippy::too_many_arguments)]
pub async fn transfer_ctoken_to_spl<R: Rpc + Indexer>(
    rpc: &mut R,
    source_ctoken_account: Pubkey,
    destination_spl_token_account: Pubkey,
    amount: u64,
    authority: &Keypair,
    mint: Pubkey,
    payer: &Keypair,
    decimals: u8,
) -> Result<Signature, RpcError> {
    let (spl_interface_pda, spl_interface_pda_bump) = find_spl_interface_pda(&mint, false);

    let transfer_ix = TransferCTokenToSpl {
        source_ctoken_account,
        destination_spl_token_account,
        amount,
        authority: authority.pubkey(),
        mint,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_interface_pda_bump,
        spl_token_program: SPL_TOKEN_PROGRAM_ID, // TODO: make dynamic
        decimals,
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    rpc.create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &signers)
        .await
}
