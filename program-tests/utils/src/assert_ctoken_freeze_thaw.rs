//! Assertion helpers for CToken freeze and thaw operations.
//!
//! These functions verify that freeze/thaw operations correctly modify
//! only the state field while preserving all other account state including
//! compression info and extensions.

use anchor_lang::AnchorDeserialize;
use light_client::rpc::Rpc;
use light_program_test::LightProgramTest;
use light_token_interface::state::{AccountState, Token};
use solana_sdk::pubkey::Pubkey;

/// Assert that a CToken freeze operation was successful.
///
/// Pattern: Get pre-state, build expected by modifying only changed fields,
/// single assert_eq against post-state.
///
/// # Arguments
/// * `rpc` - RPC client (must be LightProgramTest for pre-transaction cache)
/// * `token_account` - The token account that was frozen
pub async fn assert_ctoken_freeze(rpc: &mut LightProgramTest, token_account: Pubkey) {
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
        Token::deserialize(&mut &pre_account.data[..]).expect("Failed to deserialize pre CToken");
    let post_ctoken =
        Token::deserialize(&mut &post_account.data[..]).expect("Failed to deserialize post CToken");

    // Build expected by modifying only the changed fields from pre-state
    let expected_ctoken = Token {
        state: AccountState::Frozen,
        ..pre_ctoken
    };

    assert_eq!(
        post_ctoken, expected_ctoken,
        "CToken after freeze should have state=Frozen, all other fields unchanged"
    );
}

/// Assert that a CToken thaw operation was successful.
///
/// Pattern: Get pre-state, build expected by modifying only changed fields,
/// single assert_eq against post-state.
///
/// # Arguments
/// * `rpc` - RPC client (must be LightProgramTest for pre-transaction cache)
/// * `token_account` - The token account that was thawed
pub async fn assert_ctoken_thaw(rpc: &mut LightProgramTest, token_account: Pubkey) {
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
        Token::deserialize(&mut &pre_account.data[..]).expect("Failed to deserialize pre CToken");
    let post_ctoken =
        Token::deserialize(&mut &post_account.data[..]).expect("Failed to deserialize post CToken");

    // Build expected by modifying only the changed fields from pre-state
    let expected_ctoken = Token {
        state: AccountState::Initialized,
        ..pre_ctoken
    };

    assert_eq!(
        post_ctoken, expected_ctoken,
        "CToken after thaw should have state=Initialized, all other fields unchanged"
    );
}
