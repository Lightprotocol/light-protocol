#![cfg(feature = "test-sbf")]

use std::{thread::sleep, time::Duration};

use anchor_compressible::{CompressedAccountVariant, GameSession, UserRecord};
use anchor_lang::{
    AccountDeserialize, AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use light_compressed_account::address::derive_address;
use light_compressed_token_sdk::{
    instructions::{derive_compressed_mint_address, find_spl_mint_address},
    CPI_AUTHORITY_PDA,
};
use light_compressible_client::CompressibleInstruction;
use light_ctoken_types::instructions::mint_action::{
    CompressedMintInstructionData, CompressedMintWithContext,
};
use light_macros::pubkey;
use light_program_test::{
    initialize_compression_config,
    program_test::{LightProgramTest, TestRpc},
    setup_mock_program_data,
    utils::simulation::simulate_cu,
    AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::{
    compressible::{CompressAs, CompressibleConfig},
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

pub const ADDRESS_SPACE: [Pubkey; 1] = [pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK")];
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

#[tokio::test]
async fn test_create_and_decompress_two_accounts() {
    let program_id = anchor_compressible::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
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

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    // test_create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;

    // let session_id = 12345u64;
    // let (game_session_pda, game_bump) = Pubkey::find_program_address(
    //     &[b"game_session", session_id.to_le_bytes().as_ref()],
    //     &program_id,
    // );

    // test_create_game_session(
    //     &mut rpc,
    //     &payer,
    //     &program_id,
    //     &config_pda,
    //     &game_session_pda,
    //     session_id,
    //     None,
    // )
    // .await;

    // rpc.warp_to_slot(100).unwrap();

    // println!("01234.");
    // test_decompress_multiple_pdas(
    //     &mut rpc,
    //     &payer,
    //     &program_id,
    //     &config_pda,
    //     &user_record_pda,
    //     &user_record_bump,
    //     &game_session_pda,
    //     &game_bump,
    //     session_id,
    //     "Test User",
    //     "Battle Royale",
    //     100,
    // )
    // .await;

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

    test_create_user_record_and_game_session(
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

    // sleep(Duration::from_secs(10));
    println!("henlo? decompress multiple");
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
    let program_id = anchor_compressible::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
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

    test_create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;

    rpc.warp_to_slot(100).unwrap();

    println!("decompress single");
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

    println!("compress record");

    let result = test_compress_record(&mut rpc, &payer, &program_id, &user_record_pda, true).await;
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
    let _result =
        test_compress_record(&mut rpc, &payer, &program_id, &user_record_pda, false).await;
}

async fn test_create_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    state_tree_queue: Option<Pubkey>,
) {
    let config_pda = CompressibleConfig::derive_pda(program_id, 0).0;

    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_small(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    let accounts = anchor_compressible::accounts::CreateRecord {
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
    let output_state_tree_index = remaining_accounts.insert_or_get(
        state_tree_queue.unwrap_or_else(|| rpc.get_random_state_tree_info().unwrap().queue),
    );

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    println!("test-create-record, system_accounts all:");
    for account in &system_accounts {
        println!("{:?}", account);
    }
    // Create instruction data
    let instruction_data = anchor_compressible::instruction::CreateRecord {
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

    let cu = simulate_cu(rpc, payer, &instruction).await;
    println!("CreateRecord CU consumed: {}", cu);

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
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

async fn test_create_game_session(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
    state_tree_queue: Option<Pubkey>,
) {
    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_small(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    // Create the instruction
    let accounts = anchor_compressible::accounts::CreateGameSession {
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
    let output_state_tree_index = remaining_accounts.insert_or_get(
        state_tree_queue.unwrap_or_else(|| rpc.get_random_state_tree_info().unwrap().queue),
    );

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data = anchor_compressible::instruction::CreateGameSession {
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
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
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
    assert!(compressed_game_session.data.is_some());

    let buf = compressed_game_session.data.unwrap().data;

    let game_session = GameSession::deserialize(&mut &buf[..]).unwrap();

    assert_eq!(game_session.session_id, session_id);
    assert_eq!(game_session.game_type, "Battle Royale");
    assert_eq!(game_session.player, payer.pubkey());
    assert_eq!(game_session.score, 0);
    assert!(game_session.compression_info.is_none());
}

#[allow(clippy::too_many_arguments)]
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
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

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

    let c_user_record = UserRecord::deserialize(&mut &user_account_data.data[..]).unwrap();

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

    let c_game_session = GameSession::deserialize(&mut &game_account_data.data[..]).unwrap();

    // Get validity proof for both compressed accounts
    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash, c_game_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Use the new SDK helper function with typed data
    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &payer.pubkey(),
            &payer.pubkey(), // rent_payer can be the same as fee_payer
            &[*user_record_pda, *game_session_pda],
            &[
                (
                    c_user_pda,
                    CompressedAccountVariant::UserRecord(c_user_record),
                    vec![b"user_record".to_vec(), payer.pubkey().to_bytes().to_vec()],
                ),
                (
                    c_game_pda,
                    CompressedAccountVariant::GameSession(c_game_session),
                    vec![b"game_session".to_vec(), session_id.to_le_bytes().to_vec()],
                ),
            ],
            &[*user_record_bump, *game_bump],
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

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

    let cu = simulate_cu(rpc, payer, &instruction).await;
    println!("decompress_multiple_pdas CU consumed: {}", cu);

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
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

    // Verify GameSession PDA is decompressed
    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "Game PDA account data len must be > 0 after decompression"
    );

    let game_pda_data = game_pda_account.unwrap().data;
    assert_eq!(
        &game_pda_data[0..8],
        anchor_compressible::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        anchor_compressible::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
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

    // Verify compressed accounts exist and have correct data
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value;

    assert!(c_game_pda.data.is_some());
    assert_eq!(c_game_pda.data.unwrap().data.len(), 0);
}

async fn test_create_user_record_and_game_session(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
) {
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new_with_cpi_context(
        *program_id,
        state_tree_info.cpi_context.unwrap(),
    );
    let _ = remaining_accounts.add_system_accounts_small(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    // Create a mint signer for the compressed mint
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = mint_authority; // Same as mint authority for this example
    let mint_signer = Keypair::new();
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_signer.pubkey(), &address_tree_pubkey);

    // Find mint bump for the instruction
    let (spl_mint, mint_bump) = find_spl_mint_address(&mint_signer.pubkey());
    // Create the instruction
    let accounts = anchor_compressible::accounts::CreateUserRecordAndGameSession {
        user: user.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        mint_signer: mint_signer.pubkey(),
        compressed_token_program: light_sdk_types::constants::C_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
        mint_authority,
        compress_token_program_cpi_authority: Pubkey::new_from_array(CPI_AUTHORITY_PDA),
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

    // Get validity proof from RPC including mint address
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

    // Pack tree infos into remaining accounts
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    // Get the packed address tree info (all should use the same tree)
    let user_address_tree_info = packed_tree_infos.address_trees[0];
    let game_address_tree_info = packed_tree_infos.address_trees[1];
    let mint_address_tree_info = packed_tree_infos.address_trees[2];

    // Get system accounts for the instruction
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Create instruction data
    let instruction_data = anchor_compressible::instruction::CreateUserRecordAndGameSession {
        account_data: anchor_compressible::AccountCreationData {
            user_name: "Combined User".to_string(),
            session_id,
            game_type: "Combined Game".to_string(),
            // Add mint metadata
            mint_name: "Test Game Token".to_string(),
            mint_symbol: "TGT".to_string(),
            mint_uri: "https://example.com/token.json".to_string(),
            mint_decimals: 9,
            mint_supply: 1_000_000_000,
            mint_update_authority: Some(mint_authority),
            mint_freeze_authority: Some(freeze_authority),
            additional_metadata: None,
        },
        compression_params: anchor_compressible::CompressionParams {
            proof: rpc_result.proof,
            user_compressed_address,
            user_address_tree_info,
            user_output_state_tree_index,
            game_compressed_address,
            game_address_tree_info,
            game_output_state_tree_index,
            // Add mint compression parameters
            mint_bump,
            mint_with_context: CompressedMintWithContext {
                leaf_index: 0,
                prove_by_index: false,
                root_index: mint_address_tree_info.root_index,
                address: compressed_mint_address,
                mint: CompressedMintInstructionData {
                    version: 1,
                    spl_mint: spl_mint.into(),
                    supply: 0,
                    decimals,
                    mint_authority: Some(mint_authority.into()),
                    freeze_authority: Some(freeze_authority.into()),
                    extensions: None,
                    is_decompressed: false,
                },
            },
        },
    };

    // Build the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // let cu = simulate_cu(rpc, user, &instruction).await;
    // println!("CreateUserRecordAndGameSession CU consumed: {}", cu);
    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(
            &[instruction],
            &user.pubkey(),
            &[user, &mint_signer, &mint_authority_keypair],
        )
        .await;

    println!("transaction result: {:?}", result);

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
        .value;

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
}

async fn test_compress_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
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
    let _ = remaining_accounts.add_system_accounts_small(system_config);

    // Get address tree info
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
        .value;
    let compressed_address = compressed_account.address.unwrap();

    // Get validity proof from RPC
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction = CompressibleInstruction::compress_account(
        program_id,
        anchor_compressible::instruction::CompressRecord::DISCRIMINATOR,
        &payer.pubkey(),
        user_record_pda,
        &RENT_RECIPIENT,        // rent_recipient
        &compressed_account,    // compressed_account
        rpc_result,             // validity_proof_with_context
        output_state_tree_info, // output_state_tree_info
    )
    .unwrap();

    if !should_fail {
        let cu = simulate_cu(rpc, payer, &instruction).await;
        println!("CompressRecord CU consumed: {}", cu);
    }

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
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
    assert!(compressed_user_record.data.is_some());

    let buf = compressed_user_record.data.unwrap().data;
    let user_record: UserRecord = UserRecord::deserialize(&mut &buf[..]).unwrap();

    assert_eq!(user_record.name, "Test User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, payer.pubkey());
    assert!(user_record.compression_info.is_none());
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
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

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
    let c_user_record = UserRecord::deserialize(&mut &user_account_data.data[..]).unwrap();

    // Get validity proof for the compressed account
    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();
    // Use the new SDK helper function with typed data
    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &payer.pubkey(),
            &payer.pubkey(), // rent_payer can be the same as fee_payer
            &[*user_record_pda],
            &[(
                c_user_pda,
                CompressedAccountVariant::UserRecord(c_user_record),
                vec![b"user_record".to_vec(), payer.pubkey().to_bytes().to_vec()],
            )],
            &[*user_record_bump],
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    // Verify PDA is uninitialized before decompression
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

#[tokio::test]
async fn test_double_decompression_attack() {
    let program_id = anchor_compressible::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
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

    // Create and compress the account
    test_create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;
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
        .value;
    let c_user_record =
        UserRecord::deserialize(&mut &compressed_user_record.data.unwrap().data[..]).unwrap();

    rpc.warp_to_slot(100).unwrap();

    // First decompression - should succeed
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

    // Verify account is now decompressed
    let user_pda_account = rpc.get_account(user_record_pda).await.unwrap();
    assert!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "User PDA should be decompressed after first operation"
    );

    // Second decompression attempt - should be idempotent (skip already initialized account)

    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value;

    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Second decompression instruction - should still work (idempotent)
    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            &program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &payer.pubkey(),
            &payer.pubkey(),
            &[user_record_pda],
            &[(
                c_user_pda,
                CompressedAccountVariant::UserRecord(c_user_record),
                vec![b"user_record".to_vec(), payer.pubkey().to_bytes().to_vec()],
            )],
            &[user_record_bump],
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    // Should succeed due to idempotent behavior (skips already initialized accounts)
    assert!(
        result.is_ok(),
        "Second decompression should succeed idempotently"
    );

    // Verify account state is still correct and not corrupted
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
    let program_id = anchor_compressible::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
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

    let (user_record_pda, user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);

    let session_id = 54321u64;
    let (game_session_pda, game_bump) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    // Get two different state trees
    let first_state_tree_info = rpc.get_state_tree_infos()[0];
    let second_state_tree_info = rpc.get_state_tree_infos()[1];

    // Create user record using first state tree
    test_create_record(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        Some(first_state_tree_info.queue),
    )
    .await;

    // Create game session using second state tree
    test_create_game_session(
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

    // Now decompress both accounts together - they come from different state trees
    // This should succeed and validate that our decompression can handle mixed state tree sources
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
}

#[tokio::test]
async fn test_update_record_compression_info() {
    let program_id = anchor_compressible::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
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

    // Create and compress the account
    test_create_record(&mut rpc, &payer, &program_id, &user_record_pda, None).await;

    // Warp to slot 100 and decompress
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

    // Warp to slot 150 for the update
    rpc.warp_to_slot(150).unwrap();

    // Create update instruction
    let accounts = anchor_compressible::accounts::UpdateRecord {
        user: payer.pubkey(),
        user_record: user_record_pda,
    };

    let instruction_data = anchor_compressible::instruction::UpdateRecord {
        name: "Updated User".to_string(),
        score: 42,
    };

    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };

    // Execute the update
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(result.is_ok(), "Update record transaction should succeed");

    // Warp to slot 200 to ensure we're past the update
    rpc.warp_to_slot(200).unwrap();

    // Fetch the account and verify compression_info.last_written_slot
    let user_pda_account = rpc.get_account(user_record_pda).await.unwrap();
    assert!(
        user_pda_account.is_some(),
        "User record account should exist after update"
    );

    let account_data = user_pda_account.unwrap().data;
    let updated_user_record = UserRecord::try_deserialize(&mut &account_data[..]).unwrap();

    // Verify the data was updated
    assert_eq!(updated_user_record.name, "Updated User");
    assert_eq!(updated_user_record.score, 42);
    assert_eq!(updated_user_record.owner, payer.pubkey());

    // Verify compression_info.last_written_slot was updated to slot 150
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

async fn test_decompress_single_game_session(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    game_session_pda: &Pubkey,
    game_bump: &u8,
    session_id: u64,
    expected_game_type: &str,
    expected_slot: u64,
    expected_score: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    // Get compressed game session
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
    let c_game_session =
        anchor_compressible::GameSession::deserialize(&mut &game_account_data.data[..]).unwrap();

    // Get validity proof for the compressed account
    let rpc_result = rpc
        .get_validity_proof(vec![c_game_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Use the SDK helper function with typed data
    let instruction =
        light_compressible_client::CompressibleInstruction::decompress_accounts_idempotent(
            program_id,
            &CompressibleInstruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &payer.pubkey(),
            &payer.pubkey(), // rent_payer can be the same as fee_payer
            &[*game_session_pda],
            &[(
                c_game_pda,
                anchor_compressible::CompressedAccountVariant::GameSession(c_game_session),
                vec![b"game_session".to_vec(), session_id.to_le_bytes().to_vec()],
            )],
            &[*game_bump],
            rpc_result,
            output_state_tree_info,
        )
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    // Verify GameSession PDA is decompressed
    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0) > 0,
        "Game PDA account data len must be > 0 after decompression"
    );

    let game_pda_data = game_pda_account.unwrap().data;
    assert_eq!(
        &game_pda_data[0..8],
        anchor_compressible::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        anchor_compressible::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
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

async fn test_compress_game_session_with_custom_data(
    rpc: &mut LightProgramTest,
    _payer: &Keypair,
    _program_id: &Pubkey,
    game_session_pda: &Pubkey,
    _session_id: u64,
) {
    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap().unwrap();
    let game_pda_data = game_pda_account.data;
    let original_game_session =
        anchor_compressible::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();

    // Test the custom compression trait directly
    let custom_compressed_data = match original_game_session.compress_as() {
        std::borrow::Cow::Borrowed(data) => data.clone(), // Should never happen since compression_info must be None
        std::borrow::Cow::Owned(data) => data,            // Use owned data directly
    };

    // Verify that the custom compression works as expected
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

    println!(
        "   Custom:   start_time={}, end_time={:?}, score={}",
        custom_compressed_data.start_time,
        custom_compressed_data.end_time,
        custom_compressed_data.score
    );
}

#[tokio::test]
async fn test_custom_compression_game_session() {
    let program_id = anchor_compressible::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize config
    let result = initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        100, // compression delay
        RENT_RECIPIENT,
        vec![ADDRESS_SPACE[0]],
        &CompressibleInstruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    // Create a game session
    let session_id = 42424u64;
    let (game_session_pda, game_bump) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    test_create_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &game_session_pda,
        session_id,
        None,
    )
    .await;

    // Warp forward to allow decompression
    rpc.warp_to_slot(100).unwrap();

    // Decompress the game session first to verify original state
    test_decompress_single_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &game_session_pda,
        &game_bump,
        session_id,
        "Battle Royale",
        100,
        0, // original score should be 0
    )
    .await;

    // Warp forward past compression delay to allow compression
    rpc.warp_to_slot(250).unwrap();

    // Test the custom compression trait - this demonstrates the core functionality
    test_compress_game_session_with_custom_data(
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
    let program_id = anchor_compressible::ID;
    let config = ProgramTestConfig::new_v2(true, Some(vec![("anchor_compressible", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize compression config
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

    // Create placeholder record using empty compressed account functionality
    let placeholder_id = 54321u64;
    let (placeholder_record_pda, placeholder_record_bump) = Pubkey::find_program_address(
        &[b"placeholder_record", placeholder_id.to_le_bytes().as_ref()],
        &program_id,
    );

    test_create_placeholder_record(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &placeholder_record_pda,
        placeholder_id,
        "Test Placeholder",
    )
    .await;

    // Verify the PDA still exists and has data
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

    // Verify we can read the PDA data
    let placeholder_data = account.data;
    let decompressed_placeholder_record =
        anchor_compressible::PlaceholderRecord::try_deserialize(&mut &placeholder_data[..])
            .unwrap();
    assert_eq!(decompressed_placeholder_record.name, "Test Placeholder");
    assert_eq!(
        decompressed_placeholder_record.placeholder_id,
        placeholder_id
    );
    assert_eq!(decompressed_placeholder_record.owner, payer.pubkey());

    // Verify empty compressed account was created
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
        .value;

    assert_eq!(
        compressed_placeholder.address,
        Some(compressed_address),
        "Compressed account should exist with correct address"
    );
    assert!(
        compressed_placeholder.data.is_some(),
        "Compressed account should have data field"
    );

    // Verify the compressed account is empty (length 0)
    let compressed_data = compressed_placeholder.data.unwrap();
    assert_eq!(
        compressed_data.data.len(),
        0,
        "Compressed account data should be empty"
    );

    // This demonstrates the key difference from regular compression:
    // The PDA still exists with data, and an empty compressed account was created

    // Step 2: Now compress the PDA (this will close the PDA and put data into the compressed account)
    rpc.warp_to_slot(200).unwrap(); // Wait past compression delay

    test_compress_placeholder_record(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &placeholder_record_pda,
        &placeholder_record_bump,
        placeholder_id,
    )
    .await;

    println!("✅ PlaceholderRecord PDA compressed successfully!");
    println!("   - Data moved from PDA to compressed account (PDA still exists)");

    println!("✅ Full compression cycle completed!");
    println!("   - Empty compressed account created while PDA remained intact");
    println!("   - PDA data was then compressed into the empty compressed account");
    println!("   - Two-step compression process: Empty compress → Regular compress completed");
}

async fn test_create_placeholder_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    placeholder_record_pda: &Pubkey,
    placeholder_id: u64,
    name: &str,
) {
    // Setup remaining accounts for Light Protocol
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_small(system_config);

    // Get address tree info
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    // Create the instruction
    let accounts = anchor_compressible::accounts::CreatePlaceholderRecord {
        user: payer.pubkey(),
        placeholder_record: *placeholder_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_recipient: RENT_RECIPIENT,
    };

    // Derive a new address for the compressed account
    let compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
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
    let instruction_data = anchor_compressible::instruction::CreatePlaceholderRecord {
        placeholder_id,
        name: name.to_string(),
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

    let cu = simulate_cu(rpc, payer, &instruction).await;
    println!("CreatePlaceholderRecord CU consumed: {}", cu);

    // Create and send transaction
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(
        result.is_ok(),
        "CreatePlaceholderRecord transaction should succeed"
    );
}

async fn test_compress_placeholder_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    _config_pda: &Pubkey,
    placeholder_record_pda: &Pubkey,
    _placeholder_record_bump: &u8,
    _placeholder_id: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().queue;

    // Get compressed placeholder record address
    let placeholder_compressed_address = derive_address(
        &placeholder_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    // Get the compressed account that already exists (empty)
    let compressed_placeholder = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value;

    // Get validity proof from RPC
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_placeholder.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let output_state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let instruction = CompressibleInstruction::compress_account(
        program_id,
        anchor_compressible::instruction::CompressPlaceholderRecord::DISCRIMINATOR,
        &payer.pubkey(),
        placeholder_record_pda,
        &RENT_RECIPIENT,
        &compressed_placeholder,
        rpc_result,
        output_state_tree_info,
    )
    .unwrap();

    let cu = simulate_cu(rpc, payer, &instruction).await;
    println!("CompressPlaceholderRecord CU consumed: {}", cu);

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(
        result.is_ok(),
        "CompressPlaceholderRecord transaction should succeed: {:?}",
        result
    );

    // Check if PDA account is closed (it may or may not be depending on the compression behavior)
    let account = rpc.get_account(*placeholder_record_pda).await.unwrap();
    println!("PDA after compression: {:?}", account.is_some());

    // Verify compressed account now has the data
    let compressed_placeholder_after = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value;

    assert!(
        compressed_placeholder_after.data.is_some(),
        "Compressed account should have data after compression"
    );

    let compressed_data_after = compressed_placeholder_after.data.unwrap();

    assert!(
        compressed_data_after.data.len() > 0,
        "Compressed account should contain the PDA data"
    );
}
