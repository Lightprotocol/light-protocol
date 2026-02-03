//! Integration test for manual Light Protocol implementation.
//!
//! Tests the full lifecycle: create -> compress -> decompress

mod shared;

use light_account_pinocchio::{CompressionState, IntoVariant};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt, AccountSpec,
    CreateAccountsProofInput, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Indexer, Rpc};
use pinocchio_manual_test::{
    pda::accounts::CreatePdaParams, MinimalRecord, MinimalRecordSeeds, MinimalRecordVariant,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_signer::Signer;

/// Test the full lifecycle: create -> compress -> decompress.
#[tokio::test]
async fn test_create_compress_decompress() {
    let program_id = Pubkey::new_from_array(pinocchio_manual_test::ID);
    let (mut rpc, payer, config_pda) = shared::setup_test_env().await;

    let owner = Keypair::new().pubkey();
    let nonce: u64 = 12345;

    // Derive PDA for record
    let (record_pda, _) = Pubkey::find_program_address(
        &[b"minimal_record", owner.as_ref(), &nonce.to_le_bytes()],
        &program_id,
    );

    // Get proof for the PDA
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let params = CreatePdaParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        owner: owner.to_bytes(),
        nonce,
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(record_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let ix = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: [
            pinocchio_manual_test::discriminators::CREATE_PDA.as_slice(),
            &borsh::to_vec(&params).unwrap(),
        ]
        .concat(),
    };

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .expect("CreatePda should succeed");

    // PHASE 1: Verify account exists on-chain
    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Account should exist on-chain after creation"
    );

    // PHASE 2: Warp time to trigger forester auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // Verify account is closed on-chain (compressed by forester)
    let acc = rpc.get_account(record_pda).await.unwrap();
    assert!(
        acc.is_none() || acc.unwrap().lamports == 0,
        "Account should be closed after compression"
    );

    // PHASE 3: Verify compressed account exists
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_address = light_compressed_account::address::derive_address(
        &record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_acc = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        compressed_acc.address.unwrap(),
        compressed_address,
        "Compressed account address should match"
    );
    assert!(
        !compressed_acc.data.as_ref().unwrap().data.is_empty(),
        "Compressed account should have data"
    );

    // PHASE 4: Decompress account
    let account_interface = rpc
        .get_account_interface(&record_pda, &program_id)
        .await
        .expect("failed to get account interface");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    // Build variant using IntoVariant - verify seeds match the compressed data
    let variant = MinimalRecordSeeds {
        owner: owner.to_bytes(),
        nonce,
    }
    .into_variant(&account_interface.account.data[8..])
    .expect("Seed verification failed");

    // Build PdaSpec and create decompress instructions
    let spec = PdaSpec::new(account_interface.clone(), variant, program_id);
    let specs: Vec<AccountSpec<MinimalRecordVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, payer.pubkey(), config_pda, &rpc)
            .await
            .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&decompress_instructions, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain with correct data
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Account should exist after decompression");

    // Verify data is correct
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..])
            .expect("Failed to deserialize MinimalRecord");

    assert_eq!(record.owner, owner.to_bytes(), "Record owner should match");

    // state should be Decompressed after decompression
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed after decompression"
    );
}
