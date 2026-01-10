use anchor_lang::{
    AccountDeserialize, AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use light_compressed_account::address::derive_address;
use light_compressible::rent::{RentConfig, SLOTS_PER_EPOCH};
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
use sdk_compressible_test::UserRecord;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_system_interface::instruction as system_instruction;

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
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &compressible_instruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
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

    // Top up PDA so it's initially NOT compressible (sufficiently funded)
    // Fund exactly one epoch of rent plus compression_cost, so after one epoch passes it becomes compressible.
    let pda_account = rpc.get_account(user_record_pda).await.unwrap().unwrap();
    let bytes = pda_account.data.len() as u64;
    let rent_cfg = RentConfig::default();
    let rent_per_epoch = rent_cfg.rent_curve_per_epoch(bytes);
    let compression_cost = rent_cfg.compression_cost as u64;
    let top_up = rent_per_epoch + compression_cost;

    let transfer_ix = system_instruction::transfer(&payer.pubkey(), &user_record_pda, top_up);
    let res = rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_ok(), "Top-up transfer should succeed");

    // Immediately try to compress – should FAIL because not compressible yet (sufficiently funded)
    let result = compress_record(&mut rpc, &payer, &program_id, &user_record_pda, true).await;
    assert!(
        result.is_err(),
        "Compression should fail while sufficiently funded"
    );

    // Advance one full epoch so required_epochs increases and the account becomes compressible
    rpc.warp_to_slot(SLOTS_PER_EPOCH * 2).unwrap();

    // Now compression should SUCCEED (account no longer sufficiently funded for current+next epoch)
    let result = compress_record(&mut rpc, &payer, &program_id, &user_record_pda, false).await;
    assert!(
        result.is_ok(),
        "Compression should succeed after epochs advance"
    );
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
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &compressible_instruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
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
        system_program: solana_sdk::system_program::id(),
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
            .last_claimed_slot(),
        100
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

    let instruction = compressible_instruction::compress_accounts_idempotent(
        program_id,
        sdk_compressible_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
        &[*user_record_pda],
        &[account],
        &sdk_compressible_test::accounts::CompressAccountsIdempotent {
            fee_payer: payer.pubkey(),
            config: CompressibleConfig::derive_pda(program_id, 0).0,
            rent_sponsor: RENT_SPONSOR,
            compression_authority: payer.pubkey(),
        }
        .to_account_metas(None),
        rpc_result,
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
