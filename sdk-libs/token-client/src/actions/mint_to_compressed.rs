use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_ctoken_types::{instructions::mint_action::Recipient, state::TokenDataVersion};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::mint_to_compressed::mint_to_compressed_instruction;

/// Mints compressed tokens to recipients using a higher-level action
///
/// # Arguments
/// * `rpc` - RPC client with indexer access
/// * `spl_mint_pda` - The SPL mint PDA for the compressed mint
/// * `recipients` - Vector of Recipient structs containing recipient and amount
/// * `mint_authority` - Authority that can mint tokens
/// * `payer` - Account that pays for the transaction
/// * `lamports` - Optional lamports to add to new token accounts
pub async fn mint_to_compressed<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipients: Vec<Recipient>,
    token_data_version: TokenDataVersion,
    mint_authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Create the instruction
    let instruction = mint_to_compressed_instruction(
        rpc,
        spl_mint_pda,
        recipients,
        token_data_version,
        mint_authority.pubkey(),
        payer.pubkey(),
    )
    .await?;

    // Determine signers (deduplicate if payer and mint_authority are the same)
    let signers: Vec<&Keypair> = if payer.pubkey() == mint_authority.pubkey() {
        vec![payer]
    } else {
        vec![payer, mint_authority]
    };

    // Send the transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}
