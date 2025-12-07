use anchor_lang::{AccountDeserialize, AnchorDeserialize, InstructionData, ToAccountMetas};
use csdk_anchor_derived_test::{AccountCreationData, CompressionParams, GameSession, UserRecord};
use light_compressed_account::address::derive_address;
use light_ctoken_interface::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMintMetadata,
};
use light_ctoken_sdk::compressed_token::create_compressed_mint::{
    derive_cmint_compressed_address, find_cmint_address,
};
use light_macros::pubkey;
use light_program_test::{
    program_test::{
        initialize_compression_config, setup_mock_program_data, LightProgramTest, TestRpc,
    },
    AddressWithTree, Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::{
    compressible::CompressibleConfig,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const ADDRESS_SPACE: [Pubkey; 1] = [pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")];
const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

#[tokio::test]
async fn test_create_decompress_compress() {
    let program_id = csdk_anchor_derived_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_derived_test", program_id)]));
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
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &light_compressible_client::compressible_instruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await;
    assert!(result.is_ok(), "Initialize config should succeed");

    let session_id = 42424u64;
    let (user_record_pda, _user_record_bump) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);
    let (game_session_pda, _game_bump) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    let mint_signer_pubkey = create_user_record_and_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        &game_session_pda,
        session_id,
    )
    .await;

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

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
    assert_eq!(user_record.owner, payer.pubkey());
    assert!(user_record.compression_info.is_none());

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
    assert_eq!(game_session.game_type, "Test Game");
    assert_eq!(game_session.player, payer.pubkey());
    assert_eq!(game_session.score, 0);
    assert!(game_session.compression_info.is_none());

    let spl_mint = find_cmint_address(&mint_signer_pubkey).0;
    let (_, token_account_address) =
        csdk_anchor_derived_test::seeds::get_ctoken_signer_seeds(&payer.pubkey(), &spl_mint);

    let ctoken_accounts = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address, None, None)
        .await
        .unwrap()
        .value;

    assert!(
        !ctoken_accounts.items.is_empty(),
        "Should have compressed token accounts"
    );

    // Test decompress PDAs (UserRecord + GameSession)
    // Note: CToken decompression works but requires manual instruction building
    // because the client helper doesn't handle mixed PDA+token packing correctly
    rpc.warp_to_slot(100).unwrap();

    decompress_pdas(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        &game_session_pda,
        session_id,
        100,
    )
    .await;

    // Test compress PDAs after decompression
    rpc.warp_to_slot(200).unwrap();

    compress_pdas(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        &game_session_pda,
        session_id,
    )
    .await;
}

#[tokio::test]
async fn test_auto_compress_on_warp_forward() {
    use light_compressible::rent::SLOTS_PER_EPOCH;
    let program_id = csdk_anchor_derived_test::ID;
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("csdk_anchor_derived_test", program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Initialize compressible config
    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);
    initialize_compression_config(
        &mut rpc,
        &payer,
        &program_id,
        &payer,
        RENT_SPONSOR,
        vec![ADDRESS_SPACE[0]],
        &light_compressible_client::compressible_instruction::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR,
        None,
    )
    .await
    .expect("Initialize config should succeed");

    // PDAs
    let session_id = 5555u64;
    let (user_record_pda, _) =
        Pubkey::find_program_address(&[b"user_record", payer.pubkey().as_ref()], &program_id);
    let (game_session_pda, _) = Pubkey::find_program_address(
        &[b"game_session", session_id.to_le_bytes().as_ref()],
        &program_id,
    );

    // Create + compress initial state via helper (combined create path)
    let _mint_signer_pubkey = create_user_record_and_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        &game_session_pda,
        session_id,
    )
    .await;

    // Decompress both PDAs
    rpc.warp_to_slot(100).unwrap();
    decompress_pdas(
        &mut rpc,
        &payer,
        &program_id,
        &user_record_pda,
        &game_session_pda,
        session_id,
        100,
    )
    .await;

    // Warp two epochs to ensure PDAs are compressible
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 2).await.unwrap();

    // Also invoke auto-compress directly to ensure it's executed in this test context
    light_program_test::compressible::auto_compress_program_pdas(&mut rpc, program_id)
        .await
        .unwrap();

    // After auto-compress, PDAs should be closed or emptied
    let user_acc = rpc.get_account(user_record_pda).await.unwrap();
    let game_acc = rpc.get_account(game_session_pda).await.unwrap();
    let user_closed = user_acc.is_none()
        || user_acc
            .as_ref()
            .map(|a| a.data.is_empty() || a.lamports == 0)
            .unwrap_or(true);
    let game_closed = game_acc.is_none()
        || game_acc
            .as_ref()
            .map(|a| a.data.is_empty() || a.lamports == 0)
            .unwrap_or(true);
    assert!(
        user_closed && game_closed,
        "Auto-compress should close PDAs"
    );
}

#[allow(clippy::too_many_arguments)]
async fn decompress_pdas(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
    expected_slot: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    // Get compressed PDA accounts
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

    let c_user_pda = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let c_game_pda = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let user_account_data = c_user_pda.data.as_ref().unwrap();
    let c_user_record = UserRecord::deserialize(&mut &user_account_data.data[..]).unwrap();

    let game_account_data = c_game_pda.data.as_ref().unwrap();
    let c_game_session = GameSession::deserialize(&mut &game_account_data.data[..]).unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![c_user_pda.hash, c_game_pda.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let instruction =
        light_compressible_client::compressible_instruction::decompress_accounts_idempotent(
            program_id,
            &light_compressible_client::compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*user_record_pda, *game_session_pda],
            &[
                (
                    c_user_pda.clone(),
                    csdk_anchor_derived_test::CompressedAccountVariant::UserRecord(c_user_record),
                ),
                (
                    c_game_pda.clone(),
                    csdk_anchor_derived_test::CompressedAccountVariant::GameSession(c_game_session),
                ),
            ],
            &csdk_anchor_derived_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_sponsor: payer.pubkey(),
                ctoken_rent_sponsor: None,
                ctoken_config: None,
                ctoken_program: None,
                ctoken_cpi_authority: None,
                some_mint: payer.pubkey(),
            }
            .to_account_metas(None),
            rpc_result,
        )
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(result.is_ok(), "Decompress PDAs transaction should succeed");

    // Verify user record decompressed
    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_account.is_some(),
        "User PDA should exist after decompression"
    );
    let decompressed_user_record =
        UserRecord::try_deserialize(&mut &user_pda_account.unwrap().data[..]).unwrap();
    assert_eq!(decompressed_user_record.name, "Combined User");
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
            .last_claimed_slot(),
        expected_slot
    );

    // Verify game session decompressed
    let game_pda_account = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_account.is_some(),
        "Game PDA should exist after decompression"
    );
    let decompressed_game_session =
        GameSession::try_deserialize(&mut &game_pda_account.unwrap().data[..]).unwrap();
    assert_eq!(decompressed_game_session.session_id, session_id);
    assert_eq!(decompressed_game_session.game_type, "Test Game");
    assert_eq!(decompressed_game_session.player, payer.pubkey());
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
            .last_claimed_slot(),
        expected_slot
    );

    // Verify compressed PDA accounts are empty
    let compressed_user = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        compressed_user.data.unwrap().data.is_empty(),
        "Compressed user should be empty after decompression"
    );

    let compressed_game = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        compressed_game.data.unwrap().data.is_empty(),
        "Compressed game should be empty after decompression"
    );
}

#[allow(clippy::too_many_arguments)]
async fn compress_pdas(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
) {
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    // Get PDA accounts
    let _user_pda_account = rpc
        .get_account(*user_record_pda)
        .await
        .unwrap()
        .expect("User PDA should exist before compression");
    let _game_pda_account = rpc
        .get_account(*game_session_pda)
        .await
        .unwrap()
        .expect("Game PDA should exist before compression");

    // Get compressed account hashes for proof
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

    let compressed_user = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let compressed_game = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let _rpc_result = rpc
        .get_validity_proof(
            vec![compressed_user.hash, compressed_game.hash],
            vec![],
            None,
        )
        .await
        .unwrap()
        .value;

    // TODO: remove in separate pr
    // let instruction =
    //     light_compressible_client::compressible_instruction::compress_accounts_idempotent(
    //         program_id,
    //         csdk_anchor_derived_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
    //         &[*user_record_pda, *game_session_pda],
    //         &[user_pda_account, game_pda_account],
    //         &csdk_anchor_derived_test::accounts::CompressAccountsIdempotent {
    //             fee_payer: payer.pubkey(),
    //             config: CompressibleConfig::derive_pda(program_id, 0).0,
    //             rent_sponsor: RENT_SPONSOR,
    //             compression_authority: payer.pubkey(),
    //         }
    //         .to_account_metas(None),
    //         rpc_result,
    //     )
    //     .unwrap();

    // let result = rpc
    //     .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
    //     .await;

    // assert!(result.is_ok(), "Compress PDAs transaction should succeed");

    rpc.warp_slot_forward(light_compressible::rent::SLOTS_PER_EPOCH * 2)
        .await
        .unwrap();

    // Verify PDAs are closed
    let user_pda_after = rpc.get_account(*user_record_pda).await.unwrap();
    assert!(
        user_pda_after.is_none(),
        "User PDA should be closed after compression"
    );

    let game_pda_after = rpc.get_account(*game_session_pda).await.unwrap();
    assert!(
        game_pda_after.is_none(),
        "Game PDA should be closed after compression"
    );

    // Verify compressed PDA accounts have data
    let compressed_user_after = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(compressed_user_after.address, Some(user_compressed_address));
    let user_buf = compressed_user_after.data.unwrap().data;
    let user_record = UserRecord::deserialize(&mut &user_buf[..]).unwrap();
    assert_eq!(user_record.name, "Combined User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, payer.pubkey());
    assert!(user_record.compression_info.is_none());

    let compressed_game_after = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(compressed_game_after.address, Some(game_compressed_address));
    let game_buf = compressed_game_after.data.unwrap().data;
    let game_session = GameSession::deserialize(&mut &game_buf[..]).unwrap();
    assert_eq!(game_session.session_id, session_id);
    assert_eq!(game_session.game_type, "Test Game");
    assert!(game_session.compression_info.is_none());
}

#[allow(clippy::too_many_arguments)]
pub async fn create_user_record_and_game_session(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    session_id: u64,
) -> Pubkey {
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new_with_cpi_context(
        *program_id,
        state_tree_info.cpi_context.unwrap(),
    );
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = mint_authority;
    let mint_signer = Keypair::new();
    let compressed_mint_address =
        derive_cmint_compressed_address(&mint_signer.pubkey(), &address_tree_pubkey);

    let (spl_mint, mint_bump) = find_cmint_address(&mint_signer.pubkey());
    let accounts = csdk_anchor_derived_test::accounts::CreateUserRecordAndGameSession {
        user: user.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        mint_signer: mint_signer.pubkey(),
        ctoken_program: C_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_sponsor: RENT_SPONSOR,
        mint_authority,
        compress_token_program_cpi_authority: light_compressed_token_types::CPI_AUTHORITY_PDA
            .into(),
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

    let instruction_data = csdk_anchor_derived_test::instruction::CreateUserRecordAndGameSession {
        account_data: AccountCreationData {
            user_name: "Combined User".to_string(),
            session_id,
            game_type: "Test Game".to_string(),
            mint_name: "Test Game Token".to_string(),
            mint_symbol: "TGT".to_string(),
            mint_uri: "https://example.com/token.json".to_string(),
            mint_decimals: 9,
            mint_supply: 1_000_000_000,
            mint_update_authority: Some(mint_authority),
            mint_freeze_authority: Some(freeze_authority),
            additional_metadata: None,
        },
        compression_params: CompressionParams {
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
        "Combined creation transaction should succeed: {:?}",
        result
    );

    mint_signer.pubkey()
}
