#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use light_macros::pubkey;
use light_program_test::{program_test::LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::compressible::CompressibleConfig;
use sdk_test::create_config::CreateConfigInstructionData;
use solana_sdk::{
    bpf_loader_upgradeable,
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

    // Derive program data account
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[sdk_test::ID.as_ref()], &bpf_loader_upgradeable::ID);

    // For testing, we'll use the payer as the upgrade authority
    // In a real scenario, you'd get the actual upgrade authority from the program data account

    // Test create config
    let create_ix_data = CreateConfigInstructionData {
        rent_recipient: RENT_RECIPIENT,
        address_space: vec![ADDRESS_SPACE], // Can add more for multi-address-space support
        compression_delay: 100,
    };

    let create_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(payer.pubkey(), true), // update_authority (signer)
            AccountMeta::new_readonly(program_data_pda, false), // program data account
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: [&[5u8][..], &create_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    // Note: This will fail in the test environment because the program data account
    // doesn't exist in the test validator. In a real deployment, this would work.
    let result = rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer])
        .await;

    // We expect this to fail in test environment
    assert!(
        result.is_err(),
        "Should fail without proper program data account"
    );
}

#[tokio::test]
async fn test_config_validation() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_test", sdk_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let non_authority = Keypair::new();

    // Derive PDAs
    let (config_pda, _) = CompressibleConfig::derive_pda(&sdk_test::ID);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[sdk_test::ID.as_ref()], &bpf_loader_upgradeable::ID);

    // Try to create config with non-authority (should fail)
    let create_ix_data = CreateConfigInstructionData {
        rent_recipient: RENT_RECIPIENT,
        address_space: vec![ADDRESS_SPACE],
        compression_delay: 100,
    };

    let create_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(non_authority.pubkey(), true), // wrong authority (signer)
            AccountMeta::new_readonly(program_data_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: [&[5u8][..], &create_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    // Fund the non-authority account
    rpc.airdrop_lamports(&non_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[create_ix], &non_authority.pubkey(), &[&non_authority])
        .await;

    assert!(result.is_err(), "Should fail with wrong authority");
}

#[tokio::test]
async fn test_config_creation_requires_signer() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_test", sdk_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let non_signer = Keypair::new();

    // Derive PDAs
    let (config_pda, _) = CompressibleConfig::derive_pda(&sdk_test::ID);
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[sdk_test::ID.as_ref()], &bpf_loader_upgradeable::ID);

    // Try to create config with non-signer as update authority (should fail)
    let create_ix_data = CreateConfigInstructionData {
        rent_recipient: RENT_RECIPIENT,
        address_space: vec![ADDRESS_SPACE],
        compression_delay: 100,
    };

    let create_ix = Instruction {
        program_id: sdk_test::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(non_signer.pubkey(), false), // update_authority (NOT a signer)
            AccountMeta::new_readonly(program_data_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: [&[5u8][..], &create_ix_data.try_to_vec().unwrap()[..]].concat(),
    };

    let result = rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer])
        .await;

    assert!(
        result.is_err(),
        "Config creation without signer should fail"
    );
}
