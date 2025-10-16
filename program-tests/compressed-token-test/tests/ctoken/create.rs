use anchor_lang::prelude::AccountMeta;
use light_ctoken_types::instructions::create_ctoken_account::CreateTokenAccountInstructionData;
use rand::{
    rngs::{StdRng, ThreadRng},
    Rng, RngCore, SeedableRng,
};
use solana_sdk::instruction::Instruction;

use super::shared::*;

#[tokio::test]
async fn test_create_compressible_token_account() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Test 1: Zero epoch prefunding (immediately compressible)
    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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
                Some(rng.gen_range(0..=u16::MAX as u32))
            } else {
                None
            },
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat, // Only V3 supported
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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_token_account_fails(
            &mut context,
            compressible_data,
            "one_epoch_prefunding_forbidden",
            101, // OneEpochPrefundingNotAllowed (0x65 hex = 101 decimal)
        )
        .await;
    }

    // Test 2: Account already initialized
    // Creating the same account twice should fail.
    // Error: 6078 (AlreadyInitialized)
    {
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        // First creation succeeds
        create_and_assert_token_account(&mut context, compressible_data.clone(), "first_creation")
            .await;

        // Second creation fails
        create_and_assert_token_account_fails(
            &mut context,
            compressible_data,
            "account_already_initialized",
            0, // AlreadyInitialized system program cpi fails (for compressible accounts we create the token accounts via cpi)
        )
        .await;
    }

    // Test 3: Insufficient payer balance
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

        let create_token_account_ix =
            light_compressed_token_sdk::instructions::create_compressible_token_account(
                light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                    account_pubkey: token_account_pubkey.pubkey(),
                    mint_pubkey: context.mint_pubkey,
                    owner_pubkey: context.owner_keypair.pubkey(),
                    compressible_config: context.compressible_config,
                    rent_sponsor: context.rent_sponsor,
                    pre_pay_num_epochs: 10, // High number to require more lamports
                    lamports_per_write: Some(1000),
                    payer: poor_payer_pubkey,
                    compress_to_account_pubkey: None,
                    token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
                },
            )
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

    // Test 4: Non-compressible account already initialized
    // For non-compressible accounts, the account already exists and is owned by the program.
    // Trying to initialize it again should fail with AlreadyInitialized from our program.
    // Error: 58 (AlreadyInitialized from our program, not system program)
    {
        println!("starting test 4");
        context.token_account_keypair = Keypair::new();
        // Create the account via system program
        let rent = context
            .rpc
            .get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap();

        let create_account_ix = solana_sdk::system_instruction::create_account(
            &payer_pubkey,
            &context.token_account_keypair.pubkey(),
            rent,
            165,
            &light_compressed_token::ID,
        );

        // Send create account transaction
        context
            .rpc
            .create_and_send_transaction(
                &[create_account_ix],
                &payer_pubkey,
                &[&context.payer, &context.token_account_keypair],
            )
            .await
            .unwrap();
        // Build initialize instruction data (non-compressible)
        let init_data = CreateTokenAccountInstructionData {
            owner: context.owner_keypair.pubkey().into(),
            compressible_config: None, // Non-compressible
        };
        use anchor_lang::prelude::borsh::BorshSerialize;
        let mut data = vec![18]; // CreateTokenAccount discriminator
        init_data.serialize(&mut data).unwrap();

        // Build instruction
        let init_ix = Instruction {
            program_id: light_compressed_token::ID,
            accounts: vec![
                AccountMeta::new(context.token_account_keypair.pubkey(), true),
                AccountMeta::new_readonly(context.mint_pubkey, false),
            ],
            data: data.clone(),
        };

        // First initialization should succeed
        context
            .rpc
            .create_and_send_transaction(
                std::slice::from_ref(&init_ix),
                &payer_pubkey,
                &[&context.payer, &context.token_account_keypair],
            )
            .await
            .unwrap();
        let other_payer = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&other_payer.pubkey(), 10000000000)
            .await
            .unwrap();
        // Build instruction
        let init_ix = Instruction {
            program_id: light_compressed_token::ID,
            accounts: vec![
                AccountMeta::new(context.token_account_keypair.pubkey(), true),
                AccountMeta::new_readonly(context.mint_pubkey, false),
            ],
            data,
        };
        // Second initialization should fail with AlreadyInitialized
        let result = context
            .rpc
            .create_and_send_transaction(
                &[init_ix],
                &other_payer.pubkey(),
                &[&other_payer, &context.token_account_keypair],
            )
            .await;

        // Should fail with AlreadyInitialized (78) from our program
        light_program_test::utils::assert::assert_rpc_error(result, 0, 78).unwrap();
    }

    // Test 5: Invalid PDA seeds for compress_to_account_pubkey
    // When compress_to_account_pubkey is provided, the seeds must derive to the token account.
    // Providing invalid seeds should fail the PDA validation.
    // Error: 18002 (InvalidAccountData from CTokenError)
    {
        use light_ctoken_types::instructions::extensions::compressible::CompressToPubkey;

        context.token_account_keypair = Keypair::new();
        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Create invalid PDA config with wrong seeds that won't derive to token_account_pubkey
        let invalid_compress_to_pubkey = CompressToPubkey {
            bump: 255,
            program_id: light_compressed_token::ID.to_bytes(),
            seeds: vec![b"invalid_seed".to_vec(), b"wrong".to_vec()],
        };

        let create_token_account_ix =
            light_compressed_token_sdk::instructions::create_compressible_token_account(
                light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                    account_pubkey: token_account_pubkey,
                    mint_pubkey: context.mint_pubkey,
                    owner_pubkey: context.owner_keypair.pubkey(),
                    compressible_config: context.compressible_config,
                    rent_sponsor: context.rent_sponsor,
                    pre_pay_num_epochs: 2,
                    lamports_per_write: Some(100),
                    payer: payer_pubkey,
                    compress_to_account_pubkey: Some(invalid_compress_to_pubkey),
                    token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
                },
            )
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

        let create_token_account_ix =
            light_compressed_token_sdk::instructions::create_compressible_token_account(
                light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                    account_pubkey: context.token_account_keypair.pubkey(),
                    mint_pubkey: context.mint_pubkey,
                    owner_pubkey: context.owner_keypair.pubkey(),
                    compressible_config: fake_config_pubkey, // Wrong owner
                    rent_sponsor: context.rent_sponsor,
                    pre_pay_num_epochs: 2,
                    lamports_per_write: Some(100),
                    payer: payer_pubkey,
                    compress_to_account_pubkey: None,
                    token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
                },
            )
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

    // Test 7: Wrong account type (correct program owner, wrong discriminator)
    // Passing an account owned by the registry program but not a CompressibleConfig.
    // Using the protocol config account which has a different discriminator.
    // Error: 2 (InvalidDiscriminator from account-checks)
    {
        context.token_account_keypair = Keypair::new();

        // Use protocol config account - owned by registry but wrong type
        let wrong_account_type = context.rpc.test_accounts.protocol.governance_authority_pda;

        let create_token_account_ix =
            light_compressed_token_sdk::instructions::create_compressible_token_account(
                light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                    account_pubkey: context.token_account_keypair.pubkey(),
                    mint_pubkey: context.mint_pubkey,
                    owner_pubkey: context.owner_keypair.pubkey(),
                    compressible_config: wrong_account_type, // Wrong account type
                    rent_sponsor: context.rent_sponsor,
                    pre_pay_num_epochs: 2,
                    lamports_per_write: Some(100),
                    payer: payer_pubkey,
                    compress_to_account_pubkey: None,
                    token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
                },
            )
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
}
