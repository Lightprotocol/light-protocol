use light_ctoken_sdk::{
    compressed_token::create_compressed_mint::find_cmint_address,
    ctoken::{derive_ctoken_ata, CTokenMintTo, CreateAssociatedCTokenAccount},
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{assert_ctoken_mint_to::assert_ctoken_mint_to, Rpc};
use light_token_client::instructions::mint_action::DecompressMintParams;
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Test context for mint_to operations
struct MintToTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    cmint_pda: Pubkey,
    ctoken_account: Pubkey,
    mint_authority: Keypair,
}

/// Setup: Create CMint + CToken (without tokens)
///
/// Steps:
/// 1. Init LightProgramTest
/// 2. Create compressed mint + CMint via mint_action_comprehensive (no recipients)
/// 3. Create CToken ATA with compressible extension
async fn setup_mint_to_test() -> MintToTestContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    // Use payer as mint_authority to simplify signing
    let mint_authority = payer.insecure_clone();
    let owner_keypair = Keypair::new();

    // Derive CMint PDA
    let (cmint_pda, _) = find_cmint_address(&mint_seed.pubkey());

    // Step 1: Create CToken ATA for owner first
    let (ctoken_ata, _) = derive_ctoken_ata(&owner_keypair.pubkey(), &cmint_pda);

    let create_ata_ix =
        CreateAssociatedCTokenAccount::new(payer.pubkey(), owner_keypair.pubkey(), cmint_pda)
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Step 2: Create compressed mint + CMint (no recipients - we'll mint via CTokenMintTo)
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        Some(DecompressMintParams::default()), // Creates CMint
        false,                                 // Don't compress and close
        vec![],                                // No compressed recipients
        vec![],                                // No ctoken recipients - we'll mint separately
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

    MintToTestContext {
        rpc,
        payer,
        cmint_pda,
        ctoken_account: ctoken_ata,
        mint_authority,
    }
}

/// Test minting tokens: mint 500, mint 500, end with 1000
#[tokio::test]
#[serial]
async fn test_ctoken_mint_to() {
    let mut ctx = setup_mint_to_test().await;

    // First mint: 500 tokens
    let mint_ix_1 = CTokenMintTo {
        cmint: ctx.cmint_pda,
        destination: ctx.ctoken_account,
        amount: 500,
        authority: ctx.mint_authority.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    ctx.rpc
        .create_and_send_transaction(
            &[mint_ix_1],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority],
        )
        .await
        .unwrap();

    assert_ctoken_mint_to(&mut ctx.rpc, ctx.ctoken_account, ctx.cmint_pda, 500).await;

    // Second mint: 500 tokens
    let mint_ix_2 = CTokenMintTo {
        cmint: ctx.cmint_pda,
        destination: ctx.ctoken_account,
        amount: 500,
        authority: ctx.mint_authority.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    ctx.rpc
        .create_and_send_transaction(
            &[mint_ix_2],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority],
        )
        .await
        .unwrap();

    assert_ctoken_mint_to(&mut ctx.rpc, ctx.ctoken_account, ctx.cmint_pda, 500).await;

    // Verify final balance is 1000
    use anchor_lang::prelude::borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let ctoken_after = ctx
        .rpc
        .get_account(ctx.ctoken_account)
        .await
        .unwrap()
        .unwrap();
    let token_account: CToken =
        BorshDeserialize::deserialize(&mut ctoken_after.data.as_slice()).unwrap();
    assert_eq!(
        token_account.amount, 1000,
        "Final balance should be 1000 after minting 500 + 500"
    );
}

// ============================================================================
// MintTo Checked Tests
// ============================================================================

use light_ctoken_sdk::ctoken::CTokenMintToChecked;

#[tokio::test]
#[serial]
async fn test_ctoken_mint_to_checked_success() {
    let mut ctx = setup_mint_to_test().await;

    // Mint 500 tokens with correct decimals (8)
    let mint_ix = CTokenMintToChecked {
        cmint: ctx.cmint_pda,
        destination: ctx.ctoken_account,
        amount: 500,
        decimals: 8, // Correct decimals
        authority: ctx.mint_authority.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    ctx.rpc
        .create_and_send_transaction(
            &[mint_ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority],
        )
        .await
        .unwrap();

    // Verify balance
    use anchor_lang::prelude::borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let ctoken_after = ctx
        .rpc
        .get_account(ctx.ctoken_account)
        .await
        .unwrap()
        .unwrap();
    let token_account: CToken =
        BorshDeserialize::deserialize(&mut ctoken_after.data.as_slice()).unwrap();
    assert_eq!(token_account.amount, 500, "Balance should be 500");

    println!("test_ctoken_mint_to_checked_success: passed");
}

#[tokio::test]
#[serial]
async fn test_ctoken_mint_to_checked_wrong_decimals() {
    let mut ctx = setup_mint_to_test().await;

    // Try to mint with wrong decimals (7 instead of 8)
    let mint_ix = CTokenMintToChecked {
        cmint: ctx.cmint_pda,
        destination: ctx.ctoken_account,
        amount: 500,
        decimals: 7, // Wrong decimals
        authority: ctx.mint_authority.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    let result = ctx
        .rpc
        .create_and_send_transaction(
            &[mint_ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority],
        )
        .await;

    // Should fail with MintDecimalsMismatch (error code 18 in pinocchio)
    assert!(result.is_err(), "Mint with wrong decimals should fail");
    light_program_test::utils::assert::assert_rpc_error(result, 0, 18).unwrap();
    println!("test_ctoken_mint_to_checked_wrong_decimals: passed");
}
