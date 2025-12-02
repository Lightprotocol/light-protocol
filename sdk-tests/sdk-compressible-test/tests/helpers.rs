// Common test helpers and constants for all test files
#![allow(dead_code)]

use anchor_lang::{
    AccountDeserialize, AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use light_compressed_account::address::derive_address;
use light_compressible_client::{
    build_load_params,
    compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
    get_compressible_account::{deserialize_account, get_account_info_interface},
    CompressibleAccountInput,
};
use light_macros::pubkey;
use light_program_test::{program_test::LightProgramTest, AddressWithTree, Indexer, Rpc};
use light_sdk::{
    compressible::CompressibleConfig,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use sdk_compressible_test::{CompressedAccountVariant, GameSession, UserRecord};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub const ADDRESS_SPACE: [Pubkey; 1] = [pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")];
pub const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const CTOKEN_RENT_SPONSOR: Pubkey = pubkey!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");
pub const CTOKEN_RENT_AUTHORITY: Pubkey = pubkey!("8r3QmazwoLHYppYWysXPgUxYJ3Khn7vh3e313jYDcCKy");

// Helper functions used across multiple test files

pub async fn create_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    state_tree_queue: Option<Pubkey>,
) {
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(*program_id);
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let accounts = sdk_compressible_test::accounts::CreateRecord {
        user: payer.pubkey(),
        user_record: *user_record_pda,
        system_program: solana_sdk::system_program::ID,
        config: CompressibleConfig::derive_pda(program_id, 0).0,
        rent_sponsor: RENT_SPONSOR,
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

    let instruction_data = sdk_compressible_test::instruction::CreateRecord {
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

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;

    assert!(result.is_ok(), "Transaction should succeed");

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

    let user_record = UserRecord::deserialize(&mut &buf[..]).unwrap();

    assert_eq!(user_record.name, "Test User");
    assert_eq!(user_record.score, 11);
    assert_eq!(user_record.owner, payer.pubkey());
    assert!(user_record.compression_info.is_none());
}

pub async fn decompress_single_user_record(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    user_record_pda: &Pubkey,
    _user_record_bump: &u8,
    expected_user_name: &str,
    expected_slot: u64,
) {
    let address_tree = rpc.get_address_tree_v2();
    let address_tree_pubkey = address_tree.tree;

    // Use get_account_info_interface to fetch account info
    let account_info = get_account_info_interface(user_record_pda, program_id, &address_tree, rpc)
        .await
        .expect("Should fetch account")
        .expect("Account should exist");

    assert!(
        account_info.is_compressed,
        "Account should be compressed before decompression"
    );

    // Use deserialize_account to parse the account data
    let user_record: UserRecord =
        deserialize_account(&account_info).expect("Should deserialize user record");

    // Use build_load_params to create the decompress instruction
    let program_account_metas = sdk_compressible_test::accounts::DecompressAccountsIdempotent {
        fee_payer: payer.pubkey(),
        config: CompressibleConfig::derive_pda(program_id, 0).0,
        rent_sponsor: payer.pubkey(),
        ctoken_rent_sponsor: None,
        ctoken_config: None,
        ctoken_program: None,
        ctoken_cpi_authority: None,
        some_mint: payer.pubkey(),
        system_program: Pubkey::default(),
    }
    .to_account_metas(None);

    let instructions = build_load_params(
        rpc,
        program_id,
        &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[CompressibleAccountInput::new(
            *user_record_pda,
            account_info,
            CompressedAccountVariant::UserRecord(user_record),
        )],
        &program_account_metas,
        payer.pubkey(),
        payer.pubkey(),
        &[], // no ATAs to wrap
    )
    .await
    .expect("build_load_params should succeed");

    assert!(
        !instructions.is_empty(),
        "Should have at least one instruction"
    );

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();
    assert_eq!(
        user_pda_account.as_ref().map(|a| a.data.len()).unwrap_or(0),
        0,
        "User PDA account data len must be 0 before decompression"
    );

    let result = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await;
    assert!(result.is_ok(), "Decompress transaction should succeed");

    let user_pda_account = rpc.get_account(*user_record_pda).await.unwrap();

    let user_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
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
            .last_claimed_slot(),
        expected_slot
    );
}

pub async fn create_game_session(
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

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let accounts = sdk_compressible_test::accounts::CreateGameSession {
        player: payer.pubkey(),
        game_session: *game_session_pda,
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_sponsor: RENT_SPONSOR,
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

    let instruction_data = sdk_compressible_test::instruction::CreateGameSession {
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
