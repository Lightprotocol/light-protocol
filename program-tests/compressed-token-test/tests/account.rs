// #![cfg(feature = "test-sbf")]

use std::assert_eq;

use anchor_spl::token_2022::spl_token_2022;
use light_compressed_token_sdk::instructions::{close::close_account, create_token_account};
use light_ctoken_types::{
    state::{solana_ctoken::CompressedToken, CompressibleExtension},
    BASIC_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::Rpc;

use light_zero_copy::borsh::Deserialize;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::{pod::PodAccount, state::AccountState};

#[tokio::test]
async fn test_create_and_close_token_account() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey (we don't need actual mint for this test)
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create a new keypair for the token account
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // First create the account using system program
    let create_account_system_ix = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rpc.get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap(), // SPL token account size
        165,
        &light_compressed_token::ID, // Our program owns the account
    );

    // Then use SPL token SDK format but with our compressed token program ID
    // This tests that our create_token_account instruction is compatible with SPL SDKs
    let mut initialize_account_ix =
        create_token_account(token_account_pubkey, mint_pubkey, owner_pubkey).unwrap();
    initialize_account_ix.data.push(0);
    // Execute both instructions in one transaction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_system_ix, initialize_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    rpc.process_transaction(transaction.clone())
        .await
        .expect("Failed to create token account using SPL SDK");

    // Verify the token account was created correctly
    let account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Verify account exists and has correct owner
    assert_eq!(account_info.owner, light_compressed_token::ID);
    assert_eq!(account_info.data.len(), 165); // SPL token account size

    let pod_account = pod_from_bytes::<PodAccount>(&account_info.data)
        .expect("Failed to parse token account data");

    // Verify the token account fields
    assert_eq!(pod_account.mint, mint_pubkey);
    assert_eq!(pod_account.owner, owner_pubkey);
    assert_eq!(u64::from(pod_account.amount), 0); // Should start with zero balance
    assert_eq!(pod_account.state, AccountState::Initialized as u8);

    // Now test closing the account using SPL SDK format
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Airdrop some lamports to destination account so it exists
    rpc.context.airdrop(&destination_pubkey, 1_000_000).unwrap();

    // Get initial lamports before closing
    let initial_token_account_lamports = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Create close account instruction using SPL SDK format
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &destination_pubkey,
        &owner_pubkey,
    );

    // Execute the close instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &owner_keypair], // Need owner to sign
        blockhash,
    );

    rpc.process_transaction(close_transaction)
        .await
        .expect("Failed to close token account using SPL SDK");

    // Verify the account was closed (data should be cleared, lamports should be 0)
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(account) = closed_account {
        // Account still exists, but should have 0 lamports and cleared data
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
        assert!(
            account.data.iter().all(|&b| b == 0),
            "Closed account data should be cleared"
        );
    }

    // Verify lamports were transferred to destination
    let final_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_destination_lamports,
        initial_destination_lamports + initial_token_account_lamports,
        "Destination should receive all lamports from closed account"
    );
}

#[tokio::test]
async fn test_create_and_close_account_with_rent_authority() {
    use solana_sdk::{signature::Signer, system_instruction};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create mint
    let mint_pubkey = Pubkey::new_unique();

    // Create account owner
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create rent authority
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();

    // Create rent recipient
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Airdrop lamports to rent recipient so it exists
    rpc.context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create token account keypair
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // Create system account for token account with space for compressible extension
    let rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exempt_lamports,
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
        &light_compressed_token::ID,
    );

    // Create token account using SDK function with compressible extension
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey,
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0, // Allow immediate compression
            },
        )
        .unwrap();

    // Execute account creation
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let create_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_ix, create_token_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    rpc.process_transaction(create_transaction)
        .await
        .expect("Failed to create token account");

    // Verify the account was created correctly
    let token_account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Assert complete token account values
    assert_eq!(token_account_info.owner, light_compressed_token::ID);
    assert_eq!(
        token_account_info.data.len(),
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    );
    assert!(!token_account_info.executable);
    assert!(token_account_info.lamports > 0); // Should be rent-exempt

    let expected_token_account = CompressedToken {
        mint: mint_pubkey.into(),
        owner: owner_pubkey.into(),
        amount: 0,
        delegate: None,
        state: 1, // Initialized
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            light_ctoken_types::state::extensions::ExtensionStruct::Compressible(
                CompressibleExtension {
                    last_written_slot: 2, // Program sets this to current slot (2 in test environment)
                    slots_until_compression: 0,
                    rent_authority: rent_authority_pubkey.into(),
                    rent_recipient: rent_recipient_pubkey.into(),
                },
            ),
        ]),
    };

    let (actual_token_account, _) = CompressedToken::zero_copy_at(&token_account_info.data)
        .expect("Failed to deserialize token account with zero-copy");

    assert_eq!(actual_token_account, expected_token_account);

    // Get initial lamports before closing
    let initial_token_account_lamports = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // First, try to close with rent authority (should fail for basic token account)
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &rent_recipient_pubkey, // Use rent recipient as destination
        &rent_authority_pubkey, // Use rent authority as authority
    );

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &rent_authority_keypair], // Sign with rent authority, not owner
        blockhash,
    );

    rpc.process_transaction(close_transaction).await.unwrap();

    // Verify the account was closed (should have 0 lamports and cleared data)
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(account) = closed_account {
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
        assert!(
            account.data.iter().all(|&b| b == 0),
            "Closed account data should be cleared"
        );
    }

    // Verify lamports were transferred to rent recipient
    let final_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_recipient_lamports,
        initial_recipient_lamports + initial_token_account_lamports,
        "Rent recipient should receive all lamports from closed account"
    );
}

#[tokio::test]
async fn test_create_compressible_account_insufficient_size() {
    use light_test_utils::spl::create_mint_helper;
    use solana_sdk::{signature::Signer, system_instruction};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create mint
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

    // Create owner and rent authority keypairs
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Create token account keypair
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // Create system account with INSUFFICIENT size - too small for compressible extension
    let rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(BASIC_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exempt_lamports,
        light_ctoken_types::BASIC_TOKEN_ACCOUNT_SIZE, // Intentionally too small for compressible extension
        &light_compressed_token::ID,
    );

    // Create token account using SDK function with compressible extension
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey,
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0,
            },
        )
        .unwrap();

    // Execute account creation - this should fail with account size error
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let create_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_ix, create_token_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    let result = rpc.process_transaction(create_transaction).await;
    assert!(
        result.is_err(),
        "Expected account creation to fail due to insufficient account size"
    );

    println!("✅ Correctly failed to create compressible token account with insufficient size");
}

#[tokio::test]
async fn test_create_associated_token_account() {
    use spl_pod::bytemuck::pod_from_bytes;
    use spl_token_2022::{pod::PodAccount, state::AccountState};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the associated token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Calculate the expected associated token account address
    let (expected_ata_pubkey, bump) = Pubkey::find_program_address(
        &[
            owner_pubkey.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint_pubkey.as_ref(),
        ],
        &light_compressed_token::ID,
    );

    // Create basic ATA instruction using SDK function
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        mint_pubkey,
    )
    .unwrap();

    // Execute the instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );

    rpc.process_transaction(transaction.clone())
        .await
        .expect("Failed to create associated token account");

    // Verify the associated token account was created correctly
    let token_account_info = rpc.get_account(expected_ata_pubkey).await.unwrap().unwrap();
    {
        // Verify account exists and has correct owner
        assert_eq!(token_account_info.owner, light_compressed_token::ID);
        assert_eq!(token_account_info.data.len(), 165); // SPL token account size

        let pod_account = pod_from_bytes::<PodAccount>(&token_account_info.data)
            .expect("Failed to parse token account data");

        // Verify the token account fields
        assert_eq!(pod_account.mint, mint_pubkey);
        assert_eq!(pod_account.owner, owner_pubkey);
        assert_eq!(u64::from(pod_account.amount), 0); // Should start with zero balance
        assert_eq!(pod_account.state, AccountState::Initialized as u8);

        // Verify the PDA derivation is correct
        let (derived_ata_pubkey, derived_bump) = Pubkey::find_program_address(
            &[
                owner_pubkey.as_ref(),
                light_compressed_token::ID.as_ref(),
                mint_pubkey.as_ref(),
            ],
            &light_compressed_token::ID,
        );
        assert_eq!(expected_ata_pubkey, derived_ata_pubkey);
        assert_eq!(bump, derived_bump);
    }
    {
        let expected_token_account = CompressedToken {
            mint: mint_pubkey.into(),
            owner: owner_pubkey.into(),
            amount: 0,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extensions: None,
        };

        let (actual_token_account, _) = CompressedToken::zero_copy_at(&token_account_info.data)
            .expect("Failed to deserialize token account with zero-copy");

        assert_eq!(actual_token_account, expected_token_account);
    }

    // Test compressible associated token account creation
    println!("🧪 Testing compressible associated token account creation...");

    // Create rent authority and recipient for compressible account
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Airdrop lamports to rent recipient so it exists
    rpc.context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create a different owner for the compressible account
    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    // Calculate the expected compressible associated token account address
    let (expected_compressible_ata_pubkey, _) = Pubkey::find_program_address(
        &[
            compressible_owner_pubkey.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint_pubkey.as_ref(),
        ],
        &light_compressed_token::ID,
    );

    // Create compressible ATA instruction using SDK function
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: compressible_owner_pubkey,
            mint: mint_pubkey,
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }
    ).unwrap();

    // Execute the compressible instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let compressible_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[compressible_instruction],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );

    rpc.process_transaction(compressible_transaction)
        .await
        .expect("Failed to create compressible associated token account");

    // Verify the compressible associated token account was created correctly
    let compressible_account_info = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Verify account exists and has correct owner and size for compressible account
    assert_eq!(compressible_account_info.owner, light_compressed_token::ID);
    assert_eq!(
        compressible_account_info.data.len(),
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    ); // Should be compressible size, not basic size

    // Use zero-copy deserialization to verify the compressible account structure
    let (actual_compressible_token_account, _) =
        CompressedToken::zero_copy_at(&compressible_account_info.data)
            .expect("Failed to deserialize compressible token account with zero-copy");

    // Create expected compressible token account with compressible extension

    let expected_compressible_token_account = CompressedToken {
        mint: mint_pubkey.into(),
        owner: compressible_owner_pubkey.into(),
        amount: 0,
        delegate: None,
        state: 1, // Initialized
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            light_ctoken_types::state::extensions::ExtensionStruct::Compressible(
                CompressibleExtension {
                    last_written_slot: 2, // Program sets this to current slot
                    slots_until_compression: 0,
                    rent_authority: rent_authority_pubkey.into(),
                    rent_recipient: rent_recipient_pubkey.into(),
                },
            ),
        ]),
    };

    assert_eq!(
        actual_compressible_token_account,
        expected_compressible_token_account
    );

    // Test that we can close the compressible account using rent authority
    let initial_compressible_lamports = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Close account with rent authority
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &expected_compressible_ata_pubkey,
        &rent_recipient_pubkey,
        &rent_authority_pubkey,
    );

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &rent_authority_keypair],
        blockhash,
    );

    rpc.process_transaction(close_transaction).await.unwrap();

    // Verify the compressible account was closed and lamports transferred
    let closed_compressible_account = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap();
    if let Some(account) = closed_compressible_account {
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
    }

    let final_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_recipient_lamports,
        initial_recipient_lamports + initial_compressible_lamports,
        "Rent recipient should receive all lamports from closed compressible account"
    );

    println!("✅ Both basic and compressible associated token accounts work correctly!");
}
