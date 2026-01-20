use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressible::config::CompressibleConfig;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::registry_sdk::{
    build_claim_instruction, get_forester_epoch_pda_from_authority, REGISTRY_PROGRAM_ID,
};

/// Claim rent from compressible token accounts via the registry program
///
/// This function invokes the registry program's claim instruction,
/// which then CPIs to the compressed token program with the correct compression_authority PDA signer.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `token_accounts` - List of compressible token accounts to claim rent from
/// * `authority` - Authority that can execute the claim
/// * `payer` - Transaction fee payer
///
/// # Returns
/// `Result<Signature, RpcError>` - Transaction signature
pub async fn claim_forester<R: Rpc + Indexer>(
    rpc: &mut R,
    token_accounts: &[Pubkey],
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Compressed token program ID
    let compressed_token_program_id =
        Pubkey::from_str_const("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

    let current_epoch = 0;

    // Derive registered forester PDA for the current epoch
    let (registered_forester_pda, _) =
        get_forester_epoch_pda_from_authority(&authority.pubkey(), current_epoch);
    let config = CompressibleConfig::light_token_v1(Default::default(), Default::default());
    let compressible_config = CompressibleConfig::derive_v1_config_pda(&REGISTRY_PROGRAM_ID).0;
    let rent_sponsor = config.rent_sponsor;
    let compression_authority = config.compression_authority;

    // Build the claim instruction using local SDK
    let claim_ix = build_claim_instruction(
        authority.pubkey(),
        registered_forester_pda,
        rent_sponsor,
        compression_authority,
        compressible_config,
        compressed_token_program_id,
        token_accounts,
    );

    // Prepare signers
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[claim_ix], &payer.pubkey(), &signers)
        .await
}
