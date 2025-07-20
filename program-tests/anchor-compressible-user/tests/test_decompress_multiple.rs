#![cfg(feature = "test-sbf")]

mod common;

use anchor_compressible_user::{
    CompressedAccountData, CompressedAccountVariant, UserRecord, ADDRESS_SPACE, RENT_RECIPIENT,
};
use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_compressed_account::address::derive_address;
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::compressible::{CompressibleConfig, FromCompressedData};
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig,
};
use light_sdk::LightDiscriminator;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

// The FromCompressedData trait is now implemented in the main program lib.rs

#[tokio::test]
async fn test_all() {
    std::env::set_var("RUST_LOG", "debug");
    // let _ = env_logger::try_init();
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let program_data_pda = common::setup_mock_program_data(&mut rpc, &payer, &program_id);
    let result = common::initialize_config(
        &mut rpc,
        &payer,
        &program_id,
        config_pda,
        program_data_pda,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    test_create_record_with_config(&mut rpc, &payer, &program_id, &config_pda, &user_record_pda)
        .await;
    test_decompress_multiple_pdas(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        &bump,
    )
    .await;
}

async fn test_create_record_with_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
) {
    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Create the instruction
    let accounts = anchor_compressible_user::accounts::CreateRecordWithConfig {
        user: payer.pubkey(),
        user_record: *user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // Derive a new address for the compressed account
    let compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    println!("compressed_address: {:?}", compressed_address);
    println!("SEED1 (user_record_pda): {:?}", user_record_pda);
    println!("SEED2 (address_tree_pubkey): {:?}", address_tree_pubkey);
    println!("SEED3 (program_id): {:?}", program_id);
    // Get validity proof from RPC
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

    println!("rpc_result: {:?}", rpc_result);
    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the packed address tree info
    let address_tree_info = packed_tree_infos.address_trees[0];

    println!("packed address tree info: {:?}", address_tree_info);
    // Get output state tree index
    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::CreateRecordWithConfig {
        name: "Test User".to_string(),
        proof: rpc_result.proof,
        compressed_address,
        address_tree_info,
        output_state_tree_index,
    };

    // Build the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(result.is_ok(), "Transaction should succeed");

    // should be empty
    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_record_account.is_some(),
        "Account should exist after compression"
    );

    let account = user_record_account.unwrap();
    assert_eq!(account.lamports, 0, "Account lamports should be 0");

    let user_record_data = account.data;
    println!("user_record_data: {:?}", user_record_data);
    println!("user_record_data len: {:?}", user_record_data.len());

    assert!(user_record_data.is_empty(), "Account data should be empty");
}

async fn test_decompress_multiple_pdas(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    _config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    bump: &u8,
) {
    // TODO: dynamic
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    let compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    // Fetch the compressed account created in the previous test
    let c_pda = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value;

    // Deserialize the account data to get the UserRecord using the SDK trait
    let account_data = c_pda.data.as_ref().unwrap();

    println!("account_data: {:?}", account_data.data);
    println!("account_data len: {:?}", account_data.data.len());

    println!("c-ac disc: {:?}", account_data.discriminator);

    let mut with_discriminator = UserRecord::discriminator().to_vec();
    with_discriminator.extend_from_slice(&account_data.data);

    println!("with_discriminator: {:?}", with_discriminator);
    println!("with_discriminator len: {:?}", with_discriminator.len());

    // Use the trait to handle padding differences intelligently
    let c_user_record = UserRecord::from_compressed_data(&with_discriminator).unwrap();

    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get validity proof for the compressed account
    let rpc_result = rpc
        .get_validity_proof(vec![c_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Create the compressed account metadata
    let compressed_account_meta = CompressedAccountMeta {
        tree_info: packed_tree_infos.state_trees.unwrap().packed_tree_infos[0],
        address: c_pda.address.unwrap(),
        output_state_tree_index: 0, // Not needed for decompression
    };

    // Create the compressed account data for the instruction
    let compressed_account_data = CompressedAccountData {
        meta: compressed_account_meta,
        data: CompressedAccountVariant::UserRecord(c_user_record),
    };

    // Build instruction accounts
    let pda_accounts = vec![user_record_pda];
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Prepare bumps for the PDA
    let bumps = vec![*bump];

    let instruction_data = anchor_compressible_user::instruction::DecompressMultiplePdas {
        proof: rpc_result.proof,
        compressed_accounts: vec![compressed_account_data],
        bumps,
        system_accounts_offset,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true), // fee_payer
                AccountMeta::new(payer.pubkey(), true), // rent_payer
                AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
            ],
            pda_accounts
                .iter()
                .map(|&pda| AccountMeta::new(*pda, false))
                .collect(),
            system_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    let pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    // assert!(
    //     pda_account.is_none(),
    //     "PDA account should not exist before decompression"
    // );
    println!("pda_account BEFORE DECOMPRESSION: {:?}", pda_account);

    // Execute transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(result.is_ok(), "Decompress transaction should succeed");

    // Get just the logs
    let logs = rpc.context.get_transaction(&result.unwrap());
    println!("Transaction logs: {:#?}", logs);

    // Verify the PDA account was created and contains the correct data
    let pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        pda_account.is_some(),
        "PDA account should exist after decompression"
    );
    println!("decompressed pda_account: {:?}", pda_account);

    let pda_account_data = pda_account.unwrap().data;
    let discriminator_len = UserRecord::discriminator().len();
    let decompressed_user_record =
        UserRecord::deserialize(&mut &pda_account_data[discriminator_len..]).unwrap();

    println!("decompressed_user_record: {:?}", decompressed_user_record);
    // Verify the decompressed data matches the original
    assert_eq!(decompressed_user_record.name, "Test User");
    assert_eq!(decompressed_user_record.score, 11);
    assert_eq!(decompressed_user_record.owner, payer.pubkey());

    // Verify the compression info shows it's decompressed
    assert_eq!(
        decompressed_user_record.compression_info.is_compressed(),
        false
    );

    println!(
        "Successfully decompressed user record: {:?}",
        decompressed_user_record
    );
}
