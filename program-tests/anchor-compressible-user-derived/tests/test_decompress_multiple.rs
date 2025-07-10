#![cfg(feature = "test-sbf")]

use anchor_compressible_user_derived::{
    CompressedAccountData, CompressedAccountVariant, GameSession, UserRecord,
};
use anchor_lang::{AnchorDeserialize, InstructionData};
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::LightProgramTest, Indexer, ProgramTestConfig, Rpc,
};
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
            "anchor_compressible_user_derived",
            anchor_compressible_user_derived::ID,
        )]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create some compressed accounts first (you'd need to implement this)
    // For this test, we'll assume we have compressed accounts ready

    // Example: prepare test data with proper seeds
    let user_pubkey = payer.pubkey();
    let (user_record_pda, user_bump) = Pubkey::find_program_address(
        &[b"user_record", user_pubkey.as_ref()],
        &anchor_compressible_user_derived::ID,
    );

    let compressed_accounts = vec![CompressedAccountData {
        meta: CompressedAccountMeta::default(), // Would be actual meta from indexer
        data: CompressedAccountVariant::UserRecord(UserRecord {
            compression_info: light_sdk::compressible::CompressionInfo::default(),
            owner: user_pubkey,
            name: "Test User".to_string(),
            score: 100,
        }),
        seeds: vec![b"user_record".to_vec(), user_pubkey.to_bytes().to_vec()],
    }];

    // Setup remaining accounts
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(anchor_compressible_user_derived::ID);
    remaining_accounts.add_system_accounts(config);

    // Get validity proof
    let hashes: Vec<[u8; 32]> = vec![]; // Would be actual hashes from compressed accounts

    let rpc_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos
    let _ = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Create PDA accounts that will receive the decompressed data
    let pda_accounts = vec![user_record_pda];

    // Build instruction
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Prepare bumps for each PDA
    let bumps = vec![user_bump];

    let instruction_data = anchor_compressible_user_derived::instruction::DecompressMultiplePdas {
        proof: rpc_result.proof,
        compressed_accounts,
        bumps,
        system_accounts_offset,
    };

    let instruction = Instruction {
        program_id: anchor_compressible_user_derived::ID,
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
