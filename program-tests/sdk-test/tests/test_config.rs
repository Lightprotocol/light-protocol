#![cfg(feature = "test-sbf")]

use borsh::{BorshDeserialize, BorshSerialize};
use light_macros::pubkey;
use light_program_test::{program_test::LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::compressible::CompressibleConfig;
use sdk_test::{
    create_config::CreateConfigInstructionData, update_config::UpdateConfigInstructionData,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

pub const ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

#[tokio::test]
async fn test_create_and_update_config() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_test", sdk_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive config PDA
    let (config_pda, _) = CompressibleConfig::derive_pda(&sdk_test::ID);

    // Test create config
    let create_ix_data = CreateConfigInstructionData {
        update_authority: payer.pubkey(),
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
        compression_delay: 100,
    };

    let create_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: [&[5u8][..], &create_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify config was created
    let config_account = rpc.get_account(config_pda).await.unwrap().unwrap();
    let config_data = CompressibleConfig::try_from_slice(&config_account.data).unwrap();
    assert_eq!(config_data.update_authority, payer.pubkey());
    assert_eq!(config_data.rent_recipient, RENT_RECIPIENT);
    assert_eq!(config_data.address_space, ADDRESS_SPACE);
    assert_eq!(config_data.compression_delay, 100);

    // Test update config
    let new_rent_recipient = Pubkey::new_unique();
    let update_ix_data = UpdateConfigInstructionData {
        new_update_authority: None,
        new_rent_recipient: Some(new_rent_recipient),
        new_address_space: None,
        new_compression_delay: Some(200),
    };

    let update_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(payer.pubkey(), true),
        ],
        data: [&[6u8][..], &update_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    rpc.create_and_send_transaction(&[update_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify config was updated
    let config_account = rpc.get_account(config_pda).await.unwrap().unwrap();
    let config_data = CompressibleConfig::try_from_slice(&config_account.data).unwrap();
    assert_eq!(config_data.update_authority, payer.pubkey());
    assert_eq!(config_data.rent_recipient, new_rent_recipient);
    assert_eq!(config_data.address_space, ADDRESS_SPACE);
    assert_eq!(config_data.compression_delay, 200);
}

#[tokio::test]
async fn test_config_validation() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_test", sdk_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let non_authority = Keypair::new();

    // Create config first
    let (config_pda, _) = CompressibleConfig::derive_pda(&sdk_test::ID);
    let create_ix_data = CreateConfigInstructionData {
        update_authority: payer.pubkey(),
        rent_recipient: RENT_RECIPIENT,
        address_space: ADDRESS_SPACE,
        compression_delay: 100,
    };

    let create_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: [&[5u8][..], &create_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Try to update with non-authority (should fail)
    let update_ix_data = UpdateConfigInstructionData {
        new_update_authority: None,
        new_rent_recipient: None,
        new_address_space: None,
        new_compression_delay: Some(300),
    };

    let update_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(non_authority.pubkey(), true),
        ],
        data: [&[6u8][..], &update_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    let result = rpc
        .create_and_send_transaction(&[update_ix], &non_authority.pubkey(), &[&non_authority])
        .await;

    assert!(result.is_err(), "Update with non-authority should fail");
}
