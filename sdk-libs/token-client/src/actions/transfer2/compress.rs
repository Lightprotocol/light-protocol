use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::instructions::transfer2::{
    create_generic_transfer2_instruction, CompressInput, Transfer2InstructionType,
};

/// Create a compression instruction to convert SPL tokens to compressed tokens.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `solana_token_account` - The SPL token account to compress from
/// * `amount` - Amount of tokens to compress
/// * `to` - Recipient pubkey for the compressed tokens
/// * `authority` - Authority that can spend from the token account
/// * `payer` - Transaction fee payer
///
/// # Returns
/// `Result<Instruction, TokenSdkError>` - The compression instruction
pub async fn compress<R: Rpc + Indexer>(
    rpc: &mut R,
    solana_token_account: Pubkey,
    amount: u64,
    to: Pubkey,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Get mint from token account
    let token_account_info = rpc
        .get_account(solana_token_account)
        .await?
        .ok_or_else(|| RpcError::CustomError("Token account not found".to_string()))?;

    let pod_account = pod_from_bytes::<PodAccount>(&token_account_info.data[..165])
        .map_err(|e| RpcError::CustomError(format!("Failed to parse token account: {}", e)))?;

    let output_queue = rpc.get_random_state_tree_info()?.get_output_pubkey()?;

    let mint = pod_account.mint;

    let ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None,
            solana_token_account,
            to,
            mint,
            amount,
            authority: authority.pubkey(),
            output_queue,
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
