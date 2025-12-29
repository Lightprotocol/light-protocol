use rand::{
    rngs::{StdRng, ThreadRng},
    Rng, RngCore, SeedableRng,
};

use super::shared::*;

#[tokio::test]
async fn test_create_compressible_token_account_instruction() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Test 1: Zero epoch prefunding (immediately compressible)
    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: Some(100),
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_token_account(&mut context, compressible_data, "zero_epoch_prefunding")
            .await;
    }

    // Test 2: Three epoch prefunding
    {
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_token_account(&mut context, compressible_data, "two_epoch_prefunding")
            .await;
    }

    // Test 3: Ten epoch prefunding
    {
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 10,
            lamports_per_write: Some(100),
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_token_account(&mut context, compressible_data, "ten_epoch_prefunding")
            .await;
    }

    // Test 4: Custom fee payer (payer == rent_sponsor, payer pays everything)
    {
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: payer_pubkey,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_token_account(&mut context, compressible_data, "custom_fee_payer").await;
    }
    // Test 5: No lamports_per_write
    {
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: None,
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_token_account(&mut context, compressible_data, "No lamports_per_write")
            .await;
    }
}

#[tokio::test]
async fn test_create_account_random() {
    // Setup randomness
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.next_u64();
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\n🎲 Random Create Account Test - Seed: {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    // Run 100 random test iterations
    for iteration in 0..100 {
        println!("\n--- Random Test Iteration {} ---", iteration + 1);
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority, // Config account forces this authority.
            rent_sponsor: if rng.gen_bool(0.5) {
                payer_pubkey
            } else {
                context.rent_sponsor
            },
            num_prepaid_epochs: {
                let value = rng.gen_range(0..=255);
                if value != 1 {
                    value
                } else {
                    0
                }
            },
            lamports_per_write: if rng.gen_bool(0.5) {
                // Limit to max_top_up to avoid WriteTopUpExceedsMaximum error
                Some(rng.gen_range(0..=RentConfig::default().max_top_up as u32))
            } else {
                None
            },
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat, // Only V3 supported
            compress_to_pubkey: false, // Can only be tested with cpi
            payer: payer_pubkey,
        };
        context.token_account_keypair = Keypair::new();
        create_and_assert_token_account(
            &mut context,
            compressible_data.clone(),
            format!(
                "\n--- Random Test Iteration {} --- {:?}",
                iteration + 1,
                compressible_data
            )
            .as_str(),
        )
        .await;
    }
}

// ============================================================================
// Failing Tests
#[tokio::test]
async fn test_create_compressible_token_account_failing() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Test 1: One epoch prefunding forbidden
    // Accounts with exactly 1 epoch could become immediately compressible
    // at epoch boundaries, creating timing edge cases.
    // Error: 6116 (0x17E4) -> but on-chain returns 101 (0x65)
    {
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 1, // Forbidden value
            lamports_per_write: Some(100),
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_token_account_fails(
            &mut context,
            compressible_data,
            "one_epoch_prefunding_forbidden",
            6101, // OneEpochPrefundingNotAllowed
        )
        .await;
    }

    // Note: Test 2 (AlreadyInitialized) removed because create_pda_account now uses
    // DoS prevention logic that allows re-initialization via Assign + realloc path.
    // When an account already has lamports, it doesn't call CreateAccount.

    // Test 2: Insufficient payer balance
    // Payer doesn't have enough lamports for rent payment.
    // This will fail during the transfer_lamports_via_cpi call.
    // Error: 1 (InsufficientFunds from system program)
    {
        // Create a payer with insufficient funds (only enough for tx fees + account creation)
        let poor_payer = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&poor_payer.pubkey(), 10000) // Not enough for additional rent
            .await
            .unwrap();

        let poor_payer_pubkey = poor_payer.pubkey();
        let token_account_pubkey = Keypair::new();

        let compressible_params = CompressibleParams {
            compressible_config: context.compressible_config,
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 10, // High number to require more lamports
            lamports_per_write: Some(1000),
            compress_to_account_pubkey: None,
            token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateCTokenAccount::new(
            poor_payer_pubkey,
            token_account_pubkey.pubkey(),
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &poor_payer_pubkey,
                &[&poor_payer, &token_account_pubkey],
            )
            .await;

        // Should fail with InsufficientFunds (1) from system program
        light_program_test::utils::assert::assert_rpc_error(result, 0, 1).unwrap();
    }

    // Note: Test 4 (Non-compressible account already initialized) removed because:
    // 1. All accounts now have compression infrastructure (no pure non-compressible accounts)
    // 2. The manual instruction approach with only 2 accounts is no longer valid
    // 3. DoS prevention allows re-initialization via Assign + realloc path

    // Test 3: Invalid PDA seeds for compress_to_account_pubkey
    // When compress_to_account_pubkey is provided, the seeds must derive to the token account.
    // Providing invalid seeds should fail the PDA validation.
    // Error: 18002 (InvalidAccountData from CTokenError)
    {
        use light_ctoken_interface::instructions::extensions::CompressToPubkey;

        context.token_account_keypair = Keypair::new();
        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Create invalid PDA config with wrong seeds that won't derive to token_account_pubkey
        let invalid_compress_to_pubkey = CompressToPubkey {
            bump: 255,
            program_id: light_compressed_token::ID.to_bytes(),
            seeds: vec![b"invalid_seed".to_vec(), b"wrong".to_vec()],
        };

        let compressible_params = CompressibleParams {
            compressible_config: context.compressible_config,
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: Some(invalid_compress_to_pubkey),
            token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateCTokenAccount::new(
            payer_pubkey,
            token_account_pubkey,
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &payer_pubkey,
                &[&context.payer, &context.token_account_keypair],
            )
            .await;

        // Should fail with InvalidAccountData (18002) from CTokenError
        light_program_test::utils::assert::assert_rpc_error(result, 0, 18002).unwrap();
    }

    // Test 6: Invalid config account owner
    // Config account must be owned by the compressible program.
    // Providing a config account owned by a different program should fail.
    // Error: 3 (IncorrectProgramId from account-checks)
    {
        context.token_account_keypair = Keypair::new();

        // Create a fake config account owned by system program
        let fake_config_keypair = Keypair::new();
        let fake_config_pubkey = fake_config_keypair.pubkey();

        // Fund the fake config account (owned by system program, not compressible program)
        context
            .rpc
            .airdrop_lamports(&fake_config_pubkey, 10000000)
            .await
            .unwrap();

        let compressible_params = CompressibleParams {
            compressible_config: fake_config_pubkey, // Wrong owner
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: None,
            token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateCTokenAccount::new(
            payer_pubkey,
            context.token_account_keypair.pubkey(),
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &payer_pubkey,
                &[&context.payer, &context.token_account_keypair],
            )
            .await;

        // Should fail with AccountOwnedByWrongProgram 20001 from account-checks
        light_program_test::utils::assert::assert_rpc_error(result, 0, 20001).unwrap();
    }

    // Test 7: write_top_up exceeds max_top_up from RentConfig
    // Accounts cannot be created with lamports_per_write > max_top_up.
    // This protects against griefing attacks where recipient creates account with excessive top-up.
    // Error: 18042 (WriteTopUpExceedsMaximum from CTokenError)
    {
        context.token_account_keypair = Keypair::new();

        // Default max_top_up is 6208, so use 6209 to exceed it
        let excessive_lamports_per_write = RentConfig::default().max_top_up as u32 + 1;

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(excessive_lamports_per_write),
            account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_token_account_fails(
            &mut context,
            compressible_data,
            "write_top_up_exceeds_max_top_up",
            18042, // WriteTopUpExceedsMaximum from CTokenError
        )
        .await;
    }

    // Test 8: Wrong account type (correct program owner, wrong discriminator)
    // Passing an account owned by the registry program but not a CompressibleConfig.
    // Using the protocol config account which has a different discriminator.
    // Error: 2 (InvalidDiscriminator from account-checks)
    {
        context.token_account_keypair = Keypair::new();

        // Use protocol config account - owned by registry but wrong type
        let wrong_account_type = context.rpc.test_accounts.protocol.governance_authority_pda;

        let compressible_params = CompressibleParams {
            compressible_config: wrong_account_type, // Wrong account type
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: None,
            token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateCTokenAccount::new(
            payer_pubkey,
            context.token_account_keypair.pubkey(),
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &payer_pubkey,
                &[&context.payer, &context.token_account_keypair],
            )
            .await;

        // Should fail with InvalidDiscriminator (20000) from account-checks
        light_program_test::utils::assert::assert_rpc_error(result, 0, 20000).unwrap();
    }

    // Test 9: Non-signer custom rent payer (DoS prevention)
    // Custom rent payer must be a signer to prevent setting executable accounts as rent_sponsor.
    // This prevents DoS attacks where an attacker sets an executable account as rent_sponsor,
    // making the token account impossible to close (lamport transfers to executable accounts fail).
    // Error: 8 (MissingRequiredSignature)
    {
        context.token_account_keypair = Keypair::new();

        // Use account compression program as custom rent payer (executable, cannot sign)
        let executable_rent_payer = light_sdk::constants::ACCOUNT_COMPRESSION_PROGRAM_ID.into();

        let compressible_params = CompressibleParams {
            compressible_config: context.compressible_config,
            rent_sponsor: executable_rent_payer, // Executable account!
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: None,
            token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateCTokenAccount::new(
            payer_pubkey,
            context.token_account_keypair.pubkey(),
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &payer_pubkey,
                &[&context.payer, &context.token_account_keypair],
            )
            .await;

        // Should fail with MissingRequiredSignature (8)
        light_program_test::utils::assert::assert_rpc_error(result, 0, 8).unwrap();
    }
}
