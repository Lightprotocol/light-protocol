use light_client::rpc::Rpc;
use light_ctoken_types::state::ZExtensionStructMut;
use light_zero_copy::traits::ZeroCopyAtMut;
use solana_sdk::signer::Signer;

use super::shared::*;

// ============================================================================
// Owner-Initiated CompressAndClose Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_compress_and_close_owner_scenarios() {
    // Test 1: Owner closes account with token balance
    {
        let mut context = setup_compress_and_close_test(
            2,     // 2 prepaid epochs
            1000,  // 1000 token balance
            None,  // No time warp needed for owner
            false, // Use default rent sponsor
        )
        .await
        .unwrap();

        compress_and_close_owner_and_assert(
            &mut context,
            None, // Default destination (owner)
            "owner_with_balance",
        )
        .await;
    }

    // Test 2: Owner closes account with zero balance
    {
        let mut context = setup_compress_and_close_test(
            2,     // 2 prepaid epochs
            0,     // Zero token balance
            None,  // No time warp needed for owner
            false, // Use default rent sponsor
        )
        .await
        .unwrap();

        compress_and_close_owner_and_assert(
            &mut context,
            None, // Default destination (owner)
            "owner_zero_balance",
        )
        .await;
    }

    // Test 3: Owner closes regular 165-byte ctoken account (no compressible extension)
    {
        let mut context = setup_account_test().await.unwrap();

        // Create non-compressible token account
        create_non_compressible_token_account(&mut context, None).await;

        // Set token balance to 500
        let token_account_pubkey = context.token_account_keypair.pubkey();
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        use anchor_spl::token_2022::spl_token_2022;
        use solana_sdk::program_pack::Pack;

        let mut spl_token_account =
            spl_token_2022::state::Account::unpack_unchecked(&token_account.data[..165]).unwrap();
        spl_token_account.amount = 500;
        spl_token_2022::state::Account::pack(spl_token_account, &mut token_account.data[..165])
            .unwrap();
        context.rpc.set_account(token_account_pubkey, token_account);

        // Compress and close as owner
        compress_and_close_owner_and_assert(
            &mut context,
            None, // Default destination (owner)
            "owner_non_compressible",
        )
        .await;
    }

    // Test 4: Owner closes associated token account
    {
        let mut context = setup_account_test().await.unwrap();
        let payer_pubkey = context.payer.pubkey();

        // Create compressible ATA
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        let _ata_pubkey = create_and_assert_ata(
            &mut context,
            Some(compressible_data),
            false, // Non-idempotent
            "owner_ata",
        )
        .await;

        // Set token balance on ATA
        use light_compressed_token_sdk::instructions::create_associated_token_account::derive_ctoken_ata;
        let (ata_pubkey, _bump) =
            derive_ctoken_ata(&context.owner_keypair.pubkey(), &context.mint_pubkey);

        let mut ata_account = context.rpc.get_account(ata_pubkey).await.unwrap().unwrap();

        use anchor_spl::token_2022::spl_token_2022;
        use solana_sdk::program_pack::Pack;

        let mut spl_token_account =
            spl_token_2022::state::Account::unpack_unchecked(&ata_account.data[..165]).unwrap();
        spl_token_account.amount = 750;
        spl_token_2022::state::Account::pack(spl_token_account, &mut ata_account.data[..165])
            .unwrap();
        context.rpc.set_account(ata_pubkey, ata_account);

        // Create compress_and_close instruction manually for ATA
        use light_test_utils::assert_transfer2::assert_transfer2_compress_and_close;
        use light_token_client::instructions::transfer2::{
            create_generic_transfer2_instruction, CompressAndCloseInput, Transfer2InstructionType,
        };

        let output_queue = context
            .rpc
            .get_random_state_tree_info()
            .unwrap()
            .get_output_pubkey()
            .unwrap();

        let compress_and_close_ix = create_generic_transfer2_instruction(
            &mut context.rpc,
            vec![Transfer2InstructionType::CompressAndClose(
                CompressAndCloseInput {
                    solana_ctoken_account: ata_pubkey,
                    authority: context.owner_keypair.pubkey(),
                    output_queue,
                    destination: None,
                    is_compressible: true,
                },
            )],
            payer_pubkey,
            false,
        )
        .await
        .unwrap();

        context
            .rpc
            .create_and_send_transaction(
                &[compress_and_close_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        assert_transfer2_compress_and_close(
            &mut context.rpc,
            CompressAndCloseInput {
                solana_ctoken_account: ata_pubkey,
                authority: context.owner_keypair.pubkey(),
                output_queue,
                destination: None,
                is_compressible: true,
            },
        )
        .await;
    }
}

// ============================================================================
// Rent Authority-Initiated CompressAndClose Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_compress_and_close_rent_authority_scenarios() {
    // Test 5: Rent authority closes when is_compressible() returns true
    {
        let mut context = setup_compress_and_close_test(
            2,       // 2 prepaid epochs
            0,       // Zero balance
            Some(3), // Warp to epoch 3 (makes account compressible)
            false,   // Use default rent sponsor
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Top up rent for one more epoch (total: 2 prepaid + 1 topped up = 3 epochs)
        context
            .rpc
            .airdrop_lamports(
                &token_account_pubkey,
                RentConfig::default().get_rent(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, 1),
            )
            .await
            .unwrap();

        // Create destination for compression incentive
        let destination = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&destination.pubkey(), 1_000_000)
            .await
            .unwrap();

        // Get forester keypair
        let forester_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();

        // Compress and close using rent authority
        compress_and_close_forester(
            &mut context.rpc,
            &[token_account_pubkey],
            &forester_keypair,
            &context.payer,
            Some(destination.pubkey()),
        )
        .await
        .unwrap();

        // Assert compress and close succeeded
        use light_test_utils::assert_transfer2::assert_transfer2_compress_and_close;
        use light_token_client::instructions::transfer2::CompressAndCloseInput;

        let output_queue = context.rpc.get_random_state_tree_info().unwrap().queue;
        assert_transfer2_compress_and_close(
            &mut context.rpc,
            CompressAndCloseInput {
                solana_ctoken_account: token_account_pubkey,
                authority: context.compression_authority,
                output_queue,
                destination: Some(destination.pubkey()),
                is_compressible: true,
            },
        )
        .await;
    }

    // Test 6: Rent authority closes custom rent payer account
    {
        let mut context = setup_compress_and_close_test(
            2,       // 2 prepaid epochs
            0,       // Zero balance
            Some(2), // Warp to epoch 2 (makes account compressible)
            true,    // Use payer as rent sponsor (custom payer)
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Get forester keypair
        let forester_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();

        // Create destination for compression incentive
        let destination = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&destination.pubkey(), 1_000_000)
            .await
            .unwrap();

        // Compress and close using rent authority
        compress_and_close_forester(
            &mut context.rpc,
            &[token_account_pubkey],
            &forester_keypair,
            &context.payer,
            Some(destination.pubkey()),
        )
        .await
        .unwrap();

        // Assert compress and close succeeded
        use light_test_utils::assert_transfer2::assert_transfer2_compress_and_close;
        use light_token_client::instructions::transfer2::CompressAndCloseInput;

        let output_queue = context.rpc.get_random_state_tree_info().unwrap().queue;
        assert_transfer2_compress_and_close(
            &mut context.rpc,
            CompressAndCloseInput {
                solana_ctoken_account: token_account_pubkey,
                authority: context.compression_authority,
                output_queue,
                destination: Some(destination.pubkey()),
                is_compressible: true,
            },
        )
        .await;
    }

    // Test 7: Rent authority closes exactly when account becomes compressible (at epoch boundary)
    {
        let mut context = setup_compress_and_close_test(
            2,     // 2 prepaid epochs
            0,     // Zero balance
            None,  // Don't warp yet
            false, // Use default rent sponsor
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Warp to exactly epoch 2 (first slot of epoch 2)
        // Account created with 2 prepaid epochs
        // At epoch 2, both epochs have passed, account is now compressible
        context.rpc.warp_to_slot(SLOTS_PER_EPOCH * 2).unwrap();

        // Get forester keypair
        let forester_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();

        // Create destination for compression incentive
        let destination = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&destination.pubkey(), 1_000_000)
            .await
            .unwrap();

        // Compress and close using rent authority at exact epoch boundary
        compress_and_close_forester(
            &mut context.rpc,
            &[token_account_pubkey],
            &forester_keypair,
            &context.payer,
            Some(destination.pubkey()),
        )
        .await
        .unwrap();

        // Assert compress and close succeeded
        use light_test_utils::assert_transfer2::assert_transfer2_compress_and_close;
        use light_token_client::instructions::transfer2::CompressAndCloseInput;

        let output_queue = context.rpc.get_random_state_tree_info().unwrap().queue;
        assert_transfer2_compress_and_close(
            &mut context.rpc,
            CompressAndCloseInput {
                solana_ctoken_account: token_account_pubkey,
                authority: context.compression_authority,
                output_queue,
                destination: Some(destination.pubkey()),
                is_compressible: true,
            },
        )
        .await;
    }
}

#[tokio::test]
#[serial]
async fn test_compress_and_close_compress_to_pubkey() {
    // Test 9: compress_to_pubkey=true, account pubkey becomes owner in compressed output (PDA use case)
    {
        let mut context = setup_compress_and_close_test(
            2,     // 2 prepaid epochs
            500,   // 500 token balance
            None,  // No time warp needed for owner
            false, // Use default rent sponsor
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Manually set compress_to_pubkey=true in the compressible extension using set_account
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        use light_ctoken_types::state::ctoken::CToken;

        // Parse the CToken account
        let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut token_account.data)
            .expect("Failed to deserialize ctoken account");

        // Modify compress_to_pubkey in the compressible extension
        if let Some(extensions) = ctoken.extensions.as_mut() {
            for ext in extensions.iter_mut() {
                if let ZExtensionStructMut::Compressible(ref mut comp) = ext {
                    comp.compress_to_pubkey = 1;
                    break;
                }
            }
        }

        // Write the modified account back
        context.rpc.set_account(token_account_pubkey, token_account);

        // Execute compress_and_close using helper
        compress_and_close_owner_and_assert(
            &mut context,
            None, // Default destination (owner)
            "compress_to_pubkey_true",
        )
        .await;
    }
}

#[tokio::test]
#[serial]
async fn test_compressible_account_with_custom_rent_payer_close_with_compression_authority() {
    let mut context = setup_account_test().await.unwrap();
    let first_tx_payer = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&first_tx_payer.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let payer_pubkey = first_tx_payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Create system account with compressible size
    let rent_exemption = context
        .rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let num_prepaid_epochs = 2;
    let lamports_per_write = Some(100);

    // Initialize compressible token account
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: payer_pubkey,
                pre_pay_num_epochs: num_prepaid_epochs,
                lamports_per_write,
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();
    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_before = context
        .rpc
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .expect("Pool PDA should exist")
        .lamports;

    // Execute account creation
    context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&first_tx_payer, &context.token_account_keypair],
        )
        .await
        .unwrap();

    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        Some(CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: payer_pubkey,
            num_prepaid_epochs,
            lamports_per_write,
            compress_to_pubkey: false,
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            payer: payer_pubkey,
        }),
    )
    .await;

    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)

    // Verify payer balance decreased by exactly 11,000 lamports (the additional rent)
    let payer_balance_after = context
        .rpc
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .expect("Payer should exist")
        .lamports;
    let rent = RentConfig::default()
        .get_rent_with_compression_cost(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, num_prepaid_epochs as u64);
    let tx_fee = 10_000; // Standard transaction fee
    assert_eq!(
        pool_balance_before - payer_balance_after,
        rent_exemption + rent + tx_fee,
        "Pool PDA should have paid {} lamports for account creation (rent-exempt), and the additional rent",
        rent_exemption
    );
    // Close and compress account with rent authority
    {
        let payer_balance_before = context
            .rpc
            .get_account(payer_pubkey)
            .await
            .unwrap()
            .expect("Payer should exist")
            .lamports;
        context.rpc.warp_epoch_forward(2).await.unwrap();
        let payer_balance_after = context
            .rpc
            .get_account(payer_pubkey)
            .await
            .unwrap()
            .expect("Payer should exist")
            .lamports;
        let rent = RentConfig::default()
            .get_rent(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, num_prepaid_epochs as u64);
        assert_eq!(
            payer_balance_after,
            payer_balance_before + rent_exemption + rent,
            "Pool PDA should have paid {} lamports for account creation (rent-exempt), and the additional rent",
            rent_exemption
        );
        use light_client::indexer::Indexer;
        let compressed_token_account = context
            .rpc
            .get_compressed_token_accounts_by_owner(&context.owner_keypair.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;
        assert_eq!(compressed_token_account.len(), 1);
    }
}

// ============================================================================
// Failure Tests - Authority Validation Errors
// ============================================================================

#[tokio::test]
#[serial]
async fn test_compress_and_close_authority_errors() {
    // Test 1: Wrong authority (neither owner nor rent authority) - error 3 InvalidAccountData
    {
        let mut context = setup_compress_and_close_test(
            2,     // 2 prepaid epochs
            500,   // 500 token balance
            None,  // No time warp
            false, // Use default rent sponsor
        )
        .await
        .unwrap();

        // Create a random wrong authority
        let wrong_authority = Keypair::new();

        // Try to compress and close with wrong authority (should fail)
        // Returns ProgramError::InvalidAccountData (error code 3) - "rent authority mismatch"
        compress_and_close_and_assert_fails(
            &mut context,
            &wrong_authority,
            None, // Default destination
            "wrong_authority",
            3, // ProgramError::InvalidAccountData
        )
        .await;
    }

    // Test 2: Delegate tries to close - error 3 InvalidAccountData
    {
        let mut context = setup_compress_and_close_test(
            2,     // 2 prepaid epochs
            500,   // 500 token balance
            None,  // No time warp
            false, // Use default rent sponsor
        )
        .await
        .unwrap();

        // Create a delegate and approve some amount
        let delegate = Keypair::new();
        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Set delegate on the token account using set_account
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        use anchor_spl::token_2022::spl_token_2022;
        use solana_sdk::program_pack::Pack;

        let mut spl_token_account =
            spl_token_2022::state::Account::unpack_unchecked(&token_account.data[..165]).unwrap();
        spl_token_account.delegate = Some(delegate.pubkey()).into();
        spl_token_account.delegated_amount = 500;
        spl_token_2022::state::Account::pack(spl_token_account, &mut token_account.data[..165])
            .unwrap();
        context.rpc.set_account(token_account_pubkey, token_account);

        // Try to compress and close with delegate authority (should fail)
        // Returns ProgramError::InvalidAccountData (error code 3) - "rent authority mismatch"
        // Delegate is neither owner nor rent authority
        compress_and_close_and_assert_fails(
            &mut context,
            &delegate,
            None, // Default destination
            "delegate_authority",
            3, // ProgramError::InvalidAccountData
        )
        .await;
    }
}

// ============================================================================
// Failure Tests - Output Validation Errors (Rent Authority Only)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_compress_and_close_output_validation_errors() {
    // Note: These validation errors occur when the rent authority tries to close an account
    // but the compressed output doesn't match expected values.
    // These checks are NOT performed when the owner closes the account.

    // Test 5: Owner mismatch - compressed output owner is wrong
    // The rent authority is trying to close the account, but the compressed output
    // specifies the wrong owner pubkey
    {
        let mut context = setup_compress_and_close_test(
            2,       // 2 prepaid epochs
            500,     // 500 token balance
            Some(2), // Warp to epoch 2 (makes account compressible)
            false,   // Use default rent sponsor
        )
        .await
        .unwrap();

        let wrong_owner = Keypair::new();

        // Try to compress and close with wrong owner in output
        // This simulates a malicious forester trying to steal tokens by changing the owner
        compress_and_close_forester_with_invalid_output(
            &mut context,
            CompressAndCloseValidationError::OwnerMismatch(wrong_owner.pubkey()),
            None, // Default destination
            89,   // CompressAndCloseInvalidOwner
        )
        .await;
    }

    // Test 6: Owner mismatch when compress_to_pubkey=true (forester as signer)
    // When compress_to_pubkey=true, the compressed output owner must be the account pubkey
    // This test verifies that using the original owner fails even when the forester tries
    {
        let mut context = setup_compress_and_close_test(
            2,       // 2 prepaid epochs
            500,     // 500 token balance
            Some(2), // Warp to epoch 2 (makes account compressible)
            false,   // Use default rent sponsor
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();
        let owner_pubkey = context.owner_keypair.pubkey();

        // Set compress_to_pubkey=true in the compressible extension
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        use light_ctoken_types::state::ctoken::CToken;

        // Parse and modify the CToken account
        let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut token_account.data)
            .expect("Failed to deserialize ctoken account");

        // Set compress_to_pubkey=true in the compressible extension
        if let Some(extensions) = ctoken.extensions.as_mut() {
            for ext in extensions.iter_mut() {
                if let ZExtensionStructMut::Compressible(ref mut comp) = ext {
                    comp.compress_to_pubkey = 1;
                    break;
                }
            }
        }

        // Write the modified account back
        context.rpc.set_account(token_account_pubkey, token_account);

        // Try to compress and close with original owner (should fail)
        // When compress_to_pubkey=true, the owner should be token_account_pubkey, not owner_pubkey
        compress_and_close_forester_with_invalid_output(
            &mut context,
            CompressAndCloseValidationError::OwnerNotAccountPubkey(owner_pubkey),
            None, // Default destination
            89,   // CompressAndCloseInvalidOwner
        )
        .await;
    }

    // Test 8: Token account has delegate - should fail when forester tries to close
    // The validation checks that delegate must be None in compressed output
    // Since compressed token doesn't support delegation, any account with a delegate should fail
    {
        let mut context = setup_compress_and_close_test(
            2,       // 2 prepaid epochs
            500,     // 500 token balance
            Some(2), // Warp to epoch 2 (makes account compressible)
            false,   // Use default rent sponsor
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Set delegate on the token account using set_account
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        use anchor_spl::token_2022::spl_token_2022;
        use solana_sdk::program_pack::Pack;

        let mut spl_token_account =
            spl_token_2022::state::Account::unpack_unchecked(&token_account.data[..165]).unwrap();

        // Set a delegate with delegated amount
        let delegate = Keypair::new();
        spl_token_account.delegate = Some(delegate.pubkey()).into();
        spl_token_account.delegated_amount = 500;

        spl_token_2022::state::Account::pack(spl_token_account, &mut token_account.data[..165])
            .unwrap();
        context.rpc.set_account(token_account_pubkey, token_account);

        // Get forester keypair and setup for compress_and_close
        let forester_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();

        // Create destination for compression incentive
        let destination = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&destination.pubkey(), 1_000_000)
            .await
            .unwrap();

        // Try to compress and close via forester (should fail because delegate is present)
        // Error: CompressAndCloseDelegateNotAllowed (92 = 0x5c)
        let result = compress_and_close_forester(
            &mut context.rpc,
            &[token_account_pubkey],
            &forester_keypair,
            &context.payer,
            Some(destination.pubkey()),
        )
        .await;

        // Assert that the transaction failed with delegate not allowed error
        light_program_test::utils::assert::assert_rpc_error(result, 0, 92).unwrap();
    }

    // Test 9: Frozen account cannot be closed
    // The validation checks that account state must be Initialized, not Frozen
    {
        let mut context = setup_compress_and_close_test(
            2,       // 2 prepaid epochs
            500,     // 500 token balance
            Some(2), // Warp to epoch 2 (makes account compressible)
            false,   // Use default rent sponsor
        )
        .await
        .unwrap();

        let token_account_pubkey = context.token_account_keypair.pubkey();

        // Set account state to Frozen using set_account
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        use anchor_spl::token_2022::spl_token_2022;
        use solana_sdk::program_pack::Pack;

        let mut spl_token_account =
            spl_token_2022::state::Account::unpack_unchecked(&token_account.data[..165]).unwrap();

        // Set account state to Frozen
        spl_token_account.state = spl_token_2022::state::AccountState::Frozen;

        spl_token_2022::state::Account::pack(spl_token_account, &mut token_account.data[..165])
            .unwrap();
        context.rpc.set_account(token_account_pubkey, token_account);

        // Get forester keypair and setup for compress_and_close
        let forester_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();

        // Create destination for compression incentive
        let destination = Keypair::new();
        context
            .rpc
            .airdrop_lamports(&destination.pubkey(), 1_000_000)
            .await
            .unwrap();

        // Try to compress and close via forester (should fail because account is frozen)
        // Error: AccountFrozen
        let result = compress_and_close_forester(
            &mut context.rpc,
            &[token_account_pubkey],
            &forester_keypair,
            &context.payer,
            Some(destination.pubkey()),
        )
        .await;

        // Assert that the transaction failed with account frozen error
        // Error: InvalidAccountState (18036)
        light_program_test::utils::assert::assert_rpc_error(result, 0, 18036).unwrap();
    }
}
