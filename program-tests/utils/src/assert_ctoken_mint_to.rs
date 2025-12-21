use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_ctoken_interface::state::{CToken, CompressedMint};
use light_program_test::LightProgramTest;
use solana_sdk::pubkey::Pubkey;

/// Assert that a ctoken mint_to was successful by checking complete account state.
/// Automatically retrieves pre-transaction state from the cached context.
///
/// # Arguments
/// * `rpc` - RPC client to fetch account data (must be LightProgramTest)
/// * `ctoken_account` - Destination CToken account pubkey
/// * `cmint_account` - CMint account pubkey
/// * `mint_amount` - Amount that was minted
///
/// # Assertions
/// * CToken balance increased by mint amount
/// * CMint supply increased by mint amount
/// * Compressible extensions preserved (if present)
/// * Lamport top-ups applied correctly (if compressible)
pub async fn assert_ctoken_mint_to(
    rpc: &mut LightProgramTest,
    ctoken_account: Pubkey,
    cmint_account: Pubkey,
    mint_amount: u64,
) {
    // Get pre-transaction state from cache
    let ctoken_before = rpc
        .get_pre_transaction_account(&ctoken_account)
        .expect("CToken account should exist in pre-transaction context");
    let cmint_before = rpc
        .get_pre_transaction_account(&cmint_account)
        .expect("CMint account should exist in pre-transaction context");

    // Get post-transaction state
    let ctoken_after = rpc
        .get_account(ctoken_account)
        .await
        .expect("Failed to get CToken account after transaction")
        .expect("CToken account should exist after transaction");
    let cmint_after = rpc
        .get_account(cmint_account)
        .await
        .expect("Failed to get CMint account after transaction")
        .expect("CMint account should exist after transaction");

    // Parse accounts using Borsh
    let ctoken_parsed_before: CToken =
        BorshDeserialize::deserialize(&mut ctoken_before.data.as_slice())
            .expect("Failed to deserialize CToken before");
    let ctoken_parsed_after: CToken =
        BorshDeserialize::deserialize(&mut ctoken_after.data.as_slice())
            .expect("Failed to deserialize CToken after");
    let cmint_parsed_before: CompressedMint =
        BorshDeserialize::deserialize(&mut cmint_before.data.as_slice())
            .expect("Failed to deserialize CMint before");
    let cmint_parsed_after: CompressedMint =
        BorshDeserialize::deserialize(&mut cmint_after.data.as_slice())
            .expect("Failed to deserialize CMint after");

    // Build expected CToken state
    let mut expected_ctoken = ctoken_parsed_before.clone();
    expected_ctoken.amount += mint_amount;

    // Build expected CMint state
    let mut expected_cmint = cmint_parsed_before.clone();
    expected_cmint.base.supply += mint_amount;

    // Assert full CToken struct
    assert_eq!(
        ctoken_parsed_after, expected_ctoken,
        "CToken state mismatch after mint_to. mint_amount: {}",
        mint_amount
    );

    // Assert full CMint struct
    assert_eq!(
        cmint_parsed_after, expected_cmint,
        "CMint state mismatch after mint_to. mint_amount: {}",
        mint_amount
    );

    // Calculate expected lamport changes
    let current_slot = rpc.get_slot().await.unwrap();

    let expected_ctoken_lamport_change = calculate_expected_lamport_change(
        rpc,
        &ctoken_parsed_before.compression,
        ctoken_before.data.len(),
        current_slot,
        ctoken_before.lamports,
    )
    .await;

    let expected_cmint_lamport_change = calculate_expected_lamport_change(
        rpc,
        &cmint_parsed_before.compression,
        cmint_before.data.len(),
        current_slot,
        cmint_before.lamports,
    )
    .await;

    let actual_ctoken_lamport_change = ctoken_after.lamports.saturating_sub(ctoken_before.lamports);
    let actual_cmint_lamport_change = cmint_after.lamports.saturating_sub(cmint_before.lamports);

    // Assert lamport changes
    assert_eq!(
        (actual_ctoken_lamport_change, actual_cmint_lamport_change),
        (
            expected_ctoken_lamport_change,
            expected_cmint_lamport_change
        ),
        "Lamport changes mismatch after mint_to"
    );
}

async fn calculate_expected_lamport_change(
    rpc: &mut LightProgramTest,
    compression: &light_compressible::compression_info::CompressionInfo,
    data_len: usize,
    current_slot: u64,
    current_lamports: u64,
) -> u64 {
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(data_len)
        .await
        .unwrap();
    compression
        .calculate_top_up_lamports(
            data_len as u64,
            current_slot,
            current_lamports,
            rent_exemption,
        )
        .unwrap()
}
