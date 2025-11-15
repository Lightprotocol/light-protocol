use anchor_lang::{
    AccountDeserialize, AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use light_client::indexer::CompressedAccount;
use light_compressed_account::address::derive_address;
use light_compressed_token_sdk::{
    ctoken,
    instructions::{create_compressed_mint::find_spl_mint_address, derive_compressed_mint_address},
    pack::compat::CTokenDataWithVariant,
};
use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_compressible_client::CompressibleInstruction;
use light_ctoken_types::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMintMetadata,
};
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
use sdk_compressible_test::{
    get_ctoken_signer2_seeds, get_ctoken_signer3_seeds, get_ctoken_signer4_seeds,
    get_ctoken_signer5_seeds, get_ctoken_signer_seeds, CTokenAccountVariant,
    CompressedAccountVariant, GameSession, UserRecord,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

mod helpers;
use helpers::{create_game_session, create_record, ADDRESS_SPACE, RENT_SPONSOR};

// Tests
// 1. create and decompress two accounts and compress token accounts after
//    decompression
// 2. create and decompress accounts with different state trees
#[tokio::test]
async fn test_create_and_decompress_two_accounts() {
    let program_id = sdk_compressible_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_compressible_test", program_id)]));
    config = config.with_light_protocol_events();
    config.auto_register_custom_programs_for_pda_compression = true;

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
        vec![crate::helpers::ADDRESS_SPACE[0]],
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

    let (_, ctoken_account_address) = sdk_compressible_test::get_ctoken_signer_seeds(
        &combined_user.pubkey(),
        &ctoken_account.token.mint,
    );

    let (_, ctoken_account_address_2) =
        sdk_compressible_test::get_ctoken_signer2_seeds(&combined_user.pubkey());

    let (_, ctoken_account_address_3) =
        sdk_compressible_test::get_ctoken_signer3_seeds(&combined_user.pubkey());

    let (_, ctoken_account_address_4) = sdk_compressible_test::get_ctoken_signer4_seeds(
        &combined_user.pubkey(),
        &combined_user.pubkey(),
    );

    let (_, ctoken_account_address_5) = sdk_compressible_test::get_ctoken_signer5_seeds(
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

#[allow(clippy::too_many_arguments)]
pub async fn create_user_record_and_game_session(
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

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = mint_authority;
    let mint_signer = Keypair::new();
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_signer.pubkey(), &address_tree_pubkey);

    let (spl_mint, mint_bump) = find_spl_mint_address(&mint_signer.pubkey());
    let accounts = sdk_compressible_test::accounts::CreateUserRecordAndGameSession {
        user: user.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        mint_signer: mint_signer.pubkey(),
        ctoken_program: C_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_sponsor: RENT_SPONSOR,
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

    let instruction_data = sdk_compressible_test::instruction::CreateUserRecordAndGameSession {
        account_data: sdk_compressible_test::AccountCreationData {
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
        compression_params: sdk_compressible_test::CompressionParams {
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

#[allow(clippy::too_many_arguments)]
pub async fn decompress_multiple_pdas_with_ctoken(
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
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

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

    let ctoken_config = ctoken::config_pda();
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
            &sdk_compressible_test::accounts::DecompressAccountsIdempotent {
                fee_payer: payer.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_payer: payer.pubkey(),
                ctoken_rent_sponsor: ctoken::rent_sponsor_pda(),
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
        sdk_compressible_test::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        sdk_compressible_test::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
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
pub async fn decompress_multiple_pdas(
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
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

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
            &sdk_compressible_test::accounts::DecompressAccountsIdempotent {
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
        sdk_compressible_test::GameSession::DISCRIMINATOR,
        "Game account anchor discriminator mismatch"
    );

    let decompressed_game_session =
        sdk_compressible_test::GameSession::try_deserialize(&mut &game_pda_data[..]).unwrap();
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

#[allow(clippy::too_many_arguments)]
pub async fn compress_token_account_after_decompress(
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
        sdk_compressible_test::get_userrecord_seeds(&user.pubkey());
    let (game_session_seeds, game_session_pubkey) =
        sdk_compressible_test::get_gamesession_seeds(session_id);
    let (_, token_account_address) = get_ctoken_signer_seeds(&user.pubkey(), &mint);

    let (_, token_account_address_2) = get_ctoken_signer2_seeds(&user.pubkey());
    let (_, token_account_address_3) = get_ctoken_signer3_seeds(&user.pubkey());
    let (_, token_account_address_4) = get_ctoken_signer4_seeds(&user.pubkey(), &user.pubkey());
    let (_, token_account_address_5) = get_ctoken_signer5_seeds(&user.pubkey(), &mint, 42);
    let (_token_signer_seeds, _ctoken_1_authority_pda) =
        sdk_compressible_test::get_ctokensigner_authority_seeds();

    let (_token_signer_seeds_2, _ctoken_2_authority_pda) =
        sdk_compressible_test::get_ctokensigner2_authority_seeds();

    let (_token_signer_seeds_3, _ctoken_3_authority_pda) =
        sdk_compressible_test::get_ctokensigner3_authority_seeds();

    let (_token_signer_seeds_4, _ctoken_4_authority_pda) =
        sdk_compressible_test::get_ctokensigner4_authority_seeds();

    let (_token_signer_seeds_5, _ctoken_5_authority_pda) =
        sdk_compressible_test::get_ctokensigner5_authority_seeds();

    let _cpisigner = Pubkey::new_from_array(sdk_compressible_test::LIGHT_CPI_SIGNER.cpi_signer);

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
            sdk_compressible_test::instruction::CompressAccountsIdempotent::DISCRIMINATOR,
            &[user_record_pubkey, game_session_pubkey],
            &[user_record_account, game_session_account],
            &sdk_compressible_test::accounts::CompressAccountsIdempotent {
                fee_payer: user.pubkey(),
                config: CompressibleConfig::derive_pda(program_id, 0).0,
                rent_sponsor: RENT_SPONSOR,
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

#[tokio::test]
async fn test_create_and_decompress_accounts_with_different_state_trees() {
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
        100,
        RENT_SPONSOR,
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
