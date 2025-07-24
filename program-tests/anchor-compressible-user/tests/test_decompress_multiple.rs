#![cfg(feature = "test-sbf")]

mod common;

use anchor_compressible_user::{
    CompressedAccountData, CompressedAccountVariant, GameSession, UserRecord, ADDRESS_SPACE,
    RENT_RECIPIENT,
};
use anchor_lang::{AccountDeserialize, Discriminator, InstructionData, ToAccountMetas};
use light_compressed_account::address::derive_address;
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::{
    compressible::{CompressibleConfig, FromCompressedData},
    instruction::{account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig},
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_create_and_decompress_two_accounts() {
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let program_data_pda = common::setup_mock_program_data(&mut rpc, &payer, &program_id);
    let result = common::initialize_config(
        &mut rpc,
        &payer,
        &program_id,
        config_pda,
        program_data_pda,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    test_create_record_with_config(&mut rpc, &payer, &program_id, &config_pda, &user_record_pda)
        .await;

    let session_id = 12345u64;
    let (game_session_pda, game_bump) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    test_create_game_session_with_config(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &game_session_pda,
        session_id,
    )
    .await;

    rpc.warp_to_slot(100).unwrap();

    test_decompress_multiple_pdas(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        &user_record_bump,
        &game_session_pda,
        &game_bump,
        session_id,
        "Test User",
        "Battle Royale",
        100,
    )
    .await;

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
    let (combined_user_record_pda, combined_user_record_bump) = Pubkey::find_program_address(
        &[b"user_record", combined_user.pubkey().as_ref()],
        &program_id,
    );
    let (combined_game_session_pda, combined_game_bump) = Pubkey::find_program_address(
        &[b"game_session", combined_session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    test_create_user_record_and_game_session_with_config(
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

    test_decompress_multiple_pdas(
        &mut rpc,
        &combined_user,
        &program_id,
        &config_pda,
        &combined_user_record_pda,
        &combined_user_record_bump,
        &combined_game_session_pda,
        &combined_game_bump,
        combined_session_id,
        "Combined User",
        "Combined Game",
        200,
    )
    .await;
}

#[tokio::test]
async fn test_create_decompress_compress_single_account() {
    let program_id = anchor_compressible_user::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible_user", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let (config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    let program_data_pda = common::setup_mock_program_data(&mut rpc, &payer, &program_id);

    let result = common::initialize_config(
        &mut rpc,
        &payer,
        &program_id,
        config_pda,
        program_data_pda,
        &payer,
        100,
        RENT_RECIPIENT,
        ADDRESS_SPACE.to_vec(),
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    test_create_record_with_config(&mut rpc, &payer, &program_id, &config_pda, &user_record_pda)
        .await;

    rpc.warp_to_slot(100).unwrap();

    test_decompress_single_user_record(
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

    let result = test_compress_record_with_config(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        true,
    )
    .await;
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
    let _result = test_compress_record_with_config(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        false,
    )
    .await;
}

async fn test_create_record_with_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
) {
    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Create the instruction
    let accounts = anchor_compressible_user::accounts::CreateRecordWithConfig {
        user: payer.pubkey(),
        user_record: *user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // Derive a new address for the compressed account
    let compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    // Get validity proof from RPC
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

    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the packed address tree info
    let address_tree_info = packed_tree_infos.address_trees[0];

    // Get output state tree index
    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::CreateRecordWithConfig {
        name: "Test User".to_string(),
        proof: rpc_result.proof,
        compressed_address,
        address_tree_info,
        output_state_tree_index,
    };

    // Build the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(result.is_ok(), "Transaction should succeed");

    // should be empty
    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_record_account.is_some(),
        "Account should exist after compression"
    );

    let account = user_record_account.unwrap();
    assert_eq!(account.lamports, 0, "Account lamports should be 0");

    let user_record_data = account.data;

    assert!(user_record_data.is_empty(), "Account data should be empty");
}

async fn test_create_game_session_with_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
) {
    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Create the instruction
    let accounts = anchor_compressible_user::accounts::CreateGameSessionWithConfig {
        player: payer.pubkey(),
        game_session: *game_session_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // Derive a new address for the compressed account
    let compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    // Get validity proof from RPC
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

    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the packed address tree info
    let address_tree_info = packed_tree_infos.address_trees[0];

    // Get output state tree index
    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::CreateGameSessionWithConfig {
        session_id,
        game_type: "Battle Royale".to_string(),
        proof: rpc_result.proof,
        compressed_address,
        address_tree_info,
        output_state_tree_index,
    };

    // Build the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    assert!(result.is_ok(), "Transaction should succeed");

    // Verify the account is empty after compression
    let game_session_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_session_account.is_some(),
        "Account should exist after compression"
    );

    let account = game_session_account.unwrap();
    assert_eq!(account.lamports, 0, "Account lamports should be 0");
    assert!(account.data.is_empty(), "Account data should be empty");

    let compressed_game_session = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(compressed_game_session.address, Some(compressed_address));
    assert_eq!(compressed_game_session.data.is_some(), true);

    let buf = compressed_game_session.data.unwrap().data;

    let game_session = GameSession::from_compressed_data(&buf).unwrap();

    assert_eq!(game_session.session_id, session_id);
    assert_eq!(game_session.game_type, "Battle Royale");
    assert_eq!(game_session.player, payer.pubkey());
    assert_eq!(game_session.score, 0);
    assert_eq!(game_session.compression_info.is_compressed(), true);
}

async fn test_decompress_multiple_pdas(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    _config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    user_record_bump: &u8,
    game_session_pda: &Pubkey,
    game_bump: &u8,
    session_id: u64,
    expected_user_name: &str,
    expected_game_type: &str,
    expected_slot: u64,
) {
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // c pda USER_RECORD
    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value;

    let user_account_data = c_user_pda.data.as_ref().unwrap();

    let c_user_record = UserRecord::from_compressed_data(&user_account_data.data).unwrap();

    // c pda GAME_SESSION
    let game_compressed_address = derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value;
    let game_account_data = c_game_pda.data.as_ref().unwrap();
    // let mut game_with_discriminator =
    //     anchor_compressible_user::GameSession::discriminator().to_vec();
    // game_with_discriminator.extend_from_slice(&game_account_data.data);
    let c_game_session =
        anchor_compressible_user::GameSession::from_compressed_data(&game_account_data.data)
            .unwrap();

    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get validity proof for both compressed accounts
    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash, c_game_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // c meta USER_RECORD
    let user_compressed_meta = CompressedAccountMeta {
        tree_info: packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos[0],
        address: c_user_pda.address.unwrap(),
        output_state_tree_index: 0,
    };

    // c meta GAME_SESSION
    let game_compressed_meta = CompressedAccountMeta {
        tree_info: packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos[1],
        address: c_game_pda.address.unwrap(),
        output_state_tree_index: 0,
    };

    // c data GAME_SESSION
    let compressed_accounts = vec![
        CompressedAccountData {
            meta: user_compressed_meta,
            data: CompressedAccountVariant::UserRecord(c_user_record),
        },
        CompressedAccountData {
            meta: game_compressed_meta,
            data: CompressedAccountVariant::GameSession(c_game_session),
        },
    ];

    // Build instruction accounts
    let pda_accounts = vec![user_record_pda, game_session_pda];
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Prepare bumps for both PDAs
    let bumps = vec![*user_record_bump, *game_bump];

    let instruction_data = anchor_compressible_user::instruction::DecompressMultiplePdas {
        proof: rpc_result.proof,
        compressed_accounts,
        bumps,
        system_accounts_offset,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true), // fee_payer
                AccountMeta::new(payer.pubkey(), true), // rent_payer
                AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
            ],
            pda_accounts
                .iter()
                .map(|&pda| AccountMeta::new(*pda, false))
                .collect(),
            system_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Verify PDAs are uninitialized before decompression
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
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    // Verify UserRecord PDA is decompressed
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
    assert_eq!(
        decompressed_user_record.compression_info.is_compressed(),
        false
    );
    assert_eq!(
        decompressed_user_record
            .compression_info
            .last_written_slot(),
        expected_slot
    );

    // Verify GameSession PDA is decompressed
    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "Game PDA account data len must be > 0 after decompression"
    );

    let game_pda_data = game_pda_account.unwrap().data;
    assert_eq!(
        &game_pda_data[0..8],
        anchor_compressible_user::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        anchor_compressible_user::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
    assert_eq!(decompressed_game_session.session_id, session_id);
    assert_eq!(decompressed_game_session.game_type, expected_game_type);
    assert_eq!(decompressed_game_session.player, payer.pubkey());
    assert_eq!(decompressed_game_session.score, 0);
    assert_eq!(
        decompressed_game_session.compression_info.is_compressed(),
        false
    );
    assert_eq!(
        decompressed_game_session
            .compression_info
            .last_written_slot(),
        expected_slot
    );

    // Verify compressed accounts exist and have correct data
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(c_game_pda.data.is_some(), true);
    assert_eq!(c_game_pda.data.unwrap().data.len(), 0);
}

async fn test_create_user_record_and_game_session_with_config(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
) {
    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Create the instruction
    let accounts = anchor_compressible_user::accounts::CreateUserRecordAndGameSessionWithConfig {
        user: user.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // Derive addresses for both compressed accounts
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

    // Get validity proof from RPC
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
            ],
            None,
        )
        .await
        .unwrap()
        .value;

    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the packed address tree info (both should use the same tree)
    let user_address_tree_info = packed_tree_infos.address_trees[0];
    let game_address_tree_info = packed_tree_infos.address_trees[1];

    // Get output state tree indices
    let user_output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);
    let game_output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data =
        anchor_compressible_user::instruction::CreateUserRecordAndGameSessionWithConfig {
            user_name: "Combined User".to_string(),
            session_id,
            game_type: "Combined Game".to_string(),
            proof: rpc_result.proof,
            user_compressed_address,
            user_address_tree_info,
            user_output_state_tree_index,
            game_compressed_address,
            game_address_tree_info,
            game_output_state_tree_index,
        };

    // Build the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &user.pubkey(), &[&user])
        .await;

    assert!(
        result.is_ok(),
        "Combined creation transaction should succeed"
    );

    // Verify both accounts are empty after compression
    let user_record_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_record_account.is_some(),
        "User record account should exist after compression"
    );
    let account = user_record_account.unwrap();
    assert_eq!(
        account.lamports, 0,
        "User record account lamports should be 0"
    );
    assert!(
        account.data.is_empty(),
        "User record account data should be empty"
    );

    let game_session_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_session_account.is_some(),
        "Game session account should exist after compression"
    );
    let account = game_session_account.unwrap();
    assert_eq!(
        account.lamports, 0,
        "Game session account lamports should be 0"
    );
    assert!(
        account.data.is_empty(),
        "Game session account data should be empty"
    );

    // Verify compressed accounts exist and have correct data
    let compressed_user_record = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(
        compressed_user_record.address,
        Some(user_compressed_address)
    );
    assert_eq!(compressed_user_record.data.is_some(), true);

    let user_buf = compressed_user_record.data.unwrap().data;
    let user_record = UserRecord::from_compressed_data(&user_buf).unwrap();
    assert_eq!(user_record.name, "Combined User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, user.pubkey());

    let compressed_game_session = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(
        compressed_game_session.address,
        Some(game_compressed_address)
    );
    assert_eq!(compressed_game_session.data.is_some(), true);

    let game_buf = compressed_game_session.data.unwrap().data;
    let game_session = GameSession::from_compressed_data(&game_buf).unwrap();
    assert_eq!(game_session.session_id, session_id);
    assert_eq!(game_session.game_type, "Combined Game");
    assert_eq!(game_session.player, user.pubkey());
    assert_eq!(game_session.score, 0);
}

async fn test_compress_record_with_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    should_fail: bool,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    // Get the current decompressed user record data
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

    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    let address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;
    let compressed_address = compressed_account.address.unwrap();

    // Get validity proof from RPC
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get output state tree index
    let output_state_tree_index =
        remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);

    // Create compressed account meta
    let compressed_account_meta = CompressedAccountMeta {
        tree_info: packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos[0],
        address: compressed_address,
        output_state_tree_index,
    };

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create the instruction
    let accounts = anchor_compressible_user::accounts::CompressRecordWithConfig {
        user: payer.pubkey(),
        user_record: *user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::CompressRecordWithConfig {
        proof: rpc_result.proof,
        compressed_account_meta,
    };

    // Build the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    if should_fail {
        assert!(result.is_err(), "Compress transaction should fail");
        return result;
    } else {
        assert!(result.is_ok(), "Compress transaction should succeed");
    }

    // Verify the PDA account is now empty (compressed)
    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_account.is_some(),
        "Account should exist after compression"
    );
    let account = user_pda_account.unwrap();
    assert_eq!(
        account.lamports, 0,
        "Account lamports should be 0 after compression"
    );
    assert!(
        account.data.is_empty(),
        "Account data should be empty after compression"
    );

    // Verify the compressed account exists
    let compressed_user_record = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(compressed_user_record.address, Some(compressed_address));
    assert_eq!(compressed_user_record.data.is_some(), true);

    let buf = compressed_user_record.data.unwrap().data;
    let user_record = UserRecord::from_compressed_data(&buf).unwrap();

    assert_eq!(user_record.name, "Test User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, payer.pubkey());
    assert_eq!(user_record.compression_info.is_compressed(), true);
    Ok(result.unwrap())
}

async fn test_decompress_single_user_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    user_record_bump: &u8,
    expected_user_name: &str,
    expected_slot: u64,
) {
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();

    // Get compressed user record
    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value;

    let user_account_data = c_user_pda.data.as_ref().unwrap();
    let c_user_record = UserRecord::from_compressed_data(&user_account_data.data).unwrap();

    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    remaining_accounts.add_system_accounts(system_config);

    // Get validity proof for the compressed account
    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Pack tree infos
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Create compressed account meta
    let user_compressed_meta = CompressedAccountMeta {
        tree_info: packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos[0],
        address: c_user_pda.address.unwrap(),
        output_state_tree_index: 0,
    };

    // Create compressed account data
    let compressed_accounts = vec![CompressedAccountData {
        meta: user_compressed_meta,
        data: CompressedAccountVariant::UserRecord(c_user_record),
    }];

    // Build instruction accounts
    let pda_accounts = vec![user_record_pda];
    let system_accounts_offset = pda_accounts.len() as u8;
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Prepare bump for the PDA
    let bumps = vec![*user_record_bump];

    let instruction_data = anchor_compressible_user::instruction::DecompressMultiplePdas {
        proof: rpc_result.proof,
        compressed_accounts,
        bumps,
        system_accounts_offset,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true), // fee_payer
                AccountMeta::new(payer.pubkey(), true), // rent_payer
                AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
            ],
            pda_accounts
                .iter()
                .map(|&pda| AccountMeta::new(*pda, false))
                .collect(),
            system_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Verify PDA is uninitialized before decompression
    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert_eq!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "User PDA account data len must be 0 before decompression"
    );

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    // Verify UserRecord PDA is decompressed
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
    assert_eq!(
        decompressed_user_record.compression_info.is_compressed(),
        false
    );
    assert_eq!(
        decompressed_user_record
            .compression_info
            .last_written_slot(),
        expected_slot
    );
}
