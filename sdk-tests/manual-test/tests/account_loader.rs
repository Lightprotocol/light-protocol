//! Integration test for zero-copy AccountLoader support.
//!
//! Tests the full lifecycle: create -> compress -> decompress
//! for zero-copy accounts (ZeroCopyRecord).

mod shared;

use anchor_lang::{Discriminator, InstructionData, ToAccountMetas};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt, AccountSpec,
    CreateAccountsProofInput, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Indexer, Rpc};
use light_sdk::interface::IntoVariant;
use manual_test::{
    CreateZeroCopyParams, ZeroCopyRecord, ZeroCopyRecordSeeds, ZeroCopyRecordVariant,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test the full lifecycle for zero-copy accounts: create -> compress -> decompress.
#[tokio::test]
async fn test_zero_copy_create_compress_decompress() {
    let program_id = manual_test::ID;
    let (mut rpc, payer, config_pda) = shared::setup_test_env().await;

    let owner = Keypair::new().pubkey();
    let value: u64 = 12345;
    let name = "my_record".to_string();

    // Derive PDA for zero-copy record
    let (record_pda, _) = Pubkey::find_program_address(
        &[b"zero_copy", owner.as_ref(), name.as_bytes()],
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

    let accounts = manual_test::accounts::CreateZeroCopy {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = manual_test::instruction::CreateZeroCopy {
        params: CreateZeroCopyParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            value,
            name: name.clone(),
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateZeroCopy should succeed");

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
    let variant = ZeroCopyRecordSeeds {
        owner,
        name: name.clone(),
    }
    .into_variant(&account_interface.account.data[8..])
        .expect("Seed verification failed");

    // Verify the data from the compressed account
    assert_eq!(variant.data.value, value, "Compressed value should match");
    assert_eq!(
        Pubkey::new_from_array(variant.data.owner),
        owner,
        "Compressed owner should match"
    );

    // Build PdaSpec and create decompress instructions
    let spec = PdaSpec::new(account_interface.clone(), variant, program_id);
    let specs: Vec<AccountSpec<ZeroCopyRecordVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, payer.pubkey(), config_pda, payer.pubkey(), &rpc)
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

    // Verify discriminator is set correctly (first 8 bytes)
    let discriminator = &record_account.data[..8];
    assert_eq!(
        discriminator,
        ZeroCopyRecord::DISCRIMINATOR,
        "Discriminator should match ZeroCopyRecord::DISCRIMINATOR after decompression"
    );

    // Verify data is correct (zero-copy uses bytemuck)
    let record_bytes = &record_account.data[8..8 + core::mem::size_of::<ZeroCopyRecord>()];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(record_bytes);

    assert_eq!(
        Pubkey::new_from_array(record.owner),
        owner,
        "Record owner should match after decompression"
    );
    assert_eq!(
        record.value, value,
        "Record value should match after decompression"
    );

    // state should be Decompressed after decompression
    use light_sdk::compressible::CompressionState;
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed after decompression"
    );
}
