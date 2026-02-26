//! Approve and Revoke instruction tests for Light Token accounts.
//!
//! ## Test Matrix
//!
//! | Test Category | Approve | Revoke |
//! |--------------|---------|--------|
//! | SPL compat | test_approve_success_cases | test_revoke_success_cases |
//! | With SPL mint | test_approve_success_cases | test_revoke_success_cases |
//! | With Mint | test_approve_revoke_compressible | test_approve_revoke_compressible |
//! | Invalid ctoken (non-existent) | test_approve_fails | test_revoke_fails |
//! | Invalid ctoken (wrong owner) | test_approve_fails | test_revoke_fails |
//! | Invalid ctoken (spl account) | test_approve_fails | test_revoke_fails |
//! | Max top-up exceeded | test_approve_fails | test_revoke_fails |
//!
//! **Note**: "Invalid mint" tests not applicable - approve/revoke don't take mint as account.

use super::shared::*;

// ============================================================================
// Approve Success Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_approve_success_cases() {
    // Test 1: SPL compat (uses SPL instruction format with modifications for Light Token)
    {
        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();
        // Fund owner for compressible top-up
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let delegate = Keypair::new();
        approve_spl_compat_and_assert(&mut context, delegate.pubkey(), 100, "spl_compat").await;
    }

    // Test 2: With SPL mint + compressible extension with prepaid_epochs=2 (uses SDK instruction format)
    {
        let mut context = setup_account_test_with_created_account(Some((2, false)))
            .await
            .unwrap();
        // Fund owner for potential top-up
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let delegate = Keypair::new();
        approve_and_assert(
            &mut context,
            delegate.pubkey(),
            100,
            "with_spl_mint_compressible",
        )
        .await;
    }
}

// ============================================================================
// Approve Failure Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_approve_fails() {
    // Test 1: Invalid account - non-existent
    {
        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();
        let delegate = Keypair::new();
        let non_existent = Pubkey::new_unique();
        let owner = context.owner_keypair.insecure_clone();
        approve_and_assert_fails(
            &mut context,
            non_existent,
            delegate.pubkey(),
            &owner,
            100,
            None,
            "non_existent_account",
            6153, // NotRentExempt (SPL Token code 0 -> ErrorCode::NotRentExempt)
        )
        .await;
    }

    // Test 2: Invalid account - wrong program owner (valid Light Token data but wrong owner)
    {
        use anchor_spl::token::spl_token;
        use light_program_test::program_test::TestRpc;

        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();

        // Fund owner so the test doesn't fail due to insufficient lamports
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();

        // Get the valid Light Token account data
        let valid_account = context
            .rpc
            .get_account(context.token_account_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        // Create a new account with the same data but owned by spl_token program
        let wrong_owner_account = Keypair::new();
        let mut account_with_wrong_owner = valid_account.clone();
        account_with_wrong_owner.owner = spl_token::ID;

        context
            .rpc
            .set_account(wrong_owner_account.pubkey(), account_with_wrong_owner);

        let delegate = Keypair::new();
        let owner = context.owner_keypair.insecure_clone();
        approve_and_assert_fails(
            &mut context,
            wrong_owner_account.pubkey(),
            delegate.pubkey(),
            &owner,
            100,
            None,
            "wrong_program_owner",
            13, // InstructionError::ExternalAccountDataModified - program tried to modify account it doesn't own
        )
        .await;
    }

    // Test 3: Max top-up exceeded
    {
        let mut context = setup_account_test_with_created_account(Some((10, false)))
            .await
            .unwrap();

        // Fund owner so the test doesn't fail due to insufficient lamports
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();

        // Warp time to trigger top-up requirement (past funded epochs)
        context.rpc.warp_to_slot(SLOTS_PER_EPOCH * 12 + 1).unwrap();

        let delegate = Keypair::new();
        let token_account = context.token_account_keypair.pubkey();
        let owner = context.owner_keypair.insecure_clone();
        approve_and_assert_fails(
            &mut context,
            token_account,
            delegate.pubkey(),
            &owner,
            100,
            Some(1), // max_top_up = 1 (1,000 lamports budget, still too low for rent top-up)
            "max_topup_exceeded",
            18043, // TokenError::MaxTopUpExceeded
        )
        .await;
    }
}

// ============================================================================
// Revoke Success Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_revoke_success_cases() {
    // Test 1: SPL compat (uses SPL instruction format with modifications for Light Token)
    {
        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();
        // Fund owner for compressible top-up
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();

        // First approve a delegate using SPL compat
        let delegate = Keypair::new();
        approve_spl_compat_and_assert(
            &mut context,
            delegate.pubkey(),
            100,
            "spl_compat_approve_for_revoke",
        )
        .await;

        // Then revoke using SPL compat
        revoke_spl_compat_and_assert(&mut context, "spl_compat").await;
    }

    // Test 2: With SPL mint + compressible extension with prepaid_epochs=2 (uses SDK instruction format)
    {
        let mut context = setup_account_test_with_created_account(Some((2, false)))
            .await
            .unwrap();

        // Fund owner for potential top-up
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();

        // First approve
        let delegate = Keypair::new();
        approve_and_assert(
            &mut context,
            delegate.pubkey(),
            100,
            "sdk_approve_for_revoke",
        )
        .await;

        // Then revoke
        revoke_and_assert(&mut context, "with_spl_mint_compressible").await;
    }

    // Note: Delegate self-revoke (Token-2022 feature) is NOT supported by pinocchio-token-program.
    // The pinocchio implementation only validates against the owner, not the delegate.
}

// ============================================================================
// Revoke Failure Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_revoke_fails() {
    // Test 1: Invalid account - non-existent
    {
        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();
        let non_existent = Pubkey::new_unique();
        let owner = context.owner_keypair.insecure_clone();
        revoke_and_assert_fails(
            &mut context,
            non_existent,
            &owner,
            None,
            "non_existent_account",
            6153, // NotRentExempt (SPL Token code 0 -> ErrorCode::NotRentExempt)
        )
        .await;
    }

    // Test 2: Invalid account - wrong program owner (valid Light Token data but wrong owner)
    {
        use anchor_spl::token::spl_token;
        use light_program_test::program_test::TestRpc;

        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();

        // Fund owner so the test doesn't fail due to insufficient lamports
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();

        // Get the valid Light Token account data
        let valid_account = context
            .rpc
            .get_account(context.token_account_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        // Create a new account with the same data but owned by spl_token program
        let wrong_owner_account = Keypair::new();
        let mut account_with_wrong_owner = valid_account.clone();
        account_with_wrong_owner.owner = spl_token::ID;

        context
            .rpc
            .set_account(wrong_owner_account.pubkey(), account_with_wrong_owner);

        let owner = context.owner_keypair.insecure_clone();
        revoke_and_assert_fails(
            &mut context,
            wrong_owner_account.pubkey(),
            &owner,
            None,
            "wrong_program_owner",
            13, // InstructionError::ExternalAccountDataModified - program tried to modify account it doesn't own
        )
        .await;
    }

    // Test 3: Max top-up exceeded
    {
        let mut context = setup_account_test_with_created_account(Some((10, false)))
            .await
            .unwrap();

        // First approve to set delegate (need to do before warping)
        context
            .rpc
            .airdrop_lamports(&context.owner_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let delegate = Keypair::new();
        approve_and_assert(&mut context, delegate.pubkey(), 100, "approve_before_warp").await;

        // Warp time to trigger top-up requirement (past funded epochs)
        context.rpc.warp_to_slot(SLOTS_PER_EPOCH * 12 + 1).unwrap();

        let token_account = context.token_account_keypair.pubkey();
        let owner = context.owner_keypair.insecure_clone();
        revoke_and_assert_fails(
            &mut context,
            token_account,
            &owner,
            Some(1), // max_top_up = 1 (1,000 lamports budget, still too low for rent top-up)
            "max_topup_exceeded",
            18043, // TokenError::MaxTopUpExceeded
        )
        .await;
    }
}

// ============================================================================
// Original Compressible Test (Mint scenario with extensions)
// ============================================================================

use anchor_lang::AnchorDeserialize;
use light_program_test::program_test::TestRpc;
use light_test_utils::RpcError;
use light_token::instruction::{Approve, CreateTokenAccount, Revoke};
use light_token_interface::state::{Token, TokenDataVersion};
use solana_sdk::program_pack::Pack;

use super::extensions::setup_extensions_test;

/// Test approve and revoke with a compressible Light Token account with extensions.
/// 1. Create compressible Light Token account with all extensions
/// 2. Set token balance to 100 using set_account
/// 3. Approve 10 tokens to delegate
/// 4. Assert delegate and delegated_amount fields
/// 5. Revoke delegation
/// 6. Assert delegate cleared and delegated_amount is 0
#[tokio::test]
#[serial]
async fn test_approve_revoke_compressible() -> Result<(), RpcError> {
    use anchor_spl::token_2022::spl_token_2022;

    let mut context = setup_extensions_test().await?;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let owner = Keypair::new();
    let delegate = Keypair::new();

    // 1. Create compressible Light Token account with all extensions
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to create instruction: {}", e))
            })?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // 2. Set token balance to 100 using set_account
    let token_balance = 100u64;
    let mut token_account_info = context
        .rpc
        .get_account(account_pubkey)
        .await?
        .ok_or_else(|| RpcError::AssertRpcError("Token account not found".to_string()))?;

    let mut spl_token_account =
        spl_token_2022::state::Account::unpack_unchecked(&token_account_info.data[..165])
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to unpack: {:?}", e)))?;
    spl_token_account.amount = token_balance;
    spl_token_2022::state::Account::pack(spl_token_account, &mut token_account_info.data[..165])
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to pack: {:?}", e)))?;
    context.rpc.set_account(account_pubkey, token_account_info);

    // Verify initial state
    let account_data_initial = context.rpc.get_account(account_pubkey).await?.unwrap();
    let ctoken_initial = Token::deserialize(&mut &account_data_initial.data[..])
        .expect("Failed to deserialize Light Token");
    assert_eq!(ctoken_initial.amount, token_balance);
    assert!(ctoken_initial.delegate.is_none());
    assert_eq!(ctoken_initial.delegated_amount, 0);

    // Fund the owner for compressible top-up
    context
        .rpc
        .airdrop_lamports(&owner.pubkey(), 1_000_000_000)
        .await?;

    // 3. Approve 10 tokens to delegate
    let approve_amount = 10u64;
    let approve_ix = Approve {
        token_account: account_pubkey,
        delegate: delegate.pubkey(),
        owner: owner.pubkey(),
        amount: approve_amount,
        fee_payer: payer.pubkey(),
    }
    .instruction()
    .map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create approve instruction: {}", e))
    })?;

    context
        .rpc
        .create_and_send_transaction(&[approve_ix], &payer.pubkey(), &[&payer, &owner])
        .await?;

    // 4. Assert delegate and delegated_amount fields after approve
    assert_ctoken_approve(
        &mut context.rpc,
        account_pubkey,
        delegate.pubkey(),
        approve_amount,
    )
    .await;

    // 5. Revoke delegation
    let revoke_ix = Revoke {
        token_account: account_pubkey,
        owner: owner.pubkey(),
        fee_payer: payer.pubkey(),
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create revoke instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[revoke_ix], &payer.pubkey(), &[&payer, &owner])
        .await?;

    // 6. Assert delegate cleared and delegated_amount is 0 after revoke
    assert_ctoken_revoke(&mut context.rpc, account_pubkey).await;

    println!("Successfully tested approve and revoke with compressible Light Token");
    Ok(())
}
