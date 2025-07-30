#![cfg(feature = "test-sbf")]

use core::panic;

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::address::derive_address;
use light_compressible_client::CompressibleInstruction;
use light_program_test::{
    initialize_compression_config,
    program_test::{LightProgramTest, TestRpc},
    setup_mock_program_data, AddressWithTree, Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::{
    compressible::CompressibleConfig,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use native_compressible::{
    create_dynamic_pda::CreateDynamicPdaInstructionData, InstructionType, MyPdaAccount,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

// Test constants
const RENT_RECIPIENT: Pubkey =
    light_macros::pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
const COMPRESSION_DELAY: u64 = 200;

#[tokio::test]
async fn test_complete_compressible_flow() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("native_compressible", native_compressible::ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let _config_pda = CompressibleConfig::derive_default_pda(&native_compressible::ID).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &native_compressible::ID);

    // Get address tree for the address space
    let address_tree = rpc.get_address_tree_v2().queue;

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &native_compressible::ID,
        &payer,
        200,
        RENT_RECIPIENT,
        vec![address_tree],
        &[InstructionType::InitializeCompressionConfig as u8],
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    // 1. Create and compress account on init
    let test_data = [1u8; 31];

    let seeds: &[&[u8]] = &[b"dynamic_pda"];
    let (pda_pubkey, _bump) = Pubkey::find_program_address(seeds, &native_compressible::ID);

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &native_compressible::ID.to_bytes(),
    );

    let pda_pubkey = create_and_compress_account(&mut rpc, &payer, test_data).await;

    // get account
    let account = rpc.get_account(pda_pubkey).await.unwrap();
    assert!(account.is_some());
    assert_eq!(account.unwrap().lamports, 0);

    // get compressed account
    let compressed_account = rpc.get_compressed_account(compressed_address, None).await;
    assert!(compressed_account.is_ok());

    // 2. Wait for compression delay to pass
    rpc.warp_to_slot(COMPRESSION_DELAY + 1).unwrap();

    // 3. Decompress the account
    decompress_account(&mut rpc, &payer, &pda_pubkey, test_data).await;

    // get account
    let account = rpc.get_account(pda_pubkey).await.unwrap();
    assert!(account.is_some());
    assert!(account.unwrap().lamports > 0);
    // assert_eq!(account.unwrap().data.len(), 31);

    // 4. Verify PDA is decompressed
    verify_decompressed_account(&mut rpc, &pda_pubkey, &compressed_address, test_data).await;

    // 5. Wait for compression delay to pass again
    rpc.warp_to_slot(COMPRESSION_DELAY * 2 + 1).unwrap();

    // 6. Compress the account again
    compress_existing_account(&mut rpc, &payer, &pda_pubkey).await;

    // 7. Verify account is compressed again
    verify_compressed_account(&mut rpc, &pda_pubkey).await;
}

async fn create_and_compress_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    _test_data: [u8; 31],
) -> Pubkey {
    // Derive PDA
    let seeds: &[&[u8]] = &[b"dynamic_pda"];
    let (pda_pubkey, _bump) = Pubkey::find_program_address(seeds, &native_compressible::ID);

    // Get address tree
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    // Derive compressed address
    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &native_compressible::ID.to_bytes(),
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
    let system_config = SystemAccountMetaConfig::new(native_compressible::ID);
    let _ = remaining_accounts.add_system_accounts(system_config);

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

    // Build instruction
    let instruction = Instruction {
        program_id: native_compressible::ID,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true),  // fee_payer
                AccountMeta::new(pda_pubkey, false),     // solana_account
                AccountMeta::new(RENT_RECIPIENT, false), // rent_recipient
                AccountMeta::new_readonly(
                    CompressibleConfig::derive_default_pda(&native_compressible::ID).0,
                    false,
                ), // config
                AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
            ],
            system_accounts,
        ]
        .concat(),
        data: [
            &[InstructionType::CreateDynamicPda as u8][..],
            &instruction_data.try_to_vec().unwrap()[..],
        ]
        .concat(),
    };

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(
        result.is_ok(),
        "Create and compress failed error: {:?}",
        result.err()
    );

    pda_pubkey
}

async fn decompress_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    pda_pubkey: &Pubkey,
    test_data: [u8; 31],
) {
    // Get the compressed address
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;
    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &native_compressible::ID.to_bytes(),
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

    let instruction = CompressibleInstruction::decompress_accounts_idempotent(
        &native_compressible::ID,
        &[InstructionType::DecompressAccountsIdempotent as u8], // Use sdk-test's DecompressAccountsIdempotent discriminator
        &payer.pubkey(),
        &payer.pubkey(),
        &[*pda_pubkey],
        &[(
            compressed_account.clone(),
            my_pda_account.clone(), // MyPdaAccount implements required trait
            vec![b"dynamic_pda".to_vec()], // PDA seeds without bump
        )],
        &[Pubkey::find_program_address(&[b"dynamic_pda"], &native_compressible::ID).1], // bump seed, must match the seeds used in create_dynamic_pda
        rpc_result,
        compressed_account.tree_info,
    )
    .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(
        result.is_ok(),
        "Decompress failed error: {:?}",
        result.err()
    );
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

    let account = account.unwrap();
    assert!(account.lamports > 0, "PDA account should have lamports");

    // Get the compressed address
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;
    let compressed_address = derive_address(
        &pda_pubkey.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &native_compressible::ID.to_bytes(),
    );

    // Try to get the existing compressed account
    let compressed_account_result = rpc.get_compressed_account(compressed_address, None).await;

    if compressed_account_result.is_err() {
        panic!("Could not get compressed account");
    }

    let compressed_account = compressed_account_result.unwrap().value;

    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let instruction = CompressibleInstruction::compress_account(
        &native_compressible::ID,
        &[InstructionType::CompressDynamicPda as u8], // Use sdk-test's CompressFromPda discriminator
        &payer.pubkey(),
        pda_pubkey,
        &RENT_RECIPIENT,
        &compressed_account,
        rpc_result,
        compressed_account.tree_info,
    )
    .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(result.is_ok(), "Compress failed error: {:?}", result.err());
}

async fn verify_decompressed_account(
    rpc: &mut LightProgramTest,
    pda_pubkey: &Pubkey,
    compressed_address: &[u8; 32],
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

    // Try to deserialize the account data (skip the 8-byte discriminator)
    let solana_account = MyPdaAccount::deserialize(&mut &account.data[8..])
        .expect("Could not deserialize PDA account data");
    assert!(solana_account.compression_info.is_some());
    assert_eq!(solana_account.data, expected_data); // data matches the expected data
    assert!(
        !solana_account
            .compression_info
            .as_ref()
            .unwrap()
            .is_compressed(),
        "PDA account should not be compressed"
    );
    // slot matches the slot of the last write
    assert_eq!(
        &solana_account.compression_info.unwrap().last_written_slot(),
        &rpc.get_slot().await.unwrap()
    );

    let compressed_account = rpc.get_compressed_account(*compressed_address, None).await;
    assert!(compressed_account.is_ok());
    let compressed_account = compressed_account.unwrap().value;
    // After decompression, the compressed account data should be cleared
    // This is a known behavior - commenting out for now to see if test passes

    assert!(
        compressed_account.data.unwrap().data.as_slice().is_empty(),
        "Compressed account data must be empty"
    );
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
    } else {
        panic!("PDA account not found");
    }
}
