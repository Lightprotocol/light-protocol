use anchor_lang::{AccountDeserialize, AnchorDeserialize, Discriminator, ToAccountMetas};
use light_compressed_account::address::derive_address;
use light_compressed_token_sdk::ctoken;
use light_compressible_client::CompressibleInstruction;
use light_program_test::{
    program_test::{
        initialize_compression_config, setup_mock_program_data, LightProgramTest, TestRpc,
    },
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::compressible::{CompressAs, CompressibleConfig};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

mod helpers;
use helpers::{create_game_session, ADDRESS_SPACE, CTOKEN_RENT_SPONSOR, RENT_SPONSOR};

// Test: create, decompress game session, compress with custom data at
// compression
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
        RENT_SPONSOR,
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

#[allow(clippy::too_many_arguments)]
pub async fn decompress_single_game_session(
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
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

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

pub async fn compress_game_session_with_custom_data(
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
