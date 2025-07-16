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

use anchor_compressible_user::{ADDRESS_SPACE, RENT_RECIPIENT};
use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
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

/// Create mock program data account
fn create_mock_program_data(authority: Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 1024];
    data[0..4].copy_from_slice(&3u32.to_le_bytes());
    data[4..12].copy_from_slice(&0u64.to_le_bytes());
    data[12] = 1;
    data[13..45].copy_from_slice(authority.as_ref());
    data
}

#[tokio::test]
async fn test_initialize_config() {
    // Success: config can be initialized
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let pk = Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
    let (program_data_pda, _) = Pubkey::find_program_address(&[program_id.as_ref()], &pk);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: pk,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    let accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
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
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    rpc.airdrop_lamports(&non_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: non_authority.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &non_authority])
        .await;
    assert!(result.is_err(), "Should fail with wrong authority");
}

#[tokio::test]
async fn test_update_config() {
    // Success: authority can update config
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    let init_accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let init_instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let init_instruction = Instruction {
        program_id,
        accounts: init_accounts.to_account_metas(None),
        data: init_instruction_data.data(),
    };
    let init_result = rpc
        .create_and_send_transaction(&[init_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let config_account = rpc.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account should exist");
    let update_accounts = anchor_compressible_user::accounts::UpdateConfigSettings {
        config: config_pda,
        authority: payer.pubkey(),
    };
    let update_instruction_data = anchor_compressible_user::instruction::UpdateConfigSettings {
        new_compression_delay: Some(200),
        new_rent_recipient: Some(RENT_RECIPIENT),
        new_address_space: Some(ADDRESS_SPACE),
        new_update_authority: None,
    };
    let update_instruction = Instruction {
        program_id,
        accounts: update_accounts.to_account_metas(None),
        data: update_instruction_data.data(),
    };
    let update_result = rpc
        .create_and_send_transaction(&[update_instruction], &payer.pubkey(), &[&payer])
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
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    let accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };
    let result = rpc
        .create_and_send_transaction(&[instruction.clone()], &payer.pubkey(), &[&payer])
        .await;
    assert!(result.is_ok(), "First init should succeed");
    let reinit_result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
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
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let fake_program_data = Keypair::new();
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(fake_program_data.pubkey(), mock_account);
    let accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: fake_program_data.pubkey(),
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
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
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    let address_space_1 = ADDRESS_SPACE;
    let address_space_2 = Pubkey::new_unique();
    let init_accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let init_instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: address_space_1,
    };
    let init_instruction = Instruction {
        program_id,
        accounts: init_accounts.to_account_metas(None),
        data: init_instruction_data.data(),
    };
    let init_result = rpc
        .create_and_send_transaction(&[init_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let update_accounts = anchor_compressible_user::accounts::UpdateConfigSettings {
        config: config_pda,
        authority: payer.pubkey(),
    };
    let update_instruction_data = anchor_compressible_user::instruction::UpdateConfigSettings {
        new_compression_delay: None,
        new_rent_recipient: None,
        new_address_space: Some(address_space_2),
        new_update_authority: None,
    };
    let update_instruction = Instruction {
        program_id,
        accounts: update_accounts.to_account_metas(None),
        data: update_instruction_data.data(),
    };
    let update_result = rpc
        .create_and_send_transaction(&[update_instruction], &payer.pubkey(), &[&payer])
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
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    let init_accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let init_instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let init_instruction = Instruction {
        program_id,
        accounts: init_accounts.to_account_metas(None),
        data: init_instruction_data.data(),
    };
    let init_result = rpc
        .create_and_send_transaction(&[init_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let update_accounts = anchor_compressible_user::accounts::UpdateConfigSettings {
        config: config_pda,
        authority: non_authority.pubkey(),
    };
    let update_instruction_data = anchor_compressible_user::instruction::UpdateConfigSettings {
        new_compression_delay: Some(200),
        new_rent_recipient: None,
        new_address_space: None,
        new_update_authority: None,
    };
    let update_instruction = Instruction {
        program_id,
        accounts: update_accounts.to_account_metas(None),
        data: update_instruction_data.data(),
    };
    let update_result = rpc
        .create_and_send_transaction(
            &[update_instruction],
            &payer.pubkey(),
            &[&payer, &non_authority],
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
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    let init_accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let init_instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay: 100,
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
    };
    let init_instruction = Instruction {
        program_id,
        accounts: init_accounts.to_account_metas(None),
        data: init_instruction_data.data(),
    };
    let init_result = rpc
        .create_and_send_transaction(&[init_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(init_result.is_ok(), "Init should succeed");
    let user = payer;
    let (user_record_pda, _bump) =
        Pubkey::find_program_address(&[b"user_record", user.pubkey().as_ref()], &program_id);
    let wrong_rent_recipient = Pubkey::new_unique();
    let accounts = anchor_compressible_user::accounts::CreateRecordWithConfig {
        user: user.pubkey(),
        user_record: user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: config_pda,
        rent_recipient: wrong_rent_recipient,
    };
    let instruction_data = anchor_compressible_user::instruction::CreateRecordWithConfig {
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
