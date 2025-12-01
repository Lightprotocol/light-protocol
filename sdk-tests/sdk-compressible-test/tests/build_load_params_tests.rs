//! Tests for build_load_params

use light_compressible_client::{
    build_load_params,
    compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
    get_compressible_account::{deserialize_account, get_account_info_interface},
    CompressibleAccountInput,
};
use light_program_test::{
    program_test::{initialize_compression_config, setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use sdk_compressible_test::{CompressedAccountVariant, UserRecord};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

mod helpers;
use helpers::{create_record, ADDRESS_SPACE, RENT_SPONSOR};

#[tokio::test]
async fn test_build_load_params_single_pda() {
    let program_id = sdk_compressible_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    config = config.with_light_protocol_events();
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        None,
    )
    .await
    .expect("Initialize config should succeed");

    let (user_record_pda, _bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;

    let address_tree = rpc.get_address_tree_v2();
    let account_info =
        get_account_info_interface(&user_record_pda, &program_id, &address_tree, &mut rpc)
            .await
            .expect("Should fetch account")
            .expect("Account should exist");

    assert!(account_info.is_compressed, "Account should be compressed");

    let user_record: UserRecord = deserialize_account(&account_info).expect("Should deserialize");

    let instructions = build_load_params(
        &mut rpc,
        &program_id,
        &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[CompressibleAccountInput::new(
            user_record_pda,
            account_info,
            CompressedAccountVariant::UserRecord(user_record),
        )],
        &[],
        vec![],
    )
    .await
    .expect("build_load_params should succeed");

    assert_eq!(
        instructions.len(),
        1,
        "Should have one decompress instruction"
    );
}

#[tokio::test]
async fn test_build_load_params_empty() {
    let program_id = sdk_compressible_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();

    let instructions = build_load_params::<_, CompressedAccountVariant>(
        &mut rpc,
        &program_id,
        &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[],
        &[],
        vec![],
    )
    .await
    .expect("build_load_params should succeed");

    assert!(instructions.is_empty(), "Should have no instructions");
}
