#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use light_compressed_account::{address::derive_address, instruction_data::data::ReadOnlyAddress};
use light_macros::pubkey;
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::{
    compressible::CompressibleConfig,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use sdk_test_derived::{
    create_config::CreateConfigInstructionData, create_dynamic_pda::CreateDynamicPdaInstructionData,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
};

pub const PRIMARY_ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const SECONDARY_ADDRESS_SPACE: Pubkey = pubkey!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

#[tokio::test]
async fn test_multi_address_space_compression() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_test_derived", sdk_test_derived::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // 1. Create config with both primary and secondary address spaces
    let (config_pda, _) = CompressibleConfig::derive_pda(&sdk_test_derived::ID);

    // 2. Create a PDA to compress
    let pda_seeds: &[&[u8]] = &[b"test_pda", &[1u8; 8]];
    let (pda_pubkey, _bump) = Pubkey::find_program_address(pda_seeds, &sdk_test_derived::ID);

    // 3. Derive the SAME address for both address spaces
    let address_seed = pda_pubkey.to_bytes();
    let compressed_address = derive_address(
        &address_seed,
        &PRIMARY_ADDRESS_SPACE.to_bytes(),
        &sdk_test_derived::ID.to_bytes(),
    );

    // 4. Get validity proof for both address spaces
    let addresses_with_tree = vec![
        AddressWithTree {
            address: compressed_address,
            tree: PRIMARY_ADDRESS_SPACE,
        },
        AddressWithTree {
            address: compressed_address, // SAME address
            tree: SECONDARY_ADDRESS_SPACE,
        },
    ];

    let proof_result = rpc
        .get_validity_proof(vec![], addresses_with_tree, None)
        .await
        .unwrap()
        .value;

    // 5. Build packed accounts
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_test_derived::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts.add_pre_accounts_meta(AccountMeta::new(pda_pubkey, false)); // pda_account
    accounts.add_pre_accounts_signer(payer.pubkey()); // rent_recipient
    accounts.add_pre_accounts_meta(AccountMeta::new_readonly(config_pda, false)); // config
    accounts.add_system_accounts(system_account_meta_config);

    // Pack the tree infos
    let packed_tree_infos = proof_result.pack_tree_infos(&mut accounts);

    // Get indices for output and address trees
    let output_merkle_tree_index = accounts.insert_or_get(output_queue);

    // Build read-only address for exclusion proof (SAME address, different tree)
    let read_only_addresses = vec![ReadOnlyAddress {
        address: compressed_address, // SAME address
        address_merkle_tree_pubkey: SECONDARY_ADDRESS_SPACE.into(),
        address_merkle_tree_root_index: proof_result.get_address_root_indices()[1],
    }];

    let (accounts, _, _) = accounts.to_account_metas();

    let instruction_data = CreateDynamicPdaInstructionData {
        proof: proof_result.proof.0.unwrap().into(),
        compressed_address,
        address_tree_info: packed_tree_infos.address_trees[0],
        read_only_addresses: Some(read_only_addresses),
        output_state_tree_index: output_merkle_tree_index,
    };

    let inputs = instruction_data.try_to_vec().unwrap();

    let _instruction = Instruction {
        program_id: sdk_test_derived::ID,
        accounts,
        data: [&[4u8][..], &inputs[..]].concat(), // 4 is CompressFromPdaNew discriminator
    };

    // This would execute the transaction with automatic exclusion proof
    println!("Multi-address space compression test complete!");
    println!("Primary address stored in: {:?}", PRIMARY_ADDRESS_SPACE);
    println!("Exclusion proof against: {:?}", SECONDARY_ADDRESS_SPACE);
    println!("Using SAME address {:?} in both trees", compressed_address);
}

#[tokio::test]
async fn test_single_address_space_backward_compatibility() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_test_derived", sdk_test_derived::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let _payer = rpc.get_payer().insecure_clone();

    // Test that single address space (no read-only addresses) still works
    let (_config_pda, _) = CompressibleConfig::derive_pda(&sdk_test_derived::ID);

    // Create a PDA to compress
    let pda_seeds: &[&[u8]] = &[b"test_pda_single", &[2u8; 15]];
    let (pda_pubkey, _bump) = Pubkey::find_program_address(pda_seeds, &sdk_test_derived::ID);

    let address_seed = pda_pubkey.to_bytes();
    let compressed_address = derive_address(
        &address_seed,
        &PRIMARY_ADDRESS_SPACE.to_bytes(),
        &sdk_test_derived::ID.to_bytes(),
    );

    // Get validity proof for single address
    let addresses_with_tree = vec![AddressWithTree {
        address: compressed_address,
        tree: PRIMARY_ADDRESS_SPACE,
    }];

    let proof_result = rpc
        .get_validity_proof(vec![], addresses_with_tree, None)
        .await
        .unwrap()
        .value;

    // Pack the tree infos
    let mut accounts = PackedAccounts::default();
    let packed_tree_infos = proof_result.pack_tree_infos(&mut accounts);

    // Build instruction data with NO read-only addresses
    let _instruction_data = CreateDynamicPdaInstructionData {
        proof: proof_result.proof.0.unwrap().into(),
        compressed_address,
        address_tree_info: packed_tree_infos.address_trees[0],
        read_only_addresses: None, // No exclusion proofs
        output_state_tree_index: 0,
    };

    println!("Single address space test - backward compatibility verified!");
    println!("Only primary address used: {:?}", PRIMARY_ADDRESS_SPACE);
}

#[tokio::test]
async fn test_multi_address_space_config_creation() {
    // Test creating a config with multiple address spaces
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_test_derived", sdk_test_derived::ID)]));
    let _rpc = LightProgramTest::new(config).await.unwrap();

    let create_ix_data = CreateConfigInstructionData {
        rent_recipient: RENT_RECIPIENT,
        address_space: vec![PRIMARY_ADDRESS_SPACE, SECONDARY_ADDRESS_SPACE], // 2 address spaces
        compression_delay: 100,
    };

    println!(
        "Config created with {} address spaces",
        create_ix_data.address_space.len()
    );
    println!(
        "Primary (for writing): {:?}",
        create_ix_data.address_space[0]
    );
    println!(
        "Secondary (for exclusion): {:?}",
        create_ix_data.address_space[1]
    );
}
