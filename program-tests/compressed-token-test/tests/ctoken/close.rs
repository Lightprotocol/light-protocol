use super::shared::*;

#[tokio::test]
#[serial]
async fn test_close_compressible_token_account() {
    // Test 1: Close non-compressible account (owner authority)
    // Non-compressible accounts are 165 bytes and have no compressible extension.
    // All lamports go to destination.
    {
        let mut context = setup_account_test_with_created_account(None).await.unwrap();
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account(&mut context, destination, "non_compressible_account").await;
    }

    // Test 2: Close compressible account with zero epochs (owner authority)
    // Compressible account with 0 prepaid epochs is immediately compressible.
    // Rent exemption goes to rent_sponsor, unutilized funds to destination.
    {
        let mut context = setup_account_test_with_created_account(Some((0, false)))
            .await
            .unwrap();
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account(&mut context, destination, "compressible_zero_epochs").await;
    }

    // Test 3: Close compressible account with multiple epochs (owner authority)
    // Compressible account with 10 prepaid epochs.
    // Rent exemption goes to rent_sponsor, unutilized funds to destination.
    {
        let mut context = setup_account_test_with_created_account(Some((10, false)))
            .await
            .unwrap();
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account(&mut context, destination, "compressible_multiple_epochs")
            .await;
    }

    // Test 4: Close compressible account with payer as rent_sponsor (owner authority)
    // Payer pays for everything and receives rent back on close.
    {
        let mut context = setup_account_test_with_created_account(Some((2, true)))
            .await
            .unwrap();
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account(
            &mut context,
            destination,
            "compressible_payer_as_rent_sponsor",
        )
        .await;
    }
}

#[tokio::test]
#[serial]
async fn test_close_token_account_fails() {
    let mut context = setup_account_test_with_created_account(Some((2, false)))
        .await
        .unwrap();
    let rent_sponsor = context.rent_sponsor;
    let token_account_pubkey = context.token_account_keypair.pubkey();
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Test 5: Close with wrong owner → Error 75 (OwnerMismatch)
    {
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        let wrong_owner = Keypair::new();

        close_and_assert_token_account_fails(
            &mut context,
            destination,
            &wrong_owner,
            Some(rent_sponsor),
            "wrong_owner",
            75, // ErrorCode::OwnerMismatch
        )
        .await;
    }

    // Test 6: Close with destination == token_account → Error 4 (InvalidAccountData)
    {
        close_and_assert_token_account_fails(
            &mut context,
            token_account_pubkey, // destination same as token_account
            &owner_keypair,
            Some(rent_sponsor),
            "destination_same_as_token_account",
            3, // ProgramError::InvalidAccountData
        )
        .await;
    }

    // Test 7: Missing rent_sponsor for compressible account → Error 11 (NotEnoughAccountKeys)
    {
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account_fails(
            &mut context,
            destination,
            &owner_keypair,
            None, // Missing rent_sponsor
            "missing_rent_sponsor",
            11, // ProgramError::NotEnoughAccountKeys
        )
        .await;
    }

    // Test 8: Wrong rent_sponsor → Error 4 (InvalidAccountData)
    {
        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        let wrong_rent_sponsor = Keypair::new().pubkey();

        close_and_assert_token_account_fails(
            &mut context,
            destination,
            &owner_keypair,
            Some(wrong_rent_sponsor), // Wrong rent_sponsor
            "wrong_rent_sponsor",
            3, // ProgramError::InvalidAccountData
        )
        .await;
    }

    // Test 9: Non-zero balance → Error 6074 (NonNativeHasBalance)
    {
        // Create a fresh account for this test
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: context.payer.pubkey(),
        };
        create_and_assert_token_account(&mut context, compressible_data, "non_zero_balance_test")
            .await;

        // Get account, modify balance to 1, set account back
        let token_account_pubkey = context.token_account_keypair.pubkey();
        let mut account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        // Deserialize, modify amount, serialize back
        use light_ctoken_types::state::ctoken::CToken;
        use light_zero_copy::traits::ZeroCopyAtMut;
        let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut account.data).unwrap();
        *ctoken.amount = 1u64.into();
        drop(ctoken);

        // Set the modified account back
        context.rpc.set_account(token_account_pubkey, account);

        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account_fails(
            &mut context,
            destination,
            &owner_keypair,
            Some(rent_sponsor),
            "non_zero_balance",
            74, // ErrorCode::NonNativeHasBalance
        )
        .await;
    }

    // Test 10: Uninitialized account → Error 10 (UninitializedAccount)
    {
        // Create a fresh account for this test
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: context.payer.pubkey(),
        };
        create_and_assert_token_account(&mut context, compressible_data, "uninitialized_test")
            .await;

        // Get account, set state to Uninitialized (0), set account back
        let token_account_pubkey = context.token_account_keypair.pubkey();
        let mut account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        // Deserialize, modify state to Uninitialized, serialize back
        use light_ctoken_types::state::ctoken::CToken;
        use light_zero_copy::traits::ZeroCopyAtMut;
        use spl_token_2022::state::AccountState;
        let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut account.data).unwrap();
        *ctoken.state = AccountState::Uninitialized as u8;
        drop(ctoken);

        // Set the modified account back
        context.rpc.set_account(token_account_pubkey, account);

        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account_fails(
            &mut context,
            destination,
            &owner_keypair,
            Some(rent_sponsor),
            "uninitialized_account",
            18036, // CTokenError::InvalidAccountState
        )
        .await;
    }

    // Test 11: Frozen account → Error 6076 (AccountFrozen)
    {
        // Create a fresh account for this test
        context.token_account_keypair = Keypair::new();
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: context.payer.pubkey(),
        };
        create_and_assert_token_account(&mut context, compressible_data, "frozen_test").await;

        // Get account, set state to Frozen (2), set account back
        let token_account_pubkey = context.token_account_keypair.pubkey();
        let mut account = context
            .rpc
            .get_account(token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        // Deserialize, modify state to Frozen, serialize back
        use light_ctoken_types::state::ctoken::CToken;
        use light_zero_copy::traits::ZeroCopyAtMut;
        use spl_token_2022::state::AccountState;
        let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut account.data).unwrap();
        *ctoken.state = AccountState::Frozen as u8;
        drop(ctoken);

        // Set the modified account back
        context.rpc.set_account(token_account_pubkey, account);

        let destination = Keypair::new().pubkey();
        context
            .rpc
            .airdrop_lamports(&destination, 1_000_000)
            .await
            .unwrap();

        close_and_assert_token_account_fails(
            &mut context,
            destination,
            &owner_keypair,
            Some(rent_sponsor),
            "frozen_account",
            18036, // CTokenError::InvalidAccountState
        )
        .await;
    }
}
