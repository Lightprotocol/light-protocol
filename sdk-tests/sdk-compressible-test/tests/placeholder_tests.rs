use anchor_lang::{AccountDeserialize, Discriminator, InstructionData, ToAccountMetas};
use light_compressed_account::address::derive_address;
use light_compressible_client::compressible_instruction;
use light_program_test::{
    program_test::{
        initialize_compression_config, setup_mock_program_data, LightProgramTest, TestRpc,
    },
    Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::{
    compressible::CompressibleConfig,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use solana_account::Account;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

mod helpers;
use helpers::{ADDRESS_SPACE, RENT_SPONSOR};

// Tests for the simplest possible compression flows:
// 1. Create empty compressed account (do not compress at init)
// 2. Idempotent double compression
#[tokio::test]
async fn test_create_empty_compressed_account() {
    let program_id = sdk_compressible_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &compressible_instruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let placeholder_id = 54321u64;
    let (placeholder_record_pda, placeholder_record_bump) = Pubkey::find_program_address(
        &[b"placeholder_record", placeholder_id.to_le_bytes().as_ref()],
        &program_id,
    );

    create_placeholder_record(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &placeholder_record_pda,
        placeholder_id,
        "Test Placeholder",
    )
    .await;

    let placeholder_pda_account = rpc.get_account(placeholder_record_pda).await.unwrap();
    assert!(
        placeholder_pda_account.is_some(),
        "Placeholder PDA should exist after empty compression"
    );
    let account = placeholder_pda_account.unwrap();
    assert!(
        account.lamports > 0,
        "Placeholder PDA should have lamports (not closed)"
    );
    assert!(
        !account.data.is_empty(),
        "Placeholder PDA should have data (not closed)"
    );

    let placeholder_data = account.data;
    let decompressed_placeholder_record =
        sdk_compressible_test::PlaceholderRecord::try_deserialize(&mut &placeholder_data[..])
            .unwrap();
    assert_eq!(decompressed_placeholder_record.name, "Test Placeholder");
    assert_eq!(
        decompressed_placeholder_record.placeholder_id,
        placeholder_id
    );
    assert_eq!(decompressed_placeholder_record.owner, payer.pubkey());

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_placeholder = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(
        compressed_placeholder.address,
        Some(compressed_address),
        "Compressed account should exist with correct address"
    );
    assert!(
        compressed_placeholder.data.is_some(),
        "Compressed account should have data field"
    );

    let compressed_data = compressed_placeholder.data.unwrap();
    assert_eq!(
        compressed_data.data.len(),
        0,
        "Compressed account data should be empty"
    );

    rpc.warp_to_slot(200).unwrap();

    compress_placeholder_record(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &placeholder_record_pda,
        &placeholder_record_bump,
        placeholder_id,
    )
    .await;
}

#[tokio::test]
async fn test_double_compression_attack() {
    let program_id = sdk_compressible_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &compressible_instruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let placeholder_id = 99999u64;
    let (placeholder_record_pda, _placeholder_record_bump) = Pubkey::find_program_address(
        &[b"placeholder_record", placeholder_id.to_le_bytes().as_ref()],
        &program_id,
    );

    create_placeholder_record(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &placeholder_record_pda,
        placeholder_id,
        "Double Compression Test",
    )
    .await;

    let placeholder_pda_account = rpc.get_account(placeholder_record_pda).await.unwrap();
    assert!(
        placeholder_pda_account.is_some(),
        "Placeholder PDA should exist before compression"
    );
    let account_before = placeholder_pda_account.unwrap();
    assert!(
        account_before.lamports > 0,
        "Placeholder PDA should have lamports before compression"
    );
    assert!(
        !account_before.data.is_empty(),
        "Placeholder PDA should have data before compression"
    );

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_placeholder_before = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(
        compressed_placeholder_before.address,
        Some(compressed_address),
        "Empty compressed account should exist"
    );
    assert_eq!(
        compressed_placeholder_before
            .data
            .as_ref()
            .unwrap()
            .data
            .len(),
        0,
        "Compressed account should be empty initially"
    );

    rpc.warp_to_slot(200).unwrap();

    let first_compression_result = compress_placeholder_record_for_double_test(
        &mut rpc,
        &payer,
        &program_id,
        &placeholder_record_pda,
        placeholder_id,
        Some(account_before.clone()),
    )
    .await;
    assert!(
        first_compression_result.is_ok(),
        "First compression should succeed: {:?}",
        first_compression_result
    );

    let placeholder_pda_after_first = rpc.get_account(placeholder_record_pda).await.unwrap();
    assert!(
        placeholder_pda_after_first.is_none(),
        "PDA should not exist after first compression"
    );

    let compressed_placeholder_after_first = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let first_data_len = compressed_placeholder_after_first
        .data
        .as_ref()
        .unwrap()
        .data
        .len();
    assert!(
        first_data_len > 0,
        "Compressed account should contain data after first compression"
    );

    let second_compression_result = compress_placeholder_record_for_double_test(
        &mut rpc,
        &payer,
        &program_id,
        &placeholder_record_pda,
        placeholder_id,
        Some(account_before),
    )
    .await;

    assert!(
        second_compression_result.is_ok(),
        "Second compression should succeed idempotently: {:?}",
        second_compression_result
    );

    let placeholder_pda_after_second = rpc.get_account(placeholder_record_pda).await.unwrap();
    assert!(
        placeholder_pda_after_second.is_none(),
        "PDA should still not exist after second compression"
    );

    let compressed_placeholder_after_second = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(
        compressed_placeholder_after_first.hash, compressed_placeholder_after_second.hash,
        "Compressed account hash should be unchanged after second compression"
    );
    assert_eq!(
        compressed_placeholder_after_first
            .data
            .as_ref()
            .unwrap()
            .data,
        compressed_placeholder_after_second
            .data
            .as_ref()
            .unwrap()
            .data,
        "Compressed account data should be unchanged after second compression"
    );
}

pub async fn create_placeholder_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    placeholder_record_pda: &Pubkey,
    placeholder_id: u64,
    name: &str,
) {
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let accounts = sdk_compressible_test::accounts::CreatePlaceholderRecord {
        user: payer.pubkey(),
        placeholder_record: *placeholder_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_sponsor: RENT_SPONSOR,
    };

    let compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_program_test::AddressWithTree {
                address: compressed_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let address_tree_info = packed_tree_infos.address_trees[0];

    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = sdk_compressible_test::instruction::CreatePlaceholderRecord {
        placeholder_id,
        name: name.to_string(),
        proof: rpc_result.proof,
        compressed_address,
        address_tree_info,
        output_state_tree_index,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(
        result.is_ok(),
        "CreatePlaceholderRecord transaction should succeed"
    );
}

pub async fn compress_placeholder_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    _config_pda: &Pubkey,
    placeholder_record_pda: &Pubkey,
    _placeholder_record_bump: &u8,
    placeholder_id: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let placeholder_compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_placeholder = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![compressed_placeholder.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let _placeholder_seeds = sdk_compressible_test::get_placeholderrecord_seeds(placeholder_id);

    let account = rpc
        .get_account(*placeholder_record_pda)
        .await
        .unwrap()
        .unwrap();

    let instruction =
        light_compressible_client::compressible_instruction::compress_accounts_idempotent(
            program_id,
            sdk_compressible_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
            &[*placeholder_record_pda],
            &[account],
            &sdk_compressible_test::accounts::CompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_sponsor: RENT_SPONSOR,
            }
            .to_account_metas(None),
            rpc_result,
        )
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(
        result.is_ok(),
        "CompressPlaceholderRecord transaction should succeed: {:?}",
        result
    );

    let _account = rpc.get_account(*placeholder_record_pda).await.unwrap();

    let compressed_placeholder_after = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert!(
        compressed_placeholder_after.data.is_some(),
        "Compressed account should have data after compression"
    );

    let compressed_data_after = compressed_placeholder_after.data.unwrap();

    assert!(
        !compressed_data_after.data.is_empty(),
        "Compressed account should contain the PDA data"
    );
}

pub async fn compress_placeholder_record_for_double_test(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    placeholder_record_pda: &Pubkey,
    placeholder_id: u64,
    previous_account: Option<Account>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let placeholder_compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_placeholder = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![compressed_placeholder.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let _placeholder_seeds = sdk_compressible_test::get_placeholderrecord_seeds(placeholder_id);

    let accounts_to_compress = if let Some(account) = previous_account {
        vec![account]
    } else {
        panic!("Previous account should be provided");
    };
    let instruction =
        light_compressible_client::compressible_instruction::compress_accounts_idempotent(
            program_id,
            sdk_compressible_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
            &[*placeholder_record_pda],
            &accounts_to_compress,
            &sdk_compressible_test::accounts::CompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_sponsor: RENT_SPONSOR,
            }
            .to_account_metas(None),
            rpc_result,
        )
        .unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
