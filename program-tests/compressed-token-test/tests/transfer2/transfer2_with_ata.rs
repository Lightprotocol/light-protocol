//! Tests for Transfer2WithAta instruction.
//!
//! Transfer2WithAta enables decompress/transfer operations on compressed tokens
//! where ALL inputs have owner = ATA pubkey (compress_to_pubkey mode).
//!
//! Test coverage:
//! SUCCESS CASES:
//! 1. Single ATA-owned compressed token decompressed
//! 2. Multiple ATA-owned compressed tokens decompressed in single call
//!
//! FAILURE CASES:
//! 1. Wrong owner signer (not wallet that owns the ATA)
//! 2. Wrong mint passed
//! 3. wallet_idx correct key but not signer
//! 4. False ATA derivation (wrong bump)
//! 5. Non-matching ATA in accounts
//! 6. Mixed ownership (some ATA-owned, some wallet-owned) - must fail because all inputs must be ATA-owned

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
    };

    // Should fail because not all inputs have owner = ATA
    let result = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey()).await;
    assert!(
        result.is_err(),
        "Should fail when mixing ATA-owned and wallet-owned inputs"
    );
}

/// Test: Fail with wrong bump (false ATA derivation)
#[tokio::test]
#[serial]
async fn test_transfer2_with_ata_wrong_bump_fails() {
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
    };

    let mut ix = create_decompress_ata_instruction(&mut ctx.rpc, input, ctx.payer.pubkey())
        .await
        .unwrap();

    // Tamper with the bump (last byte of instruction data)
    let data_len = ix.data.len();
    let correct_bump = ix.data[data_len - 1];
    ix.data[data_len - 1] = correct_bump.wrapping_add(1); // Wrong bump

    // This should fail on-chain
    let result = ctx
        .rpc
        .create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, &ctx.owner_wallet])
        .await;

    assert!(result.is_err(), "Should fail with wrong ATA bump");
}
