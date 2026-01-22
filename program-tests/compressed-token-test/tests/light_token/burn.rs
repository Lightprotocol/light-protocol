//! Burn instruction tests for Light Token accounts.
//!
//! ## Test Matrix
//!
//! | Test Category | Test Name |
//! |--------------|-----------|
//! | With Mint (partial burn) | test_burn_success_cases |
//! | With Mint (full balance) | test_burn_success_cases |
//! | Invalid mint (wrong mint) | test_burn_fails |
//! | Invalid ctoken (non-existent) | test_burn_fails |
//! | Invalid ctoken (wrong owner) | test_burn_fails |
//! | Insufficient balance | test_burn_fails |
//! | Wrong authority | test_burn_fails |
//!
//! **Note**: Burn requires a real Mint account (owned by ctoken program) for supply tracking.
//! This is different from approve/revoke which only modify the Light Token account.
//!
//! **Note**: Max top-up exceeded test requires compressible accounts with time warp.
//! For comprehensive max_top_up testing, see sdk-tests/sdk-light-token-test/tests/test_burn.rs
use light_compressed_token_sdk::compressed_token::create_compressed_mint::find_mint_address;
use light_program_test::{
    program_test::TestRpc, utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig,
};
use light_test_utils::assert_ctoken_burn::assert_ctoken_burn;
use light_token::instruction::{derive_token_ata, Burn, CreateAssociatedTokenAccount, MintTo};
use light_token_client::instructions::mint_action::DecompressMintParams;

use super::shared::*;

// ============================================================================
// Burn Success Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_burn_success_cases() {
    // Test 1: Basic burn with Mint (no top-up needed)
    {
        let mut ctx = setup_burn_test().await;
        let burn_amount = 50u64;

        // Burn 50 tokens
        let burn_ix = Burn {
            source: ctx.ctoken_account,
            mint: ctx.mint_pda,
            amount: burn_amount,
            authority: ctx.owner_keypair.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        ctx.rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.owner_keypair],
            )
            .await
            .unwrap();

        // Assert burn was successful using assert_ctoken_burn
        assert_ctoken_burn(&mut ctx.rpc, ctx.ctoken_account, ctx.mint_pda, burn_amount).await;

        println!("test_burn_success_cases: basic burn passed");
    }

    // Test 2: Burn full balance
    {
        let mut ctx = setup_burn_test().await;
        let burn_amount = 100u64;

        // Burn all 100 tokens
        let burn_ix = Burn {
            source: ctx.ctoken_account,
            mint: ctx.mint_pda,
            amount: burn_amount,
            authority: ctx.owner_keypair.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        ctx.rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.owner_keypair],
            )
            .await
            .unwrap();

        // Assert burn was successful using assert_ctoken_burn
        assert_ctoken_burn(&mut ctx.rpc, ctx.ctoken_account, ctx.mint_pda, burn_amount).await;

        println!("test_burn_success_cases: burn full balance passed");
    }
}

// ============================================================================
// Burn Failure Cases
// ============================================================================

/// Error codes used in burn validation (mapped to ErrorCode enum variants)
mod error_codes {
    /// Insufficient funds to complete the operation (SplInsufficientFunds = 6154)
    pub const INSUFFICIENT_FUNDS: u32 = 6154;
    /// Authority doesn't match token account owner (OwnerMismatch = 6075)
    pub const OWNER_MISMATCH: u32 = 6075;
}

#[tokio::test]
#[serial]
async fn test_burn_fails() {
    // Test 1: Invalid mint - wrong mint (different Mint)
    {
        let mut ctx = setup_burn_test().await;

        // Create a different Mint
        let other_mint_seed = Keypair::new();
        let (other_mint_pda, _) = find_mint_address(&other_mint_seed.pubkey());

        // Try to burn with wrong mint
        let burn_ix = Burn {
            source: ctx.ctoken_account,
            mint: other_mint_pda, // Wrong mint
            amount: 50,
            authority: ctx.owner_keypair.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        let result = ctx
            .rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.owner_keypair],
            )
            .await;

        // Non-existent Mint returns NotRentExempt (SPL Token code 0 -> 6153)
        assert_rpc_error(result, 0, 6153).unwrap();
        println!("test_burn_fails: wrong mint passed");
    }

    // Test 2: Invalid ctoken - non-existent account
    {
        let mut ctx = setup_burn_test().await;

        let non_existent = Pubkey::new_unique();

        let burn_ix = Burn {
            source: non_existent,
            mint: ctx.mint_pda,
            amount: 50,
            authority: ctx.owner_keypair.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        let result = ctx
            .rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.owner_keypair],
            )
            .await;

        // Non-existent Light Token account returns NotRentExempt (SPL Token code 0 -> 6153)
        assert_rpc_error(result, 0, 6153).unwrap();
        println!("test_burn_fails: non-existent account passed");
    }

    // Test 3: Invalid ctoken - wrong program owner
    {
        use anchor_spl::token::spl_token;

        let mut ctx = setup_burn_test().await;

        // Get the valid Light Token account data
        let valid_account = ctx
            .rpc
            .get_account(ctx.ctoken_account)
            .await
            .unwrap()
            .unwrap();

        // Create a new account with same data but owned by spl_token program
        let wrong_owner_account = Keypair::new();
        let mut account_with_wrong_owner = valid_account.clone();
        account_with_wrong_owner.owner = spl_token::ID;

        ctx.rpc
            .set_account(wrong_owner_account.pubkey(), account_with_wrong_owner);

        let burn_ix = Burn {
            source: wrong_owner_account.pubkey(),
            mint: ctx.mint_pda,
            amount: 50,
            authority: ctx.owner_keypair.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        let result = ctx
            .rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.owner_keypair],
            )
            .await;

        // Expect ExternalAccountDataModified error (Solana code 13)
        // This happens when trying to modify an account not owned by the program
        assert_rpc_error(result, 0, 13).unwrap();
        println!("test_burn_fails: wrong program owner passed");
    }

    // Test 4: Insufficient balance
    {
        let mut ctx = setup_burn_test().await;

        // Try to burn more than balance (100 tokens)
        let burn_ix = Burn {
            source: ctx.ctoken_account,
            mint: ctx.mint_pda,
            amount: 200, // More than 100 balance
            authority: ctx.owner_keypair.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        let result = ctx
            .rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.owner_keypair],
            )
            .await;

        // Expect InsufficientFunds error (SPL Token code 1)
        assert_rpc_error(result, 0, error_codes::INSUFFICIENT_FUNDS).unwrap();
        println!("test_burn_fails: insufficient balance passed");
    }

    // Test 5: Wrong authority
    {
        let mut ctx = setup_burn_test().await;

        // Use a different authority (not the owner)
        let wrong_authority = Keypair::new();
        ctx.rpc
            .airdrop_lamports(&wrong_authority.pubkey(), 1_000_000_000)
            .await
            .unwrap();

        let burn_ix = Burn {
            source: ctx.ctoken_account,
            mint: ctx.mint_pda,
            amount: 50,
            authority: wrong_authority.pubkey(),
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        let result = ctx
            .rpc
            .create_and_send_transaction(
                &[burn_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &wrong_authority],
            )
            .await;

        // Expect OwnerMismatch error (SPL Token code 4)
        assert_rpc_error(result, 0, error_codes::OWNER_MISMATCH).unwrap();
        println!("test_burn_fails: wrong authority passed");
    }

    // Test 6: Max top-up exceeded
    // Note: This requires compressible accounts that need top-up after time warp.
    // The current setup creates non-compressible accounts, so max_top_up test
    // would need additional setup. For comprehensive max_top_up testing, see
    // sdk-tests/sdk-light-token-test/tests/test_burn.rs
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Test context for burn operations
struct BurnTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    mint_pda: Pubkey,
    ctoken_account: Pubkey,
    owner_keypair: Keypair,
}

/// Setup: Create Mint + Light Token with 100 tokens
///
/// Steps:
/// 1. Init LightProgramTest
/// 2. Create compressed mint + Mint via mint_action_comprehensive
/// 3. Create Light Token ATA
/// 4. Mint 100 tokens
async fn setup_burn_test() -> BurnTestContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let mint_authority = payer.insecure_clone();
    let owner_keypair = Keypair::new();

    // Derive Mint PDA
    let (mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // Step 1: Create Light Token ATA for owner
    let (ctoken_ata, _) = derive_token_ata(&owner_keypair.pubkey(), &mint_pda);

    let create_ata_ix =
        CreateAssociatedTokenAccount::new(payer.pubkey(), owner_keypair.pubkey(), mint_pda)
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Step 2: Create compressed mint + Mint (no recipients)
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        Some(DecompressMintParams::default()), // Creates Mint
        false,                                 // Don't compress and close
        vec![],                                // No compressed recipients
        vec![],                                // No ctoken recipients
        None,                                  // No mint authority update
        None,                                  // No freeze authority update
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals: 8,
            supply: 0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: None,
            metadata: None,
            version: 3,
        }),
    )
    .await
    .unwrap();

    // Step 3: Mint 100 tokens to the Light Token account
    let mint_ix = MintTo {
        mint: mint_pda,
        destination: ctoken_ata,
        amount: 100,
        authority: mint_authority.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[mint_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Fund owner for transaction fees
    rpc.airdrop_lamports(&owner_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    BurnTestContext {
        rpc,
        payer,
        mint_pda,
        ctoken_account: ctoken_ata,
        owner_keypair,
    }
}

// ============================================================================
// Burn Checked Tests
// ============================================================================

use light_token::instruction::BurnChecked;

/// MintDecimalsMismatch error code (SplMintDecimalsMismatch = 6166)
const MINT_DECIMALS_MISMATCH: u32 = 6166;

#[tokio::test]
#[serial]
async fn test_burn_checked_success() {
    let mut ctx = setup_burn_test().await;
    let burn_amount = 50u64;

    // Burn 50 tokens with correct decimals (8)
    let burn_ix = BurnChecked {
        source: ctx.ctoken_account,
        mint: ctx.mint_pda,
        amount: burn_amount,
        decimals: 8, // Correct decimals
        authority: ctx.owner_keypair.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    ctx.rpc
        .create_and_send_transaction(
            &[burn_ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.owner_keypair],
        )
        .await
        .unwrap();

    // Assert burn was successful using assert_ctoken_burn
    assert_ctoken_burn(&mut ctx.rpc, ctx.ctoken_account, ctx.mint_pda, burn_amount).await;

    println!("test_burn_checked_success: passed");
}

#[tokio::test]
#[serial]
async fn test_burn_checked_wrong_decimals() {
    let mut ctx = setup_burn_test().await;

    // Try to burn with wrong decimals (7 instead of 8)
    let burn_ix = BurnChecked {
        source: ctx.ctoken_account,
        mint: ctx.mint_pda,
        amount: 50,
        decimals: 7, // Wrong decimals
        authority: ctx.owner_keypair.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    let result = ctx
        .rpc
        .create_and_send_transaction(
            &[burn_ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.owner_keypair],
        )
        .await;

    // Expect MintDecimalsMismatch error (SPL Token code 18)
    assert_rpc_error(result, 0, MINT_DECIMALS_MISMATCH).unwrap();
    println!("test_burn_checked_wrong_decimals: passed");
}
