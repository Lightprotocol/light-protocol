use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_registry::{
    accounts::ClaimContext as ClaimAccounts,
    utils::get_forester_epoch_pda_from_authority,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use std::str::FromStr;

/// Claim rent from compressible token accounts via the registry program
///
/// This function invokes the registry program's claim instruction,
/// which then CPIs to the compressed token program with the correct rent_authority PDA signer.
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
    // Registry and compressed token program IDs
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressed_token_program_id =
        Pubkey::from_str("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m").unwrap();

    let current_epoch = 0;

    // Derive registered forester PDA for the current epoch
    let (registered_forester_pda, _) =
        get_forester_epoch_pda_from_authority(&authority.pubkey(), current_epoch);

    // Derive CompressibleConfig PDA (version 1)
    let version: u64 = 1;
    let (compressible_config, _) = Pubkey::find_program_address(
        &[b"compressible_config", &version.to_le_bytes()],
        &registry_program_id,
    );

    // Derive rent_authority PDA (uses u16 version)
    let (rent_authority, _) = Pubkey::find_program_address(
        &[
            b"rent_authority".as_slice(),
            (version as u16).to_le_bytes().as_slice(),
            &[0],
        ],
        &registry_program_id,
    );

    // Derive rent_recipient PDA from the compressed token program (not registry)
    let (rent_recipient, _) = Pubkey::find_program_address(
        &[
            b"rent_recipient".as_slice(),
            (version as u16).to_le_bytes().as_slice(),
            &[0],
        ],
        &compressed_token_program_id,
    );

    // Build accounts using Anchor's account abstraction
    let claim_accounts = ClaimAccounts {
        authority: authority.pubkey(),
        registered_forester_pda,
        rent_recipient,
        rent_authority,
        compressible_config,
        compressed_token_program: compressed_token_program_id,
    };

    // Get account metas from Anchor accounts
    let mut accounts = claim_accounts.to_account_metas(Some(true));

    // Add token accounts as remaining accounts
    for token_account in token_accounts {
        accounts.push(solana_sdk::instruction::AccountMeta::new(*token_account, false));
    }

    // Create Anchor instruction with proper discriminator
    // The registry program's claim function doesn't take any instruction data
    // beyond the discriminator, so we just need to generate the discriminator
    use light_registry::instruction::Claim;
    let instruction = Claim {};
    let instruction_data = instruction.data();

    // Create the instruction
    let claim_ix = Instruction {
        program_id: registry_program_id,
        accounts,
        data: instruction_data,
    };

    // Prepare signers
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[claim_ix], &payer.pubkey(), &signers)
        .await
}