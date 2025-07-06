#![cfg(feature = "test-sbf")]

use anchor_compressible_user::{CompressedUserRecord, UserRecord};
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
            "anchor_compressible_user",
            anchor_compressible_user::ID,
        )]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create some compressed accounts first (you'd need to implement this)
    // For this test, we'll assume we have compressed accounts ready

    // Prepare test data
    let compressed_accounts = vec![
        // These would be actual compressed accounts from the indexer
        // For now, we'll create mock data
    ];

    // Setup remaining accounts
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(anchor_compressible_user::ID);
    remaining_accounts.add_system_accounts(config);

    // Get validity proof
    let hashes: Vec<[u8; 32]> = compressed_accounts.iter().map(|acc| acc.hash).collect();

    let rpc_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos
    let _ = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Create PDA accounts that will receive the decompressed data
    let pda_accounts = vec![
        // These would be the PDA addresses to decompress into
    ];

    // Build instruction
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = anchor_compressible_user::instruction::DecompressMultiplePdas {
        proof: rpc_result.proof,
        compressed_accounts: vec![], // Would contain actual compressed account data
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

    assert!(result.is_ok(), "Transaction should succeed");

    // Verify PDAs were decompressed correctly
    // You would check that the PDAs now contain the expected data
}
