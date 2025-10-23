use anchor_lang::{
    AccountDeserialize, AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use csdk_anchor_test::{
    get_ctoken_signer2_seeds, get_ctoken_signer3_seeds, get_ctoken_signer4_seeds,
    get_ctoken_signer5_seeds, get_ctoken_signer_seeds, CTokenAccountVariant,
    CompressedAccountVariant, GameSession, UserRecord,
};
use light_client::indexer::CompressedAccount;
use light_compressed_account::address::derive_address;
use light_compressed_token_sdk::instructions::create_compressed_mint::{
    derive_ctoken_mint_address, find_spl_mint_address,
};
use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_compressible_client::CompressibleInstruction;
use light_ctoken_types::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMintMetadata,
};
use light_macros::pubkey;
use light_program_test::{
    program_test::{
        initialize_compression_config, setup_mock_program_data, LightProgramTest, TestRpc,
    },
    utils::simulation::simulate_cu,
    AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::{
    compressible::{CompressAs, CompressibleConfig},
    instruction::{PackedAccounts, SystemAccountMetaConfig},
    token::CTokenDataWithVariant,
};
use light_sdk_types::C_TOKEN_PROGRAM_ID;
use light_token_client::ctoken;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub const ADDRESS_SPACE: [Pubkey; 1] = [pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")];
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const CTOKEN_RENT_SPONSOR: Pubkey = pubkey!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");
pub const CTOKEN_RENT_AUTHORITY: Pubkey = pubkey!("8r3QmazwoLHYppYWysXPgUxYJ3Khn7vh3e313jYDcCKy");
#[tokio::test]
async fn test_create_and_decompress_two_accounts() {
    let program_id = csdk_anchor_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();

    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let combined_user = Keypair::new();
    let fund_user_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &combined_user.pubkey(),
        1e9 as u64,
    );
    let fund_result = rpc
        .create_and_send_transaction(&[fund_user_ix], &payer.pubkey(), &[&payer])
        .await;
    assert!(fund_result.is_ok(), "Funding combined user should succeed");
    let combined_session_id = 99999u64;
    let (combined_user_record_pda, _combined_user_record_bump) = Pubkey::find_program_address(
        &[b"user_record", combined_user.pubkey().as_ref()],
        &program_id,
    );
    let (combined_game_session_pda, _combined_game_bump) = Pubkey::find_program_address(
        &[b"game_session", combined_session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    let (
        ctoken_account,
        _mint_signer,
        ctoken_account_2,
        ctoken_account_3,
        ctoken_account_4,
        ctoken_account_5,
    ) = create_user_record_and_game_session(
        &mut rpc,
        &combined_user,
        &program_id,
        &config_pda,
        &combined_user_record_pda,
        &combined_game_session_pda,
        combined_session_id,
    )
    .await;

    rpc.warp_to_slot(200).unwrap();

    let (_, ctoken_account_address) = csdk_anchor_test::get_ctoken_signer_seeds(
        &combined_user.pubkey(),
        &ctoken_account.token.mint,
    );

    let (_, ctoken_account_address_2) =
        csdk_anchor_test::get_ctoken_signer2_seeds(&combined_user.pubkey());

    let (_, ctoken_account_address_3) =
        csdk_anchor_test::get_ctoken_signer3_seeds(&combined_user.pubkey());

    let (_, ctoken_account_address_4) = csdk_anchor_test::get_ctoken_signer4_seeds(
        &combined_user.pubkey(),
        &combined_user.pubkey(),
    );

    let (_, ctoken_account_address_5) = csdk_anchor_test::get_ctoken_signer5_seeds(
        &combined_user.pubkey(),
        &ctoken_account.token.mint,
        42,
    );

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let compressed_user_record_address = derive_address(
        &combined_user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let compressed_game_session_address = derive_address(
        &combined_game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let user_record_before_decompression: CompressedAccount = rpc
        .get_compressed_account(compressed_user_record_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let game_session_before_decompression: CompressedAccount = rpc
        .get_compressed_account(compressed_game_session_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    decompress_multiple_pdas_with_ctoken(
        &mut rpc,
        &combined_user,
        &program_id,
        &combined_user_record_pda,
        &combined_game_session_pda,
        combined_session_id,
        "Combined User",
        "Combined Game",
        200,
        ctoken_account.clone(),
        ctoken_account_address,
        ctoken_account_2.clone(),
        ctoken_account_address_2,
        ctoken_account_3.clone(),
        ctoken_account_address_3,
        ctoken_account_4.clone(),
        ctoken_account_address_4,
        ctoken_account_5.clone(),
        ctoken_account_address_5,
    )
    .await;

    rpc.warp_to_slot(300).unwrap();

    compress_token_account_after_decompress(
        &mut rpc,
        &combined_user,
        &program_id,
        &config_pda,
        ctoken_account_address,
        ctoken_account_address_2,
        ctoken_account_address_3,
        ctoken_account_address_4,
        ctoken_account_address_5,
        ctoken_account.token.mint,
        ctoken_account.token.amount,
        &combined_user_record_pda,
        &combined_game_session_pda,
        combined_session_id,
        user_record_before_decompression.hash,
        game_session_before_decompression.hash,
    )
    .await;
}

#[tokio::test]
async fn test_create_decompress_compress_single_account() {
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
        RENT_RECIPIENT,
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
    let _result = compress_record(&mut rpc, &payer, &program_id, &user_record_pda, false).await;
}

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
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;
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
                ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
                ctoken_config: ctoken::derive_ctoken_program_config(None).0,
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

#[tokio::test]
async fn test_create_and_decompress_accounts_with_different_state_trees() {
    let program_id = csdk_anchor_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, _user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    let session_id = 54321u64;
    let (game_session_pda, _game_bump) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    let first_state_tree_info = rpc.get_state_tree_infos()[0];
    let second_state_tree_info = rpc.get_state_tree_infos()[1];

    create_record(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        Some(first_state_tree_info.queue),
    )
    .await;

    create_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &game_session_pda,
        session_id,
        Some(second_state_tree_info.queue),
    )
    .await;

    rpc.warp_to_slot(100).unwrap();

    decompress_multiple_pdas(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        &game_session_pda,
        session_id,
        "Test User",
        "Battle Royale",
        100,
    )
    .await;
}

#[tokio::test]
async fn test_update_record_compression_info() {
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
        RENT_RECIPIENT,
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

    let accounts = csdk_anchor_test::accounts::UpdateRecord {
        user: payer.pubkey(),
        user_record: user_record_pda,
    };

    let instruction_data = csdk_anchor_test::instruction::UpdateRecord {
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

#[tokio::test]
async fn test_custom_compression_game_session() {
    let program_id = csdk_anchor_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let session_id = 42424u64;
    let (game_session_pda, _game_bump) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    create_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &game_session_pda,
        session_id,
        None,
    )
    .await;

    rpc.warp_to_slot(100).unwrap();

    decompress_single_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &game_session_pda,
        &_game_bump,
        session_id,
        "Battle Royale",
        100,
        0,
    )
    .await;

    rpc.warp_to_slot(250).unwrap();

    compress_game_session_with_custom_data(
        &mut rpc,
        &payer,
        &program_id,
        &game_session_pda,
        session_id,
    )
    .await;
}

#[tokio::test]
async fn test_create_empty_compressed_account() {
    let program_id = csdk_anchor_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
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
        csdk_anchor_test::PlaceholderRecord::try_deserialize(&mut &placeholder_data[..]).unwrap();
    assert_eq!(decompressed_placeholder_record.name, "Test Placeholder");
    assert_eq!(
        decompressed_placeholder_record.placeholder_id,
        placeholder_id
    );
    assert_eq!(decompressed_placeholder_record.owner, payer.pubkey());

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;
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

async fn create_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    state_tree_queue: Option<Pubkey>,
) {
    let config_pda = CompressibleConfig::derive_pda(program_id, 0).0;

    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let accounts = csdk_anchor_test::accounts::CreateRecord {
        user: payer.pubkey(),
        user_record: *user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    let compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
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

    let output_state_tree_index = remaining_accounts.insert_or_get(
        state_tree_queue.unwrap_or_else(|| rpc.get_random_state_tree_info().unwrap().queue),
    );

    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = csdk_anchor_test::instruction::CreateRecord {
        name: "Test User".to_string(),
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

    let _cu = simulate_cu(rpc, payer, &instruction).await;

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(result.is_ok(), "Transaction should succeed");

    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_record_account.is_none(),
        "Account should not exist after compression"
    );
}

async fn create_game_session(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
    state_tree_queue: Option<Pubkey>,
) {
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let accounts = csdk_anchor_test::accounts::CreateGameSession {
        player: payer.pubkey(),
        game_session: *game_session_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    let compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
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

    let output_state_tree_index = remaining_accounts.insert_or_get(
        state_tree_queue.unwrap_or_else(|| rpc.get_random_state_tree_info().unwrap().queue),
    );

    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = csdk_anchor_test::instruction::CreateGameSession {
        session_id,
        game_type: "Battle Royale".to_string(),
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

    assert!(result.is_ok(), "Transaction should succeed");

    let game_session_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_session_account.is_none(),
        "Account should not exist after compression"
    );

    let compressed_game_session = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(compressed_game_session.address, Some(compressed_address));
    assert!(compressed_game_session.data.is_some());

    let buf = compressed_game_session.data.as_ref().unwrap().data.clone();

    let game_session = GameSession::deserialize(&mut &buf[..]).unwrap();

    assert_eq!(game_session.session_id, session_id);
    assert_eq!(game_session.game_type, "Battle Royale");
    assert_eq!(game_session.player, payer.pubkey());
    assert_eq!(game_session.score, 0);
    assert!(game_session.compression_info.is_none());
}

#[allow(clippy::too_many_arguments)]
async fn decompress_multiple_pdas_with_ctoken(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
    expected_user_name: &str,
    expected_game_type: &str,
    expected_slot: u64,
    ctoken_account: light_client::indexer::CompressedTokenAccount,
    native_token_account: Pubkey,
    ctoken_account_2: light_client::indexer::CompressedTokenAccount,
    native_token_account_2: Pubkey,
    ctoken_account_3: light_client::indexer::CompressedTokenAccount,
    native_token_account_3: Pubkey,
    ctoken_account_4: light_client::indexer::CompressedTokenAccount,
    native_token_account_4: Pubkey,
    ctoken_account_5: light_client::indexer::CompressedTokenAccount,
    native_token_account_5: Pubkey,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let user_account_data = c_user_pda.data.as_ref().unwrap();
    let c_user_record = UserRecord::deserialize(&mut &user_account_data.data[..]).unwrap();

    let game_compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let game_account_data = c_game_pda.data.as_ref().unwrap();
    let c_game_session = GameSession::deserialize(&mut &game_account_data.data[..]).unwrap();

    let rpc_result = rpc
        .get_validity_proof(
            vec![
                c_user_pda.hash,
                c_game_pda.hash,
                ctoken_account.clone().account.hash,
                ctoken_account_2.clone().account.hash,
                ctoken_account_3.clone().account.hash,
                ctoken_account_4.clone().account.hash,
                ctoken_account_5.clone().account.hash,
            ],
            vec![],
            None,
        )
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let ctoken_config = ctoken::derive_ctoken_program_config(None).0;
    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[
                *user_record_pda,
                *game_session_pda,
                native_token_account,
                native_token_account_2,
                native_token_account_3,
                native_token_account_4,
                native_token_account_5,
            ],
            &[
                (
                    c_user_pda.clone(),
                    CompressedAccountVariant::UserRecord(c_user_record),
                ),
                (
                    c_game_pda.clone(),
                    CompressedAccountVariant::GameSession(c_game_session),
                ),
                (
                    {
                        let acc = ctoken_account.clone().account;
                        let _token = ctoken_account.clone().token;
                        acc
                    },
                    CompressedAccountVariant::CTokenData(CTokenDataWithVariant::<
                        CTokenAccountVariant,
                    > {
                        variant: CTokenAccountVariant::CTokenSigner,
                        token_data: ctoken_account.clone().token,
                    }),
                ),
                (
                    ctoken_account_2.clone().account,
                    CompressedAccountVariant::CTokenData(CTokenDataWithVariant::<
                        CTokenAccountVariant,
                    > {
                        variant: CTokenAccountVariant::CTokenSigner2,
                        token_data: ctoken_account_2.clone().token,
                    }),
                ),
                (
                    ctoken_account_3.clone().account,
                    CompressedAccountVariant::CTokenData(CTokenDataWithVariant::<
                        CTokenAccountVariant,
                    > {
                        variant: CTokenAccountVariant::CTokenSigner3,
                        token_data: ctoken_account_3.clone().token,
                    }),
                ),
                (
                    ctoken_account_4.clone().account,
                    CompressedAccountVariant::CTokenData(CTokenDataWithVariant::<
                        CTokenAccountVariant,
                    > {
                        variant: CTokenAccountVariant::CTokenSigner4,
                        token_data: ctoken_account_4.clone().token,
                    }),
                ),
                (
                    ctoken_account_5.clone().account,
                    CompressedAccountVariant::CTokenData(CTokenDataWithVariant::<
                        CTokenAccountVariant,
                    > {
                        variant: CTokenAccountVariant::CTokenSigner5,
                        token_data: ctoken_account_5.clone().token,
                    }),
                ),
            ],
            &csdk_anchor_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_payer: payer.pubkey(),
                ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
                ctoken_config,
                ctoken_program: ctoken::id(),
                ctoken_cpi_authority: ctoken::cpi_authority(),
                some_mint: ctoken_account.token.mint,
            }
            .to_account_metas(None),
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert_eq!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "User PDA account data len must be 0 before decompression"
    );

    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert_eq!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "Game PDA account data len must be 0 before decompression"
    );

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "User PDA account data len must be > 0 after decompression"
    );

    let user_pda_data = user_pda_account.unwrap().data;
    assert_eq!(
        &user_pda_data[0..8],
        UserRecord::DISCRIMINATOR,
        "User account anchor discriminator mismatch"
    );

    let decompressed_user_record = UserRecord::try_deserialize(&mut &user_pda_data[..]).unwrap();
    assert_eq!(decompressed_user_record.name, expected_user_name);
    assert_eq!(decompressed_user_record.score, 11);
    assert_eq!(decompressed_user_record.owner, payer.pubkey());
    assert!(!decompressed_user_record
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
    assert_eq!(
        decompressed_user_record
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        expected_slot
    );

    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "Game PDA account data len must be > 0 after decompression"
    );

    let game_pda_data = game_pda_account.unwrap().data;
    assert_eq!(
        &game_pda_data[0..8],
        csdk_anchor_test::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        csdk_anchor_test::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
    assert_eq!(decompressed_game_session.session_id, session_id);
    assert_eq!(decompressed_game_session.game_type, expected_game_type);
    assert_eq!(decompressed_game_session.player, payer.pubkey());
    assert_eq!(decompressed_game_session.score, 0);
    assert!(!decompressed_game_session
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
    assert_eq!(
        decompressed_game_session
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        expected_slot
    );

    let token_account_data = rpc
        .get_account(native_token_account)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !token_account_data.data.is_empty(),
        "Token account should have data"
    );
    assert_eq!(token_account_data.owner, C_TOKEN_PROGRAM_ID.into());

    let compressed_user_record_data = rpc
        .get_compressed_account(c_user_pda.clone().address.unwrap(), None)
        .await
        .unwrap()
        .value
        .unwrap();
    let compressed_game_session_data = rpc
        .get_compressed_account(c_game_pda.clone().address.unwrap(), None)
        .await
        .unwrap()
        .value
        .unwrap();
    for ctoken in [
        &ctoken_account,
        &ctoken_account_2,
        &ctoken_account_3,
        &ctoken_account_4,
        &ctoken_account_5,
    ] {
        let response = rpc
            .get_compressed_account_by_hash(ctoken.clone().account.hash, None)
            .await
            .unwrap();
        assert!(
            response.value.is_none(),
            "Compressed token account should have value == None after being closed"
        );
    }

    assert!(
        compressed_user_record_data.data.unwrap().data.is_empty(),
        "Compressed user record should be closed/empty after decompression"
    );
    assert!(
        compressed_game_session_data.data.unwrap().data.is_empty(),
        "Compressed game session should be closed/empty after decompression"
    );
}

#[allow(clippy::too_many_arguments)]
async fn decompress_multiple_pdas(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
    expected_user_name: &str,
    expected_game_type: &str,
    expected_slot: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let user_account_data = c_user_pda.data.as_ref().unwrap();

    let c_user_record = UserRecord::deserialize(&mut &user_account_data.data[..]).unwrap();

    let game_compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let game_account_data = c_game_pda.data.as_ref().unwrap();

    let c_game_session = GameSession::deserialize(&mut &game_account_data.data[..]).unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash, c_game_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*user_record_pda, *game_session_pda],
            &[
                (
                    c_user_pda,
                    CompressedAccountVariant::UserRecord(c_user_record),
                ),
                (
                    c_game_pda,
                    CompressedAccountVariant::GameSession(c_game_session),
                ),
            ],
            &csdk_anchor_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_payer: payer.pubkey(),
                ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
                ctoken_config: ctoken::derive_ctoken_program_config(None).0,
                ctoken_program: ctoken::id(),
                ctoken_cpi_authority: ctoken::cpi_authority(),
                some_mint: payer.pubkey(),
            }
            .to_account_metas(None),
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert_eq!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "User PDA account data len must be 0 before decompression"
    );

    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert_eq!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "Game PDA account data len must be 0 before decompression"
    );

    let _cu = simulate_cu(rpc, payer, &instruction).await;

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "User PDA account data len must be > 0 after decompression"
    );

    let user_pda_data = user_pda_account.unwrap().data;
    assert_eq!(
        &user_pda_data[0..8],
        UserRecord::DISCRIMINATOR,
        "User account anchor discriminator mismatch"
    );

    let decompressed_user_record = UserRecord::try_deserialize(&mut &user_pda_data[..]).unwrap();
    assert_eq!(decompressed_user_record.name, expected_user_name);
    assert_eq!(decompressed_user_record.score, 11);
    assert_eq!(decompressed_user_record.owner, payer.pubkey());
    assert!(!decompressed_user_record
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
    assert_eq!(
        decompressed_user_record
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        expected_slot
    );

    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "Game PDA account data len must be > 0 after decompression"
    );

    let game_pda_data = game_pda_account.unwrap().data;
    assert_eq!(
        &game_pda_data[0..8],
        csdk_anchor_test::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        csdk_anchor_test::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
    assert_eq!(decompressed_game_session.session_id, session_id);
    assert_eq!(decompressed_game_session.game_type, expected_game_type);
    assert_eq!(decompressed_game_session.player, payer.pubkey());
    assert_eq!(decompressed_game_session.score, 0);
    assert!(!decompressed_game_session
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
    assert_eq!(
        decompressed_game_session
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        expected_slot
    );

    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert!(c_game_pda.data.is_some());
    assert_eq!(c_game_pda.data.unwrap().data.len(), 0);
}

async fn create_user_record_and_game_session(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
) -> (
    light_client::indexer::CompressedTokenAccount,
    Pubkey,
    light_client::indexer::CompressedTokenAccount,
    light_client::indexer::CompressedTokenAccount,
    light_client::indexer::CompressedTokenAccount,
    light_client::indexer::CompressedTokenAccount,
) {
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new_with_cpi_context(
        *program_id,
        state_tree_info.cpi_context.unwrap(),
    );
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = mint_authority;
    let mint_signer = Keypair::new();
    let compressed_mint_address =
        derive_ctoken_mint_address(&mint_signer.pubkey(), &address_tree_pubkey);

    let (spl_mint, mint_bump) = find_spl_mint_address(&mint_signer.pubkey());
    let accounts = csdk_anchor_test::accounts::CreateUserRecordAndGameSession {
        user: user.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        mint_signer: mint_signer.pubkey(),
        ctoken_program: light_sdk_types::constants::C_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
        mint_authority,
        compress_token_program_cpi_authority: Pubkey::new_from_array(CPI_AUTHORITY_PDA),
    };
    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let game_compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![
                AddressWithTree {
                    address: user_compressed_address,
                    tree: address_tree_pubkey,
                },
                AddressWithTree {
                    address: game_compressed_address,
                    tree: address_tree_pubkey,
                },
                AddressWithTree {
                    address: compressed_mint_address,
                    tree: address_tree_pubkey,
                },
            ],
            None,
        )
        .await
        .unwrap()
        .value;

    let user_output_state_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);
    let game_output_state_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);
    let _mint_output_state_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);

    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let user_address_tree_info = packed_tree_infos.address_trees[0];
    let game_address_tree_info = packed_tree_infos.address_trees[1];
    let mint_address_tree_info = packed_tree_infos.address_trees[2];

    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = csdk_anchor_test::instruction::CreateUserRecordAndGameSession {
        account_data: csdk_anchor_test::AccountCreationData {
            user_name: "Combined User".to_string(),
            session_id,
            game_type: "Combined Game".to_string(),
            mint_name: "Test Game Token".to_string(),
            mint_symbol: "TGT".to_string(),
            mint_uri: "https://example.com/token.json".to_string(),
            mint_decimals: 9,
            mint_supply: 1_000_000_000,
            mint_update_authority: Some(mint_authority),
            mint_freeze_authority: Some(freeze_authority),
            additional_metadata: None,
        },
        compression_params: csdk_anchor_test::CompressionParams {
            proof: rpc_result.proof,
            user_compressed_address,
            user_address_tree_info,
            user_output_state_tree_index,
            game_compressed_address,
            game_address_tree_info,
            game_output_state_tree_index,
            mint_bump,
            mint_with_context: CompressedMintWithContext {
                leaf_index: 0,
                prove_by_index: false,
                root_index: mint_address_tree_info.root_index,
                address: compressed_mint_address,
                mint: CompressedMintInstructionData {
                    supply: 0,
                    decimals,
                    metadata: CompressedMintMetadata {
                        version: 3,
                        mint: spl_mint.into(),
                        spl_mint_initialized: false,
                    },
                    mint_authority: Some(mint_authority.into()),
                    freeze_authority: Some(freeze_authority.into()),
                    extensions: None,
                },
            },
        },
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    let result = rpc
        .create_and_send_transaction(
            &[instruction],
            &user.pubkey(),
            &[user, &mint_signer, &mint_authority_keypair],
        )
        .await;

    assert!(
        result.is_ok(),
        "Combined creation transaction should succeed"
    );

    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_record_account.is_none(),
        "User record account should not exist after compression"
    );

    let game_session_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_session_account.is_none(),
        "Game session account should not exist after compression"
    );

    let compressed_user_record = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(
        compressed_user_record.address,
        Some(user_compressed_address)
    );
    assert!(compressed_user_record.data.is_some());

    let user_buf = compressed_user_record.data.unwrap().data;

    let user_record = UserRecord::deserialize(&mut &user_buf[..]).unwrap();

    assert_eq!(user_record.name, "Combined User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, user.pubkey());

    let compressed_game_session = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert_eq!(
        compressed_game_session.address,
        Some(game_compressed_address)
    );
    assert!(compressed_game_session.data.is_some());

    let game_buf = compressed_game_session.data.unwrap().data;
    let game_session = GameSession::deserialize(&mut &game_buf[..]).unwrap();
    assert_eq!(game_session.session_id, session_id);
    assert_eq!(game_session.game_type, "Combined Game");
    assert_eq!(game_session.player, user.pubkey());
    assert_eq!(game_session.score, 0);

    let token_account_address = get_ctoken_signer_seeds(
        &user.pubkey(),
        &find_spl_mint_address(&mint_signer.pubkey()).0,
    )
    .1;

    let mint = find_spl_mint_address(&mint_signer.pubkey()).0;
    let token_account_address_2 = get_ctoken_signer2_seeds(&user.pubkey()).1;
    let token_account_address_3 = get_ctoken_signer3_seeds(&user.pubkey()).1;
    let token_account_address_4 = get_ctoken_signer4_seeds(&user.pubkey(), &user.pubkey()).1;
    let token_account_address_5 = get_ctoken_signer5_seeds(&user.pubkey(), &mint, 42).1;

    let ctoken_accounts = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_2 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_2, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_3 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_3, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_4 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_4, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_5 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_5, None, None)
        .await
        .unwrap()
        .value;

    assert!(
        !ctoken_accounts.items.is_empty(),
        "Should have at least one compressed token account"
    );
    assert!(
        !ctoken_accounts_2.items.is_empty(),
        "Should have at least one compressed token account 2"
    );
    assert!(
        !ctoken_accounts_3.items.is_empty(),
        "Should have at least one compressed token account 3"
    );
    assert!(
        !ctoken_accounts_4.items.is_empty(),
        "Should have at least one compressed token account 4"
    );
    assert!(
        !ctoken_accounts_5.items.is_empty(),
        "Should have at least one compressed token account 5"
    );

    let ctoken_account = ctoken_accounts.items[0].clone();
    let ctoken_account_2 = ctoken_accounts_2.items[0].clone();
    let ctoken_account_3 = ctoken_accounts_3.items[0].clone();
    let ctoken_account_4 = ctoken_accounts_4.items[0].clone();
    let ctoken_account_5 = ctoken_accounts_5.items[0].clone();

    (
        ctoken_account,
        mint_signer.pubkey(),
        ctoken_account_2,
        ctoken_account_3,
        ctoken_account_4,
        ctoken_account_5,
    )
}

async fn compress_record(
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

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

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
        csdk_anchor_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
        &[*user_record_pda],
        &[account],
        &csdk_anchor_test::accounts::CompressAccountsIdempotent {
            fee_payer: payer.pubkey(),
            config: CompressibleConfig::derive_pda(program_id, 0).0,
            rent_recipient: RENT_RECIPIENT,
        }
        .to_account_metas(None),
        vec![csdk_anchor_test::get_userrecord_seeds(&payer.pubkey()).0],
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

async fn decompress_single_user_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    _user_record_bump: &u8,
    expected_user_name: &str,
    expected_slot: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let user_account_data = c_user_pda.data.as_ref().unwrap();
    let c_user_record = UserRecord::deserialize(&mut &user_account_data.data[..]).unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();
    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*user_record_pda],
            &[(
                c_user_pda,
                CompressedAccountVariant::UserRecord(c_user_record),
            )],
            &csdk_anchor_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_payer: payer.pubkey(),
                ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
                ctoken_config: ctoken::derive_ctoken_program_config(None).0,
                ctoken_program: ctoken::id(),
                ctoken_cpi_authority: ctoken::cpi_authority(),
                some_mint: payer.pubkey(),
            }
            .to_account_metas(None),
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert_eq!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "User PDA account data len must be 0 before decompression"
    );

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();

    let compressed_account = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(compressed_account.data.unwrap().data.is_empty());

    assert!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "User PDA account data len must be > 0 after decompression"
    );

    let user_pda_data = user_pda_account.unwrap().data;
    assert_eq!(
        &user_pda_data[0..8],
        UserRecord::DISCRIMINATOR,
        "User account anchor discriminator mismatch"
    );

    let decompressed_user_record = UserRecord::try_deserialize(&mut &user_pda_data[..]).unwrap();
    assert_eq!(decompressed_user_record.name, expected_user_name);
    assert_eq!(decompressed_user_record.score, 11);
    assert_eq!(decompressed_user_record.owner, payer.pubkey());
    assert!(!decompressed_user_record
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
    assert_eq!(
        decompressed_user_record
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        expected_slot
    );
}

async fn create_placeholder_record(
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

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let accounts = csdk_anchor_test::accounts::CreatePlaceholderRecord {
        user: payer.pubkey(),
        placeholder_record: *placeholder_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    let compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
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

    let instruction_data = csdk_anchor_test::instruction::CreatePlaceholderRecord {
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

async fn compress_placeholder_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    _config_pda: &Pubkey,
    placeholder_record_pda: &Pubkey,
    _placeholder_record_bump: &u8,
    placeholder_id: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

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

    let placeholder_seeds = csdk_anchor_test::get_placeholderrecord_seeds(placeholder_id);

    let account = rpc
        .get_account(*placeholder_record_pda)
        .await
        .unwrap()
        .unwrap();
    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction =
        light_compressible_client::CompressibleInstruction::compress_accounts_idempotent(
            program_id,
            csdk_anchor_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
            &[*placeholder_record_pda],
            &[account],
            &csdk_anchor_test::accounts::CompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_recipient: RENT_RECIPIENT,
            }
            .to_account_metas(None),
            vec![placeholder_seeds.0],
            rpc_result,
            output_state_tree_info,
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

async fn compress_placeholder_record_for_double_test(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    placeholder_record_pda: &Pubkey,
    placeholder_id: u64,
    previous_account: Option<Account>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

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

    let placeholder_seeds = csdk_anchor_test::get_placeholderrecord_seeds(placeholder_id);

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let accounts_to_compress = if let Some(account) = previous_account {
        vec![account]
    } else {
        panic!("Previous account should be provided");
    };
    let instruction =
        light_compressible_client::CompressibleInstruction::compress_accounts_idempotent(
            program_id,
            csdk_anchor_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
            &[*placeholder_record_pda],
            &accounts_to_compress,
            &csdk_anchor_test::accounts::CompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_recipient: RENT_RECIPIENT,
            }
            .to_account_metas(None),
            vec![placeholder_seeds.0],
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

#[allow(clippy::too_many_arguments)]
async fn decompress_single_game_session(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    game_session_pda: &Pubkey,
    _game_bump: &u8,
    session_id: u64,
    expected_game_type: &str,
    expected_slot: u64,
    expected_score: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let game_compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let game_account_data = c_game_pda.data.as_ref().unwrap();
    let c_game_session =
        csdk_anchor_test::GameSession::deserialize(&mut &game_account_data.data[..]).unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![c_game_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*game_session_pda],
            &[(
                c_game_pda,
                csdk_anchor_test::CompressedAccountVariant::GameSession(c_game_session),
            )],
            &csdk_anchor_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_payer: payer.pubkey(),
                ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
                ctoken_config: ctoken::derive_ctoken_program_config(None).0,
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
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "Game PDA account data len must be > 0 after decompression"
    );

    let game_pda_data = game_pda_account.unwrap().data;
    assert_eq!(
        &game_pda_data[0..8],
        csdk_anchor_test::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        csdk_anchor_test::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
    assert_eq!(decompressed_game_session.session_id, session_id);
    assert_eq!(decompressed_game_session.game_type, expected_game_type);
    assert_eq!(decompressed_game_session.player, payer.pubkey());
    assert_eq!(decompressed_game_session.score, expected_score);
    assert!(!decompressed_game_session
        .compression_info
        .as_ref()
        .unwrap()
        .is_compressed());
    assert_eq!(
        decompressed_game_session
            .compression_info
            .as_ref()
            .unwrap()
            .last_written_slot(),
        expected_slot
    );
}

async fn compress_game_session_with_custom_data(
    rpc: &mut LightProgramTest,
    _payer: &Keypair,
    _program_id: &Pubkey,
    game_session_pda: &Pubkey,
    _session_id: u64,
) {
    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap().unwrap();
    let game_pda_data = game_pda_account.data;
    let original_game_session =
        csdk_anchor_test::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();

    let custom_compressed_data = match original_game_session.compress_as() {
        std::borrow::Cow::Borrowed(data) => data.clone(),
        std::borrow::Cow::Owned(data) => data,
    };

    assert_eq!(
        custom_compressed_data.session_id, original_game_session.session_id,
        "Session ID should be kept"
    );
    assert_eq!(
        custom_compressed_data.player, original_game_session.player,
        "Player should be kept"
    );
    assert_eq!(
        custom_compressed_data.game_type, original_game_session.game_type,
        "Game type should be kept"
    );
    assert_eq!(
        custom_compressed_data.start_time, 0,
        "Start time should be RESET to 0"
    );
    assert_eq!(
        custom_compressed_data.end_time, None,
        "End time should be RESET to None"
    );
    assert_eq!(
        custom_compressed_data.score, 0,
        "Score should be RESET to 0"
    );
}

#[tokio::test]
async fn test_double_compression_attack() {
    let program_id = csdk_anchor_test::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100,
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
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

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;
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

#[allow(clippy::too_many_arguments)]
async fn compress_token_account_after_decompress(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    program_id: &Pubkey,
    _config_pda: &Pubkey,
    token_account_address: Pubkey,
    _token_account_address_2: Pubkey,
    _token_account_address_3: Pubkey,
    _token_account_address_4: Pubkey,
    _token_account_address_5: Pubkey,
    mint: Pubkey,
    amount: u64,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
    user_record_hash_before_decompression: [u8; 32],
    game_session_hash_before_decompression: [u8; 32],
) {
    let token_account_data = rpc.get_account(token_account_address).await.unwrap();
    assert!(
        token_account_data.is_some(),
        "Token account should exist before compression"
    );

    let account = token_account_data.unwrap();

    assert!(
        account.lamports > 0,
        "Token account should have lamports before compression"
    );
    assert!(
        !account.data.is_empty(),
        "Token account should have data before compression"
    );

    let (user_record_seeds, user_record_pubkey) =
        csdk_anchor_test::get_userrecord_seeds(&user.pubkey());
    let (game_session_seeds, game_session_pubkey) =
        csdk_anchor_test::get_gamesession_seeds(session_id);
    let (_, token_account_address) = get_ctoken_signer_seeds(&user.pubkey(), &mint);

    let (_, token_account_address_2) = get_ctoken_signer2_seeds(&user.pubkey());
    let (_, token_account_address_3) = get_ctoken_signer3_seeds(&user.pubkey());
    let (_, token_account_address_4) = get_ctoken_signer4_seeds(&user.pubkey(), &user.pubkey());
    let (_, token_account_address_5) = get_ctoken_signer5_seeds(&user.pubkey(), &mint, 42);
    let (_token_signer_seeds, _ctoken_1_authority_pda) =
        csdk_anchor_test::get_ctokensigner_authority_seeds();

    let (_token_signer_seeds_2, _ctoken_2_authority_pda) =
        csdk_anchor_test::get_ctokensigner2_authority_seeds();

    let (_token_signer_seeds_3, _ctoken_3_authority_pda) =
        csdk_anchor_test::get_ctokensigner3_authority_seeds();

    let (_token_signer_seeds_4, _ctoken_4_authority_pda) =
        csdk_anchor_test::get_ctokensigner4_authority_seeds();

    let (_token_signer_seeds_5, _ctoken_5_authority_pda) =
        csdk_anchor_test::get_ctokensigner5_authority_seeds();

    let _cpisigner = Pubkey::new_from_array(csdk_anchor_test::LIGHT_CPI_SIGNER.cpi_signer);

    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap().unwrap();
    let game_session_account = rpc.get_account(*game_session_pda).await.unwrap().unwrap();
    let _token_account = rpc
        .get_account(token_account_address)
        .await
        .unwrap()
        .unwrap();
    let _token_account_2 = rpc
        .get_account(token_account_address_2)
        .await
        .unwrap()
        .unwrap();
    let _token_account_3 = rpc
        .get_account(token_account_address_3)
        .await
        .unwrap()
        .unwrap();
    let _token_account_4 = rpc
        .get_account(token_account_address_4)
        .await
        .unwrap()
        .unwrap();
    let _token_account_5 = rpc
        .get_account(token_account_address_5)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(*user_record_pda, user_record_pubkey);
    assert_eq!(*game_session_pda, game_session_pubkey);

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let compressed_user_record_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let compressed_game_session_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let user_record: CompressedAccount = rpc
        .get_compressed_account(compressed_user_record_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let game_session: CompressedAccount = rpc
        .get_compressed_account(compressed_game_session_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let user_record_hash = user_record.hash;
    let game_session_hash = game_session.hash;

    assert_ne!(
        user_record_hash, user_record_hash_before_decompression,
        "User record hash NOT_EQUAL before and after compression"
    );
    assert_ne!(
        game_session_hash, game_session_hash_before_decompression,
        "Game session hash NOT_EQUAL before and after compression"
    );

    let proof_with_context = rpc
        .get_validity_proof(vec![user_record_hash, game_session_hash], vec![], None)
        .await
        .unwrap()
        .value;

    let random_tree_info = rpc.get_random_state_tree_info().unwrap();
    let instruction =
        light_compressible_client::CompressibleInstruction::compress_accounts_idempotent(
            program_id,
            csdk_anchor_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
            &[user_record_pubkey, game_session_pubkey],
            &[user_record_account, game_session_account],
            &csdk_anchor_test::accounts::CompressAccountsIdempotent {
                fee_payer: user.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_recipient: RENT_RECIPIENT,
            }
            .to_account_metas(None),
            vec![user_record_seeds, game_session_seeds],
            proof_with_context,
            random_tree_info,
        )
        .unwrap();

    for _account in instruction.accounts.iter() {}

    let result = rpc
        .create_and_send_transaction(&[instruction], &user.pubkey(), &[user])
        .await;

    assert!(
        result.is_ok(),
        "PDA compression should succeed: {:?}",
        result
    );

    rpc.warp_slot_forward(20000).await.unwrap();

    let token_account_after = rpc.get_account(token_account_address).await.unwrap();
    assert!(
        token_account_after.is_none(),
        "Token account should not exist after compression"
    );
    let token_account_after_2 = rpc.get_account(token_account_address_2).await.unwrap();
    assert!(
        token_account_after_2.is_none(),
        "Token account 2 should not exist after compression"
    );
    let token_account_after_3 = rpc.get_account(token_account_address_3).await.unwrap();
    assert!(
        token_account_after_3.is_none(),
        "Token account 3 should not exist after compression"
    );
    let token_account_after_4 = rpc.get_account(token_account_address_4).await.unwrap();
    assert!(
        token_account_after_4.is_none(),
        "Token account 4 should not exist after compression"
    );
    let token_account_after_5 = rpc.get_account(token_account_address_5).await.unwrap();
    assert!(
        token_account_after_5.is_none(),
        "Token account 5 should not exist after compression"
    );

    let ctoken_accounts = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_2 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_2, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_3 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_3, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_4 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_4, None, None)
        .await
        .unwrap()
        .value;
    let ctoken_accounts_5 = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address_5, None, None)
        .await
        .unwrap()
        .value;

    assert!(
        !ctoken_accounts.items.is_empty(),
        "Should have at least one compressed token account after compression"
    );
    assert!(
        !ctoken_accounts_2.items.is_empty(),
        "Should have at least one compressed token account 2 after compression"
    );
    assert!(
        !ctoken_accounts_3.items.is_empty(),
        "Should have at least one compressed token account 3 after compression"
    );
    assert!(
        !ctoken_accounts_4.items.is_empty(),
        "Should have at least one compressed token account 4 after compression"
    );
    assert!(
        !ctoken_accounts_5.items.is_empty(),
        "Should have at least one compressed token account 5 after compression"
    );

    let ctoken = &ctoken_accounts.items[0];
    assert_eq!(
        ctoken.token.mint, mint,
        "Compressed token should have the same mint"
    );
    assert_eq!(
        ctoken.token.owner, token_account_address,
        "Compressed token owner should be the token account address"
    );
    assert_eq!(
        ctoken.token.amount, amount,
        "Compressed token should have the same amount"
    );
    let ctoken2 = &ctoken_accounts_2.items[0];
    assert_eq!(
        ctoken2.token.mint, mint,
        "Compressed token 2 should have the same mint"
    );
    assert_eq!(
        ctoken2.token.owner, token_account_address_2,
        "Compressed token 2 owner should be the token account address"
    );
    assert_eq!(
        ctoken2.token.amount, amount,
        "Compressed token 2 should have the same amount"
    );
    let ctoken3 = &ctoken_accounts_3.items[0];
    assert_eq!(
        ctoken3.token.mint, mint,
        "Compressed token 3 should have the same mint"
    );
    assert_eq!(
        ctoken3.token.owner, token_account_address_3,
        "Compressed token 3 owner should be the token account address"
    );
    assert_eq!(
        ctoken3.token.amount, amount,
        "Compressed token 3 should have the same amount"
    );
    let ctoken4 = &ctoken_accounts_4.items[0];
    assert_eq!(
        ctoken4.token.mint, mint,
        "Compressed token 4 should have the same mint"
    );
    assert_eq!(
        ctoken4.token.owner, token_account_address_4,
        "Compressed token 4 owner should be the token account address"
    );
    assert_eq!(
        ctoken4.token.amount, amount,
        "Compressed token 4 should have the same amount"
    );
    let ctoken5 = &ctoken_accounts_5.items[0];
    assert_eq!(
        ctoken5.token.mint, mint,
        "Compressed token 5 should have the same mint"
    );
    assert_eq!(
        ctoken5.token.owner, token_account_address_5,
        "Compressed token 5 owner should be the token account address"
    );
    assert_eq!(
        ctoken5.token.amount, amount,
        "Compressed token 5 should have the same amount"
    );
    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    let game_session_account = rpc.get_account(*game_session_pda).await.unwrap();
    let token_account = rpc.get_account(token_account_address).await.unwrap();
    let token_account_3 = rpc.get_account(token_account_address_3).await.unwrap();
    let token_account_4 = rpc.get_account(token_account_address_4).await.unwrap();
    let token_account_5 = rpc.get_account(token_account_address_5).await.unwrap();

    assert!(
        user_record_account.is_none(),
        "User record account should be None"
    );
    assert!(
        game_session_account.is_none(),
        "Game session account should be None"
    );
    assert!(token_account.is_none(), "Token account should be None");
    assert!(
        user_record_account
            .map(|a| a.data.is_empty())
            .unwrap_or(true),
        "User record account should be empty"
    );
    assert!(
        game_session_account
            .map(|a| a.data.is_empty())
            .unwrap_or(true),
        "Game session account should be empty"
    );
    assert!(
        token_account.map(|a| a.data.is_empty()).unwrap_or(true),
        "Token account should be empty"
    );
    assert!(
        token_account_3.map(|a| a.data.is_empty()).unwrap_or(true),
        "Token account 3 should be empty"
    );
    assert!(
        token_account_4.map(|a| a.data.is_empty()).unwrap_or(true),
        "Token account 4 should be empty"
    );
    assert!(
        token_account_5.map(|a| a.data.is_empty()).unwrap_or(true),
        "Token account 5 should be empty"
    );
}
