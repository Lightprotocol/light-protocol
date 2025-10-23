use anchor_lang::{
    AccountDeserialize, AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use sdk_compressible_test::UserRecord;
use light_compressed_account::address::derive_address;
use light_compressible_client::CompressibleInstruction;
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
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

mod helpers;
use helpers::{create_record, decompress_single_user_record, ADDRESS_SPACE, RENT_SPONSOR};

// Tests
// 1. init compressed, decompress, and compress
// 2. update_record bumps compression info
#[tokio::test]
async fn test_create_decompress_compress_single_account() {
    let program_id = sdk_compressible_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;

    rpc.warp_to_slot(100).unwrap();

    decompress_single_user_record(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        &user_record_bump,
        "Test User",
        100,
    )
    .await;

    rpc.warp_to_slot(101).unwrap();

    let result = compress_record(&mut rpc, &payer, &program_id, &user_record_pda, true).await;
    assert!(result.is_err(), "Compression should fail due to slot delay");
    if let Err(err) = result {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("Custom(16001)"),
            "Expected error message about slot delay, got: {}",
            err_msg
        );
    }
    rpc.warp_to_slot(200).unwrap();
    let result = compress_record(&mut rpc, &payer, &program_id, &user_record_pda, false).await;
    assert!(result.is_ok(), "Compression should succeed");
}

#[tokio::test]
async fn test_update_record_compression_info() {
    let program_id = sdk_compressible_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;

    rpc.warp_to_slot(100).unwrap();
    decompress_single_user_record(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        &user_record_bump,
        "Test User",
        100,
    )
    .await;

    rpc.warp_to_slot(150).unwrap();

    let accounts = sdk_compressible_test::accounts::UpdateRecord {
        user: payer.pubkey(),
        user_record: user_record_pda,
    };

    let instruction_data = sdk_compressible_test::instruction::UpdateRecord {
        name: "Updated User".to_string(),
        score: 42,
    };

    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(result.is_ok(), "Update record transaction should succeed");

    rpc.warp_to_slot(200).unwrap();

    let user_pda_account = rpc.get_account(user_record_pda).await.unwrap();
    assert!(
        user_pda_account.is_some(),
        "User record account should exist after update"
    );

    let account_data = user_pda_account.unwrap().data;
    let updated_user_record = UserRecord::try_deserialize(&mut &account_data[..]).unwrap();

    assert_eq!(updated_user_record.name, "Updated User");
    assert_eq!(updated_user_record.score, 42);
    assert_eq!(updated_user_record.owner, payer.pubkey());

    assert_eq!(
        updated_user_record
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        150
    );
    assert!(!updated_user_record
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
}

pub async fn compress_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    should_fail: bool,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_account.is_some(),
        "User PDA account should exist before compression"
    );
    let account = user_pda_account.unwrap();
    assert!(
        account.lamports > 0,
        "Account should have lamports before compression"
    );
    assert!(
        !account.data.is_empty(),
        "Account data should not be empty before compression"
    );

    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let compressed_address = compressed_account.address.unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction = CompressibleInstruction::compress_accounts_idempotent(
        program_id,
        sdk_compressible_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
        &[*user_record_pda],
        &[account],
        &sdk_compressible_test::accounts::CompressAccountsIdempotent {
            fee_payer: payer.pubkey(),
            config: CompressibleConfig::derive_pda(program_id, 0).0,
            rent_sponsor: RENT_SPONSOR,
        }
        .to_account_metas(None),
        vec![sdk_compressible_test::get_userrecord_seeds(&payer.pubkey()).0],
        rpc_result,
        output_state_tree_info,
    )
    .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    if should_fail {
        assert!(result.is_err(), "Compress transaction should fail");
        return result;
    } else {
        assert!(result.is_ok(), "Compress transaction should succeed");
    }

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_account.is_none(),
        "Account should not exist after compression"
    );

    let compressed_user_record = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(compressed_user_record.address, Some(compressed_address));
    assert!(compressed_user_record.data.is_some());

    let buf = compressed_user_record.data.unwrap().data;
    let user_record: UserRecord = UserRecord::deserialize(&mut &buf[..]).unwrap();

    assert_eq!(user_record.name, "Test User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, payer.pubkey());
    assert!(user_record.compression_info.is_none());
    Ok(result.unwrap())
}
