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

/// Creates a mock program data account for testing
fn create_mock_program_data(authority: Pubkey) -> Vec<u8> {
    // Create a larger buffer to match what the SDK expects
    let mut data = vec![0u8; 1024]; // Larger buffer
                                    // Set discriminator to 3 (ProgramData variant)
    data[0..4].copy_from_slice(&3u32.to_le_bytes());
    // Set slot (8 bytes) - can be 0 for testing
    data[4..12].copy_from_slice(&0u64.to_le_bytes());
    // Set authority exists flag
    data[12] = 1;
    // Set authority pubkey
    data[13..45].copy_from_slice(authority.as_ref());
    data
}

#[tokio::test]
async fn test_initialize_config() {
    let program_id = anchor_compressible_user::ID;

    // Set up the test environment with light-program-test
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive config PDA
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);

    let pk = Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
    println!("bpf_loader_upgradeable: {:?}", bpf_loader_upgradeable::ID);
    // Derive program data account
    let (program_data_pda, _) = Pubkey::find_program_address(&[program_id.as_ref()], &pk);

    // Create mock program data account with payer as upgrade authority
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: pk,
        executable: false,
        rent_epoch: 0,
    };

    // Set the mock account in the test environment
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

    // Execute the transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    // Now this should succeed with the mock program data account
    assert!(
        result.is_ok(),
        "Initialize config should succeed with mock program data account"
    );
}

#[tokio::test]
async fn test_config_validation() {
    let program_id = anchor_compressible_user::ID;

    // Set up the test environment with light-program-test
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a non-authority keypair
    let non_authority = Keypair::new();

    // Derive PDAs
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);

    // Create mock program data account with payer as upgrade authority (not non_authority)
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };

    // Set the mock account in the test environment
    rpc.set_account(program_data_pda, mock_account);

    // Fund the non-authority account
    rpc.airdrop_lamports(&non_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Try to create config with non-authority (should fail)
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
    let program_id = anchor_compressible_user::ID;

    // Set up the test environment with light-program-test
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive PDAs
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);

    // Create mock program data account with payer as upgrade authority
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };

    // Set the mock account in the test environment
    rpc.set_account(program_data_pda, mock_account);

    // First, initialize the config
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

    // Execute the initialization
    let init_result = rpc
        .create_and_send_transaction(&[init_instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(
        init_result.is_ok(),
        "Initialize config should succeed: {:?}",
        init_result.err()
    );

    // Verify config was created successfully
    let config_account = rpc.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account should exist");

    println!("✓ Config account created successfully");
    println!(
        "  Account data length: {}",
        config_account.as_ref().unwrap().data.len()
    );
    println!("  Expected max length: {}", CompressibleConfig::LEN);

    // Now test updating the config
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

    // Execute the update transaction
    let update_result = rpc
        .create_and_send_transaction(&[update_instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(
        update_result.is_ok(),
        "Update config should succeed: {:?}",
        update_result.err()
    );

    println!("✓ Config updated successfully");
}
