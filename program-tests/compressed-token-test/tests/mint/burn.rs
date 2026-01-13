use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{assert_ctoken_burn::assert_ctoken_burn, Rpc};
use light_token_client::instructions::mint_action::DecompressMintParams;
use light_token_interface::instructions::mint_action::Recipient;
use light_token_sdk::{
    compressed_token::create_compressed_mint::find_mint_address,
    token::{derive_token_ata, Burn, CreateAssociatedTokenAccount},
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Test context for burn operations
struct BurnTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    cmint_pda: Pubkey,
    ctoken_account: Pubkey,
    owner_keypair: Keypair,
}

/// Setup: Create CMint + Light Token with tokens minted
///
/// Steps:
/// 1. Init LightProgramTest
/// 2. Create compressed mint + CMint via mint_action_comprehensive
/// 3. Create Light Token ATA with compressible extension
/// 4. Mint tokens to Light Token via mint_action_comprehensive
async fn setup_burn_test(mint_amount: u64) -> BurnTestContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    // Use payer as mint_authority to simplify signing
    let mint_authority = payer.insecure_clone();
    let owner_keypair = Keypair::new();

    // Derive CMint PDA
    let (cmint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // Step 1: Create Light Token ATA for owner first (needed before minting)
    let (ctoken_ata, _) = derive_token_ata(&owner_keypair.pubkey(), &cmint_pda);

    let create_ata_ix =
        CreateAssociatedTokenAccount::new(payer.pubkey(), owner_keypair.pubkey(), cmint_pda)
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Step 2: Create compressed mint + CMint + mint tokens in one call
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        Some(DecompressMintParams::default()), // Creates CMint
        false,                                 // Don't compress and close
        vec![],                                // No compressed recipients
        vec![Recipient {
            recipient: owner_keypair.pubkey().into(),
            amount: mint_amount,
        }], // Mint to Light Token in same tx
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

    BurnTestContext {
        rpc,
        payer,
        cmint_pda,
        ctoken_account: ctoken_ata,
        owner_keypair,
    }
}

/// Test burning tokens: mint 1000, burn 500, burn 500, end with 0
#[tokio::test]
#[serial]
async fn test_ctoken_burn() {
    let mut ctx = setup_burn_test(1000).await;

    // First burn: 500 tokens (half)
    let burn_ix_1 = Burn {
        source: ctx.ctoken_account,
        cmint: ctx.cmint_pda,
        amount: 500,
        authority: ctx.owner_keypair.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    ctx.rpc
        .create_and_send_transaction(
            &[burn_ix_1],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.owner_keypair],
        )
        .await
        .unwrap();

    assert_ctoken_burn(&mut ctx.rpc, ctx.ctoken_account, ctx.cmint_pda, 500).await;

    // Second burn: 500 tokens (remaining half)
    let burn_ix_2 = Burn {
        source: ctx.ctoken_account,
        cmint: ctx.cmint_pda,
        amount: 500,
        authority: ctx.owner_keypair.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    ctx.rpc
        .create_and_send_transaction(
            &[burn_ix_2],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.owner_keypair],
        )
        .await
        .unwrap();

    assert_ctoken_burn(&mut ctx.rpc, ctx.ctoken_account, ctx.cmint_pda, 500).await;

    // Verify final balance is 0
    use anchor_lang::prelude::borsh::BorshDeserialize;
    use light_token_interface::state::Token;
    let ctoken_after = ctx
        .rpc
        .get_account(ctx.ctoken_account)
        .await
        .unwrap()
        .unwrap();
    let token_account: Token =
        BorshDeserialize::deserialize(&mut ctoken_after.data.as_slice()).unwrap();
    assert_eq!(
        token_account.amount, 0,
        "Final balance should be 0 after burning entire amount"
    );
}
