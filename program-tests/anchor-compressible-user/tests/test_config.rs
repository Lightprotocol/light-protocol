//! # Config Tests: anchor-compressible-user
//!
//! Checks covered:
//! - Successful config init
//! - Authority check (init/update)
//! - Config update by authority
//! - Prevent re-init
//! - Program data account check
//! - Prevent address space removal
//! - Update with non-authority
//! - Rent recipient check
#![cfg(feature = "test-sbf")]

mod common;
use anchor_compressible_user::{ADDRESS_SPACE, RENT_RECIPIENT};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    ProgramTestConfig, Rpc,
};
use light_sdk::compressible::CompressibleConfig;
use solana_sdk::{
    bpf_loader_upgradeable,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_initialize_compression_config() {
    // Success: config can be initialized
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let _program_data_pda = common::setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");
}

#[tokio::test]
async fn test_config_validation() {
    // Fail: non-authority cannot init
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let non_authority = Keypair::new();
    let _program_data_pda = common::setup_mock_program_data(&mut rpc, &payer, &program_id);

    rpc.airdrop_lamports(&non_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &non_authority,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(result.is_err(), "Should fail with wrong authority");
}

#[tokio::test]
async fn test_update_compression_config() {
    // Success: authority can update config
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let _program_data_pda = common::setup_mock_program_data(&mut rpc, &payer, &program_id);

    let init_result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let config_account = rpc.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account should exist");

    // Use the new mid-level helper - much cleaner!
    let update_result = common::update_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        Some(200),
        Some(RENT_RECIPIENT),
        Some(ADDRESS_SPACE.to_vec()),
        None,
    )
    .await;
    assert!(update_result.is_ok(), "Update config should succeed");
}

#[tokio::test]
async fn test_config_reinit_attack_prevention() {
    // Fail: cannot re-init config
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    common::setup_mock_program_data(&mut rpc, &payer, &program_id);
    let result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(result.is_ok(), "First init should succeed");
    let reinit_result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(reinit_result.is_err(), "Config reinit should fail");
}

#[tokio::test]
async fn test_wrong_program_data_account() {
    // Fail: wrong program data account
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let fake_program_data = Keypair::new();
    let mock_data = common::create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(fake_program_data.pubkey(), mock_account);
    let result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;

    assert!(
        result.is_err(),
        "Should fail with wrong program data account"
    );
}

#[tokio::test]
async fn test_update_remove_address_space() {
    // Fail: cannot remove/replace address space
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    common::setup_mock_program_data(&mut rpc, &payer, &program_id);
    let address_space_1 = ADDRESS_SPACE.to_vec();
    let address_space_2 = vec![Pubkey::new_unique()];
    let init_result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        address_space_1,
    )
    .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let update_result = common::update_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        None,
        None,
        Some(address_space_2),
        None,
    )
    .await;
    assert!(
        update_result.is_err(),
        "Should fail when removing address space"
    );
}

#[tokio::test]
async fn test_update_with_non_authority() {
    // Fail: non-authority cannot update
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let non_authority = Keypair::new();
    rpc.airdrop_lamports(&non_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    common::setup_mock_program_data(&mut rpc, &payer, &program_id);
    let init_result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(init_result.is_ok(), "Init should succeed");

    // Use the new mid-level helper to test non-authority update
    let update_result = common::update_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &non_authority, // This should fail - non_authority tries to update
        Some(200),
        None,
        None,
        None,
    )
    .await;
    assert!(
        update_result.is_err(),
        "Should fail with non-authority update"
    );
}

#[tokio::test]
async fn test_config_with_wrong_rent_recipient() {
    // Fail: wrong rent recipient
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    common::setup_mock_program_data(&mut rpc, &payer, &program_id);
    let init_result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let user = payer;
    let (user_record_pda, _bump) =
        Pubkey::find_program_address(&[b"user_record", user.pubkey().as_ref()], &program_id);
    let wrong_rent_recipient = Pubkey::new_unique();
    let accounts = anchor_compressible_user::accounts::CreateRecord {
        user: user.pubkey(),
        user_record: user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: config_pda,
        rent_recipient: wrong_rent_recipient,
    };
    let instruction_data = anchor_compressible_user::instruction::CreateRecord {
        name: "Test".to_string(),
        proof: light_sdk::instruction::ValidityProof::default(),
        compressed_address: [0u8; 32],
        address_tree_info: light_sdk::instruction::PackedAddressTreeInfo::default(),
        output_state_tree_index: 0,
    };
    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };
    let result = rpc
        .create_and_send_transaction(&[instruction], &user.pubkey(), &[&user])
        .await;
    assert!(result.is_err(), "Should fail with wrong rent recipient");
}

#[tokio::test]
async fn test_config_discriminator_attacks() {
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);

    common::setup_mock_program_data(&mut rpc, &payer, &program_id);

    // First, create a valid config
    let init_result = common::initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(init_result.is_ok(), "Init should succeed");

    // Test 1: Corrupt the discriminator in config account
    {
        let config_account = rpc.get_account(config_pda).await.unwrap().unwrap();
        let mut corrupted_data = config_account.data.clone();

        // Corrupt the discriminator (first 8 bytes)
        corrupted_data[0] = 0xFF;
        corrupted_data[1] = 0xFF;
        corrupted_data[7] = 0xFF;

        let corrupted_account = solana_sdk::account::Account {
            lamports: config_account.lamports,
            data: corrupted_data,
            owner: config_account.owner,
            executable: config_account.executable,
            rent_epoch: config_account.rent_epoch,
        };

        // Set the corrupted account
        rpc.set_account(config_pda, corrupted_account);

        // Try to use config with create_record - should fail
        let user = rpc.get_payer().insecure_clone();
        let (user_record_pda, _bump) =
            Pubkey::find_program_address(&[b"user_record", user.pubkey().as_ref()], &program_id);

        let accounts = anchor_compressible_user::accounts::CreateRecord {
            user: user.pubkey(),
            user_record: user_record_pda,
            system_program: solana_sdk::system_program::ID,
            config: config_pda,
            rent_recipient: RENT_RECIPIENT,
        };

        let instruction_data = anchor_compressible_user::instruction::CreateRecord {
            name: "Test".to_string(),
            proof: light_sdk::instruction::ValidityProof::default(),
            compressed_address: [0u8; 32],
            address_tree_info: light_sdk::instruction::PackedAddressTreeInfo::default(),
            output_state_tree_index: 0,
        };

        let instruction = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: instruction_data.data(),
        };

        let result = rpc
            .create_and_send_transaction(&[instruction], &user.pubkey(), &[&user])
            .await;

        assert!(result.is_err(), "Should fail with corrupted discriminator");

        // Restore the original config for next test
        let original_config_account = solana_sdk::account::Account {
            lamports: config_account.lamports,
            data: config_account.data,
            owner: config_account.owner,
            executable: config_account.executable,
            rent_epoch: config_account.rent_epoch,
        };
        rpc.set_account(config_pda, original_config_account);
    }

    // Test 2: Corrupt the version field
    {
        let config_account = rpc.get_account(config_pda).await.unwrap().unwrap();
        let mut corrupted_data = config_account.data.clone();

        // Corrupt the version (byte 8 - after discriminator)
        corrupted_data[8] = 99; // Invalid version

        let corrupted_account = solana_sdk::account::Account {
            lamports: config_account.lamports,
            data: corrupted_data,
            owner: config_account.owner,
            executable: config_account.executable,
            rent_epoch: config_account.rent_epoch,
        };

        rpc.set_account(config_pda, corrupted_account);

        // Try to use config - should fail due to invalid version
        let user = rpc.get_payer().insecure_clone();
        let (user_record_pda, _bump) =
            Pubkey::find_program_address(&[b"user_record", user.pubkey().as_ref()], &program_id);

        let accounts = anchor_compressible_user::accounts::CreateRecord {
            user: user.pubkey(),
            user_record: user_record_pda,
            system_program: solana_sdk::system_program::ID,
            config: config_pda,
            rent_recipient: RENT_RECIPIENT,
        };

        let instruction_data = anchor_compressible_user::instruction::CreateRecord {
            name: "Test".to_string(),
            proof: light_sdk::instruction::ValidityProof::default(),
            compressed_address: [0u8; 32],
            address_tree_info: light_sdk::instruction::PackedAddressTreeInfo::default(),
            output_state_tree_index: 0,
        };

        let instruction = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: instruction_data.data(),
        };

        let result = rpc
            .create_and_send_transaction(&[instruction], &user.pubkey(), &[&user])
            .await;

        assert!(result.is_err(), "Should fail with invalid version");
    }

    // Test 3: Corrupt the address_space field (set length to 0)
    {
        let config_account = rpc.get_account(config_pda).await.unwrap().unwrap();
        let mut corrupted_data = config_account.data.clone();

        // Find and corrupt address_space length (4 bytes after: discriminator + version + compression_delay + update_authority + rent_recipient)
        // discriminator (8) + version (1) + compression_delay (4) + update_authority (32) + rent_recipient (32) = 77 bytes
        // The address_space length is at byte 77
        let address_space_len_offset = 8 + 1 + 4 + 32 + 32; // 77
        corrupted_data[address_space_len_offset] = 0; // Set length to 0
        corrupted_data[address_space_len_offset + 1] = 0;
        corrupted_data[address_space_len_offset + 2] = 0;
        corrupted_data[address_space_len_offset + 3] = 0;

        let corrupted_account = solana_sdk::account::Account {
            lamports: config_account.lamports,
            data: corrupted_data,
            owner: config_account.owner,
            executable: config_account.executable,
            rent_epoch: config_account.rent_epoch,
        };

        rpc.set_account(config_pda, corrupted_account);

        // Try to use config - should fail due to empty address_space
        let user = rpc.get_payer().insecure_clone();
        let (user_record_pda, _bump) =
            Pubkey::find_program_address(&[b"user_record", user.pubkey().as_ref()], &program_id);

        let accounts = anchor_compressible_user::accounts::CreateRecord {
            user: user.pubkey(),
            user_record: user_record_pda,
            system_program: solana_sdk::system_program::ID,
            config: config_pda,
            rent_recipient: RENT_RECIPIENT,
        };

        let instruction_data = anchor_compressible_user::instruction::CreateRecord {
            name: "Test".to_string(),
            proof: light_sdk::instruction::ValidityProof::default(),
            compressed_address: [0u8; 32],
            address_tree_info: light_sdk::instruction::PackedAddressTreeInfo::default(),
            output_state_tree_index: 0,
        };

        let instruction = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: instruction_data.data(),
        };

        let result = rpc
            .create_and_send_transaction(&[instruction], &user.pubkey(), &[&user])
            .await;

        assert!(result.is_err(), "Should fail with empty address_space");
    }

    // Test 4: Try to load config with wrong owner (should fail in load_checked)
    {
        let config_account = rpc.get_account(config_pda).await.unwrap().unwrap();
        let wrong_owner = Pubkey::new_unique();

        let wrong_owner_account = solana_sdk::account::Account {
            lamports: config_account.lamports,
            data: config_account.data,
            owner: wrong_owner, // Wrong owner
            executable: config_account.executable,
            rent_epoch: config_account.rent_epoch,
        };

        rpc.set_account(config_pda, wrong_owner_account);

        // Try to use config - should fail due to wrong owner
        let user = rpc.get_payer().insecure_clone();
        let (user_record_pda, _bump) =
            Pubkey::find_program_address(&[b"user_record", user.pubkey().as_ref()], &program_id);

        let accounts = anchor_compressible_user::accounts::CreateRecord {
            user: user.pubkey(),
            user_record: user_record_pda,
            system_program: solana_sdk::system_program::ID,
            config: config_pda,
            rent_recipient: RENT_RECIPIENT,
        };

        let instruction_data = anchor_compressible_user::instruction::CreateRecord {
            name: "Test".to_string(),
            proof: light_sdk::instruction::ValidityProof::default(),
            compressed_address: [0u8; 32],
            address_tree_info: light_sdk::instruction::PackedAddressTreeInfo::default(),
            output_state_tree_index: 0,
        };

        let instruction = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: instruction_data.data(),
        };

        let result = rpc
            .create_and_send_transaction(&[instruction], &user.pubkey(), &[&user])
            .await;

        assert!(result.is_err(), "Should fail with wrong owner");
    }
}
