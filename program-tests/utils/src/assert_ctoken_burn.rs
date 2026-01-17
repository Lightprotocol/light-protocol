use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_compressible::compression_info::CompressionInfo;
use light_program_test::LightProgramTest;
use light_token_interface::state::{extensions::ExtensionStruct, Mint, Token};
use solana_sdk::pubkey::Pubkey;

/// Extract CompressionInfo from Light Token's Compressible extension
fn get_ctoken_compression_info(ctoken: &Token) -> Option<CompressionInfo> {
    ctoken
        .extensions
        .as_ref()?
        .iter()
        .find_map(|ext| match ext {
            ExtensionStruct::Compressible(comp) => Some(comp.info),
            _ => None,
        })
}

/// Assert that a ctoken burn was successful by checking complete account state.
/// Automatically retrieves pre-transaction state from the cached context.
///
/// # Arguments
/// * `rpc` - RPC client to fetch account data (must be LightProgramTest)
/// * `ctoken_account` - Source Light Token account pubkey
/// * `mint_account` - Mint account pubkey
/// * `burn_amount` - Amount that was burned
///
/// # Assertions
/// * Light Token balance decreased by burn amount
/// * Mint supply decreased by burn amount
/// * Compressible extensions preserved (if present)
/// * Lamport top-ups applied correctly (if compressible)
pub async fn assert_ctoken_burn(
    rpc: &mut LightProgramTest,
    ctoken_account: Pubkey,
    mint_account: Pubkey,
    burn_amount: u64,
) {
    // Get pre-transaction state from cache
    let ctoken_before = rpc
        .get_pre_transaction_account(&ctoken_account)
        .expect("Light Token account should exist in pre-transaction context");
    let mint_before = rpc
        .get_pre_transaction_account(&mint_account)
        .expect("Mint account should exist in pre-transaction context");

    // Get post-transaction state
    let ctoken_after = rpc
        .get_account(ctoken_account)
        .await
        .expect("Failed to get Light Token account after transaction")
        .expect("Light Token account should exist after transaction");
    let mint_after = rpc
        .get_account(mint_account)
        .await
        .expect("Failed to get Mint account after transaction")
        .expect("Mint account should exist after transaction");

    // Parse accounts using Borsh
    let ctoken_parsed_before: Token =
        BorshDeserialize::deserialize(&mut ctoken_before.data.as_slice())
            .expect("Failed to deserialize Light Token before");
    let ctoken_parsed_after: Token =
        BorshDeserialize::deserialize(&mut ctoken_after.data.as_slice())
            .expect("Failed to deserialize Light Token after");
    let mint_parsed_before: Mint = BorshDeserialize::deserialize(&mut mint_before.data.as_slice())
        .expect("Failed to deserialize Mint before");
    let mint_parsed_after: Mint = BorshDeserialize::deserialize(&mut mint_after.data.as_slice())
        .expect("Failed to deserialize Mint after");

    // Build expected Light Token state
    let mut expected_ctoken = ctoken_parsed_before.clone();
    expected_ctoken.amount -= burn_amount;

    // Build expected Mint state
    let mut expected_mint = mint_parsed_before.clone();
    expected_mint.base.supply -= burn_amount;

    // Assert full Light Token struct
    assert_eq!(
        ctoken_parsed_after, expected_ctoken,
        "Light Token state mismatch after burn. burn_amount: {}",
        burn_amount
    );

    // Assert full Mint struct
    assert_eq!(
        mint_parsed_after, expected_mint,
        "Mint state mismatch after burn. burn_amount: {}",
        burn_amount
    );

    // Calculate expected lamport changes only if account is compressible
    if let Some(ctoken_compression) = get_ctoken_compression_info(&ctoken_parsed_before) {
        let current_slot = rpc.get_slot().await.unwrap();

        let expected_ctoken_lamport_change = calculate_expected_lamport_change(
            rpc,
            &ctoken_compression,
            ctoken_before.data.len(),
            current_slot,
            ctoken_before.lamports,
        )
        .await;

        let expected_mint_lamport_change = calculate_expected_lamport_change(
            rpc,
            &mint_parsed_before.compression,
            mint_before.data.len(),
            current_slot,
            mint_before.lamports,
        )
        .await;

        let actual_ctoken_lamport_change =
            ctoken_after.lamports.saturating_sub(ctoken_before.lamports);
        let actual_mint_lamport_change = mint_after.lamports.saturating_sub(mint_before.lamports);

        // Assert lamport changes
        assert_eq!(
            (actual_ctoken_lamport_change, actual_mint_lamport_change),
            (expected_ctoken_lamport_change, expected_mint_lamport_change),
            "Lamport changes mismatch after burn"
        );
    }
}

async fn calculate_expected_lamport_change(
    _rpc: &mut LightProgramTest,
    compression: &light_compressible::compression_info::CompressionInfo,
    data_len: usize,
    current_slot: u64,
    current_lamports: u64,
) -> u64 {
    compression
        .calculate_top_up_lamports(data_len as u64, current_slot, current_lamports)
        .unwrap()
}
