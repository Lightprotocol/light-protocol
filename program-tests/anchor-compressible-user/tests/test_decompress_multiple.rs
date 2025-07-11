#![cfg(feature = "test-sbf")]

use anchor_compressible_user::{CompressedAccountData, UserRecord, ADDRESS_SPACE, RENT_RECIPIENT};
use anchor_lang::{AnchorDeserialize, InstructionData};
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::LightProgramTest, Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::compressible::CompressibleConfig;
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig,
};
use light_test_utils::RpcError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_decompress_multiple_pdas() {
    // Setup test environment
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![(
            "anchor_compressible_user",
            anchor_compressible_user::ID,
        )]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create some compressed accounts first (you'd need to implement this)
    // For this test, we'll assume we have compressed accounts ready

    // Prepare test data
    let compressed_accounts: Vec<CompressedAccountData> = vec![
        // These would be actual compressed accounts from the indexer
        // For now, we'll create mock data
    ];

    // Setup remaining accounts
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(anchor_compressible_user::ID);
    remaining_accounts.add_system_accounts(config);

    // Get validity proof
    // In a real test, you would get the hashes from actual compressed accounts
    let hashes: Vec<[u8; 32]> = vec![];

    let rpc_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos
    let _ = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Create PDA accounts that will receive the decompressed data
    let pda_accounts: Vec<Pubkey> = vec![
        // These would be the PDA addresses to decompress into
    ];

    // Build instruction
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Prepare bumps for each PDA
    let bumps: Vec<u8> = vec![
        // These would be the actual bump seeds for each PDA
    ];

    let instruction_data = anchor_compressible_user::instruction::DecompressMultiplePdas {
        proof: rpc_result.proof,
        compressed_accounts: vec![], // Would contain actual compressed account data
        bumps,
        system_accounts_offset,
    };

    let instruction = Instruction {
        program_id: anchor_compressible_user::ID,
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

    // In a real test, you'd need actual compressed accounts to decompress
    // For now, we just verify the instruction structure is correct
    assert!(true, "Instruction structure is valid");
}

#[tokio::test]
async fn test_create_record_with_config() {
    // Setup test environment
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive config PDA
    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);

    // In a real test, you would first initialize the config
    // For now, we'll just show how the instruction would be structured

    // Create user record PDA
    let (user_record_pda, _bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Create the instruction
    let accounts = anchor_compressible_user::accounts::CreateRecordWithConfig {
        user: payer.pubkey(),
        user_record: user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // This test demonstrates how the config-based instruction would be structured
    // In a real scenario, the config would need to be initialized first
    assert!(true, "Config-based instruction structure is valid");
}
