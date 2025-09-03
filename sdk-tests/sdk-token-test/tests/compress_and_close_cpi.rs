//#![cfg(feature = "test-sbf")]

use anchor_lang::InstructionData;
use light_compressed_token_sdk::instructions::{
    create_compressible_associated_token_account, find_spl_mint_address, CTokenDefaultAccounts,
    CreateCompressibleAssociatedTokenAccountInputs,
};
use light_ctoken_types::{instructions::mint_action::Recipient, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use light_test_utils::airdrop_lamports;
use light_token_client::{actions::mint_action_comprehensive, instructions::mint_action::NewMint};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

/// Test the original compress_and_close_cpi_indices instruction with manual indices
/// This test verifies that CompressAndClose mode works correctly through CPI with manual index management
#[tokio::test]
async fn test_compress_and_close_cpi_indices() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create compressed mint
    let mint_seed = Keypair::new();
    let mint_pubkey = find_spl_mint_address(&mint_seed.pubkey()).0;
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create owner and rent authority keypairs
    let owner = Keypair::new();
    let rent_authority = Keypair::new();
    let rent_recipient = Pubkey::new_unique();

    // Fund accounts
    airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    airdrop_lamports(&mut rpc, &rent_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    // Create compressible associated token account
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    // Derive the ATA address that will actually be created
    use light_compressed_token_sdk::instructions::derive_ctoken_ata;
    let (token_account_pubkey, _) = derive_ctoken_ata(&owner.pubkey(), &mint_pubkey);

    {
        let create_token_account_ix = create_compressible_associated_token_account(
            CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                mint: mint_pubkey,
                owner: owner.pubkey(),
                rent_authority: rent_authority.pubkey(),
                rent_recipient,
                slots_until_compression: 0,
            },
        )
        .unwrap();

        rpc.create_and_send_transaction(&[create_token_account_ix], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }

    let mint_amount = 1000;
    mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &payer,
        &payer,
        false,
        Vec::new(),
        vec![Recipient {
            recipient: owner.pubkey().into(),
            amount: mint_amount,
        }],
        None,
        None,
        None,
        Some(NewMint {
            decimals,
            mint_authority,
            supply: 0,
            freeze_authority: None,
            metadata: None,
            version: 3,
        }),
    )
    .await
    .unwrap();
    // Get initial rent recipient balance
    let initial_recipient_balance = rpc
        .get_account(rent_recipient)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    // Prepare accounts for CPI instruction
    let mut remaining_accounts = PackedAccounts::default();

    // Get output tree for compression
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();
    let output_tree_index = output_tree_info
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    // Pack accounts needed for CompressAndClose
    let recipient_index = remaining_accounts.insert_or_get(owner.pubkey());
    let mint_index = remaining_accounts.insert_or_get(mint_pubkey);
    let source_index = remaining_accounts.insert_or_get(token_account_pubkey);
    let authority_index =
        remaining_accounts.insert_or_get_config(rent_authority.pubkey(), true, false);
    let rent_recipient_index = remaining_accounts.insert_or_get(rent_recipient);

    // Add light system program accounts
    let config = SystemAccountMetaConfig::new(sdk_token_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();

    // Add compressed token program
    let default_pubkeys = CTokenDefaultAccounts::default();
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(
        default_pubkeys.compressed_token_program,
        false,
    ));

    // Add compressed token CPI authority
    remaining_accounts
        .add_pre_accounts_meta(AccountMeta::new(default_pubkeys.cpi_authority_pda, false));
    // Add accounts to instruction
    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    // Create the compress_and_close_cpi_indices instruction data
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpiIndices {
        output_tree_index,
        recipient_index,
        mint_index,
        source_index,
        authority_index,
        rent_recipient_index,
        system_accounts_offset: system_accounts_start_offset as u8,
    };

    // Create the instruction
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![AccountMeta::new_readonly(payer_pubkey, true)],
            account_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Execute transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer, &rent_authority],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    rpc.process_transaction(transaction).await.unwrap();

    // Verify compressed account was created
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    assert_eq!(compressed_accounts[0].token.amount, mint_amount);
    assert_eq!(compressed_accounts[0].token.mint, mint_pubkey);

    // Verify source account is closed
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(acc) = closed_account {
        assert_eq!(
            acc.lamports, 0,
            "Account should have 0 lamports after closing"
        );
    }

    // Verify rent was transferred to recipient
    let final_recipient_balance = rpc
        .get_account(rent_recipient)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    assert_eq!(
        final_recipient_balance,
        initial_recipient_balance + rent_exemption,
        "Rent recipient should receive exact rent exemption amount"
    );

    println!("✅ CompressAndClose CPI test passed!");
}

// /// Test compress_and_close_cpi with zero balance
// #[tokio::test]
// async fn test_compress_and_close_cpi_zero_balance() {
//     let mut config = ProgramTestConfig::default();
//     let mut program_test = LightProgramTest::new(config).await.unwrap();

//     let mut rpc = program_test;
//     let payer = rpc.get_payer().insecure_clone();
//     let payer_pubkey = payer.pubkey();

//     // Create mint
//     let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

//     // Create owner and rent authority
//     let owner = Keypair::new();
//     let rent_authority = Keypair::new();
//     let rent_recipient = Pubkey::new_unique();

//     // Fund accounts
//     airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
//         .await
//         .unwrap();
//     airdrop_lamports(&mut rpc, &rent_authority.pubkey(), 10_000_000_000)
//         .await
//         .unwrap();

//     // Create compressible token account with zero balance
//     let token_account_keypair = Keypair::new();
//     let token_account_pubkey = token_account_keypair.pubkey();

//     let rent_exemption = rpc
//         .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
//         .await
//         .unwrap();

//     let create_account_ix = system_instruction::create_account(
//         &payer_pubkey,
//         &token_account_pubkey,
//         rent_exemption,
//         COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
//         &CTokenDefaultAccounts::default().compressed_token_program,
//     );

//     let create_token_account_ix =
//         create_compressible_token_account(CreateCompressibleTokenAccount {
//             account_pubkey: token_account_pubkey,
//             mint_pubkey,
//             owner_pubkey: owner.pubkey(),
//             rent_authority: rent_authority.pubkey(),
//             rent_recipient,
//             slots_until_compression: 0,
//         })
//         .unwrap();

//     rpc.create_and_send_transaction(
//         &[create_account_ix, create_token_account_ix],
//         &payer_pubkey,
//         &[&payer, &token_account_keypair],
//     )
//     .await
//     .unwrap();

//     // Don't mint any tokens - test with 0 balance

//     // Get initial rent recipient balance
//     let initial_recipient_balance = rpc
//         .get_account(rent_recipient)
//         .await
//         .unwrap()
//         .map(|acc| acc.lamports)
//         .unwrap_or(0);

//     // Prepare accounts for CPI instruction
//     let mut remaining_accounts = PackedAccounts::default();

//     let output_tree_info = rpc.get_random_state_tree_info().unwrap();
//     let output_tree_index = output_tree_info
//         .pack_output_tree_index(&mut remaining_accounts)
//         .unwrap();

//     let recipient_index = remaining_accounts.insert_or_get(owner.pubkey());
//     let mint_index = remaining_accounts.insert_or_get(mint_pubkey);
//     let source_index = remaining_accounts.insert_or_get(token_account_pubkey);
//     let authority_index =
//         remaining_accounts.insert_or_get_config(rent_authority.pubkey(), true, false);
//     let rent_recipient_index = remaining_accounts.insert_or_get(rent_recipient);

//     let system_accounts_start_offset = remaining_accounts.pre_accounts.len() as u8;
//     let env = rpc.get_env_accounts();
//     remaining_accounts.add_system_accounts(&env);
//     remaining_accounts.add_pre_accounts_meta(AccountMeta::new_readonly(
//         light_compressed_token_sdk::CTokenDefaultAccounts::default().compressed_token_program,
//         false,
//     ));

//     // Create and execute the instruction
//     let instruction_data = sdk_token_test::instruction::CompressAndCloseCpi {
//         output_tree_index,
//         recipient_index,
//         mint_index,
//         source_index,
//         authority_index,
//         rent_recipient_index,
//         system_accounts_offset: system_accounts_start_offset,
//     };

//     let (account_metas, _, _) = remaining_accounts.to_account_metas();
//     let instruction = Instruction {
//         program_id: sdk_token_test::ID,
//         accounts: [
//             vec![AccountMeta::new_readonly(payer_pubkey, true)],
//             account_metas,
//         ]
//         .concat(),
//         data: instruction_data.data(),
//     };

//     let transaction = Transaction::new_signed_with_payer(
//         &[instruction],
//         Some(&payer_pubkey),
//         &[&payer, &rent_authority],
//         rpc.get_latest_blockhash().await.unwrap(),
//     );

//     rpc.process_transaction(transaction).await.unwrap();

//     // Verify compressed account with 0 balance was created
//     let compressed_accounts = rpc
//         .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
//         .await
//         .unwrap()
//         .value
//         .items;

//     assert_eq!(compressed_accounts.len(), 1);
//     assert_eq!(
//         compressed_accounts[0].token.amount, 0,
//         "Should compress 0 tokens"
//     );

//     // Verify rent transfer
//     let final_recipient_balance = rpc
//         .get_account(rent_recipient)
//         .await
//         .unwrap()
//         .map(|acc| acc.lamports)
//         .unwrap_or(0);

//     assert_eq!(
//         final_recipient_balance,
//         initial_recipient_balance + rent_exemption,
//         "Rent recipient should receive rent even with 0 token balance"
//     );

//     println!("✅ CompressAndClose CPI with zero balance test passed!");
// }

/// Test the high-level compress_and_close_cpi function
/// This test uses the SDK's compress_and_close_ctoken_accounts which handles all index discovery
#[tokio::test]
async fn test_compress_and_close_cpi_high_level() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create compressed mint
    let mint_seed = Keypair::new();
    let mint_pubkey = find_spl_mint_address(&mint_seed.pubkey()).0;
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create owner and use as rent authority too
    let owner = Keypair::new();
    let rent_recipient = owner.pubkey(); // Use owner as rent recipient

    // Fund accounts
    airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Create compressible associated token account first
    use light_compressed_token_sdk::instructions::derive_ctoken_ata;
    let (token_account_pubkey, _) = derive_ctoken_ata(&owner.pubkey(), &mint_pubkey);

    {
        let create_token_account_ix = create_compressible_associated_token_account(
            CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                mint: mint_pubkey,
                owner: owner.pubkey(),
                rent_authority: owner.pubkey(), // Use owner as rent authority
                rent_recipient,
                slots_until_compression: 0,
            },
        )
        .unwrap();

        rpc.create_and_send_transaction(&[create_token_account_ix], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }

    // Mint tokens using mint_action_comprehensive
    let mint_amount = 1000;
    mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &payer,
        &payer,
        false,
        Vec::new(),
        vec![Recipient {
            recipient: owner.pubkey().into(),
            amount: mint_amount,
        }],
        None,
        None,
        None,
        Some(NewMint {
            decimals,
            mint_authority,
            supply: 0,
            freeze_authority: None,
            metadata: None,
            version: 3,
        }),
    )
    .await
    .unwrap();

    // Prepare accounts for CPI instruction - using high-level function
    // Mirror the exact setup from test_compress_and_close_cpi_indices
    let mut remaining_accounts = PackedAccounts::default();

    // Get output tree for compression
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();
    output_tree_info
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    // Pack accounts needed for CompressAndClose (same as indices test)
    remaining_accounts.insert_or_get(owner.pubkey()); // recipient
    remaining_accounts.insert_or_get(mint_pubkey);
    remaining_accounts.insert_or_get(token_account_pubkey); // source ctoken account
    remaining_accounts.insert_or_get(owner.pubkey()); // authority (using owner since no rent authority)
    remaining_accounts.insert_or_get_config(rent_recipient, false, true); // rent recipient must be writable

    // Add light system program accounts
    let config = SystemAccountMetaConfig::new(sdk_token_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();

    // Add compressed token program
    let default_pubkeys = CTokenDefaultAccounts::default();
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(
        default_pubkeys.compressed_token_program,
        false,
    ));

    // Add compressed token CPI authority
    remaining_accounts
        .add_pre_accounts_meta(AccountMeta::new(default_pubkeys.cpi_authority_pda, false));

    // Add accounts to instruction
    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    // Create the compress_and_close_cpi instruction data for high-level function
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpi {
        with_rent_authority: false, // Don't use rent authority from extension
        system_accounts_offset: system_accounts_start_offset as u8, // No accounts before system accounts in remaining_accounts
    };

    // Create the instruction - OneCTokenAccount expects [signer, ctoken_account, ...remaining]
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![
                AccountMeta::new(payer_pubkey, true), // signer (mutable)
                AccountMeta::new(token_account_pubkey, false), // ctoken_account (mutable)
            ],
            account_metas, // remaining accounts (trees, mint, owner, etc.)
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Execute transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    rpc.process_transaction(transaction).await.unwrap();

    // Verify compressed account was created
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    assert_eq!(compressed_accounts[0].token.amount, mint_amount);
    assert_eq!(compressed_accounts[0].token.mint, mint_pubkey);

    // Verify source account is closed
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(acc) = closed_account {
        assert_eq!(
            acc.lamports, 0,
            "Account should have 0 lamports after closing"
        );
    }

    println!("✅ CompressAndClose CPI high-level test passed!");
}
