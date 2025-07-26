#![cfg(feature = "test-sbf")]

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::compressible::CompressibleInstruction;
use light_compressed_account::address::derive_address;
use light_program_test::{initialize_compression_config, setup_mock_program_data};
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    AddressWithTree, Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::{
    compressible::CompressibleConfig,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use sdk_test::{
    compress_dynamic_pda::CompressFromPdaInstructionData,
    create_dynamic_pda::CreateDynamicPdaInstructionData,
    decompress_dynamic_pda::{DecompressToPdaInstructionData, MyCompressedAccount, MyPdaAccount},
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

// Test constants
const RENT_RECIPIENT: Pubkey =
    light_macros::pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
const COMPRESSION_DELAY: u64 = 100;

#[tokio::test]
async fn test_complete_compressible_flow() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_test", sdk_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&sdk_test::ID).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &sdk_test::ID);

    // Get address tree for the address space
    let address_tree = rpc.get_address_merkle_tree_v2();

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &sdk_test::ID,
        &payer,
        100,
        RENT_RECIPIENT,
        vec![address_tree],
        &[5u8],
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");
    println!("Starting complete compressible flow test");

    // 1. Create and compress account on init
    let test_data = [42u8; 31];
    let pda_pubkey = create_and_compress_account(&mut rpc, &payer, test_data).await;
    println!("Created and compressed PDA: {}", pda_pubkey);

    // 2. Wait for compression delay to pass
    rpc.warp_to_slot(COMPRESSION_DELAY + 1).unwrap();
    println!("Warped to slot {}", COMPRESSION_DELAY + 1);

    // 3. Decompress the account
    decompress_account(&mut rpc, &payer, &pda_pubkey, test_data).await;
    println!("Decompressed account");

    // 4. Verify PDA is decompressed
    verify_decompressed_account(&mut rpc, &pda_pubkey, test_data).await;

    // 5. Wait for compression delay to pass again
    rpc.warp_to_slot(COMPRESSION_DELAY * 2 + 2).unwrap();
    println!("Warped to slot {}", COMPRESSION_DELAY * 2 + 2);

    // 6. Compress the account again
    compress_existing_account(&mut rpc, &payer, &pda_pubkey).await;
    println!("Compressed account again");

    // 7. Verify account is compressed again
    verify_compressed_account(&mut rpc, &pda_pubkey).await;

    println!("Complete compressible flow test passed!");
}

async fn create_and_compress_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    test_data: [u8; 31],
) -> Pubkey {
    // Derive PDA
    let seeds: &[&[u8]] = &[b"test_pda", &test_data];
    let (pda_pubkey, _bump) = Pubkey::find_program_address(seeds, &sdk_test::ID);

    // Get address tree
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Derive compressed address
    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &sdk_test::ID.to_bytes(),
    );

    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compressed_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Setup remaining accounts
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(sdk_test::ID);
    remaining_accounts.add_system_accounts(system_config);

    // Pack tree infos
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let address_tree_info = packed_tree_infos.address_trees[0];

    // Get output state tree index
    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data for create_dynamic_pda
    let instruction_data = CreateDynamicPdaInstructionData {
        proof: rpc_result.proof,
        compressed_address,
        address_tree_info,
        output_state_tree_index,
    };

    // Debug: log the proof and instruction data details
    println!("Proof: {:?}", instruction_data.proof);
    println!(
        "Compressed address length: {}",
        instruction_data.compressed_address.len()
    );
    println!(
        "Address tree info: {:?}",
        instruction_data.address_tree_info
    );
    println!(
        "Output state tree index: {}",
        instruction_data.output_state_tree_index
    );

    let serialized = instruction_data.try_to_vec().unwrap();
    println!("Serialized instruction data length: {}", serialized.len());

    // Build instruction
    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true),  // fee_payer
                AccountMeta::new(pda_pubkey, false),     // pda_account
                AccountMeta::new(RENT_RECIPIENT, false), // rent_recipient
                AccountMeta::new_readonly(CompressibleConfig::derive_pda(&sdk_test::ID).0, false), // config
            ],
            system_accounts,
        ]
        .concat(),
        data: [&[4u8][..], &instruction_data.try_to_vec().unwrap()[..]].concat(),
    };

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(result.is_ok(), "Create and compress failed");

    pda_pubkey
}

async fn decompress_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    pda_pubkey: &Pubkey,
    test_data: [u8; 31],
) {
    // Get the compressed address
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();
    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &sdk_test::ID.to_bytes(),
    );

    // Try to get the compressed account from the indexer
    let compressed_account_result = rpc.get_compressed_account(compressed_address, None).await;

    if compressed_account_result.is_err() {
        panic!("Could not get compressed account");
    }

    let compressed_account = compressed_account_result.unwrap().value;

    // Create MyPdaAccount from the test data
    let my_pda_account = MyPdaAccount {
        compression_info: None, // Will be set during decompression
        data: test_data,
    };

    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let instruction = CompressibleInstruction::decompress_multiple_accounts_idempotent(
        &sdk_test::ID,
        &[7u8], // Use sdk-test's DecompressMultipleAccountsIdempotent discriminator
        &payer.pubkey(),
        &payer.pubkey(),
        &[*pda_pubkey],
        &[(
            compressed_account.clone(),
            my_pda_account.clone(), // MyPdaAccount implements required trait
        )],
        &[Pubkey::find_program_address(&[pda_pubkey.as_ref()], &sdk_test::ID).1], // bump seed, adjust if needed
        rpc_result,
        compressed_account.tree_info,
    )
    .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    match result {
        Ok(_) => println!("Successfully decompressed account"),
        Err(e) => println!("Decompress failed: {:?}", e),
    }
}

async fn compress_existing_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    pda_pubkey: &Pubkey,
) {
    // Get the account data first
    let account = rpc.get_account(*pda_pubkey).await.unwrap();
    if account.is_none() {
        println!("PDA account not found, cannot compress");
        return;
    }

    // Get the compressed address
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();
    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &sdk_test::ID.to_bytes(),
    );

    // Try to get the existing compressed account
    let compressed_account_result = rpc.get_compressed_account(compressed_address, None).await;

    if compressed_account_result.is_err() {
        println!("Could not get compressed account, skipping compress test");
        return;
    }

    let compressed_account = compressed_account_result.unwrap().value;

    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Setup remaining accounts
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(sdk_test::ID);
    remaining_accounts.add_system_accounts(system_config);

    // Pack tree infos
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the tree info for the compressed account
    let queue_index = remaining_accounts.insert_or_get(compressed_account.tree_info.queue);
    let tree_info = packed_tree_infos
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos
        .iter()
        .find(|pti| {
            pti.queue_pubkey_index == queue_index && pti.leaf_index == compressed_account.leaf_index
        })
        .copied()
        .unwrap();

    let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data = CompressFromPdaInstructionData {
        proof: rpc_result.proof,
        compressed_account_meta: light_sdk::instruction::account_meta::CompressedAccountMeta {
            tree_info,
            address: compressed_account.address.unwrap_or([0u8; 32]),
            output_state_tree_index: packed_tree_infos
                .state_trees
                .as_ref()
                .unwrap()
                .output_tree_index,
        },
        system_accounts_offset: system_accounts_offset as u8,
    };

    // Build instruction
    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true),  // user
                AccountMeta::new(*pda_pubkey, false),    // pda_account
                AccountMeta::new(RENT_RECIPIENT, false), // rent_recipient
                AccountMeta::new_readonly(CompressibleConfig::derive_pda(&sdk_test::ID).0, false), // config
            ],
            system_accounts,
        ]
        .concat(),
        data: [&[3u8][..], &instruction_data.try_to_vec().unwrap()[..]].concat(),
    };

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    match result {
        Ok(_) => println!("Successfully compressed existing account"),
        Err(e) => println!("Compress existing account failed: {:?}", e),
    }
}

async fn verify_decompressed_account(
    rpc: &mut LightProgramTest,
    pda_pubkey: &Pubkey,
    expected_data: [u8; 31],
) {
    let account = rpc.get_account(*pda_pubkey).await.unwrap();

    assert!(
        account.is_some(),
        "PDA account not found after decompression"
    );

    let account = account.unwrap();
    assert!(
        account.data.len() > 8,
        "PDA account not properly decompressed (empty data)"
    );

    // Try to deserialize the account data
    let pda_account = MyPdaAccount::try_from_slice(&account.data[8..])
        .expect("Could not deserialize PDA account data");

    assert_eq!(
        pda_account.data, expected_data,
        "PDA data does not match expected data"
    );
    println!("PDA successfully decompressed with correct data");
}

async fn verify_compressed_account(rpc: &mut LightProgramTest, pda_pubkey: &Pubkey) {
    let account = rpc.get_account(*pda_pubkey).await.unwrap();

    if let Some(account) = account {
        assert_eq!(
            account.lamports, 0,
            "PDA account should have 0 lamports when compressed"
        );
        assert!(
            account.data.is_empty(),
            "PDA account should have empty data when compressed"
        );
        println!("PDA successfully compressed (empty account)");
    } else {
        // Account not found is also valid for compressed state
        println!("PDA account not found (also valid compressed state)");
    }
}
