use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_compressible_client::{
    get_create_accounts_proof, AccountInterfaceExt, CreateAccountsProofInput,
    InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token_sdk::token::find_mint_address as find_cmint_address;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// 2 PDAs + 1 CMint + 1 Vault + 1 User ATA, all in one instruction with single proof.
/// After init: all accounts on-chain + parseable.
/// After warp: all cold (auto-compressed) with non-empty compressed data.
#[tokio::test]
async fn test_create_pdas_and_mint_auto() {
    use csdk_anchor_full_derived_test::{
        instruction_accounts::{LP_MINT_SIGNER_SEED, VAULT_SEED},
        FullAutoWithMintParams, GameSession,
    };
    use light_token_interface::state::Token;
    use light_token_sdk::token::{
        get_associated_token_address_and_bump, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR,
    };

    // Helpers
    async fn assert_onchain_exists(rpc: &mut LightProgramTest, pda: &Pubkey) {
        assert!(rpc.get_account(*pda).await.unwrap().is_some());
    }
    async fn assert_onchain_closed(rpc: &mut LightProgramTest, pda: &Pubkey) {
        let acc = rpc.get_account(*pda).await.unwrap();
        assert!(acc.is_none() || acc.unwrap().lamports == 0);
    }
    fn parse_token(data: &[u8]) -> Token {
        borsh::BorshDeserialize::deserialize(&mut &data[..]).unwrap()
    }
    async fn assert_compressed_exists_with_data(rpc: &mut LightProgramTest, addr: [u8; 32]) {
        let acc = rpc
            .get_compressed_account(addr, None)
            .await
            .unwrap()
            .value
            .unwrap();
        assert_eq!(acc.address.unwrap(), addr);
        assert!(!acc.data.as_ref().unwrap().data.is_empty());
    }
    async fn assert_compressed_token_exists(
        rpc: &mut LightProgramTest,
        owner: &Pubkey,
        expected_amount: u64,
    ) {
        let accs = rpc
            .get_compressed_token_accounts_by_owner(owner, None, None)
            .await
            .unwrap()
            .value
            .items;
        assert!(!accs.is_empty());
        assert_eq!(accs[0].token.amount, expected_amount);
    }

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
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
    let (cmint_pda, _) = find_cmint_address(&mint_signer_pda);
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, cmint_pda.as_ref()], &program_id);
    let (vault_authority_pda, _) = Pubkey::find_program_address(&[b"vault_authority"], &program_id);
    let (user_ata_pda, user_ata_bump) =
        get_associated_token_address_and_bump(&payer.pubkey(), &cmint_pda);

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
        light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
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
        cmint: cmint_pda,
        vault: vault_pda,
        vault_authority: vault_authority_pda,
        user_ata: user_ata_pda,
        compression_config: config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
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
    assert_onchain_exists(&mut rpc, &user_record_pda).await;
    assert_onchain_exists(&mut rpc, &game_session_pda).await;
    assert_onchain_exists(&mut rpc, &cmint_pda).await;
    assert_onchain_exists(&mut rpc, &vault_pda).await;
    assert_onchain_exists(&mut rpc, &user_ata_pda).await;

    // Parse and verify CToken data
    let vault_data = parse_token(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    assert_eq!(vault_data.owner, vault_authority_pda.to_bytes());
    assert_eq!(vault_data.amount, vault_mint_amount);

    let user_ata_data = parse_token(&rpc.get_account(user_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user_ata_data.owner, payer.pubkey().to_bytes());
    assert_eq!(user_ata_data.amount, user_ata_mint_amount);

    // Verify compressed addresses registered (empty data - decompressed to on-chain)
    let compressed_cmint = rpc
        .get_compressed_account(mint_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(compressed_cmint.address.unwrap(), mint_compressed_address);
    assert!(compressed_cmint.data.as_ref().unwrap().data.is_empty());

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
    assert_onchain_closed(&mut rpc, &user_record_pda).await;
    assert_onchain_closed(&mut rpc, &game_session_pda).await;
    assert_onchain_closed(&mut rpc, &cmint_pda).await;
    assert_onchain_closed(&mut rpc, &vault_pda).await;
    assert_onchain_closed(&mut rpc, &user_ata_pda).await;

    // Compressed accounts should exist with non-empty data
    assert_compressed_exists_with_data(&mut rpc, user_compressed_address).await;
    assert_compressed_exists_with_data(&mut rpc, game_compressed_address).await;
    assert_compressed_exists_with_data(&mut rpc, mint_compressed_address).await;

    // Compressed token accounts should exist with correct balances
    assert_compressed_token_exists(&mut rpc, &vault_pda, vault_mint_amount).await;
    assert_compressed_token_exists(&mut rpc, &user_ata_pda, user_ata_mint_amount).await;

    // PHASE 3: Decompress all accounts via create_load_accounts_instructions
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        GameSessionSeeds, TokenAccountVariant, UserRecordSeeds,
    };
    use light_compressible_client::{
        create_load_accounts_instructions, AccountInterface, RentFreeDecompressAccount,
    };

    // Fetch unified interfaces (hot/cold transparent)
    let user_interface = rpc
        .get_account_info_interface(&user_record_pda, &program_id)
        .await
        .expect("failed to get user");
    assert!(user_interface.is_cold, "UserRecord should be cold");

    let game_interface = rpc
        .get_account_info_interface(&game_session_pda, &program_id)
        .await
        .expect("failed to get game");
    assert!(game_interface.is_cold, "GameSession should be cold");

    let vault_interface = rpc
        .get_token_account_interface(&vault_pda)
        .await
        .expect("failed to get vault");
    assert!(vault_interface.is_cold, "Vault should be cold");
    assert_eq!(vault_interface.amount(), vault_mint_amount); // Build RentFreeDecompressAccount - From impls convert interfaces directly
    let program_owned_accounts = vec![
        RentFreeDecompressAccount::from_seeds(
            AccountInterface::from(&user_interface),
            UserRecordSeeds {
                authority: authority.pubkey(),
                mint_authority: mint_authority.pubkey(),
                owner,
                category_id,
            },
        )
        .expect("UserRecord seed verification failed"),
        RentFreeDecompressAccount::from_seeds(
            AccountInterface::from(&game_interface),
            GameSessionSeeds {
                fee_payer: payer.pubkey(),
                authority: authority.pubkey(),
                session_id,
            },
        )
        .expect("GameSession seed verification failed"),
        RentFreeDecompressAccount::from_ctoken(
            AccountInterface::from(&vault_interface),
            TokenAccountVariant::Vault { cmint: cmint_pda },
        )
        .expect("CToken variant construction failed"),
    ];

    // get_ata_interface: fetches ATA with unified handling using standard SPL types
    let ata_interface = rpc
        .get_ata_interface(&payer.pubkey(), &cmint_pda)
        .await
        .expect("get_ata_interface should succeed");
    assert!(ata_interface.is_cold(), "ATA should be cold after warp");
    assert_eq!(ata_interface.amount(), user_ata_mint_amount);
    assert_eq!(ata_interface.mint(), cmint_pda);
    assert_eq!(ata_interface.owner(), ata_interface.pubkey()); // ctoken ATA owner = ATA address

    // Fetch mint interface
    let mint_interface = rpc
        .get_mint_interface(&mint_signer_pda)
        .await
        .expect("get_mint_interface should succeed");
    assert!(mint_interface.is_cold(), "Mint should be cold after warp");

    // Load accounts if needed
    let all_instructions = create_load_accounts_instructions(
        &program_owned_accounts,
        std::slice::from_ref(&ata_interface.inner),
        std::slice::from_ref(&mint_interface),
        program_id,
        payer.pubkey(),
        config_pda,
        payer.pubkey(), // rent_sponsor
        &rpc,
    )
    .await
    .expect("create_load_accounts_instructions should succeed");

    // Expected: 1 PDA+Token ix + 2 ATA ixs (1 create_ata + 1 decompress) + 1 mint ix = 4
    assert_eq!(
        all_instructions.len(),
        4,
        "Should have 4 instructions: 1 PDA+Token, 1 create_ata, 1 decompress_ata, 1 mint"
    );

    // Execute all instructions
    rpc.create_and_send_transaction(&all_instructions, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // Assert all accounts are back on-chain
    assert_onchain_exists(&mut rpc, &user_record_pda).await;
    assert_onchain_exists(&mut rpc, &game_session_pda).await;
    assert_onchain_exists(&mut rpc, &vault_pda).await;
    assert_onchain_exists(&mut rpc, &user_ata_pda).await;
    assert_onchain_exists(&mut rpc, &cmint_pda).await;

    // Verify balances
    let vault_after = parse_token(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    assert_eq!(vault_after.amount, vault_mint_amount);

    let user_ata_after = parse_token(&rpc.get_account(user_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user_ata_after.amount, user_ata_mint_amount);

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
    let compression_info = game_session.compression_info.clone();

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
    use light_token_sdk::token::{
        find_mint_address as find_cmint_address, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR,
    };

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

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
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
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

    // Verify compressed addresses registered
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let mint_a_compressed_address =
        light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
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
        light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
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

    // Verify both compressed mint accounts have empty data (decompressed to on-chain)
    assert!(
        compressed_mint_a.data.as_ref().unwrap().data.is_empty(),
        "Mint A compressed data should be empty (decompressed)"
    );
    assert!(
        compressed_mint_b.data.as_ref().unwrap().data.is_empty(),
        "Mint B compressed data should be empty (decompressed)"
    );
}

/// Test creating multiple mints (3) in a single instruction.
/// Verifies multi-mint support in the RentFree macro scales beyond 2.
#[tokio::test]
async fn test_create_multi_mints() {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CreateThreeMintsParams, MINT_SIGNER_A_SEED, MINT_SIGNER_B_SEED, MINT_SIGNER_C_SEED,
    };
    use light_token_sdk::token::{
        find_mint_address as find_cmint_address, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR,
    };

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

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
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
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
}
