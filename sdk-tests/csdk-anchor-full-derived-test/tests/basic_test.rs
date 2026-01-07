use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::{
    AccountCreationData, CompressionParams, GameSession, UserRecord,
};
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
    program_test::{setup_mock_program_data, LightProgramTest},
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
async fn test_create_with_complex_seeds() {
    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize compression config using the macro-generated instruction
    let config_instruction =
        csdk_anchor_full_derived_test::instruction::InitializeCompressionConfig {
            rent_sponsor: RENT_SPONSOR,
            compression_authority: payer.pubkey(),
            rent_config: light_compressible::rent::RentConfig::default(),
            write_top_up: 5_000,
            address_space: vec![ADDRESS_SPACE[0]],
        };
    let config_accounts = csdk_anchor_full_derived_test::accounts::InitializeCompressionConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: _program_data_pda,
        authority: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let instruction = Instruction {
        program_id,
        accounts: config_accounts.to_account_metas(None),
        data: config_instruction.data(),
    };
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(
        result.is_ok(),
        "Initialize config should succeed: {:?}",
        result
    );

    // Create additional signers for complex seeds
    let authority = Keypair::new();
    let mint_authority_keypair = Keypair::new();
    let some_account = Keypair::new();

    let session_id = 42424u64;
    let category_id = 777u64;

    // Calculate PDAs with complex seeds using ctx accounts
    let (user_record_pda, _user_record_bump) = Pubkey::find_program_address(
        &[
            b"user_record",
            authority.pubkey().as_ref(),
            mint_authority_keypair.pubkey().as_ref(),
            payer.pubkey().as_ref(), // owner from instruction data
            category_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    // GameSession uses max_key(ctx.user, ctx.authority) for the seed
    let max_key_result =
        csdk_anchor_full_derived_test::max_key(&payer.pubkey(), &authority.pubkey());
    let (game_session_pda, _game_bump) = Pubkey::find_program_address(
        &[
            b"game_session",
            max_key_result.as_ref(),
            session_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    let mint_signer_pubkey = create_user_record_and_game_session(
        &mut rpc,
        &payer,
        &program_id,
        &config_pda,
        &user_record_pda,
        &game_session_pda,
        &authority,
        &mint_authority_keypair,
        &some_account,
        session_id,
        category_id,
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

    // Verify compressed user record
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

    assert_eq!(user_record.name, "Complex User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, payer.pubkey());
    assert_eq!(user_record.category_id, category_id);
    assert!(user_record.compression_info.is_none());

    // Verify compressed game session
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
    assert_eq!(game_session.game_type, "Complex Game");
    assert_eq!(game_session.player, payer.pubkey());
    assert_eq!(game_session.score, 0);
    assert!(game_session.compression_info.is_none());

    // Verify CToken account
    let spl_mint = find_cmint_address(&mint_signer_pubkey).0;
    let (_, token_account_address) =
        csdk_anchor_full_derived_test::get_ctokensigner_seeds(&payer.pubkey(), &spl_mint);

    let ctoken_accounts = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address, None, None)
        .await
        .unwrap()
        .value;

    assert!(
        !ctoken_accounts.items.is_empty(),
        "Should have compressed token accounts"
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn create_user_record_and_game_session(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    authority: &Keypair,
    mint_authority_keypair: &Keypair,
    some_account: &Keypair,
    session_id: u64,
    category_id: u64,
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
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = mint_authority;
    let mint_signer = Keypair::new();
    let compressed_mint_address =
        derive_cmint_compressed_address(&mint_signer.pubkey(), &address_tree_pubkey);

    let (spl_mint, mint_bump) = find_cmint_address(&mint_signer.pubkey());
    let accounts = csdk_anchor_full_derived_test::accounts::CreateUserRecordAndGameSession {
        user: user.pubkey(),
        mint_signer: mint_signer.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        authority: authority.pubkey(),
        mint_authority,
        some_account: some_account.pubkey(),
        ctoken_program: C_TOKEN_PROGRAM_ID.into(),
        compress_token_program_cpi_authority: light_ctoken_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_sponsor: RENT_SPONSOR,
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

    let instruction_data =
        csdk_anchor_full_derived_test::instruction::CreateUserRecordAndGameSession {
            account_data: AccountCreationData {
                // Instruction data fields (accounts come from ctx.accounts.*)
                owner: user.pubkey(),
                category_id,
                user_name: "Complex User".to_string(),
                session_id,
                game_type: "Complex Game".to_string(),
                placeholder_id: 0,
                counter: 0,
                mint_name: "Complex Token".to_string(),
                mint_symbol: "CPLX".to_string(),
                mint_uri: "https://example.com/complex.json".to_string(),
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
                    mint: Some(CompressedMintInstructionData {
                        supply: 0,
                        decimals,
                        metadata: CompressedMintMetadata {
                            version: 3,
                            mint: spl_mint.into(),
                            cmint_decompressed: false,
                            compressed_address: compressed_mint_address,
                        },
                        mint_authority: Some(mint_authority.into()),
                        freeze_authority: Some(freeze_authority.into()),
                        extensions: None,
                    }),
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
            &[user, &mint_signer, mint_authority_keypair, authority],
        )
        .await;

    assert!(
        result.is_ok(),
        "Complex seed creation transaction should succeed: {:?}",
        result
    );

    mint_signer.pubkey()
}
