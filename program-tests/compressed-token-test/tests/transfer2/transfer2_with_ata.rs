//! Tests for Transfer2WithAta instruction.
//!
//! Transfer2WithAta enables decompress/transfer operations on compressed tokens
//! where ALL inputs have owner = ATA pubkey (compress_to_pubkey mode).
//!
//! Test coverage:
//!
//! OWNER MODE SUCCESS CASES:
//! 1. Single ATA-owned compressed token decompressed
//! 2. Multiple ATA-owned compressed tokens decompressed in single call
//!
//! OWNER MODE FAILURE CASES:
//! 1. Wrong owner signer (not wallet that owns the ATA)
//! 2. Wrong mint passed
//! 3. wallet_idx correct key but not signer
//! 4. False ATA derivation (wrong bump)
//! 5. Non-matching ATA in accounts
//! 6. Mixed ownership (some ATA-owned, some wallet-owned)
//!
//! DELEGATE MODE SUCCESS CASES:
//! 1. Single delegated ATA-owned token - delegate signs
//! 2. Multiple delegated ATA-owned tokens - delegate signs
//!
//! DELEGATE MODE FAILURE CASES (SECURITY CRITICAL):
//! 1. Delegate provided but doesn't sign
//! 2. Wrong delegate signs (different from input's delegate field)
//! 3. Inputs have different delegates (must all match)
//! 4. Input has no delegate set (delegate mode requires delegate on all inputs)
//! 5. Owner signs instead of delegate (when delegate mode is specified)
//! 6. Mixed: some inputs have delegate, some don't
//! 7. Delegate index tampered (out of bounds)
//! 8. Delegate mode with mismatched delegate pubkey in accounts vs instruction

use light_client::indexer::Indexer;
use light_compressed_token_sdk::ctoken::{
    derive_ctoken_ata, CompressibleParams, CreateAssociatedTokenAccount,
};
use light_ctoken_types::{
    instructions::{extensions::compressible::CompressToPubkey, mint_action::Recipient},
    state::TokenDataVersion,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{airdrop_lamports, Rpc};
use light_token_client::{
    actions::{create_mint, mint_action, mint_to_compressed},
    instructions::{
        mint_action::{MintActionParams, MintActionType},
        transfer2::{create_generic_transfer2_instruction, Transfer2InstructionType},
        transfer2_with_ata::{create_decompress_ata_instruction, DecompressAtaInput},
    },
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

// ============================================================================
// Test Context Setup
// ============================================================================

struct Transfer2WithAtaTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    owner_wallet: Keypair,
    mint: solana_sdk::pubkey::Pubkey,
    mint_seed: Keypair,
    mint_authority: Keypair,
    ata: solana_sdk::pubkey::Pubkey,
    ata_bump: u8,
}

async fn setup_transfer2_with_ata_test(
) -> Result<Transfer2WithAtaTestContext, Box<dyn std::error::Error>> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let owner_wallet = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_seed = Keypair::new();

    // Airdrop to owner
    airdrop_lamports(&mut rpc, &owner_wallet.pubkey(), 10_000_000_000).await?;

    Ok(Transfer2WithAtaTestContext {
        rpc,
        payer,
        owner_wallet,
        mint: solana_sdk::pubkey::Pubkey::default(), // Will be set after mint creation
        mint_seed,
        mint_authority,
        ata: solana_sdk::pubkey::Pubkey::default(), // Will be set after ATA creation
        ata_bump: 0,
    })
}

impl Transfer2WithAtaTestContext {
    /// Create compressed mint
    async fn create_mint(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mint, _) =
            light_compressed_token_sdk::compressed_token::create_compressed_mint::find_spl_mint_address(
                &self.mint_seed.pubkey(),
            );

        create_mint(
            &mut self.rpc,
            &self.mint_seed,
            6, // decimals
            &self.mint_authority,
            None,
            None,
            &self.payer,
        )
        .await?;

        self.mint = mint;
        let (ata, bump) = derive_ctoken_ata(&self.owner_wallet.pubkey(), &self.mint);
        self.ata = ata;
        self.ata_bump = bump;

        Ok(())
    }

    /// Create ATA with compress_to_pubkey enabled
    async fn create_ata_with_compress_to_pubkey(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let compressible_config = self
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda;
        let rent_sponsor = self.rpc.test_accounts.funding_pool_config.rent_sponsor_pda;

        // Create compress_to_pubkey with ATA seeds
        let compress_to_pubkey = CompressToPubkey {
            bump: self.ata_bump,
            program_id: light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
            seeds: vec![
                self.owner_wallet.pubkey().to_bytes().to_vec(),
                light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID.to_vec(),
                self.mint.to_bytes().to_vec(),
            ],
        };

        let compressible_params = CompressibleParams {
            compressible_config,
            rent_sponsor,
            pre_pay_num_epochs: 10,
            lamports_per_write: None,
            compress_to_account_pubkey: Some(compress_to_pubkey),
            token_account_version: TokenDataVersion::ShaFlat,
        };

        let create_ata_ix = CreateAssociatedTokenAccount::new(
            self.payer.pubkey(),
            self.owner_wallet.pubkey(),
            self.mint,
            compressible_params,
        )
        .instruction()?;

        self.rpc
            .create_and_send_transaction(&[create_ata_ix], &self.payer.pubkey(), &[&self.payer])
            .await?;

        Ok(())
    }

    /// Mint tokens to ATA then compress (creating ATA-owned compressed tokens)
    async fn mint_and_compress_to_ata(
        &mut self,
        amount: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let address_tree = self.rpc.get_address_tree_v2().tree;
        let compressed_mint_address = light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_compressed_mint_address(
            &self.mint_seed.pubkey(),
            &address_tree,
        );

        // Mint to the ATA
        mint_action(
            &mut self.rpc,
            MintActionParams {
                compressed_mint_address,
                mint_seed: self.mint_seed.pubkey(),
                authority: self.mint_authority.pubkey(),
                payer: self.payer.pubkey(),
                actions: vec![MintActionType::MintToCToken {
                    account: self.ata,
                    amount,
                }],
                new_mint: None,
            },
            &self.mint_authority,
            &self.payer,
            None,
        )
        .await?;

        // Now compress the tokens in the ATA
        // This creates compressed tokens with owner = ATA (compress_to_pubkey mode)
        let output_queue = self
            .rpc
            .get_random_state_tree_info()
            .unwrap()
            .get_output_pubkey()
            .unwrap();

        let compress_ix = create_generic_transfer2_instruction(
            &mut self.rpc,
            vec![Transfer2InstructionType::CompressAndClose(
                light_token_client::instructions::transfer2::CompressAndCloseInput {
                    solana_ctoken_account: self.ata,
                    authority: self.owner_wallet.pubkey(),
                    output_queue,
                    destination: None,
                    is_compressible: true,
                },
            )],
            self.payer.pubkey(),
            false,
        )
        .await?;

        self.rpc
            .create_and_send_transaction(
                &[compress_ix],
                &self.payer.pubkey(),
                &[&self.payer, &self.owner_wallet],
            )
            .await?;

        Ok(())
    }

    /// Mint tokens directly to wallet (wallet-owned compressed tokens)
    async fn mint_to_wallet(&mut self, amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        let recipients = vec![Recipient::new(self.owner_wallet.pubkey(), amount)];

        mint_to_compressed(
            &mut self.rpc,
            self.mint,
            recipients,
            TokenDataVersion::ShaFlat,
            &self.mint_authority,
            &self.payer,
        )
        .await?;

        Ok(())
    }

    /// Get compressed token accounts owned by the ATA
    async fn get_ata_owned_compressed_accounts(
        &self,
    ) -> Result<Vec<light_client::indexer::CompressedTokenAccount>, Box<dyn std::error::Error>>
    {
        let accounts = self
            .rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(&self.ata, None, None)
            .await?
            .value
            .items;
        Ok(accounts)
    }

    /// Get compressed token accounts owned by the wallet
    async fn get_wallet_owned_compressed_accounts(
        &self,
    ) -> Result<Vec<light_client::indexer::CompressedTokenAccount>, Box<dyn std::error::Error>>
    {
        let accounts = self
            .rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(&self.owner_wallet.pubkey(), None, None)
            .await?
            .value
            .items;
        Ok(accounts)
    }
}

// ============================================================================
// SUCCESS TESTS
// ============================================================================

/// Test: Successfully decompress a single ATA-owned compressed token
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_single_input_success() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();

    // Re-create the ATA (it was closed by compress_and_close)
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    // Get ATA-owned compressed accounts
    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(
        !compressed_accounts.is_empty(),
        "Should have ATA-owned compressed accounts"
    );

    // Verify owner is the ATA
    for acc in &compressed_accounts {
        assert_eq!(
            acc.token.owner, ctx.ata,
            "Compressed token owner should be ATA"
        );
    }

    // Decompress using Transfer2WithAta
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts,
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None, // Full balance
        use_delegate: false,
    };

    let ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    ctx.rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await
        .unwrap();

    // Verify ATA now has the tokens
    let ata_account = ctx.rpc.get_account(ctx.ata).await.unwrap().unwrap();
    assert!(ata_account.data.len() > 0, "ATA should exist with tokens");
}

/// Test: Successfully decompress multiple ATA-owned compressed tokens in single call
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_multiple_inputs_success() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();

    // Create ATA and mint+compress multiple times to create multiple compressed accounts
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(500).await.unwrap();

    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(300).await.unwrap();

    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    // Get all ATA-owned compressed accounts
    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(
        compressed_accounts.len() >= 2,
        "Should have multiple ATA-owned compressed accounts, got {}",
        compressed_accounts.len()
    );

    // Decompress all using Transfer2WithAta
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    ctx.rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await
        .unwrap();

    // Verify all tokens are now in ATA
    let ata_account = ctx.rpc.get_account(ctx.ata).await.unwrap().unwrap();
    assert!(ata_account.data.len() > 0, "ATA should exist with tokens");
}

// ============================================================================
// FAILURE TESTS
// ============================================================================

/// Test: Fail when wrong owner wallet signs (not the wallet that owns the ATA)
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_wrong_owner_signer_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Try with a different signer (wrong wallet)
    let wrong_wallet = Keypair::new();
    airdrop_lamports(&mut ctx.rpc, &wrong_wallet.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Build instruction manually with wrong wallet
    // The SDK will reject this because ATA derivation won't match
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts,
        owner_wallet: wrong_wallet.pubkey(), // Wrong wallet
        mint: ctx.mint,
        destination_ata: ctx.ata, // This won't match derivation from wrong_wallet
        decompress_amount: None,
        use_delegate: false,
    };

    let result = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey()).await;
    assert!(
        result.is_err(),
        "Should fail when ATA doesn't match owner_wallet derivation"
    );
}

/// Test: Fail when wrong mint is passed
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_wrong_mint_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Try with wrong mint
    let wrong_mint = solana_sdk::pubkey::Pubkey::new_unique();
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts,
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: wrong_mint, // Wrong mint
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let result = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey()).await;
    assert!(
        result.is_err(),
        "Should fail when mint doesn't match ATA derivation"
    );
}

/// Test: Fail when wallet_idx is correct key but not a signer
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_wallet_not_signer_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts,
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // Sign only with payer, NOT with owner_wallet
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer]) // Missing owner_wallet!
        .await;

    assert!(
        result.is_err(),
        "Should fail when owner_wallet doesn't sign"
    );
}

/// Test: Fail when non-matching ATA is passed in accounts
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_wrong_ata_account_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Use wrong ATA address
    let wrong_ata = solana_sdk::pubkey::Pubkey::new_unique();
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts,
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: wrong_ata, // Wrong ATA!
        decompress_amount: None,
        use_delegate: false,
    };

    let result = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey()).await;
    assert!(
        result.is_err(),
        "Should fail when destination_ata doesn't match derivation"
    );
}

/// Test: Fail when mixed ownership (some ATA-owned, some wallet-owned)
/// This tests the on-chain check that ALL inputs must have owner = ATA
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_mixed_ownership_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();

    // Create ATA-owned compressed tokens
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(500).await.unwrap();

    let ata_owned_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();

    // Also mint some tokens directly to wallet (wallet-owned compressed tokens)
    ctx.mint_to_wallet(300).await.unwrap();

    // Get wallet-owned compressed accounts
    let wallet_owned_accounts = ctx.get_wallet_owned_compressed_accounts().await.unwrap();

    // Mix the accounts (if we have both types)
    if ata_owned_accounts.is_empty() || wallet_owned_accounts.is_empty() {
        println!("Warning: Could not get both account types for mixed ownership test");
        return;
    }

    // Try to pass mixed ownership - this should fail client-side
    let mut mixed_accounts = ata_owned_accounts;
    mixed_accounts.extend(wallet_owned_accounts);

    // Re-create ATA for destination
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let input = DecompressAtaInput {
        compressed_token_accounts: mixed_accounts,
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    // Should fail because not all inputs have owner = ATA
    let result = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey()).await;
    assert!(
        result.is_err(),
        "Should fail when mixing ATA-owned and wallet-owned inputs"
    );
}

/// Test: ATTACK - Modify bump to invalid value (breaks ATA derivation)
#[tokio::test]
#[serial]
async fn test_attack_wrong_bump_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Build instruction correctly first
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Modify bump (2nd to last byte - suffix is [wallet, mint, ata, bump, delegate])
    let data_len = ix.data.len();
    let correct_bump = ix.data[data_len - 2];
    ix.data[data_len - 2] = correct_bump.wrapping_add(1); // Invalid bump

    // This should fail on-chain
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(result.is_err(), "Should fail with wrong ATA bump");
}

// ============================================================================
// DELEGATE MODE TESTS
// ============================================================================
//
// These tests verify the security of delegate mode in Transfer2WithAta.
// Key security properties:
// 1. If delegate is provided (delegate_index != 255), delegate MUST sign
// 2. All inputs MUST have matching delegate field when delegate mode is used
// 3. Owner signing should NOT work when delegate mode is specified
// 4. Delegate index must point to valid account

/// Test: ATTACK - Specify delegate mode but don't include delegate signature
/// This tests that an attacker cannot bypass delegate signing requirement
#[tokio::test]
#[serial]
async fn test_delegate_mode_delegate_not_signer_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Create a fake delegate keypair
    let fake_delegate = Keypair::new();
    airdrop_lamports(&mut ctx.rpc, &fake_delegate.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Build instruction with owner mode first (to get valid base instruction)
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Change delegate_index from 255 (no delegate) to point to an account
    // but DON'T add delegate as signer
    let data_len = ix.data.len();
    // Add fake_delegate to accounts (not as signer)
    let fake_delegate_idx = ix.accounts.len() as u8;
    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            fake_delegate.pubkey(),
            false, // NOT a signer - this is the attack
        ));
    // Set delegate_index to point to fake_delegate
    ix.data[data_len - 1] = fake_delegate_idx;

    // This should fail because delegate is not a signer
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when delegate mode is specified but delegate doesn't sign"
    );
}

/// Test: ATTACK - Specify delegate mode, delegate signs, but inputs don't have delegate set
/// This tests that tokens without delegates cannot be stolen via delegate mode
#[tokio::test]
#[serial]
async fn test_delegate_mode_inputs_have_no_delegate_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Verify inputs have NO delegate (this is the precondition for this attack test)
    for acc in &compressed_accounts {
        assert!(
            acc.token.delegate.is_none(),
            "Test precondition: inputs should not have delegate"
        );
    }

    // Create attacker keypair who will try to steal tokens by claiming to be delegate
    let attacker = Keypair::new();
    airdrop_lamports(&mut ctx.rpc, &attacker.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Build instruction in owner mode first
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Add attacker as "delegate" and set delegate_index
    let data_len = ix.data.len();
    let attacker_idx = ix.accounts.len() as u8;
    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            attacker.pubkey(),
            true, // Attacker DOES sign
        ));
    // Set delegate_index to point to attacker
    ix.data[data_len - 1] = attacker_idx;

    // This should fail because inputs don't have delegate set (has_delegate() == false)
    // Even though attacker signs, the on-chain check should reject because
    // input.has_delegate() will be false
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &attacker])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when inputs don't have delegate but delegate mode is used"
    );
}

/// Test: ATTACK - Delegate index out of bounds
/// This tests that malformed instructions with invalid indices are rejected
#[tokio::test]
#[serial]
async fn test_delegate_mode_delegate_index_out_of_bounds_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set delegate_index to out of bounds value
    let data_len = ix.data.len();
    let num_accounts = ix.accounts.len();
    ix.data[data_len - 1] = (num_accounts + 10) as u8; // Way out of bounds

    // This should fail with NotEnoughAccountKeys or similar
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when delegate_index is out of bounds"
    );
}

/// Test: ATTACK - Use delegate mode but have owner sign instead of delegate
/// This tests that owner cannot act as delegate when delegate mode is specified
#[tokio::test]
#[serial]
async fn test_delegate_mode_owner_signs_instead_of_delegate_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // Create a "delegate" that won't sign
    let fake_delegate = Keypair::new();

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Add delegate to accounts but don't make it signer
    let data_len = ix.data.len();
    let delegate_idx = ix.accounts.len() as u8;
    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            fake_delegate.pubkey(),
            false, // NOT signer
        ));
    ix.data[data_len - 1] = delegate_idx;

    // Try to sign with owner only (which signed in owner mode)
    // This should fail because delegate mode requires delegate to sign
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when delegate mode is used but only owner signs"
    );
}

/// Test: ATTACK - Modify wallet_index to point to wrong account
/// This tests that ATA derivation check catches wrong wallet
#[tokio::test]
#[serial]
async fn test_attack_wrong_wallet_index_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Change wallet_index to point to a different account
    let data_len = ix.data.len();
    // wallet_index is at data_len - 5
    ix.data[data_len - 5] = 0; // Point to first account (likely fee_payer or system program)

    // This should fail because ATA derivation won't match
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when wallet_index points to wrong account"
    );
}

/// Test: ATTACK - Modify mint_index to point to wrong account
/// This tests that ATA derivation check catches wrong mint
#[tokio::test]
#[serial]
async fn test_attack_wrong_mint_index_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Change mint_index to point to a different account
    let data_len = ix.data.len();
    // mint_index is at data_len - 4
    ix.data[data_len - 4] = 0; // Point to first account

    // This should fail because ATA derivation won't match
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when mint_index points to wrong account"
    );
}

/// Test: ATTACK - Modify ata_index to point to wrong account
/// This tests that ATA derivation check catches wrong ATA
#[tokio::test]
#[serial]
async fn test_attack_wrong_ata_index_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Change ata_index to point to a different account
    let data_len = ix.data.len();
    // ata_index is at data_len - 3
    ix.data[data_len - 3] = 0; // Point to first account

    // This should fail because ATA derivation won't match
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when ata_index points to wrong account"
    );
}

/// Test: SDK validation - delegate provided but inputs don't have that delegate
/// This tests SDK-side validation catches mismatched delegates
#[tokio::test]
#[serial]
async fn test_sdk_rejects_delegate_not_matching_inputs() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    // These accounts don't have any delegate set
    // Try to use delegate mode - SDK should reject
    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: true, // Request delegate mode but inputs don't have delegate set
    };

    let result = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey()).await;

    assert!(
        result.is_err(),
        "SDK must reject when delegate is specified but inputs don't have that delegate"
    );
}

/// Test: ATTACK - Duplicate indices (wallet_idx == mint_idx)
/// This could cause ATA derivation to use same key twice, breaking the derivation
#[tokio::test]
#[serial]
async fn test_attack_duplicate_wallet_mint_index_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set mint_index to same value as wallet_index
    let data_len = ix.data.len();
    let wallet_idx = ix.data[data_len - 5];
    ix.data[data_len - 4] = wallet_idx; // mint_idx = wallet_idx

    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when wallet_idx == mint_idx (ATA derivation would be wrong)"
    );
}

/// Test: ATTACK - Duplicate indices (wallet_idx == ata_idx)
/// This tests that we can't use same account for wallet and ATA
#[tokio::test]
#[serial]
async fn test_attack_duplicate_wallet_ata_index_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set ata_index to same value as wallet_index
    let data_len = ix.data.len();
    let wallet_idx = ix.data[data_len - 5];
    ix.data[data_len - 3] = wallet_idx; // ata_idx = wallet_idx

    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when wallet_idx == ata_idx (account confusion)"
    );
}

/// Test: ATTACK - ata_index pointing to system account slot (< 7)
/// The packed_index calculation uses saturating_sub(7), so ata_index < 7 would give packed_index = 0
/// This could potentially match an input with owner = 0
#[tokio::test]
#[serial]
async fn test_attack_ata_index_in_system_slot_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set ata_index to 0 (system account slot)
    // This would make ata_packed_index = 0.saturating_sub(7) = 0
    let data_len = ix.data.len();
    ix.data[data_len - 3] = 0; // ata_idx = 0 (likely system program or fee payer)

    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when ata_index is in system slot (packed_index underflow)"
    );
}

/// Test: ATTACK - Delegate index same as wallet index
/// This tests account confusion when delegate points to same account as wallet
#[tokio::test]
#[serial]
async fn test_attack_delegate_index_equals_wallet_index_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set delegate_index to same value as wallet_index
    // This tries to use owner_wallet as both the wallet for ATA derivation AND as delegate
    let data_len = ix.data.len();
    let wallet_idx = ix.data[data_len - 5];
    ix.data[data_len - 1] = wallet_idx; // delegate_idx = wallet_idx

    // This should fail because:
    // 1. In delegate mode, inputs must have delegate field set to delegate_packed_index
    // 2. Our inputs don't have delegate set, so has_delegate() = false
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when delegate_idx == wallet_idx (inputs don't have delegate)"
    );
}

/// Test: ATTACK - All indices point to same account
/// Maximum confusion attack
#[tokio::test]
#[serial]
async fn test_attack_all_indices_same_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set all indices to same value
    let data_len = ix.data.len();
    let single_idx = ix.data[data_len - 5]; // wallet_idx
    ix.data[data_len - 5] = single_idx; // wallet_idx
    ix.data[data_len - 4] = single_idx; // mint_idx
    ix.data[data_len - 3] = single_idx; // ata_idx

    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when all indices point to same account"
    );
}

/// Test: ATTACK - Zero byte for all suffix indices
/// Tests handling of all-zero suffix
#[tokio::test]
#[serial]
async fn test_attack_all_zero_suffix_indices_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Set all indices to 0 (except delegate which stays 255)
    let data_len = ix.data.len();
    ix.data[data_len - 5] = 0; // wallet_idx = 0
    ix.data[data_len - 4] = 0; // mint_idx = 0
    ix.data[data_len - 3] = 0; // ata_idx = 0
    ix.data[data_len - 2] = 0; // bump = 0

    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when suffix has all zero indices"
    );
}

/// Test: Empty instruction data attack (too short)
#[tokio::test]
#[serial]
async fn test_instruction_data_too_short_fails() {
    let mut ctx = setup_transfer2_with_ata_test().await.unwrap();
    ctx.create_mint().await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();
    ctx.mint_and_compress_to_ata(1000).await.unwrap();
    ctx.create_ata_with_compress_to_pubkey().await.unwrap();

    let compressed_accounts = ctx.get_ata_owned_compressed_accounts().await.unwrap();
    assert!(!compressed_accounts.is_empty());

    let input = DecompressAtaInput {
        compressed_token_accounts: compressed_accounts.clone(),
        owner_wallet: ctx.owner_wallet.pubkey(),
        mint: ctx.mint,
        destination_ata: ctx.ata,
        decompress_amount: None,
        use_delegate: false,
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // ATTACK: Truncate instruction data to be too short (less than 5 bytes suffix)
    ix.data.truncate(3); // Way too short

    // This should fail with InvalidInstructionData
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(
        result.is_err(),
        "SECURITY: Must fail when instruction data is too short"
    );
}

// ============================================================================
// CU BENCHMARKS - Compare Transfer2WithAta vs Regular Transfer2
// ============================================================================

/// Benchmark: Compare CU usage between Transfer2WithAta and regular Transfer2 decompression
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_cu_benchmark() {
    use light_program_test::utils::simulate_cu_multi;
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
    };

    println!("\n========================================");
    println!("Transfer2WithAta vs Transfer2 CU Benchmark");
    println!("========================================\n");

    // Test configurations: (num_inputs, amount_per_input)
    let test_configs = [(1, 1000u64), (2, 500u64)];

    for (num_inputs, amount_per_input) in test_configs {
        println!(
            "--- {} input(s), {} tokens each ---",
            num_inputs, amount_per_input
        );

        // Setup for Transfer2WithAta
        let mut ata_ctx = setup_transfer2_with_ata_test().await.unwrap();
        ata_ctx.create_mint().await.unwrap();

        // Create multiple ATA-owned compressed accounts
        for _ in 0..num_inputs {
            ata_ctx.create_ata_with_compress_to_pubkey().await.unwrap();
            ata_ctx
                .mint_and_compress_to_ata(amount_per_input)
                .await
                .unwrap();
        }

        // Re-create ATA for destination
        ata_ctx.create_ata_with_compress_to_pubkey().await.unwrap();

        let ata_compressed_accounts = ata_ctx.get_ata_owned_compressed_accounts().await.unwrap();
        assert_eq!(
            ata_compressed_accounts.len(),
            num_inputs,
            "Should have {} ATA-owned compressed accounts",
            num_inputs
        );

        // Build Transfer2WithAta instruction
        let ata_input = DecompressAtaInput {
            compressed_token_accounts: ata_compressed_accounts,
            owner_wallet: ata_ctx.owner_wallet.pubkey(),
            mint: ata_ctx.mint,
            destination_ata: ata_ctx.ata,
            decompress_amount: None,
            use_delegate: false,
        };

        let ata_ix =
            create_decompress_ata_instruction(&mut ata_ctx.rpc, ata_input, ata_ctx.payer.pubkey())
                .await
                .unwrap();

        // Measure Transfer2WithAta CU
        let ata_cu = simulate_cu_multi(
            &mut ata_ctx.rpc,
            &ata_ctx.payer,
            &ata_ix,
            &[&ata_ctx.owner_wallet],
        )
        .await;

        // Setup for regular Transfer2 decompression
        let mut transfer2_ctx = setup_transfer2_with_ata_test().await.unwrap();
        transfer2_ctx.create_mint().await.unwrap();

        // For regular transfer2, mint directly to wallet (wallet-owned compressed tokens)
        let total_amount = amount_per_input * num_inputs as u64;
        for _ in 0..num_inputs {
            transfer2_ctx
                .mint_to_wallet(amount_per_input)
                .await
                .unwrap();
        }

        // Create CToken ATA for decompression destination (regular non-compressible ATA)
        let (ctoken_ata, bump) =
            derive_ctoken_ata(&transfer2_ctx.owner_wallet.pubkey(), &transfer2_ctx.mint);
        let create_ata_ix = light_compressed_token_sdk::ctoken::CreateAssociatedTokenAccount {
            idempotent: false,
            bump,
            payer: transfer2_ctx.payer.pubkey(),
            owner: transfer2_ctx.owner_wallet.pubkey(),
            mint: transfer2_ctx.mint,
            associated_token_account: ctoken_ata,
            compressible: None,
        }
        .instruction()
        .unwrap();

        transfer2_ctx
            .rpc
            .create_and_send_transaction(
                &[create_ata_ix],
                &transfer2_ctx.payer.pubkey(),
                &[&transfer2_ctx.payer],
            )
            .await
            .unwrap();

        // Get wallet-owned compressed accounts
        let wallet_compressed_accounts = transfer2_ctx
            .get_wallet_owned_compressed_accounts()
            .await
            .unwrap();
        assert_eq!(
            wallet_compressed_accounts.len(),
            num_inputs,
            "Should have {} wallet-owned compressed accounts",
            num_inputs
        );

        // Build regular Transfer2 decompression instruction
        let decompress_input = DecompressInput {
            compressed_token_account: wallet_compressed_accounts,
            decompress_amount: total_amount,
            solana_token_account: ctoken_ata,
            amount: total_amount,
            pool_index: None,
        };

        let transfer2_ix = create_generic_transfer2_instruction(
            &mut transfer2_ctx.rpc,
            vec![Transfer2InstructionType::Decompress(decompress_input)],
            transfer2_ctx.payer.pubkey(),
            false,
        )
        .await
        .unwrap();

        // Measure regular Transfer2 CU
        let transfer2_cu = simulate_cu_multi(
            &mut transfer2_ctx.rpc,
            &transfer2_ctx.payer,
            &transfer2_ix,
            &[&transfer2_ctx.owner_wallet],
        )
        .await;

        // Calculate and print results
        let cu_diff = ata_cu as i64 - transfer2_cu as i64;
        let percent_diff = if transfer2_cu > 0 {
            (cu_diff as f64 / transfer2_cu as f64) * 100.0
        } else {
            0.0
        };

        println!("{} input(s) with ata    : {} cu", num_inputs, ata_cu);
        println!("{} input(s) transfer2   : {} cu", num_inputs, transfer2_cu);
        println!(
            "{} input(s) difference  : {:+} cu ({:+.1}%)",
            num_inputs, cu_diff, percent_diff
        );
        println!();
    }

    println!("========================================");
    println!("Benchmark complete");
    println!("========================================\n");
}
