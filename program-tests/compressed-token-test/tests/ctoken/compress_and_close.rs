use super::shared::*;
use light_test_utils::spl::create_mint_helper;

// ============================================================================
// Owner-Initiated CompressAndClose Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_compress_and_close_owner_scenarios() {
    // Test 1: Owner closes account with token balance
    {
        let mut context = setup_compress_and_close_test(
            2,      // 2 prepaid epochs
            1000,   // 1000 token balance
            None,   // No time warp needed for owner
            false,  // Use default rent sponsor
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

        // Update context to point to ATA
        context.token_account_keypair = Keypair::new();
        // We need to create a dummy keypair, but the actual pubkey doesn't matter
        // because compress_and_close_owner_and_assert uses context.token_account_keypair.pubkey()
        // We need to set it to the ATA pubkey by creating a keypair wrapper
        // Actually, we need to modify the context differently - let me use the direct approach

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


// Functional Tests (Successful Operations)

// Owner-Initiated CompressAndClose:
// 1. test_compress_and_close_owner_with_balance - Owner closes account with token balance
// 2. test_compress_and_close_owner_zero_balance - Owner closes account with zero balance
// 3. test_compress_and_close_owner_non_compressible - Owner closes regular 165-byte ctoken account (no compressible extension)
// 4. test_compress_and_close_owner_ata - Owner closes associated token account

// Rent Authority-Initiated CompressAndClose:
// 5. test_compress_and_close_rent_authority_when_compressible - Rent authority closes when is_compressible() returns true (already exists as
// test_compress_and_close_with_compression_authority)
// 6. test_compress_and_close_rent_authority_custom_payer - Rent authority closes custom rent payer account (already exists)
// 7. test_compress_and_close_rent_authority_at_epoch_boundary - Rent authority closes exactly when account becomes compressible

// compress_to_pubkey Flag Tests:
// 8. test_compress_and_close_preserve_owner - compress_to_pubkey=false, owner preserved in compressed output
// 9. test_compress_and_close_to_pubkey - compress_to_pubkey=true, account pubkey becomes owner in compressed output (PDA use case)

// Lamport Distribution Tests:
// 10. test_compress_and_close_lamport_distribution - Verify rent exemption + completed epochs → rent_sponsor, unutilized → destination, incentive → forester
// 11. test_compress_and_close_lamport_distribution_custom_payer - Same but with custom rent payer

// Multiple Operations:
// 12. test_compress_and_close_multiple_accounts - Multiple CompressAndClose operations in single transaction
// 13. test_compress_and_close_with_other_compressions - CompressAndClose mixed with regular compress/decompress in same tx

// ---
// Failing Tests (Error Cases)

// Authority Validation Errors:
// 1. test_compress_and_close_missing_authority - No authority provided (error 6088 - CompressAndCloseAuthorityMissing)
// 2. test_compress_and_close_non_signer_authority - Authority not signing (error 20009 - InvalidSigner)
// 3. test_compress_and_close_wrong_authority - Authority is neither owner nor rent authority (error 6075 - OwnerMismatch)
// 4. test_compress_and_close_delegate_authority - Delegate tries to close (error 6092 - CompressAndCloseDelegateNotAllowed)

// Compressed Output Validation Errors (Rent Authority Only):
// 5. test_compress_and_close_amount_mismatch - Compressed output amount != full balance (error 6090 - CompressAndCloseAmountMismatch)
// 6. test_compress_and_close_balance_mismatch - Token balance != compressed output amount (error: CompressAndCloseBalanceMismatch)
// 7. test_compress_and_close_owner_mismatch_normal - Owner mismatch when compress_to_pubkey=false (error: CompressAndCloseInvalidOwner)
// 8. test_compress_and_close_owner_mismatch_to_pubkey - Owner != account pubkey when compress_to_pubkey=true (error: CompressAndCloseInvalidOwner)
// 9. test_compress_and_close_delegate_in_output - Delegate present in compressed output (error 6092 - CompressAndCloseDelegateNotAllowed)
// 10. test_compress_and_close_wrong_version - Wrong version in compressed output (error: CompressAndCloseInvalidVersion)
// 11. test_compress_and_close_version_mismatch - Version mismatch between output and compressible extension (error: CompressAndCloseInvalidVersion)

// Compressibility State Errors:
// 12. test_compress_and_close_not_compressible - Rent authority tries to close before account is compressible (should fail with validation error)

// Missing Accounts:
// 13. test_compress_and_close_missing_destination - No destination account provided (error 6087 - CompressAndCloseDestinationMissing)
// 14. test_compress_and_close_missing_compressed_output - Rent authority closes but no compressed output exists (error: validation fails)

/// Test compress_and_close with rent authority:
/// 1. Create compressible token account with rent authority
/// 2. Compress and close account using rent authority
/// 3. Verify rent goes to rent recipient
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_compression_authority() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let mint_pubkey = create_mint_helper(&mut context.rpc, &context.payer).await;

    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(150),
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await
        .unwrap();

    // Top up rent for one more epoch (total: 2 prepaid + 1 topped up = 3 epochs)
    context
        .rpc
        .airdrop_lamports(
            &token_account_pubkey,
            RentConfig::default().get_rent(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, 1),
        )
        .await
        .unwrap();

    // Advance to epoch 1 (account not yet compressible - still has 2 epochs remaining)
    // Account was created with 2 epochs prepaid + 1 topped up = 3 epochs total
    // At epoch 1, only 1 epoch has passed, so 2 epochs of funding remain
    context.rpc.warp_to_slot(SLOTS_PER_EPOCH + 1).unwrap();
    let forster_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();
    // This doesnt work anymore we need to invoke the registry program now
    // // Compress and close using rent authority (with 0 balance)
    let result = compress_and_close_forester(
        &mut context.rpc,
        &[token_account_pubkey],
        &forster_keypair,
        &context.payer,
        None,
    )
    .await;

    assert!(
        result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("invalid account data for instruction"),
        "{}",
        result.unwrap_err().to_string()
    );
    // Advance to epoch 3 to make the account compressible
    // Account was created with 2 epochs prepaid + 1 topped up = 3 epochs total
    // At epoch 3, all 3 epochs have passed, so the account is now compressible
    context.rpc.warp_to_slot((SLOTS_PER_EPOCH * 3) + 1).unwrap();

    // Create a fresh destination pubkey to receive the compression incentive
    let destination = solana_sdk::signature::Keypair::new();
    println!("Test destination pubkey: {:?}", destination.pubkey());

    // Airdrop lamports to destination so it exists and can receive the compression incentive
    context
        .rpc
        .airdrop_lamports(&destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    compress_and_close_forester(
        &mut context.rpc,
        &[token_account_pubkey],
        &forster_keypair,
        &context.payer,
        Some(destination.pubkey()),
    )
    .await
    .unwrap();
    // Use the new assert_transfer2_compress_and_close for comprehensive validation
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
        .get_rent_with_compression_cost(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, num_prepaid_epochs);
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
        let rent =
            RentConfig::default().get_rent(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, num_prepaid_epochs);
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
