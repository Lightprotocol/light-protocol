use anchor_lang::{AccountDeserialize, AnchorDeserialize, ToAccountMetas};
use csdk_anchor_test::{CompressedAccountVariant, UserRecord};
use light_compressed_account::address::derive_address;
use light_compressible_client::CompressibleInstruction;
use light_program_test::{
    program_test::{
        initialize_compression_config, setup_mock_program_data, LightProgramTest, TestRpc,
    },
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::compressible::CompressibleConfig;
use light_token_client::ctoken;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

mod helpers;
use helpers::{
    create_record, decompress_single_user_record, ADDRESS_SPACE, CTOKEN_RENT_SPONSOR, RENT_SPONSOR,
};

#[tokio::test]
async fn test_double_decompression_attack() {
    let program_id = csdk_anchor_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_test", program_id)]));
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
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let compressed_user_record = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let c_user_record =
        UserRecord::deserialize(&mut &compressed_user_record.data.unwrap().data[..]).unwrap();

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

    let user_pda_account = rpc.get_account(user_record_pda).await.unwrap();
    assert!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "User PDA should be decompressed after first operation"
    );

    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            &program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[user_record_pda],
            &[(
                c_user_pda,
                CompressedAccountVariant::UserRecord(c_user_record),
            )],
            &csdk_anchor_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(&program_id, 0).0,
                rent_payer: payer.pubkey(),
                ctoken_rent_sponsor: ctoken::rent_sponsor_pda(),
                ctoken_config: ctoken::config_pda(),
                ctoken_program: ctoken::id(),
                ctoken_cpi_authority: ctoken::cpi_authority(),
                some_mint: payer.pubkey(),
            }
            .to_account_metas(None),
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(
        result.is_ok(),
        "Second decompression should succeed idempotently"
    );

    let user_pda_account = rpc.get_account(user_record_pda).await.unwrap();
    let user_pda_data = user_pda_account.unwrap().data;
    let decompressed_user_record = UserRecord::try_deserialize(&mut &user_pda_data[..]).unwrap();

    assert_eq!(decompressed_user_record.name, "Test User");
    assert_eq!(decompressed_user_record.score, 11);
    assert_eq!(decompressed_user_record.owner, payer.pubkey());
    assert!(!decompressed_user_record
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
}
