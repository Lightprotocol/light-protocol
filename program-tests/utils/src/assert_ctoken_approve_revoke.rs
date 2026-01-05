//! Assertion helpers for CToken approve and revoke operations.
//!
//! These functions verify that approve/revoke operations correctly modify
//! only the delegate and delegated_amount fields while preserving all other
//! account state including compression info and extensions.

use anchor_lang::AnchorDeserialize;
use light_client::rpc::Rpc;
use light_ctoken_interface::state::CToken;
use light_program_test::LightProgramTest;
use solana_sdk::pubkey::Pubkey;

/// Assert that a CToken approve operation was successful.
///
/// Pattern: Get pre-state, build expected by modifying only changed fields,
/// single assert_eq against post-state.
///
/// # Arguments
/// * `rpc` - RPC client (must be LightProgramTest for pre-transaction cache)
/// * `token_account` - The token account that was approved
/// * `delegate` - The delegate pubkey that was approved
/// * `amount` - The amount that was approved
pub async fn assert_ctoken_approve(
    rpc: &mut LightProgramTest,
    token_account: Pubkey,
    delegate: Pubkey,
    amount: u64,
) {
    // Get pre-transaction state from cache
    let pre_account = rpc
        .get_pre_transaction_account(&token_account)
        .expect("Token account should exist in pre-transaction context");

    // Get post-transaction state
    let post_account = rpc
        .get_account(token_account)
        .await
        .expect("Failed to get account after transaction")
        .expect("Token account should exist after transaction");

    // Parse pre and post CToken states
    let pre_ctoken =
        CToken::deserialize(&mut &pre_account.data[..]).expect("Failed to deserialize pre CToken");
    let post_ctoken = CToken::deserialize(&mut &post_account.data[..])
        .expect("Failed to deserialize post CToken");

    // Build expected by modifying only the changed fields from pre-state
    let expected_ctoken = CToken {
        delegate: Some(delegate.to_bytes().into()),
        delegated_amount: amount,
        ..pre_ctoken
    };

    assert_eq!(
        post_ctoken, expected_ctoken,
        "CToken after approve should have delegate={} and delegated_amount={}, all other fields unchanged",
        delegate, amount
    );
}

/// Assert that a CToken revoke operation was successful.
///
/// Pattern: Get pre-state, build expected by modifying only changed fields,
/// single assert_eq against post-state.
///
/// # Arguments
/// * `rpc` - RPC client (must be LightProgramTest for pre-transaction cache)
/// * `token_account` - The token account that was revoked
pub async fn assert_ctoken_revoke(rpc: &mut LightProgramTest, token_account: Pubkey) {
    // Get pre-transaction state from cache
    let pre_account = rpc
        .get_pre_transaction_account(&token_account)
        .expect("Token account should exist in pre-transaction context");

    // Get post-transaction state
    let post_account = rpc
        .get_account(token_account)
        .await
        .expect("Failed to get account after transaction")
        .expect("Token account should exist after transaction");

    // Parse pre and post CToken states
    let pre_ctoken =
        CToken::deserialize(&mut &pre_account.data[..]).expect("Failed to deserialize pre CToken");
    let post_ctoken = CToken::deserialize(&mut &post_account.data[..])
        .expect("Failed to deserialize post CToken");

    // Build expected by modifying only the changed fields from pre-state
    let expected_ctoken = CToken {
        delegate: None,
        delegated_amount: 0,
        ..pre_ctoken
    };

    assert_eq!(
        post_ctoken, expected_ctoken,
        "CToken after revoke should have delegate=None and delegated_amount=0, all other fields unchanged"
    );
}
