mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{
    get_create_accounts_proof, AccountInterfaceExt, CreateAccountsProofInput,
    InitializeRentFreeConfig,
};
use light_compressible::{rent::SLOTS_PER_EPOCH, DECOMPRESSED_PDA_DISCRIMINATOR};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::utils::derive_rent_sponsor_pda;
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::find_mint_address as find_cmint_address;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// 2 PDAs + 1 Mint + 1 Vault + 1 User ATA, all in one instruction with single proof.
/// After init: all accounts on-chain + parseable.
/// After warp: all cold (auto-compressed) with non-empty compressed data.
#[tokio::test]
async fn test_create_pdas_and_mint_auto() {
    use csdk_anchor_full_derived_test::{
        instruction_accounts::{LP_MINT_SIGNER_SEED, VAULT_SEED},
        FullAutoWithMintParams, GameSession,
    };
    use light_token::instruction::{
        get_associated_token_address_and_bump, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR,
    };
    use light_token_interface::state::Token;

    // Helper
    fn parse_token(data: &[u8]) -> Token {
        borsh::BorshDeserialize::deserialize(&mut &data[..]).unwrap()
    }

    let program_id = csdk_anchor_full_derived_test::ID;
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    )
    .with_decoders(vec![
        Box::new(csdk_anchor_full_derived_test::CsdkTestInstructionDecoder),
        Box::new(csdk_anchor_full_derived_test::CsdkAnchorFullDerivedTestInstructionDecoder),
    ])
    .with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Derive rent sponsor PDA for this program
    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    // Fund the rent sponsor PDA so it can pay for decompression
    rpc.airdrop_lamports(&rent_sponsor, 10_000_000_000)
        .await
        .expect("Airdrop to rent sponsor should succeed");

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    let authority = Keypair::new();
    let mint_authority = Keypair::new();

    let owner = payer.pubkey();
    let category_id = 111u64;
    let session_id = 222u64;
    let vault_mint_amount = 100u64;
    let user_ata_mint_amount = 50u64;

    // Derive PDAs
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[LP_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_pda, _) = find_cmint_address(&mint_signer_pda);
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint_pda.as_ref()], &program_id);
    let (vault_authority_pda, _) = Pubkey::find_program_address(&[b"vault_authority"], &program_id);
    let (user_ata_pda, user_ata_bump) =
        get_associated_token_address_and_bump(&payer.pubkey(), &mint_pda);

    let (user_record_pda, _) = Pubkey::find_program_address(
        &[
            b"user_record",
            authority.pubkey().as_ref(),
            mint_authority.pubkey().as_ref(),
            owner.as_ref(),
            category_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    let max_key_result =
        csdk_anchor_full_derived_test::max_key(&payer.pubkey(), &authority.pubkey());
    let (game_session_pda, _) = Pubkey::find_program_address(
        &[
            csdk_anchor_full_derived_test::GAME_SESSION_SEED.as_bytes(),
            max_key_result.as_ref(),
            session_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(user_record_pda),
            CreateAccountsProofInput::pda(game_session_pda),
            CreateAccountsProofInput::mint(mint_signer_pda),
        ],
    )
    .await
    .unwrap();

    // Derive compressed addresses for later assertions
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let user_compressed_address = light_compressed_account::address::derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let game_compressed_address = light_compressed_account::address::derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let mint_compressed_address =
        light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
            &mint_signer_pda,
            &address_tree_pubkey,
        );

    let accounts = csdk_anchor_full_derived_test::accounts::CreatePdasAndMintAuto {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_authority: mint_authority.pubkey(),
        mint_signer: mint_signer_pda,
        user_record: user_record_pda,
        game_session: game_session_pda,
        mint: mint_pda,
        vault: vault_pda,
        vault_authority: vault_authority_pda,
        user_ata: user_ata_pda,
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    // Simplified instruction data - just pass create_accounts_proof directly
    let instruction_data = csdk_anchor_full_derived_test::instruction::CreatePdasAndMintAuto {
        params: FullAutoWithMintParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            category_id,
            session_id,
            mint_signer_bump,
            vault_bump,
            user_ata_bump,
            vault_mint_amount,
            user_ata_mint_amount,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(
        &[instruction],
        &payer.pubkey(),
        &[&payer, &authority, &mint_authority],
    )
    .await
    .unwrap();

    // PHASE 1: After init - all accounts on-chain and parseable
    shared::assert_onchain_exists(&mut rpc, &user_record_pda, "UserRecord").await;
    shared::assert_onchain_exists(&mut rpc, &game_session_pda, "GameSession").await;
    shared::assert_onchain_exists(&mut rpc, &mint_pda, "Mint").await;
    shared::assert_onchain_exists(&mut rpc, &vault_pda, "Vault").await;
    shared::assert_onchain_exists(&mut rpc, &user_ata_pda, "UserATA").await;

    // Full-struct assertion for UserRecord after init
    {
        let account = rpc.get_account(user_record_pda).await.unwrap().unwrap();
        let user_record: csdk_anchor_full_derived_test::UserRecord =
            borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
        let expected = csdk_anchor_full_derived_test::UserRecord {
            compression_info: shared::expected_compression_info(&user_record.compression_info),
            owner: payer.pubkey(),
            name: "Auto Created User With Mint".to_string(),
            score: 0,
            category_id,
        };
        assert_eq!(user_record, expected, "UserRecord should match after init");
    }

    // Parse and verify CToken data with full-struct comparison
    let vault_data = parse_token(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let expected_vault = Token {
            mint: mint_pda.into(),
            owner: vault_authority_pda.into(),
            amount: vault_mint_amount,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: vault_data.extensions.clone(),
        };
        assert_eq!(vault_data, expected_vault, "vault should match after init");
    }

    let user_ata_data = parse_token(&rpc.get_account(user_ata_pda).await.unwrap().unwrap().data);
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let expected_ata = Token {
            mint: mint_pda.into(),
            owner: payer.pubkey().into(),
            amount: user_ata_mint_amount,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: user_ata_data.extensions.clone(),
        };
        assert_eq!(
            user_ata_data, expected_ata,
            "user ATA should match after init"
        );
    }

    // Verify compressed addresses registered (decompressed PDA: data contains PDA pubkey)
    let compressed_cmint = rpc
        .get_compressed_account(mint_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(compressed_cmint.address.unwrap(), mint_compressed_address);
    // Decompressed PDAs have DECOMPRESSED_PDA_DISCRIMINATOR and data contains PDA pubkey
    let cmint_data = compressed_cmint.data.as_ref().unwrap();
    assert_eq!(cmint_data.discriminator, DECOMPRESSED_PDA_DISCRIMINATOR);
    assert_eq!(cmint_data.data, mint_pda.to_bytes().to_vec());

    // Verify GameSession initial state before compression
    // Fields with compress_as overrides should have their original values
    let initial_game_session_data = rpc
        .get_account(game_session_pda)
        .await
        .unwrap()
        .expect("GameSession should exist after init");
    let initial_game_session: GameSession =
        borsh::BorshDeserialize::deserialize(&mut &initial_game_session_data.data[8..])
            .expect("Failed to deserialize initial GameSession");

    // Verify initial state: start_time should be hardcoded value (2)
    assert_eq!(
        initial_game_session.start_time, 2,
        "Initial start_time should be 2 (hardcoded non-zero), got: {}",
        initial_game_session.start_time
    );
    assert_eq!(
        initial_game_session.session_id, session_id,
        "session_id should be preserved"
    );
    assert_eq!(
        initial_game_session.player,
        payer.pubkey(),
        "player should be payer"
    );
    assert_eq!(
        initial_game_session.game_type, "Auto Game With Mint",
        "game_type should match"
    );
    assert_eq!(
        initial_game_session.end_time, None,
        "end_time should be None"
    );
    assert_eq!(initial_game_session.score, 0, "score should be 0");

    // Store initial start_time for comparison after decompress
    let initial_start_time = initial_game_session.start_time;

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // After warp: all on-chain accounts should be closed
    shared::assert_onchain_closed(&mut rpc, &user_record_pda, "UserRecord").await;
    shared::assert_onchain_closed(&mut rpc, &game_session_pda, "GameSession").await;
    shared::assert_onchain_closed(&mut rpc, &mint_pda, "Mint").await;
    shared::assert_onchain_closed(&mut rpc, &vault_pda, "Vault").await;
    shared::assert_onchain_closed(&mut rpc, &user_ata_pda, "UserATA").await;

    // Compressed accounts should exist with non-empty data
    shared::assert_compressed_exists_with_data(&mut rpc, user_compressed_address, "UserRecord")
        .await;
    shared::assert_compressed_exists_with_data(&mut rpc, game_compressed_address, "GameSession")
        .await;
    shared::assert_compressed_exists_with_data(&mut rpc, mint_compressed_address, "Mint").await;

    // Compressed token accounts should exist with correct balances
    shared::assert_compressed_token_exists(&mut rpc, &vault_pda, vault_mint_amount, "Vault").await;
    shared::assert_compressed_token_exists(
        &mut rpc,
        &user_ata_pda,
        user_ata_mint_amount,
        "UserATA",
    )
    .await;

    // PHASE 3: Decompress all accounts via create_load_instructions
    use anchor_lang::AnchorDeserialize;
    use csdk_anchor_full_derived_test::{
        csdk_anchor_full_derived_test::{
            GameSessionSeeds, LightAccountVariant, UserRecordSeeds, VaultSeeds,
        },
        GameSession as GameSessionState, UserRecord,
    };
    use light_client::interface::{
        create_load_instructions, AccountInterface, AccountSpec, ColdContext, PdaSpec,
    };
    use light_account::TokenDataWithSeeds;

    // Fetch unified interfaces (hot/cold transparent)
    let user_interface = rpc
        .get_account_interface(&user_record_pda, &program_id)
        .await
        .expect("failed to get user");
    assert!(user_interface.is_cold(), "UserRecord should be cold");

    let game_interface = rpc
        .get_account_interface(&game_session_pda, &program_id)
        .await
        .expect("failed to get game");
    assert!(game_interface.is_cold(), "GameSession should be cold");

    let vault_interface = rpc
        .get_token_account_interface(&vault_pda)
        .await
        .expect("failed to get vault");
    assert!(vault_interface.is_cold(), "Vault should be cold");
    assert_eq!(vault_interface.amount(), vault_mint_amount);

    // Build PdaSpec for UserRecord
    let user_data = UserRecord::deserialize(&mut &user_interface.account.data[8..])
        .expect("Failed to parse UserRecord");
    let user_variant = LightAccountVariant::UserRecord {
        seeds: UserRecordSeeds {
            authority: authority.pubkey(),
            mint_authority: mint_authority.pubkey(),
            owner,
            category_id,
        },
        data: user_data,
    };
    let user_spec = PdaSpec::new(user_interface.clone(), user_variant, program_id);

    // Build PdaSpec for GameSession
    let game_data = GameSessionState::deserialize(&mut &game_interface.account.data[8..])
        .expect("Failed to parse GameSession");
    let game_variant = LightAccountVariant::GameSession {
        seeds: GameSessionSeeds {
            fee_payer: payer.pubkey(),
            authority: authority.pubkey(),
            session_id,
        },
        data: game_data,
    };
    let game_spec = PdaSpec::new(game_interface.clone(), game_variant, program_id);

    // Build PdaSpec for Vault (CToken)
    // Vault is fetched as token account but decompressed as PDA, so convert cold context
    let token =
        light_token_interface::state::Token::deserialize(&mut &vault_interface.account.data[..])
            .expect("Failed to parse Token");
    let vault_variant = LightAccountVariant::Vault(TokenDataWithSeeds {
        seeds: VaultSeeds { mint: mint_pda },
        token_data: token,
    });
    let vault_compressed = vault_interface
        .compressed()
        .expect("cold vault must have compressed data");
    // Convert TokenAccountInterface to AccountInterface with ColdContext::Account
    let vault_interface_for_pda = AccountInterface {
        key: vault_interface.key,
        account: vault_interface.account.clone(),
        cold: Some(ColdContext::Account(vault_compressed.account.clone())),
    };
    let vault_spec = PdaSpec::new(vault_interface_for_pda, vault_variant, program_id);

    // get_ata_interface: fetches ATA with unified handling using standard SPL types
    let ata_interface = rpc
        .get_ata_interface(&payer.pubkey(), &mint_pda)
        .await
        .expect("get_ata_interface should succeed");
    assert!(ata_interface.is_cold(), "ATA should be cold after warp");
    assert_eq!(ata_interface.amount(), user_ata_mint_amount);
    assert_eq!(ata_interface.mint(), mint_pda);
    // After fix: parsed.owner = wallet_owner (payer), not ATA address
    assert_eq!(ata_interface.owner(), payer.pubkey());

    // Use TokenAccountInterface directly for ATA
    // (no separate AtaSpec needed - TokenAccountInterface has all the data)

    // Fetch mint interface
    let mint_interface = rpc
        .get_mint_interface(&mint_pda)
        .await
        .expect("get_mint_interface should succeed");
    assert!(mint_interface.is_cold(), "Mint should be cold after warp");

    // Convert MintInterface to AccountInterface for use in AccountSpec
    let (compressed, _mint_data) = mint_interface
        .compressed()
        .expect("cold mint must have compressed data");
    let mint_account_interface = AccountInterface {
        key: mint_pda,
        account: solana_account::Account {
            lamports: 0,
            data: vec![],
            owner: light_token::instruction::LIGHT_TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
        cold: Some(ColdContext::Account(compressed.clone())),
    };

    // Build AccountSpec slice for all accounts
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![
        AccountSpec::Pda(user_spec),
        AccountSpec::Pda(game_spec),
        AccountSpec::Pda(vault_spec),
        AccountSpec::Ata(ata_interface.clone()),
        AccountSpec::Mint(mint_account_interface),
    ];

    // Load all accounts with single call
    let all_instructions = create_load_instructions(&specs, payer.pubkey(), config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    println!("all_instructions.len() = {:?}", all_instructions);

    // Expected: 1 PDA+Token ix + 2 ATA ixs (1 create_ata + 1 decompress) + 1 mint ix = 4
    assert_eq!(
        all_instructions.len(),
        6,
        "Should have 6 instructions: 1 PDA, 1 Token, 2 create_ata, 1 decompress_ata, 1 mint"
    );

    // Capture rent sponsor balance before decompression
    let rent_sponsor_balance_before = rpc
        .get_account(rent_sponsor)
        .await
        .expect("get rent sponsor account")
        .map(|a| a.lamports)
        .unwrap_or(0);

    // Execute all instructions
    rpc.create_and_send_transaction(&all_instructions, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // Assert rent sponsor paid for the decompressed PDA accounts
    shared::assert_rent_sponsor_paid_for_accounts(
        &mut rpc,
        &rent_sponsor,
        rent_sponsor_balance_before,
        &[user_record_pda, game_session_pda],
    )
    .await;

    // Assert all accounts are back on-chain
    shared::assert_onchain_exists(&mut rpc, &user_record_pda, "UserRecord").await;
    shared::assert_onchain_exists(&mut rpc, &game_session_pda, "GameSession").await;
    shared::assert_onchain_exists(&mut rpc, &vault_pda, "Vault").await;
    shared::assert_onchain_exists(&mut rpc, &user_ata_pda, "UserATA").await;
    shared::assert_onchain_exists(&mut rpc, &mint_pda, "Mint").await;

    // Full-struct assertion for UserRecord after decompression
    {
        let account = rpc.get_account(user_record_pda).await.unwrap().unwrap();
        let user_record: csdk_anchor_full_derived_test::UserRecord =
            borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
        let expected = csdk_anchor_full_derived_test::UserRecord {
            compression_info: shared::expected_compression_info(&user_record.compression_info),
            owner: payer.pubkey(),
            name: "Auto Created User With Mint".to_string(),
            score: 0,
            category_id,
        };
        assert_eq!(
            user_record, expected,
            "UserRecord should match after decompression"
        );
    }

    // Verify balances with full-struct comparison
    let vault_after = parse_token(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let expected_vault = Token {
            mint: mint_pda.into(),
            owner: vault_authority_pda.into(),
            amount: vault_mint_amount,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: vault_after.extensions.clone(),
        };
        assert_eq!(
            vault_after, expected_vault,
            "vault should match after decompression"
        );
    }

    let user_ata_after = parse_token(&rpc.get_account(user_ata_pda).await.unwrap().unwrap().data);
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let expected_ata = Token {
            mint: mint_pda.into(),
            owner: payer.pubkey().into(),
            amount: user_ata_mint_amount,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: user_ata_after.extensions.clone(),
        };
        assert_eq!(
            user_ata_after, expected_ata,
            "user ATA should match after decompression"
        );
    }

    // Verify compressed vault token is consumed
    let remaining_vault = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(remaining_vault.is_empty());

    // PHASE 4: Verify compress_as field overrides on GameSession
    // After decompress, fields with #[compress_as(...)] should be reset to override values
    let game_session_data = rpc
        .get_account(game_session_pda)
        .await
        .unwrap()
        .expect("GameSession account should exist");
    let game_session: GameSession =
        borsh::BorshDeserialize::deserialize(&mut &game_session_data.data[8..]) // Skip anchor discriminator
            .expect("Failed to deserialize GameSession");

    // Verify start_time was reset by compress_as override
    // Initial: Clock timestamp (non-zero), After decompress: 0
    assert_ne!(
        initial_start_time, 0,
        "Initial start_time should have been non-zero"
    );
    assert_eq!(
        game_session.start_time, 0,
        "start_time should be reset to 0 by compress_as override (was: {})",
        initial_start_time
    );

    // Extract runtime-specific value (compression_info set during transaction)
    let compression_info = game_session.compression_info;

    // Build expected struct with compress_as overrides applied:
    // #[compress_as(start_time = 0, end_time = None, score = 0)]
    let expected_game_session = GameSession {
        compression_info,       // Runtime-specific, extracted from actual
        session_id,             // 222 - preserved
        player: payer.pubkey(), // Preserved
        game_type: "Auto Game With Mint".to_string(), // Preserved
        start_time: 0,          // compress_as override (was Clock timestamp)
        end_time: None,         // compress_as override
        score: 0,               // compress_as override
    };

    // Single assert comparing full struct
    assert_eq!(
        game_session, expected_game_session,
        "GameSession should match expected after decompress with compress_as overrides"
    );
}

/// Test creating 2 mints in a single instruction.
/// Verifies multi-mint support in the RentFree macro.
#[tokio::test]
async fn test_create_two_mints() {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CreateTwoMintsParams, MINT_SIGNER_A_SEED, MINT_SIGNER_B_SEED,
    };
    use light_token::instruction::{
        find_mint_address as find_cmint_address, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR,
    };

    let ctx = shared::SharedTestContext::new_with_config(|config| {
        config.with_decoders(vec![
            Box::new(csdk_anchor_full_derived_test::CsdkTestInstructionDecoder),
            Box::new(csdk_anchor_full_derived_test::CsdkAnchorFullDerivedTestInstructionDecoder),
        ])
    })
    .await;

    let shared::SharedTestContext {
        mut rpc,
        payer,
        config_pda,
        rent_sponsor: _,
        program_id,
    } = ctx;

    let authority = Keypair::new();

    // Derive PDAs for both mint signers
    let (mint_signer_a_pda, mint_signer_a_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_A_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_signer_b_pda, mint_signer_b_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_B_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDAs
    let (cmint_a_pda, _) = find_cmint_address(&mint_signer_a_pda);
    let (cmint_b_pda, _) = find_cmint_address(&mint_signer_b_pda);

    // Get proof for both mints
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::mint(mint_signer_a_pda),
            CreateAccountsProofInput::mint(mint_signer_b_pda),
        ],
    )
    .await
    .unwrap();

    // Debug: Check proof contents
    println!(
        "proof_result.create_accounts_proof.proof.0.is_some() = {:?}",
        proof_result.create_accounts_proof.proof.0.is_some()
    );
    println!(
        "proof_result.remaining_accounts.len() = {:?}",
        proof_result.remaining_accounts.len()
    );

    let accounts = csdk_anchor_full_derived_test::accounts::CreateTwoMints {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer_a: mint_signer_a_pda,
        mint_signer_b: mint_signer_b_pda,
        cmint_a: cmint_a_pda,
        cmint_b: cmint_b_pda,
        compression_config: config_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::CreateTwoMints {
        params: CreateTwoMintsParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_a_bump,
            mint_signer_b_bump,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateTwoMints should succeed");

    // Verify both mints exist on-chain
    let cmint_a_account = rpc
        .get_account(cmint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist on-chain");
    let cmint_b_account = rpc
        .get_account(cmint_b_pda)
        .await
        .unwrap()
        .expect("Mint B should exist on-chain");

    // Parse and verify mint data
    use light_token_interface::state::Mint;
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_a_account.data[..])
        .expect("Failed to deserialize Mint A");
    let mint_b: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_b_account.data[..])
        .expect("Failed to deserialize Mint B");

    // Verify decimals match what was specified in #[light_account(init)]
    assert_eq!(mint_a.base.decimals, 6, "Mint A should have 6 decimals");
    assert_eq!(mint_b.base.decimals, 9, "Mint B should have 9 decimals");

    // Verify mint authorities
    assert_eq!(
        mint_a.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint A authority should be fee_payer"
    );
    assert_eq!(
        mint_b.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint B authority should be fee_payer"
    );

    // Full Mint struct assertions
    {
        use light_token_interface::state::mint::BaseMint;
        let expected_mint_a = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 6,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint_a.metadata.clone(),
            reserved: mint_a.reserved,
            account_type: mint_a.account_type,
            compression: mint_a.compression,
            extensions: mint_a.extensions.clone(),
        };
        assert_eq!(
            mint_a, expected_mint_a,
            "mint_a should match expected full struct"
        );

        let expected_mint_b = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint_b.metadata.clone(),
            reserved: mint_b.reserved,
            account_type: mint_b.account_type,
            compression: mint_b.compression,
            extensions: mint_b.extensions.clone(),
        };
        assert_eq!(
            mint_b, expected_mint_b,
            "mint_b should match expected full struct"
        );
    }

    // Verify compressed addresses registered
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let mint_a_compressed_address =
        light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
            &mint_signer_a_pda,
            &address_tree_pubkey,
        );
    let compressed_mint_a = rpc
        .get_compressed_account(mint_a_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        compressed_mint_a.address.unwrap(),
        mint_a_compressed_address,
        "Mint A compressed address should be registered"
    );

    let mint_b_compressed_address =
        light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
            &mint_signer_b_pda,
            &address_tree_pubkey,
        );
    let compressed_mint_b = rpc
        .get_compressed_account(mint_b_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        compressed_mint_b.address.unwrap(),
        mint_b_compressed_address,
        "Mint B compressed address should be registered"
    );

    // Verify both compressed mint accounts have decompressed PDA format (data contains PDA pubkey)
    assert_eq!(
        compressed_mint_a.data.as_ref().unwrap().data,
        cmint_a_pda.to_bytes(),
        "Mint A decompressed PDA data should contain the PDA pubkey"
    );
    assert_eq!(
        compressed_mint_b.data.as_ref().unwrap().data,
        cmint_b_pda.to_bytes(),
        "Mint B decompressed PDA data should contain the PDA pubkey"
    );
}

/// Test creating multiple mints (3) in a single instruction.
/// Verifies multi-mint support in the RentFree macro scales beyond 2.
#[tokio::test]
async fn test_create_multi_mints() {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CreateThreeMintsParams, MINT_SIGNER_A_SEED, MINT_SIGNER_B_SEED, MINT_SIGNER_C_SEED,
    };
    use light_token::instruction::{
        find_mint_address as find_cmint_address, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR,
    };

    let shared::SharedTestContext {
        mut rpc,
        payer,
        config_pda,
        rent_sponsor: _,
        program_id,
    } = shared::SharedTestContext::new().await;

    let authority = Keypair::new();

    // Derive PDAs for all 3 mint signers
    let (mint_signer_a_pda, mint_signer_a_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_A_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_signer_b_pda, mint_signer_b_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_B_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_signer_c_pda, mint_signer_c_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_C_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDAs
    let (cmint_a_pda, _) = find_cmint_address(&mint_signer_a_pda);
    let (cmint_b_pda, _) = find_cmint_address(&mint_signer_b_pda);
    let (cmint_c_pda, _) = find_cmint_address(&mint_signer_c_pda);

    // Get proof for all 3 mints
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::mint(mint_signer_a_pda),
            CreateAccountsProofInput::mint(mint_signer_b_pda),
            CreateAccountsProofInput::mint(mint_signer_c_pda),
        ],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::CreateThreeMints {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer_a: mint_signer_a_pda,
        mint_signer_b: mint_signer_b_pda,
        mint_signer_c: mint_signer_c_pda,
        cmint_a: cmint_a_pda,
        cmint_b: cmint_b_pda,
        cmint_c: cmint_c_pda,
        compression_config: config_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::CreateThreeMints {
        params: CreateThreeMintsParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_a_bump,
            mint_signer_b_bump,
            mint_signer_c_bump,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateThreeMints should succeed");

    // Verify all 3 mints exist on-chain
    use light_token_interface::state::Mint;

    let cmint_a_account = rpc
        .get_account(cmint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist on-chain");
    let cmint_b_account = rpc
        .get_account(cmint_b_pda)
        .await
        .unwrap()
        .expect("Mint B should exist on-chain");
    let cmint_c_account = rpc
        .get_account(cmint_c_pda)
        .await
        .unwrap()
        .expect("Mint C should exist on-chain");

    // Parse and verify mint data
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_a_account.data[..])
        .expect("Failed to deserialize Mint A");
    let mint_b: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_b_account.data[..])
        .expect("Failed to deserialize Mint B");
    let mint_c: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_c_account.data[..])
        .expect("Failed to deserialize Mint C");

    // Verify decimals match what was specified in #[light_account(init)]
    assert_eq!(mint_a.base.decimals, 6, "Mint A should have 6 decimals");
    assert_eq!(mint_b.base.decimals, 8, "Mint B should have 8 decimals");
    assert_eq!(mint_c.base.decimals, 9, "Mint C should have 9 decimals");

    // Verify mint authorities
    assert_eq!(
        mint_a.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint A authority should be fee_payer"
    );
    assert_eq!(
        mint_b.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint B authority should be fee_payer"
    );
    assert_eq!(
        mint_c.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint C authority should be fee_payer"
    );

    // Full Mint struct assertions
    {
        use light_token_interface::state::mint::BaseMint;
        let expected_mint_a = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 6,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint_a.metadata.clone(),
            reserved: mint_a.reserved,
            account_type: mint_a.account_type,
            compression: mint_a.compression,
            extensions: mint_a.extensions.clone(),
        };
        assert_eq!(
            mint_a, expected_mint_a,
            "mint_a should match expected full struct"
        );

        let expected_mint_b = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 8,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint_b.metadata.clone(),
            reserved: mint_b.reserved,
            account_type: mint_b.account_type,
            compression: mint_b.compression,
            extensions: mint_b.extensions.clone(),
        };
        assert_eq!(
            mint_b, expected_mint_b,
            "mint_b should match expected full struct"
        );

        let expected_mint_c = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint_c.metadata.clone(),
            reserved: mint_c.reserved,
            account_type: mint_c.account_type,
            compression: mint_c.compression,
            extensions: mint_c.extensions.clone(),
        };
        assert_eq!(
            mint_c, expected_mint_c,
            "mint_c should match expected full struct"
        );
    }
}

/// Helper function to set up test context for D9 instruction data tests.
/// Returns (rpc, payer, program_id, config_pda, rent_sponsor).
async fn setup_d9_test_context() -> (LightProgramTest, Keypair, Pubkey, Pubkey, Pubkey) {
    let ctx = shared::SharedTestContext::new().await;
    (
        ctx.rpc,
        ctx.payer,
        ctx.program_id,
        ctx.config_pda,
        ctx.rent_sponsor,
    )
}

/// Test D9InstrSinglePubkey - seeds = [b"instr_single", params.owner.as_ref()]
#[tokio::test]
async fn test_d9_instr_single_pubkey() {
    use csdk_anchor_full_derived_test::D9SinglePubkeyParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let owner = Keypair::new().pubkey();
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"instr_single", owner.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrSinglePubkey {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_single_pubkey_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrSinglePubkey {
        params: D9SinglePubkeyParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrSinglePubkey should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrU64 - seeds = [b"instr_u64_", params.amount.to_le_bytes().as_ref()]
#[tokio::test]
async fn test_d9_instr_u64() {
    use csdk_anchor_full_derived_test::D9U64Params;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let amount = 12345u64;
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"instr_u64_", amount.to_le_bytes().as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrU64 {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_u64_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrU64 {
        _params: D9U64Params {
            create_accounts_proof: proof_result.create_accounts_proof,
            amount,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrU64 should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrMultiField - seeds = [b"instr_multi", params.owner.as_ref(), &params.amount.to_le_bytes()]
#[tokio::test]
async fn test_d9_instr_multi_field() {
    use csdk_anchor_full_derived_test::D9MultiFieldParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let owner = Keypair::new().pubkey();
    let amount = 99999u64;
    let (record_pda, _) = Pubkey::find_program_address(
        &[b"instr_multi", owner.as_ref(), &amount.to_le_bytes()],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrMultiField {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_multi_field_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrMultiField {
        params: D9MultiFieldParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            amount,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrMultiField should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrMixedCtx - seeds = [b"instr_mixed", authority.key().as_ref(), params.data_key.as_ref()]
#[tokio::test]
async fn test_d9_instr_mixed_ctx() {
    use csdk_anchor_full_derived_test::D9MixedCtxParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;
    let authority = Keypair::new();

    let data_key = Keypair::new().pubkey();
    let (record_pda, _) = Pubkey::find_program_address(
        &[
            b"instr_mixed",
            authority.pubkey().as_ref(),
            data_key.as_ref(),
        ],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrMixedCtx {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_mixed_ctx_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrMixedCtx {
        params: D9MixedCtxParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            data_key,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("D9InstrMixedCtx should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrTriple - seeds = [b"instr_triple", params.key_a.as_ref(), params.value_b.to_le_bytes().as_ref()]
#[tokio::test]
async fn test_d9_instr_triple() {
    use csdk_anchor_full_derived_test::D9TripleParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let key_a = Keypair::new().pubkey();
    let value_b = 777u64;
    let flag_c = 42u8;
    let (record_pda, _) = Pubkey::find_program_address(
        &[
            b"instr_triple",
            key_a.as_ref(),
            value_b.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrTriple {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_triple_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrTriple {
        params: D9TripleParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            key_a,
            value_b,
            flag_c,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrTriple should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrBigEndian - seeds = [b"instr_be", &params.value.to_be_bytes()]
#[tokio::test]
async fn test_d9_instr_big_endian() {
    use csdk_anchor_full_derived_test::D9BigEndianParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let value = 0xDEADBEEFu64;
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"instr_be", &value.to_be_bytes()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrBigEndian {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_big_endian_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrBigEndian {
        _params: D9BigEndianParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            value,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrBigEndian should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrMultiU64 - seeds = [b"multi_u64", params.id.to_le_bytes().as_ref(), params.counter.to_le_bytes().as_ref()]
#[tokio::test]
async fn test_d9_instr_multi_u64() {
    use csdk_anchor_full_derived_test::D9MultiU64Params;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let id = 100u64;
    let counter = 200u64;
    let (record_pda, _) = Pubkey::find_program_address(
        &[
            b"multi_u64",
            id.to_le_bytes().as_ref(),
            counter.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrMultiU64 {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_multi_u64_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrMultiU64 {
        _params: D9MultiU64Params {
            create_accounts_proof: proof_result.create_accounts_proof,
            id,
            counter,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrMultiU64 should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrChainedAsRef - seeds = [b"instr_chain", params.key.as_ref()]
#[tokio::test]
async fn test_d9_instr_chained_as_ref() {
    use csdk_anchor_full_derived_test::D9ChainedAsRefParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let key = Keypair::new().pubkey();
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"instr_chain", key.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrChainedAsRef {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_chained_as_ref_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrChainedAsRef {
        params: D9ChainedAsRefParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            key,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrChainedAsRef should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrConstMixed - seeds = [D9_INSTR_SEED, params.owner.as_ref()]
#[tokio::test]
async fn test_d9_instr_const_mixed() {
    use csdk_anchor_full_derived_test::{
        instructions::d9_seeds::instruction_data::D9_INSTR_SEED, D9ConstMixedParams,
    };

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;

    let owner = Keypair::new().pubkey();
    let (record_pda, _) =
        Pubkey::find_program_address(&[D9_INSTR_SEED, owner.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrConstMixed {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_const_mixed_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrConstMixed {
        params: D9ConstMixedParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("D9InstrConstMixed should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}

/// Test D9InstrComplexMixed - seeds = [b"complex", authority.key().as_ref(), params.data_owner.as_ref(), &params.data_amount.to_le_bytes()]
#[tokio::test]
async fn test_d9_instr_complex_mixed() {
    use csdk_anchor_full_derived_test::D9ComplexMixedParams;

    let (mut rpc, payer, program_id, config_pda, rent_sponsor) = setup_d9_test_context().await;
    let authority = Keypair::new();

    let data_owner = Keypair::new().pubkey();
    let data_amount = 55555u64;
    let (record_pda, _) = Pubkey::find_program_address(
        &[
            b"complex",
            authority.pubkey().as_ref(),
            data_owner.as_ref(),
            &data_amount.to_le_bytes(),
        ],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrComplexMixed {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        d9_instr_complex_mixed_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrComplexMixed {
        params: D9ComplexMixedParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            data_owner,
            data_amount,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("D9InstrComplexMixed should succeed");

    assert!(
        rpc.get_account(record_pda).await.unwrap().is_some(),
        "Record PDA should exist"
    );
}
