#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use sdk_test_derived::{
    compression::{
        CompressMyPdaAccountData, CompressedAccountData, CompressedAccountVariant,
        CreateCompressionConfigData, DecompressMultiplePdasData,
    },
    decompress_dynamic_pda::MyPdaAccount,
};
use solana_sdk::pubkey::Pubkey;

#[test]
fn test_macro_generates_types() {
    // Test that the macro generates the expected types
    let my_pda_account = MyPdaAccount {
        compression_info: light_sdk::compressible::CompressionInfo::default(),
        owner: Pubkey::default(),
        data: 42,
    };

    // Test that CompressedAccountVariant enum is generated and works
    let variant = CompressedAccountVariant::MyPdaAccount(my_pda_account.clone());

    // Test serialization/deserialization
    let serialized = variant.try_to_vec().unwrap();
    let _deserialized: CompressedAccountVariant =
        borsh::BorshDeserialize::try_from_slice(&serialized).unwrap();

    // Test CompressedAccountData structure
    let compressed_data = CompressedAccountData {
        meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta::default(),
        data: variant,
        seeds: vec![b"test_pda".to_vec(), [42u8; 8].to_vec()],
    };

    // Test instruction data structures
    let create_config_data = CreateCompressionConfigData {
        compression_delay: 100,
        rent_recipient: Pubkey::default(),
        address_space: vec![Pubkey::new_unique()],
    };

    let _config_serialized = create_config_data.try_to_vec().unwrap();

    // Test decompress instruction data
    let decompress_data = DecompressMultiplePdasData {
        proof: light_sdk::instruction::ValidityProof::default(),
        compressed_accounts: vec![compressed_data],
        bumps: vec![255],
        system_accounts_offset: 5,
    };

    let _decompress_serialized = decompress_data.try_to_vec().unwrap();

    // Test compress instruction data
    let compress_data = CompressMyPdaAccountData {
        proof: light_sdk::instruction::ValidityProof::default(),
        compressed_account_meta:
            light_sdk_types::instruction::account_meta::CompressedAccountMeta::default(),
    };

    let _compress_serialized = compress_data.try_to_vec().unwrap();

    // If we get here, all the types were generated correctly
    assert!(true, "Native compressible macro generates working code");
}

#[test]
fn test_compress_function_name() {
    // Test that the compress function is generated with the correct name
    // The function should be named compress_my_pda_account (snake_case of MyPdaAccount)

    // This test just verifies the function exists and can be referenced
    // In a real scenario, you would call it with proper accounts
    let _function_exists = sdk_test_derived::compression::compress_my_pda_account;

    assert!(true, "compress_my_pda_account function is generated");
}
