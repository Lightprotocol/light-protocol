use super::shared::*;

#[tokio::test]
async fn test_create_compressible_ata() {
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

        create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "zero_epoch_prefunding",
        )
        .await;
    }

    // Test 2: Two epoch prefunding
    {
        // Use different mint for second ATA
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "two_epoch_prefunding",
        )
        .await;
    }

    // Test 3: Ten epoch prefunding
    {
        // Use different mint for third ATA
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 10,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "ten_epoch_prefunding",
        )
        .await;
    }

    // Test 4: Custom fee payer (payer == rent_sponsor, payer pays everything)
    {
        // Use different mint for fourth ATA
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: payer_pubkey,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "custom_fee_payer",
        )
        .await;
    }

    // Test 5: No lamports_per_write
    {
        // Use different mint for fifth ATA
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: None,
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "no_lamports_per_write",
        )
        .await;
    }
}

#[tokio::test]
async fn test_create_ata_idempotent() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Test 1: Create ATA with idempotent mode (first creation succeeds)
    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        let ata_pubkey = create_and_assert_ata(
            &mut context,
            Some(compressible_data.clone()),
            true, // Idempotent
            "idempotent_first_creation",
        )
        .await;

        // Test 2: Create same ATA again with idempotent mode (should succeed without error)
        let ata_pubkey_second = create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            true, // Idempotent
            "idempotent_second_creation",
        )
        .await;

        // Test 3: Verify both creations returned the same address
        assert_eq!(
            ata_pubkey, ata_pubkey_second,
            "Both idempotent creations should return the same ATA address"
        );

        // Verify the account still has the same properties (unchanged by second creation)
        let account = context.rpc.get_account(ata_pubkey).await.unwrap().unwrap();

        // Should still be 260 bytes (compressible)
        assert_eq!(
            account.data.len(),
            light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize,
            "Account should still be compressible size after idempotent recreation"
        );
    }
}

#[tokio::test]
async fn test_create_non_compressible_ata() {
    let mut context = setup_account_test().await.unwrap();

    create_and_assert_ata(
        &mut context,
        None,  // Non-compressible
        false, // Non-idempotent
        "non_compressible_ata",
    )
    .await;
}

// ============================================================================
// Failing Tests
#[tokio::test]
async fn test_create_ata_failing() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Test 1: One epoch prefunding forbidden
    // Accounts with exactly 1 epoch could become immediately compressible
    // at epoch boundaries, creating timing edge cases.
    // Error: 101 (OneEpochPrefundingNotAllowed)
    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 1, // Forbidden value
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_ata_fails(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "one_epoch_prefunding_forbidden",
            101, // OneEpochPrefundingNotAllowed (0x65 hex = 101 decimal)
        )
        .await;
    }

    // Test 2: Account already initialized (non-idempotent)
    // Creating the same ATA twice with non-idempotent mode should fail.
    // Error: 18 (IllegalOwner - account is no longer owned by system program)
    {
        // Use a different mint for this test
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

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
        create_and_assert_ata(
            &mut context,
            Some(compressible_data.clone()),
            false, // Non-idempotent
            "first_creation",
        )
        .await;

        // Second creation fails with IllegalOwner
        create_and_assert_ata_fails(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "account_already_initialized",
            29, // IllegalOwner - account is no longer system-owned
        )
        .await;
    }

    // Test 3: Insufficient payer balance
    // Payer doesn't have enough lamports for rent payment.
    // Error: 1 (InsufficientFunds from system program)
    {
        // Create a payer with insufficient funds (only enough for tx fees)
        let poor_payer = solana_sdk::signature::Keypair::new();
        context
            .rpc
            .airdrop_lamports(&poor_payer.pubkey(), 10000) // Not enough for rent
            .await
            .unwrap();

        let poor_payer_pubkey = poor_payer.pubkey();

        // Use different mint and owner for this test
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();
        let new_owner = solana_sdk::signature::Keypair::new();

        let create_ata_ix = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
                payer: poor_payer_pubkey,
                owner: new_owner.pubkey(),
                mint: context.mint_pubkey,
                compressible_config: context.compressible_config,
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: 10, // High number to require more lamports
                lamports_per_write: Some(1000),
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(&[create_ata_ix], &poor_payer_pubkey, &[&poor_payer])
            .await;

        // Should fail with InsufficientFunds (1) from system program
        light_program_test::utils::assert::assert_rpc_error(result, 0, 1).unwrap();
    }

    // Test 4: compress_to_account_pubkey provided (forbidden for ATAs)
    // ATAs cannot use compress_to_account_pubkey option.
    // Error: 2 (InvalidInstructionData)
    {
        use anchor_lang::prelude::borsh::BorshSerialize;
        use light_ctoken_types::instructions::{
            create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
            extensions::compressible::{CompressToPubkey, CompressibleExtensionInstructionData},
        };
        use solana_sdk::instruction::Instruction;

        // Use different mint for this test
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();
        let (ata_pubkey, bump) =
            derive_ctoken_ata(&context.owner_keypair.pubkey(), &context.mint_pubkey);

        // Manually build instruction data with compress_to_account_pubkey (forbidden)
        let compress_to_pubkey = CompressToPubkey {
            bump: 255,
            program_id: light_compressed_token::ID.to_bytes(),
            seeds: vec![b"test".to_vec()],
        };

        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            owner: context.owner_keypair.pubkey().into(),
            mint: context.mint_pubkey.into(),
            bump,
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat as u8,
                rent_payment: 2,
                has_top_up: 1,
                write_top_up: 100,
                compress_to_account_pubkey: Some(compress_to_pubkey), // Forbidden for ATAs!
            }),
        };

        let mut data = vec![100]; // CreateAssociatedTokenAccount discriminator
        instruction_data.serialize(&mut data).unwrap();

        let ix = Instruction {
            program_id: light_compressed_token::ID,
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(payer_pubkey, true),
                solana_sdk::instruction::AccountMeta::new(ata_pubkey, false),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    solana_sdk::pubkey::Pubkey::default(),
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    context.compressible_config,
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new(context.rent_sponsor, false),
            ],
            data,
        };

        let result = context
            .rpc
            .create_and_send_transaction(&[ix], &payer_pubkey, &[&context.payer])
            .await;

        // Should fail with InvalidInstructionData (2)
        light_program_test::utils::assert::assert_rpc_error(result, 0, 2).unwrap();
    }

    // Test 5: Invalid PDA derivation (wrong bump)
    // ATAs must use the correct bump derived from [owner, program_id, mint]
    // Error: 21 (ProgramFailedToComplete - provided seeds do not result in valid address)
    {
        use anchor_lang::prelude::borsh::BorshSerialize;
        use light_ctoken_types::instructions::{
            create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
            extensions::compressible::CompressibleExtensionInstructionData,
        };
        use solana_sdk::instruction::Instruction;

        // Use different mint for this test
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();
        let (ata_pubkey, correct_bump) =
            derive_ctoken_ata(&context.owner_keypair.pubkey(), &context.mint_pubkey);

        // Manually build instruction data with WRONG bump
        let wrong_bump = if correct_bump == 255 {
            254
        } else {
            correct_bump + 1
        };

        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            owner: context.owner_keypair.pubkey().into(),
            mint: context.mint_pubkey.into(),
            bump: wrong_bump, // Wrong bump!
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat as u8,
                rent_payment: 2,
                has_top_up: 1,
                write_top_up: 100,
                compress_to_account_pubkey: None,
            }),
        };

        let mut data = vec![100]; // CreateAssociatedTokenAccount discriminator
        instruction_data.serialize(&mut data).unwrap();

        let ix = Instruction {
            program_id: light_compressed_token::ID,
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(payer_pubkey, true),
                solana_sdk::instruction::AccountMeta::new(ata_pubkey, false),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    solana_sdk::pubkey::Pubkey::default(),
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    context.compressible_config,
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new(context.rent_sponsor, false),
            ],
            data,
        };

        let result = context
            .rpc
            .create_and_send_transaction(&[ix], &payer_pubkey, &[&context.payer])
            .await;

        // Wrong bump can trigger either ProgramFailedToComplete (21) or PrivilegeEscalation (19)
        // depending on runtime state - accept either
        let is_valid_error =
            light_program_test::utils::assert::assert_rpc_error(result.clone(), 0, 21).is_ok()
                || light_program_test::utils::assert::assert_rpc_error(result, 0, 19).is_ok();

        assert!(
            is_valid_error,
            "Expected either ProgramFailedToComplete (21) or PrivilegeEscalation (19)"
        );
    }

    // Test 6: Invalid config account owner
    // Compressible config must be owned by the compressed-token program
    // Error: 14 (InvalidAccountOwner)
    {
        use light_compressed_token_sdk::instructions::create_compressible_associated_token_account;

        // Use different mint for this test
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        // Use system program pubkey as fake config (wrong owner)
        let fake_config = solana_sdk::system_program::ID;

        let create_ata_ix = create_compressible_associated_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                owner: context.owner_keypair.pubkey(),
                mint: context.mint_pubkey,
                compressible_config: fake_config, // Wrong owner!
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(&[create_ata_ix], &payer_pubkey, &[&context.payer])
            .await;

        // Should fail with InvalidAccountOwner (20001)
        light_program_test::utils::assert::assert_rpc_error(result, 0, 20001).unwrap();
    }

    // Test 7: Wrong account type (correct program owner, wrong discriminator)
    // Passing an account owned by the registry program but not a CompressibleConfig.
    // Using the protocol config account which has a different discriminator.
    // Error: 20000 (InvalidDiscriminator from account-checks)
    {
        use light_compressed_token_sdk::instructions::create_compressible_associated_token_account;

        // Use different mint for this test
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        // Use protocol config account - owned by registry but wrong type
        let wrong_account_type = context.rpc.test_accounts.protocol.governance_authority_pda;

        let create_ata_ix = create_compressible_associated_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                owner: context.owner_keypair.pubkey(),
                mint: context.mint_pubkey,
                compressible_config: wrong_account_type, // Wrong account type
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();

        let result = context
            .rpc
            .create_and_send_transaction(&[create_ata_ix], &payer_pubkey, &[&context.payer])
            .await;

        // Should fail with InvalidDiscriminator (20000) from account-checks
        light_program_test::utils::assert::assert_rpc_error(result, 0, 20000).unwrap();
    }
}

#[tokio::test]
async fn test_ata_multiple_mints_same_owner() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let owner = context.owner_keypair.pubkey();

    // Create 3 different ATAs for the same owner with different mints
    let mint1 = solana_sdk::pubkey::Pubkey::new_unique();
    let mint2 = solana_sdk::pubkey::Pubkey::new_unique();
    let mint3 = solana_sdk::pubkey::Pubkey::new_unique();

    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 2,
        lamports_per_write: Some(100),
        account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };

    // Create ATA for mint1
    context.mint_pubkey = mint1;
    let ata1 = create_and_assert_ata(
        &mut context,
        Some(compressible_data.clone()),
        false,
        "ata_mint1",
    )
    .await;

    // Create ATA for mint2
    context.mint_pubkey = mint2;
    let ata2 = create_and_assert_ata(
        &mut context,
        Some(compressible_data.clone()),
        false,
        "ata_mint2",
    )
    .await;

    // Create ATA for mint3
    context.mint_pubkey = mint3;
    let ata3 = create_and_assert_ata(
        &mut context,
        Some(compressible_data.clone()),
        false,
        "ata_mint3",
    )
    .await;

    // Verify all three ATAs are different addresses
    assert_ne!(ata1, ata2, "ATA for mint1 and mint2 should be different");
    assert_ne!(ata1, ata3, "ATA for mint1 and mint3 should be different");
    assert_ne!(ata2, ata3, "ATA for mint2 and mint3 should be different");

    // Verify each ATA is derived correctly for its mint
    let (expected_ata1, _) = derive_ctoken_ata(&owner, &mint1);
    let (expected_ata2, _) = derive_ctoken_ata(&owner, &mint2);
    let (expected_ata3, _) = derive_ctoken_ata(&owner, &mint3);

    assert_eq!(ata1, expected_ata1, "ATA1 should match expected derivation");
    assert_eq!(ata2, expected_ata2, "ATA2 should match expected derivation");
    assert_eq!(ata3, expected_ata3, "ATA3 should match expected derivation");
}

#[tokio::test]
async fn test_ata_multiple_owners_same_mint() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Use the same mint for all ATAs
    let mint = solana_sdk::pubkey::Pubkey::new_unique();
    context.mint_pubkey = mint;

    // Create 3 different owners
    let owner1 = context.owner_keypair.pubkey();
    let owner2 = solana_sdk::pubkey::Pubkey::new_unique();
    let owner3 = solana_sdk::pubkey::Pubkey::new_unique();

    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 2,
        lamports_per_write: Some(100),
        account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };

    // Create ATAs for each owner with the same mint
    let create_ata_ix1 = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: owner1,
            mint,
            compressible_config: context.compressible_config,
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        },
    )
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ata_ix1], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    let (ata1, _) = derive_ctoken_ata(&owner1, &mint);

    // Assert ATA1 was created correctly
    assert_create_associated_token_account(
        &mut context.rpc,
        owner1,
        mint,
        Some(compressible_data.clone()),
    )
    .await;

    let create_ata_ix2 = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: owner2,
            mint,
            compressible_config: context.compressible_config,
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        },
    )
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ata_ix2], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    let (ata2, _) = derive_ctoken_ata(&owner2, &mint);

    // Assert ATA2 was created correctly
    assert_create_associated_token_account(
        &mut context.rpc,
        owner2,
        mint,
        Some(compressible_data.clone()),
    )
    .await;

    let create_ata_ix3 = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: owner3,
            mint,
            compressible_config: context.compressible_config,
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        },
    )
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ata_ix3], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    let (ata3, _) = derive_ctoken_ata(&owner3, &mint);

    // Assert ATA3 was created correctly
    assert_create_associated_token_account(
        &mut context.rpc,
        owner3,
        mint,
        Some(compressible_data.clone()),
    )
    .await;

    // Verify all three ATAs are different addresses (different owners, same mint)
    assert_ne!(ata1, ata2, "ATA for owner1 and owner2 should be different");
    assert_ne!(ata1, ata3, "ATA for owner1 and owner3 should be different");
    assert_ne!(ata2, ata3, "ATA for owner2 and owner3 should be different");

    // Verify each ATA is derived correctly for its owner
    let (expected_ata1, _) = derive_ctoken_ata(&owner1, &mint);
    let (expected_ata2, _) = derive_ctoken_ata(&owner2, &mint);
    let (expected_ata3, _) = derive_ctoken_ata(&owner3, &mint);

    assert_eq!(ata1, expected_ata1, "ATA1 should match expected derivation");
    assert_eq!(ata2, expected_ata2, "ATA2 should match expected derivation");
    assert_eq!(ata3, expected_ata3, "ATA3 should match expected derivation");
}

#[tokio::test]
async fn test_create_ata_random() {
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng, RngCore, SeedableRng,
    };
    // Setup randomness
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.next_u64();
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\nRandom Create ATA Test - Seed: {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    // Run 100 random test iterations
    for iteration in 0..100 {
        println!("\n--- Random ATA Test Iteration {} ---", iteration + 1);

        // Airdrop more lamports to payer periodically to avoid running out of funds
        if iteration % 100 == 0 {
            context
                .rpc
                .airdrop_lamports(&payer_pubkey, 10_000_000_000)
                .await
                .unwrap();
        }

        // Use different mint for each iteration
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority, // Config account forces this authority.
            rent_sponsor: if rng.gen_bool(0.5) {
                payer_pubkey
            } else {
                context.rent_sponsor
            },
            num_prepaid_epochs: {
                let value = rng.gen_range(0..=100);
                // Avoid 1 epoch which is forbidden
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
            compress_to_pubkey: false, // Cannot be used with ATAs
            payer: payer_pubkey,
        };

        create_and_assert_ata(
            &mut context,
            Some(compressible_data.clone()),
            false,
            format!(
                "\n--- Random ATA Test Iteration {} --- {:?}",
                iteration + 1,
                compressible_data
            )
            .as_str(),
        )
        .await;
    }
}
