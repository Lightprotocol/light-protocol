#![cfg(feature = "test-sbf")]

mod common;

use anchor_compressible_user::{
    CompressedAccountData, CompressedAccountVariant, UserRecord, ADDRESS_SPACE, RENT_RECIPIENT,
};
use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_compressed_account::{address::derive_address, hashv_to_bn254_field_size_be};
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::LightProgramTest, AddressWithTree, Indexer,
    ProgramTestConfig, Rpc,
};
use light_sdk::compressible::CompressibleConfig;
use light_sdk::compressible::CompressionInfo;
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta, PackedAccounts, PackedAddressTreeInfo,
    SystemAccountMetaConfig,
};
use light_sdk::LightDiscriminator;
use light_test_utils::RpcError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_all() {
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

    let (user_record_pda, _bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    test_create_record_with_config(&mut rpc, &payer, &program_id, &config_pda, &user_record_pda)
        .await;
    test_decompress_multiple_pdas(&mut rpc, &payer, &program_id, &config_pda, &user_record_pda)
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

    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the packed address tree info
    let address_tree_info = packed_tree_infos.address_trees[0];

    // Get output state tree index
    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    println!("...Output state tree index: {:?}", output_state_tree_index);

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    println!(
        "system accounts: LEN: {:?} {:#?}",
        system_accounts.len(),
        system_accounts
    );
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

    // Verify the user record was created
    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_record_account.is_some(),
        "User record PDA should exist"
    );

    // Deserialize and verify the user record data
    let user_record_data = user_record_account.unwrap().data;
    let discriminator_len = UserRecord::discriminator().len();
    println!("user_record_data: {:?}", user_record_data);
    println!("user_record_data len {:?}", user_record_data.len());
    let user_record = UserRecord::deserialize(&mut &user_record_data[discriminator_len..]).unwrap(); // Skip discriminator
    assert_eq!(user_record.name, "Test User");
    assert_eq!(user_record.score, 0);
    println!(
        "user_record.compression_info: {:?}",
        user_record.compression_info.state
    );
    assert_eq!(user_record.compression_info.is_compressed(), true);
}

#[derive(AnchorDeserialize)]
struct UserRecordWithoutCompressionInfo {
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
}

async fn test_decompress_multiple_pdas(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
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

    // Deserialize the account data to get the UserRecord
    let account_data = c_pda.data.as_ref().unwrap();
    println!("account_data: {:?}", account_data);
    println!("account_data len: {:?}", account_data.data.len());

    // FIXME: need skip compression info
    // 2324 extra zeroes padding ??.
    let temp_record =
        UserRecordWithoutCompressionInfo::deserialize(&mut &account_data.data.as_slice()[..])
            .unwrap(); // Skip discriminator

    // not ideal because we pass empty 9 bytes.
    // Convert back to UserRecord with default compression info
    let user_record = UserRecord {
        compression_info: CompressionInfo::default(),
        owner: temp_record.owner,
        name: temp_record.name,
        score: temp_record.score,
    };

    // Create a new PDA that will receive the decompressed data
    let decompress_user = Keypair::new();
    let (decompress_pda, decompress_bump) = Pubkey::find_program_address(
        &[b"user_record", decompress_user.pubkey().as_ref()],
        program_id,
    );

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
        data: CompressedAccountVariant::UserRecord(user_record),
    };

    // Build instruction accounts
    let pda_accounts = vec![decompress_pda];
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Prepare bumps for the PDA
    let bumps = vec![decompress_bump];

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
                .map(|&pda| AccountMeta::new(pda, false))
                .collect(),
            system_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Execute transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(result.is_ok(), "Decompress transaction should succeed");

    // Verify the PDA account was created and contains the correct data
    let pda_account = rpc.get_account(decompress_pda).await.unwrap();
    assert!(
        pda_account.is_some(),
        "PDA account should exist after decompression"
    );

    let pda_account_data = pda_account.unwrap().data;
    let discriminator_len = UserRecord::discriminator().len();
    let decompressed_user_record =
        UserRecord::deserialize(&mut &pda_account_data[discriminator_len..]).unwrap();

    // Verify the decompressed data matches the original
    assert_eq!(decompressed_user_record.name, "Test User");
    assert_eq!(decompressed_user_record.score, 0);
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
