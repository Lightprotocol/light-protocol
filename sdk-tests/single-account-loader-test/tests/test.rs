//! Integration test for single AccountLoader (zero-copy) macro validation.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_account::{derive_rent_sponsor_pda, IntoVariant};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountSpec, CreateAccountsProofInput,
    InitializeRentFreeConfig, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use single_account_loader_test::{
    single_account_loader_test::{LightAccountVariant, RecordSeeds},
    CreateRecordParams, ZeroCopyRecord, RECORD_SEED,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test creating a single compressible zero-copy PDA using the macro.
/// Validates that `#[light_account(init, zero_copy)]` works with AccountLoader.
#[tokio::test]
async fn test_create_zero_copy_record() {
    let program_id = single_account_loader_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_account_loader_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Derive rent sponsor PDA for this program
    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    let owner = Keypair::new().pubkey();

    // Derive PDA for record using the same seeds as the program
    let (record_pda, _) = Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    // Get proof for the PDA
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = single_account_loader_test::accounts::CreateRecord {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_account_loader_test::instruction::CreateRecord {
        params: CreateRecordParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateRecord should succeed");

    // Verify PDA exists on-chain
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist on-chain");

    // Parse and verify record data using bytemuck (zero-copy deserialization)
    // Skip the 8-byte discriminator
    let discriminator_len = 8;
    let data = &record_account.data[discriminator_len..];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(data);

    // Verify owner field
    assert_eq!(record.owner, owner, "Record owner should match");

    // Verify counter field
    assert_eq!(record.counter, 0, "Record counter should be 0");

    // Verify compression_info is set (state == Decompressed indicates initialized)
    use light_account::CompressionState;
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed (initialized)"
    );
    assert_eq!(
        record.compression_info.config_version, 1,
        "config_version should be 1"
    );
}

/// Test the full lifecycle of a zero-copy PDA: create -> compress -> decompress.
/// Validates that the macro correctly handles Pod accounts through all phases.
#[tokio::test]
async fn test_zero_copy_record_full_lifecycle() {
    let program_id = single_account_loader_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_account_loader_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Derive rent sponsor PDA for this program
    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    let owner = Keypair::new().pubkey();

    // Derive PDA for record using the same seeds as the program
    let (record_pda, _) = Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    // Get proof for the PDA
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = single_account_loader_test::accounts::CreateRecord {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_account_loader_test::instruction::CreateRecord {
        params: CreateRecordParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateRecord should succeed");

    // PHASE 1: Verify account exists on-chain
    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Account should exist on-chain after creation"
    );

    // PHASE 2: Warp time to trigger forester auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // Verify account is closed on-chain (compressed by forester)
    let acc = rpc.get_account(record_pda).await.unwrap();
    assert!(
        acc.is_none() || acc.unwrap().lamports == 0,
        "Account should be closed after compression"
    );

    // PHASE 3: Verify compressed account exists
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_address = light_compressed_account::address::derive_address(
        &record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_acc = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        compressed_acc.address.unwrap(),
        compressed_address,
        "Compressed account address should match"
    );
    assert!(
        !compressed_acc.data.as_ref().unwrap().data.is_empty(),
        "Compressed account should have data"
    );

    // PHASE 4: Decompress account
    let account_interface = rpc
        .get_account_interface(&record_pda, None)
        .await
        .expect("failed to get account interface")
        .value
        .expect("account interface should exist");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    // Build variant using IntoVariant - verify seeds match the compressed data
    let variant = RecordSeeds { owner }
        .into_variant(&account_interface.account.data[8..])
        .expect("Seed verification failed");

    // Build PdaSpec and create decompress instructions
    let spec = PdaSpec::new(account_interface.clone(), variant, program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, payer.pubkey(), config_pda, &rpc)
            .await
            .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&decompress_instructions, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain with correct data
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Account should exist after decompression");

    // Verify data is correct using bytemuck (zero-copy deserialization)
    let discriminator_len = 8;
    let data = &record_account.data[discriminator_len..];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(data);

    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(record.counter, 0, "Record counter should still be 0");
    // state should be Decompressed after decompression
    use light_account::CompressionState;
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed after decompression"
    );
    assert!(
        record.compression_info.config_version >= 1,
        "config_version should be >= 1 after decompression"
    );
}
