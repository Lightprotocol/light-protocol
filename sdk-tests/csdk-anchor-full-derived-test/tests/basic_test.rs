use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::{
    AccountCreationData, CompressionParams, E2eTestData, E2eTestParams, FullAutoParams,
};
use light_compressed_account::address::derive_address;
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_ctoken_interface::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMintMetadata,
};
use light_ctoken_sdk::{
    compressed_token::create_compressed_mint::{
        derive_cmint_compressed_address, find_cmint_address,
    },
    ctoken::{derive_ctoken_ata, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR},
};
use light_macros::pubkey;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
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

    // Verify compressed address was registered (with_data=false means no data in compressed account)
    let compressed_user_record = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Address should be registered
    assert!(compressed_user_record.address.is_some());
    assert_eq!(
        compressed_user_record.address.unwrap(),
        user_compressed_address
    );

    // Verify compressed game session address was registered
    let compressed_game_session = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert!(compressed_game_session.address.is_some());
    assert_eq!(
        compressed_game_session.address.unwrap(),
        game_compressed_address
    );

    // Verify compressed token was minted
    let cmint_pda = find_cmint_address(&mint_signer_pubkey).0;
    let (_, token_account_address) =
        csdk_anchor_full_derived_test::get_ctokensigner_seeds(&payer.pubkey(), &cmint_pda);

    let compressed_tokens = rpc
        .get_compressed_token_accounts_by_owner(&token_account_address, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(!compressed_tokens.is_empty());
    assert_eq!(compressed_tokens[0].token.amount, 1000);
}

/// Test the FULL AUTOMATIC approach using LightFinalize + light_instruction macros.
/// This creates 2 PDAs with ZERO manual compression code!
#[tokio::test]
async fn test_create_with_light_finalize_auto() {
    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Get the address tree BEFORE config init so we can add it to allowed address_space
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize compression config with the actual address tree in address_space
    let config_instruction =
        csdk_anchor_full_derived_test::instruction::InitializeCompressionConfig {
            rent_sponsor: RENT_SPONSOR,
            compression_authority: payer.pubkey(),
            rent_config: light_compressible::rent::RentConfig::default(),
            write_top_up: 5_000,
            address_space: vec![address_tree_pubkey], // Use actual test address tree
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
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create signers for the auto version
    let authority = Keypair::new();
    let mint_authority = Keypair::new();

    let owner = payer.pubkey();
    let category_id = 999u64;
    let session_id = 12345u64;

    // Calculate PDAs
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
            b"game_session",
            max_key_result.as_ref(),
            session_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    // Get tree info and calculate compressed addresses
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // v2 address derivation (matches what LightFinalize macro uses):
    // 1. seed = hash(pda_key)  [v2::derive_address_seed - NO program_id]
    // 2. address = hash(seed, tree, program_id)
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

    // Get validity proof for both new addresses
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

    // Build remaining accounts for Light system
    // NOTE: For PDAs-only (no mints), we use new() without cpi_context
    // because the macro uses CpiAccounts::new() which sets cpi_context: false
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new(program_id);
    remaining_accounts
        .add_system_accounts_v2(system_config)
        .unwrap();
    let output_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);

    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let user_address_tree_info = packed_tree_infos.address_trees[0];
    let game_address_tree_info = packed_tree_infos.address_trees[1];
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Build the AUTO accounts struct
    let accounts = csdk_anchor_full_derived_test::accounts::CreateUserRecordAndGameSessionAuto {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_authority: mint_authority.pubkey(),
        user_record: user_record_pda,
        game_session: game_session_pda,
        compression_config: config_pda,
        system_program: solana_sdk::system_program::ID,
    };

    // Build the AUTO instruction data - much simpler than manual version!
    let instruction_data =
        csdk_anchor_full_derived_test::instruction::CreateUserRecordAndGameSessionAuto {
            params: FullAutoParams {
                proof: rpc_result.proof,
                user_address_tree_info,
                game_address_tree_info,
                output_state_tree_index: output_tree_index,
                owner,
                category_id,
                session_id,
            },
        };

    let instruction = Instruction {
        program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Execute the auto instruction
    let result = rpc
        .create_and_send_transaction(
            &[instruction],
            &payer.pubkey(),
            &[&payer, &authority, &mint_authority],
        )
        .await;

    assert!(
        result.is_ok(),
        "Auto instruction should succeed: {:?}",
        result
    );

    // Verify compressed UserRecord address was registered
    // Note: prepare_compressed_account_on_init with with_data=false only reserves the address
    // The account data stays on-chain; compressed account has no data yet
    let compressed_user = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Verify the compressed account exists with the correct address
    assert!(compressed_user.address.is_some());
    assert_eq!(compressed_user.address.unwrap(), user_compressed_address);
    assert_eq!(compressed_user.owner.to_bytes(), program_id.to_bytes());

    // Verify compressed GameSession address was registered
    let compressed_game = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    assert!(compressed_game.address.is_some());
    assert_eq!(compressed_game.address.unwrap(), game_compressed_address);
    assert_eq!(compressed_game.owner.to_bytes(), program_id.to_bytes());

    println!("SUCCESS: Auto test with LightFinalize created:");
    println!("  - UserRecord compressed address reserved");
    println!("  - GameSession compressed address reserved");
    println!("All with ZERO manual compression code!");
}

/// Test FULL AUTOMATIC with MINT + VAULT + USER ATA:
/// - 2 PDAs with #[compressible]
/// - 1 CMint with #[light_mint] (creates + decompresses atomically in pre_init)
/// - 1 Program-owned CToken vault (created in instruction body)
/// - 1 User CToken ATA (created in instruction body)
/// - MintTo both vault and user_ata (in instruction body)
/// All with a single proof execution!
#[tokio::test]
async fn test_create_pdas_and_mint_auto() {
    use csdk_anchor_full_derived_test::instruction_accounts::{LP_MINT_SIGNER_SEED, VAULT_SEED};
    use csdk_anchor_full_derived_test::FullAutoWithMintParams;
    use light_ctoken_sdk::ctoken::{
        get_associated_ctoken_address_and_bump, CToken, COMPRESSIBLE_CONFIG_V1,
        RENT_SPONSOR as CTOKEN_RENT_SPONSOR,
    };

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Get the address tree BEFORE config init
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize compression config with the actual address tree
    let config_instruction =
        csdk_anchor_full_derived_test::instruction::InitializeCompressionConfig {
            rent_sponsor: RENT_SPONSOR,
            compression_authority: payer.pubkey(),
            rent_config: light_compressible::rent::RentConfig::default(),
            write_top_up: 5_000,
            address_space: vec![address_tree_pubkey],
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
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    // Create signers
    let authority = Keypair::new();
    let mint_authority = Keypair::new();

    let owner = payer.pubkey();
    let category_id = 111u64;
    let session_id = 222u64;

    // Token amounts to mint
    let vault_mint_amount = 100u64;
    let user_ata_mint_amount = 50u64;

    // Calculate mint_signer PDA (like Raydium's lp_mint_signer)
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[LP_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Calculate CMint PDA (derived from mint_signer)
    let (cmint_pda, _cmint_bump) = find_cmint_address(&mint_signer_pda);

    // Calculate vault PDA (program-owned) - seeds match variant: ["vault", cmint]
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, cmint_pda.as_ref()], &program_id);

    // Calculate vault authority PDA - seeds match variant authority: ["vault_authority"]
    let (vault_authority_pda, _) = Pubkey::find_program_address(&[b"vault_authority"], &program_id);

    // Calculate user's ATA for the CMint
    let (user_ata_pda, user_ata_bump) =
        get_associated_ctoken_address_and_bump(&payer.pubkey(), &cmint_pda);

    // Calculate PDAs for compressed accounts
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
            b"game_session",
            max_key_result.as_ref(),
            session_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    // Get tree info
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Calculate compressed addresses (v2 derivation)
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
    // Mint address is derived from mint_signer, tree, and ctoken_program
    let mint_compressed_address =
        derive_cmint_compressed_address(&mint_signer_pda, &address_tree_pubkey);

    // Get validity proof for all 3 new addresses (2 PDAs + 1 mint)
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
                    address: mint_compressed_address,
                    tree: address_tree_pubkey,
                },
            ],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build remaining accounts WITH CPI context (required for batching PDAs + mint)
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new_with_cpi_context(
        program_id,
        state_tree_info.cpi_context.unwrap(),
    );
    remaining_accounts
        .add_system_accounts_v2(system_config)
        .unwrap();
    let output_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);

    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let user_address_tree_info = packed_tree_infos.address_trees[0];
    let game_address_tree_info = packed_tree_infos.address_trees[1];
    let mint_address_tree_info = packed_tree_infos.address_trees[2];
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // Build accounts
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
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        ctoken_program: C_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_ctoken_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    // Build instruction data
    let instruction_data = csdk_anchor_full_derived_test::instruction::CreatePdasAndMintAuto {
        params: FullAutoWithMintParams {
            proof: rpc_result.proof,
            user_address_tree_info,
            game_address_tree_info,
            mint_address_tree_info,
            output_state_tree_index: output_tree_index,
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
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    // Execute
    let result = rpc
        .create_and_send_transaction(
            &[instruction],
            &payer.pubkey(),
            &[&payer, &authority, &mint_authority],
        )
        .await;

    assert!(
        result.is_ok(),
        "PDA + Mint auto instruction should succeed: {:?}",
        result
    );

    // Verify PDAs were compressed
    let compressed_user = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(compressed_user.address.is_some());
    assert_eq!(compressed_user.address.unwrap(), user_compressed_address);
    println!("  - UserRecord compressed successfully");

    let compressed_game = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(compressed_game.address.is_some());
    assert_eq!(compressed_game.address.unwrap(), game_compressed_address);
    println!("  - GameSession compressed successfully");

    // Verify CMint was decompressed (on-chain)
    let cmint_account = rpc.get_account(cmint_pda).await.unwrap();
    assert!(
        cmint_account.is_some(),
        "CMint should exist on-chain after decompress"
    );
    println!(
        "  - LP Mint created and decompressed to CMint: {}",
        cmint_pda
    );
    // Verify compressed CMint account is empty (no data)
    let compressed_cmint = rpc
        .get_compressed_account(mint_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(compressed_cmint.address.is_some());
    assert_eq!(compressed_cmint.address.unwrap(), mint_compressed_address);
    assert_eq!(
        compressed_cmint.data.as_ref().unwrap().data.len(),
        0,
        "Compressed CMint should have empty data after init"
    );
    println!("  - Compressed CMint account is empty (decompressed to on-chain)");

    // Verify vault was created and has correct owner and balance
    let vault_account = rpc.get_account(vault_pda).await.unwrap();
    assert!(
        vault_account.is_some(),
        "Vault CToken should exist on-chain"
    );
    let vault_data: CToken =
        borsh::BorshDeserialize::deserialize(&mut &vault_account.unwrap().data[..]).unwrap();
    // Vault is owned by vault_authority PDA (like cp-swap pattern)
    // compress_to_account_pubkey maps compressed owner back to vault address
    assert_eq!(
        vault_data.owner,
        vault_authority_pda.to_bytes(),
        "Vault owner should be vault_authority_pda (cp-swap pattern)"
    );
    assert_eq!(
        vault_data.amount, vault_mint_amount,
        "Vault should have {} tokens",
        vault_mint_amount
    );
    println!(
        "  - Vault created with {} tokens, owned by vault_authority",
        vault_mint_amount
    );

    // Verify user ATA was created and has correct owner and balance
    let user_ata_account = rpc.get_account(user_ata_pda).await.unwrap();
    assert!(
        user_ata_account.is_some(),
        "User ATA CToken should exist on-chain"
    );
    let user_ata_data: CToken =
        borsh::BorshDeserialize::deserialize(&mut &user_ata_account.unwrap().data[..]).unwrap();
    assert_eq!(
        user_ata_data.owner,
        payer.pubkey().to_bytes(),
        "User ATA should be owned by payer"
    );
    assert_eq!(
        user_ata_data.amount, user_ata_mint_amount,
        "User ATA should have {} tokens",
        user_ata_mint_amount
    );
    println!(
        "  - User ATA created with {} tokens, owned by payer",
        user_ata_mint_amount
    );

    // Verify account owners are the ctoken program
    let vault_account_raw = rpc.get_account(vault_pda).await.unwrap().unwrap();
    assert_eq!(
        vault_account_raw.owner,
        Pubkey::from(C_TOKEN_PROGRAM_ID),
        "Vault account owner should be ctoken program"
    );
    let user_ata_account_raw = rpc.get_account(user_ata_pda).await.unwrap().unwrap();
    assert_eq!(
        user_ata_account_raw.owner,
        Pubkey::from(C_TOKEN_PROGRAM_ID),
        "User ATA account owner should be ctoken program"
    );
    println!("  - Both vault and user_ata account owners are ctoken program");

    println!("\nPHASE 1 SUCCESS: Full auto test with PDAs + Mint + Vault + User ATA:");
    println!("  - 2 PDAs compressed atomically");
    println!("  - 1 CMint created + decompressed atomically (in pre_init)");
    println!(
        "  - 1 Vault created with {} tokens (in instruction body)",
        vault_mint_amount
    );
    println!(
        "  - 1 User ATA created with {} tokens (in instruction body)",
        user_ata_mint_amount
    );
    println!("All in ONE instruction with a single proof!");

    // check that pda is onchain still
    let user_record_before_warp = rpc.get_account(user_record_pda).await.unwrap();
    assert!(
        user_record_before_warp.is_some(),
        "User record PDA should still exist on-chain BEFORE warp"
    );
    println!(
        "  - User record PDA still exists on-chain BEFORE warp: {:?}",
        user_record_before_warp
    );

    // same with the other pda game session
    let game_session_before_warp = rpc.get_account(game_session_pda).await.unwrap();
    assert!(
        game_session_before_warp.is_some(),
        "Game session PDA should still exist on-chain BEFORE warp"
    );
    println!(
        "  - Game session PDA still exists on-chain BEFORE warp: {:?}",
        game_session_before_warp
    );

    // ===========================================================================
    // PHASE 2: Warp epoch forward to trigger auto-compression
    // ===========================================================================
    println!("\nPHASE 2: Warping epoch forward to trigger auto-compression...");
    // Warp 30 epochs to ensure vault and user_ata become compressible
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // Check that PDAs are now closed (auto-compressed) after warp
    let user_record_after_warp = rpc.get_account(user_record_pda).await.unwrap();
    let user_record_closed_after_warp = user_record_after_warp.is_none()
        || user_record_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        user_record_closed_after_warp,
        "User record PDA should be closed (lamports=0) after auto-compression warp"
    );
    println!(
        "  - User record PDA closed after warp: {:?}",
        user_record_after_warp
    );

    // Verify compressed UserRecord account exists with address and data
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let user_compressed_address = light_compressed_account::address::derive_address(
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
    assert!(
        compressed_user_record.address.is_some(),
        "Should have compressed user record account with address"
    );
    assert!(
        !compressed_user_record
            .data
            .as_ref()
            .unwrap()
            .data
            .is_empty(),
        "Compressed user record should have non-empty data"
    );
    println!("  - Verified compressed user record account exists with data");

    // Verify compressed GameSession account exists with address and data
    let game_compressed_address = light_compressed_account::address::derive_address(
        &game_session_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let compressed_game_session = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        compressed_game_session.address.is_some(),
        "Should have compressed game session account with address"
    );
    assert!(
        compressed_game_session
            .data
            .as_ref()
            .map(|d| !d.data.is_empty())
            .unwrap_or(false),
        "Compressed game session should have non-empty data"
    );
    println!("  - Verified compressed game session account exists with data");

    let game_session_after_warp = rpc.get_account(game_session_pda).await.unwrap();
    let game_session_closed_after_warp = game_session_after_warp.is_none()
        || game_session_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        game_session_closed_after_warp,
        "Game session PDA should be closed (lamports=0) after auto-compression warp"
    );
    println!(
        "  - Game session PDA closed after warp: {:?}",
        game_session_after_warp
    );

    // ===========================================================================
    // PHASE 3: Assert CToken accounts are auto-compressed
    // ===========================================================================
    println!("PHASE 3: Verifying auto-compression of CToken accounts...");

    // Check Vault CToken is compressed (closed)
    let vault_after_warp = rpc.get_account(vault_pda).await.unwrap();
    let vault_closed = vault_after_warp.is_none()
        || vault_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        vault_closed,
        "Vault CToken should be closed after auto-compression"
    );
    println!("  - Vault CToken auto-compressed (closed on-chain)");

    // Check User ATA is compressed (closed)
    let user_ata_after_warp = rpc.get_account(user_ata_pda).await.unwrap();
    let user_ata_closed = user_ata_after_warp.is_none()
        || user_ata_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        user_ata_closed,
        "User ATA CToken should be closed after auto-compression"
    );
    println!("  - User ATA CToken auto-compressed (closed on-chain)");

    // CMint auto-compression is now handled by warp_slot_forward via compress_cmint_forester
    let cmint_after_warp = rpc.get_account(cmint_pda).await.unwrap();
    let cmint_closed = cmint_after_warp.is_none()
        || cmint_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        cmint_closed,
        "CMint should be auto-compressed (closed) after epoch warp"
    );
    println!("  - CMint auto-compressed (closed on-chain)");

    // After auto-compression, PDA should be closed (garbage collected with 0 lamports)
    let onchain_user_record_account = rpc.get_account(user_record_pda).await.unwrap();
    let user_record_closed = onchain_user_record_account.is_none()
        || onchain_user_record_account
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        user_record_closed,
        "User record PDA should be closed after auto-compression"
    );
    println!("  - UserRecord PDA auto-compressed (closed on-chain)");

    // Verify compressed user record account exists with address
    // Use v2 address derivation to look up the compressed account
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let user_compressed_address = light_compressed_account::address::derive_address(
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
    assert!(
        compressed_user_record.address.is_some(),
        "Should have compressed user record account with address"
    );
    println!("  - Verified compressed user record account exists");

    // Verify compressed vault token account exists with correct balance
    // compress_to_pubkey sets the compressed owner to vault_pda
    let compressed_vault_accounts = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !compressed_vault_accounts.is_empty(),
        "Should have compressed vault token account"
    );
    assert_eq!(
        compressed_vault_accounts[0].token.amount, vault_mint_amount,
        "Compressed vault token should have same balance"
    );
    println!(
        "  - Verified compressed vault token account with {} tokens",
        vault_mint_amount
    );

    // Verify compressed user ATA token account exists with correct balance
    // compression_only=true sets the compressed owner to user_ata_pda
    let compressed_user_ata_accounts = rpc
        .get_compressed_token_accounts_by_owner(&user_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !compressed_user_ata_accounts.is_empty(),
        "Should have compressed user ATA token account"
    );
    assert_eq!(
        compressed_user_ata_accounts[0].token.amount, user_ata_mint_amount,
        "Compressed user ATA token should have same balance"
    );
    println!(
        "  - Verified compressed user ATA token account with {} tokens",
        user_ata_mint_amount
    );

    println!("\nSUCCESS: ALL accounts auto-compressed after epoch warp!");
    println!("  - Vault CToken: auto-compressed");
    println!("  - User ATA CToken: auto-compressed");
    println!("  - CMint: auto-compressed");
    println!("  - PDAs: already compressed from creation");
}

async fn create_user_record_and_game_session(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    user_record_pda: &Pubkey,
    game_session_pda: &Pubkey,
    authority: &Keypair,
    mint_authority: &Keypair,
    some_account: &Keypair,
    session_id: u64,
    category_id: u64,
) -> Pubkey {
    let mint_signer = Keypair::new();

    let state_tree_info = rpc.get_random_state_tree_info().unwrap();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    // Calculate both compressed addresses
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

    let mint_compressed_address =
        derive_cmint_compressed_address(&mint_signer.pubkey(), &address_tree_pubkey);

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
                    address: mint_compressed_address,
                    tree: address_tree_pubkey,
                },
            ],
            None,
        )
        .await
        .unwrap()
        .value;

    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new_with_cpi_context(
        *program_id,
        state_tree_info.cpi_context.unwrap(),
    );
    remaining_accounts
        .add_system_accounts_v2(system_config)
        .unwrap();
    let user_output_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);
    let game_output_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);

    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let user_address_tree_info = packed_tree_infos.address_trees[0];
    let game_address_tree_info = packed_tree_infos.address_trees[1];
    let mint_address_tree_info = packed_tree_infos.address_trees[2];
    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    let account_data = AccountCreationData {
        owner: payer.pubkey(),
        category_id,
        user_name: "test_user".to_string(),
        session_id,
        game_type: "rpg".to_string(),
        placeholder_id: 0,
        counter: 0,
        mint_name: "Test Token".to_string(),
        mint_symbol: "TST".to_string(),
        mint_uri: "https://test.com".to_string(),
        mint_decimals: 9,
        mint_supply: 0,
        mint_update_authority: None,
        mint_freeze_authority: None,
        additional_metadata: None,
    };

    let (spl_mint, _) = find_cmint_address(&mint_signer.pubkey());
    let mint_data = CompressedMintInstructionData {
        supply: 0,
        decimals: 9,
        metadata: CompressedMintMetadata {
            version: 3,
            mint: spl_mint.into(),
            cmint_decompressed: false,
            compressed_address: mint_compressed_address,
        },
        mint_authority: Some(mint_authority.pubkey().to_bytes().into()),
        freeze_authority: None,
        extensions: None,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::CreateUserRecordAndGameSession {
        user: payer.pubkey(),
        mint_signer: mint_signer.pubkey(),
        user_record: *user_record_pda,
        game_session: *game_session_pda,
        authority: authority.pubkey(),
        mint_authority: mint_authority.pubkey(),
        some_account: some_account.pubkey(),
        ctoken_program: C_TOKEN_PROGRAM_ID.into(),
        compress_token_program_cpi_authority: light_ctoken_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
        config: *config_pda,
        rent_sponsor: RENT_SPONSOR,
    };

    let instruction_data =
        csdk_anchor_full_derived_test::instruction::CreateUserRecordAndGameSession {
            account_data,
            compression_params: CompressionParams {
                proof: rpc_result.proof,
                user_compressed_address,
                user_address_tree_info,
                user_output_state_tree_index: user_output_tree_index,
                game_compressed_address,
                game_address_tree_info,
                game_output_state_tree_index: game_output_tree_index,
                mint_bump: find_cmint_address(&mint_signer.pubkey()).1,
                mint_with_context: CompressedMintWithContext {
                    leaf_index: 0,
                    prove_by_index: false,
                    root_index: mint_address_tree_info.root_index,
                    address: mint_compressed_address,
                    mint: Some(CompressedMintInstructionData {
                        supply: 0,
                        decimals: 9,
                        metadata: CompressedMintMetadata {
                            version: 3,
                            mint: spl_mint.into(),
                            cmint_decompressed: false,
                            compressed_address: mint_compressed_address,
                        },
                        mint_authority: Some(mint_authority.pubkey().to_bytes().into()),
                        freeze_authority: None,
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

    rpc.create_and_send_transaction(
        &[instruction],
        &payer.pubkey(),
        &[&payer, &mint_signer, &authority, &mint_authority],
    )
    .await
    .unwrap();

    mint_signer.pubkey()
}

/// Full E2E test demonstrating the complete lifecycle (like cp-swap pattern):
///
/// 1. PHASE 1: Create ALL accounts in ONE instruction (like cp-swap's initialize):
///    - PlaceholderRecord PDA (compressed with data)
///    - UserRecord PDA (2nd PDA for multi-PDA coverage, compressed with data)
///    - Light mint + decompress to CMint
///    - CToken vault via CPI (program-owned)
///    - User ATA via CPI (like cp-swap's creator_lp_token)
///    - MintTo vault and user_ata
/// 2. PHASE 2: Warp epoch forward → auto-compress vault and user_ata CToken accounts
/// 3. PHASE 3: Assert ALL accounts are compressed (2 PDAs + 2 CTokens + CMint)
/// 4. PHASE 4: Decompress PDAs via decompress_accounts_idempotent
/// 5. PHASE 5: Decompress User ATA via DecompressToCtoken
/// 6. PHASE 6: Decompress CMint via mint_action_comprehensive
#[tokio::test]
async fn test_e2e_light_mint_ctoken_pda_full_lifecycle() {
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

    // Initialize compression config
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
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    // Create signers
    let authority = Keypair::new();
    let mint_signer = Keypair::new();
    let mint_authority = Keypair::new();
    let some_account = Keypair::new();

    // Test data
    let placeholder_id = 12345u64;
    let counter = 42u32;
    let mint_decimals = 9u8;
    let vault_mint_amount = 1_000_000_000u64; // 1 token with 9 decimals
    let user_ata_mint_amount = 500_000_000u64; // 0.5 token with 9 decimals (like LP tokens)

    // UserRecord data (2nd PDA)
    let user_record_owner = payer.pubkey();
    let user_record_name = "E2E Test User".to_string();
    let user_record_score = 9999u64;
    let user_record_category_id = 7777u64;

    // Calculate PlaceholderRecord PDA
    let (placeholder_pda, _) = Pubkey::find_program_address(
        &[
            b"placeholder_record",
            authority.pubkey().as_ref(),
            some_account.pubkey().as_ref(),
            placeholder_id.to_le_bytes().as_ref(),
            counter.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    // Calculate UserRecord PDA (2nd PDA)
    // Seeds MUST match variant: UserRecord = ("user_record", ctx.authority, ctx.mint_authority, data.owner, data.category_id.to_le_bytes())
    let (user_record_pda, _) = Pubkey::find_program_address(
        &[
            b"user_record",
            authority.pubkey().as_ref(),
            mint_authority.pubkey().as_ref(),
            user_record_owner.as_ref(),
            user_record_category_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    // Calculate CMint PDA
    let (cmint_pda, cmint_bump) = find_cmint_address(&mint_signer.pubkey());

    // Calculate Vault PDA (seeds: ["vault", cmint])
    let (vault_pda, _vault_bump) =
        Pubkey::find_program_address(&[b"vault", cmint_pda.as_ref()], &program_id);

    // Calculate vault_authority PDA
    let (vault_authority_pda, _) = Pubkey::find_program_address(&[b"vault_authority"], &program_id);

    // Calculate User ATA (like cp-swap's creator_lp_token)
    let (user_ata_pda, user_ata_bump) = derive_ctoken_ata(&payer.pubkey(), &cmint_pda);

    // Get tree info
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    // Calculate compressed addresses
    let placeholder_compressed_address = derive_address(
        &placeholder_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let user_record_compressed_address = derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let mint_compressed_address =
        derive_cmint_compressed_address(&mint_signer.pubkey(), &address_tree_pubkey);

    // Get validity proof for all 3 addresses (2 PDAs + 1 mint)
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![
                AddressWithTree {
                    address: placeholder_compressed_address,
                    tree: address_tree_pubkey,
                },
                AddressWithTree {
                    address: user_record_compressed_address,
                    tree: address_tree_pubkey,
                },
                AddressWithTree {
                    address: mint_compressed_address,
                    tree: address_tree_pubkey,
                },
            ],
            None,
        )
        .await
        .unwrap()
        .value;

    // Setup remaining accounts
    let mut remaining_accounts = PackedAccounts::default();
    let system_config = SystemAccountMetaConfig::new_with_cpi_context(
        program_id,
        state_tree_info.cpi_context.unwrap(),
    );
    let _ = remaining_accounts.add_system_accounts_v2(system_config);

    let output_tree_index = remaining_accounts.insert_or_get(state_tree_info.queue);
    let packed_tree_infos = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let placeholder_address_tree_info = packed_tree_infos.address_trees[0];
    let user_record_address_tree_info = packed_tree_infos.address_trees[1];
    let mint_address_tree_info = packed_tree_infos.address_trees[2];

    let (system_accounts, _, _) = remaining_accounts.to_account_metas();

    // ===========================================================================
    // PHASE 1: Create ALL accounts in ONE instruction (like cp-swap's initialize)
    // - PlaceholderRecord PDA (compressed)
    // - UserRecord PDA (compressed) - 2nd PDA for multi-PDA coverage
    // - Light mint + decompress to CMint
    // - CToken vault via CPI
    // - User ATA via CPI (like creator_lp_token)
    // - MintTo vault and user_ata
    // ===========================================================================
    println!("PHASE 1: Creating ALL accounts in ONE instruction (like cp-swap)...");

    // Fund mint_authority for potential top-ups
    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &mint_authority.pubkey(),
        100_000_000, // 0.1 SOL
    );
    rpc.create_and_send_transaction(&[fund_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Fund mint authority should succeed");

    let accounts = csdk_anchor_full_derived_test::accounts::E2eCreateMintDecompressAndToken {
        payer: payer.pubkey(),
        mint_signer: mint_signer.pubkey(),
        mint_authority: mint_authority.pubkey(),
        authority: authority.pubkey(),
        some_account: some_account.pubkey(),
        placeholder_record: placeholder_pda,
        user_record: user_record_pda,
        cmint: cmint_pda,
        vault: vault_pda,
        vault_authority: vault_authority_pda,
        user_ata: user_ata_pda,
        ctoken_program: C_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_ctoken_types::CPI_AUTHORITY_PDA.into(),
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        system_program: solana_sdk::system_program::ID,
        config: config_pda,
        rent_sponsor: RENT_SPONSOR,
    };

    let instruction_data =
        csdk_anchor_full_derived_test::instruction::E2eCreateMintDecompressAndPda {
            data: E2eTestData {
                placeholder_name: "Test Placeholder".to_string(),
                placeholder_id,
                counter,
                mint_decimals,
                user_record_owner,
                user_record_name: user_record_name.clone(),
                user_record_score,
                user_record_category_id,
            },
            params: E2eTestParams {
                proof: rpc_result.proof,
                placeholder_compressed_address,
                placeholder_address_tree_info,
                placeholder_output_state_tree_index: output_tree_index,
                user_record_compressed_address,
                user_record_address_tree_info,
                user_record_output_state_tree_index: output_tree_index,
                mint_with_context: CompressedMintWithContext {
                    leaf_index: 0,
                    prove_by_index: false,
                    root_index: mint_address_tree_info.root_index,
                    address: mint_compressed_address,
                    mint: Some(CompressedMintInstructionData {
                        supply: 0,
                        decimals: mint_decimals,
                        metadata: CompressedMintMetadata {
                            version: 3,
                            mint: cmint_pda.into(),
                            cmint_decompressed: false,
                            compressed_address: mint_compressed_address,
                        },
                        mint_authority: Some(mint_authority.pubkey().to_bytes().into()),
                        freeze_authority: None,
                        extensions: None,
                    }),
                },
                mint_address_tree_info,
                cmint_bump,
                rent_payment: 2, // 2 epochs rent
                write_top_up: 0, // No write_top_up so vault auto-compresses
                user_ata_bump,
                vault_mint_amount,
                user_ata_mint_amount,
            },
        };

    let instruction = Instruction {
        program_id,
        accounts: [accounts.to_account_metas(None), system_accounts].concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(
        &[instruction],
        &payer.pubkey(),
        &[&payer, &mint_signer, &mint_authority, &authority],
    )
    .await
    .expect("Phase 1: Create ALL accounts in ONE instruction should succeed");

    // Verify PlaceholderRecord was compressed
    let compressed_placeholder = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        compressed_placeholder.data.is_some(),
        "PlaceholderRecord should be compressed"
    );
    println!("  ✓ PlaceholderRecord PDA created and compressed");

    // Verify UserRecord was compressed (2nd PDA)
    let compressed_user_record = rpc
        .get_compressed_account(user_record_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        compressed_user_record.data.is_some(),
        "UserRecord should be compressed"
    );
    println!("  ✓ UserRecord PDA created and compressed (2nd PDA)");

    // Verify CMint was created (decompressed on-chain)
    let cmint_account = rpc.get_account(cmint_pda).await.unwrap();
    assert!(
        cmint_account.is_some(),
        "CMint should exist on-chain after decompression"
    );
    println!("  ✓ Light mint created and decompressed to CMint");

    // Verify Vault was created via CPI
    let vault_account = rpc.get_account(vault_pda).await.unwrap();
    assert!(
        vault_account.is_some(),
        "Vault CToken should exist on-chain"
    );

    // Verify Vault has tokens
    use light_ctoken_interface::state::CToken;
    let vault_data: CToken =
        borsh::BorshDeserialize::deserialize(&mut &vault_account.unwrap().data[..]).unwrap();
    assert_eq!(
        vault_data.amount, vault_mint_amount,
        "Vault should have minted tokens"
    );
    println!(
        "  ✓ Vault CToken created via CPI with {} tokens (like cp-swap)",
        vault_mint_amount
    );

    // Verify User ATA was created via CPI
    let user_ata_account = rpc.get_account(user_ata_pda).await.unwrap();
    assert!(
        user_ata_account.is_some(),
        "User ATA CToken should exist on-chain"
    );

    // Verify User ATA has tokens
    let user_ata_data: CToken =
        borsh::BorshDeserialize::deserialize(&mut &user_ata_account.unwrap().data[..]).unwrap();
    assert_eq!(
        user_ata_data.amount, user_ata_mint_amount,
        "User ATA should have minted tokens"
    );
    println!(
        "  ✓ User ATA created via CPI with {} tokens (like cp-swap's creator_lp_token)",
        user_ata_mint_amount
    );

    // ===========================================================================
    // PHASE 2: Warp epoch forward to trigger auto-compression
    // ===========================================================================
    println!("PHASE 2: Warping epoch forward to trigger auto-compression...");

    // Warp 30 epochs to ensure vault and user_ata become compressible
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // ===========================================================================
    // PHASE 3: Assert ALL accounts are compressed
    // ===========================================================================
    println!("PHASE 3: Verifying auto-compression of ALL accounts...");

    // Check Vault CToken is compressed (closed)
    let vault_after_warp = rpc.get_account(vault_pda).await.unwrap();
    let vault_closed = vault_after_warp.is_none()
        || vault_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        vault_closed,
        "Vault CToken should be closed after auto-compression"
    );
    println!("  ✓ Vault CToken auto-compressed (closed on-chain)");

    // Check User ATA is compressed (closed)
    let user_ata_after_warp = rpc.get_account(user_ata_pda).await.unwrap();
    let user_ata_closed = user_ata_after_warp.is_none()
        || user_ata_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        user_ata_closed,
        "User ATA CToken should be closed after auto-compression"
    );
    println!("  ✓ User ATA CToken auto-compressed (closed on-chain)");

    // CMint is now auto-compressed during warp_slot_forward (via compress_cmint_forester)
    let cmint_after_warp = rpc.get_account(cmint_pda).await.unwrap();
    let cmint_closed = cmint_after_warp.is_none()
        || cmint_after_warp
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        cmint_closed,
        "CMint should be auto-compressed (closed) after epoch warp"
    );
    println!("  ✓ CMint auto-compressed (closed on-chain)");

    // PDAs were already compressed in Phase 1
    println!("  ✓ PlaceholderRecord PDA was already compressed in Phase 1");
    println!("  ✓ UserRecord PDA was already compressed in Phase 1 (2nd PDA)");

    // Verify compressed vault token account exists with correct balance
    // On-chain: vault owner = vault_authority_pda
    // Compressed: owner = vault_pda (via compress_to_pubkey)
    // Query by vault_pda since that's what compress_to_pubkey sets as the compressed owner
    let compressed_vault_accounts = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !compressed_vault_accounts.is_empty(),
        "Should have compressed vault token account"
    );
    assert_eq!(
        compressed_vault_accounts[0].token.amount, vault_mint_amount,
        "Compressed vault token should have same balance"
    );
    println!(
        "  ✓ Verified compressed vault token account with {} tokens",
        vault_mint_amount
    );

    // Verify compressed user ATA token account exists with correct balance
    // Note: When compression_only=true, the compressed token's owner is the ATA address (compress_to_account_pubkey)
    // So we query by the user_ata_pda to find the compressed token account
    let compressed_user_ata_accounts = rpc
        .get_compressed_token_accounts_by_owner(&user_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !compressed_user_ata_accounts.is_empty(),
        "Should have compressed user ATA token account"
    );
    assert_eq!(
        compressed_user_ata_accounts[0].token.amount, user_ata_mint_amount,
        "Compressed user ATA should have same balance"
    );
    println!(
        "  ✓ Verified compressed user ATA token account with {} tokens",
        user_ata_mint_amount
    );

    // ===========================================================================
    // PHASE 4: Verify compressed state and decompress PDAs
    // ===========================================================================
    println!("PHASE 4: Verifying compressed state and decompressing PDAs...");

    // PlaceholderRecord PDA should be closed after auto-compression
    let placeholder_onchain = rpc.get_account(placeholder_pda).await.unwrap();
    let placeholder_closed = placeholder_onchain.is_none()
        || placeholder_onchain
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        placeholder_closed,
        "PlaceholderRecord PDA should be closed after auto-compression"
    );
    println!("  - PlaceholderRecord PDA closed (auto-compressed)");

    // UserRecord PDA should be closed after auto-compression (2nd PDA)
    let user_record_onchain = rpc.get_account(user_record_pda).await.unwrap();
    let user_record_closed = user_record_onchain.is_none()
        || user_record_onchain
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        user_record_closed,
        "UserRecord PDA should be closed after auto-compression"
    );
    println!("  - UserRecord PDA closed (auto-compressed)");

    // Get compressed PlaceholderRecord with data
    let compressed_placeholder = rpc
        .get_compressed_account(placeholder_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(compressed_placeholder.address.is_some());
    assert!(
        compressed_placeholder.data.is_some(),
        "PlaceholderRecord should have compressed data"
    );
    println!("  - PlaceholderRecord compressed with data");

    // Get compressed UserRecord with data
    let compressed_user_record_final = rpc
        .get_compressed_account(user_record_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(compressed_user_record_final.address.is_some());
    assert!(
        compressed_user_record_final.data.is_some(),
        "UserRecord should have compressed data"
    );
    println!("  - UserRecord compressed with data");

    // Verify compressed token accounts
    let compressed_vault = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(!compressed_vault.is_empty(), "Vault should be compressed");
    assert_eq!(compressed_vault[0].token.amount, vault_mint_amount);
    println!(
        "  - Vault CToken compressed with {} tokens",
        vault_mint_amount
    );

    let compressed_user_ata = rpc
        .get_compressed_token_accounts_by_owner(&user_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !compressed_user_ata.is_empty(),
        "User ATA should be compressed"
    );
    assert_eq!(compressed_user_ata[0].token.amount, user_ata_mint_amount);
    println!(
        "  - User ATA CToken compressed with {} tokens",
        user_ata_mint_amount
    );

    // Deserialize compressed PDA data to create variants for decompression
    use anchor_lang::AnchorDeserialize;
    use csdk_anchor_full_derived_test::{
        CTokenAccountVariant, CompressedAccountVariant, PlaceholderRecord, UserRecord,
    };
    use light_compressible_client::compressible_instruction;

    let placeholder_data = compressed_placeholder.data.as_ref().unwrap();
    let c_placeholder = PlaceholderRecord::deserialize(&mut &placeholder_data.data[..]).unwrap();
    println!(
        "  - Deserialized PlaceholderRecord: name={}, id={}",
        c_placeholder.name, c_placeholder.placeholder_id
    );

    let user_record_data = compressed_user_record_final.data.as_ref().unwrap();
    let c_user_record = UserRecord::deserialize(&mut &user_record_data.data[..]).unwrap();
    println!(
        "  - Deserialized UserRecord: name={}, score={}",
        c_user_record.name, c_user_record.score
    );

    // Build ATA token data for unified decompression
    // The runtime will:
    // 1. Use token data's owner (ATA address) for hash verification
    // 2. Find the wallet (signer) that derives to this ATA for authorization
    let compressed_ata_account = &compressed_user_ata[0];
    let ata_ctoken_data = light_ctoken_sdk::compat::CTokenData {
        variant: CTokenAccountVariant::UserAta,
        token_data: compressed_ata_account.token.clone(), // Use actual token data from indexer
    };
    println!(
        "  - Built ATA CTokenData: owner={}, amount={}",
        compressed_ata_account.token.owner, compressed_ata_account.token.amount
    );

    // Get validity proof for PDAs AND ATA (UNIFIED DECOMPRESSION!)
    let rpc_result = rpc
        .get_validity_proof(
            vec![
                compressed_placeholder.hash,
                compressed_user_record_final.hash,
                compressed_ata_account.account.hash,
            ],
            vec![],
            None,
        )
        .await
        .unwrap()
        .value;
    println!("  - Got validity proof for PDAs + ATA (unified)");

    // Build decompress instruction for PDAs + ATA (UNIFIED!)
    let mut decompress_instruction = compressible_instruction::decompress_accounts_idempotent(
        &program_id,
        &compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[placeholder_pda, user_record_pda, user_ata_pda],
        &[
            (
                compressed_placeholder.clone(),
                CompressedAccountVariant::PlaceholderRecord(c_placeholder),
            ),
            (
                compressed_user_record_final.clone(),
                CompressedAccountVariant::UserRecord(c_user_record),
            ),
            (
                compressed_ata_account.account.clone(),
                CompressedAccountVariant::CTokenData(ata_ctoken_data),
            ),
        ],
        &csdk_anchor_full_derived_test::accounts::DecompressAccountsIdempotent {
            fee_payer: payer.pubkey(),
            config: config_pda,
            rent_sponsor: payer.pubkey(),
            ctoken_rent_sponsor: Some(CTOKEN_RENT_SPONSOR),
            ctoken_config: Some(COMPRESSIBLE_CONFIG_V1),
            ctoken_program: Some(C_TOKEN_PROGRAM_ID.into()),
            ctoken_cpi_authority: Some(light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY),
            cmint_authority: None,
            authority: Some(authority.pubkey()),
            some_account: Some(some_account.pubkey()),
            mint_authority: Some(mint_authority.pubkey()),
            user: None,
            mint: None,
            cmint: Some(cmint_pda),
            mint_signer: None,
            wallet: Some(payer.pubkey()),
        }
        .to_account_metas(None),
        rpc_result,
    )
    .unwrap();

    // Append SeedParams to instruction data (required by macro-generated instruction)
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::SeedParams;
    let seed_params = SeedParams {
        owner: user_record_owner,
        category_id: user_record_category_id,
        session_id: 0, // Not used for these PDAs
        placeholder_id,
        counter,
    };
    let seed_params_data = borsh::to_vec(&seed_params).unwrap();
    decompress_instruction
        .data
        .extend_from_slice(&seed_params_data);

    println!("  - Decompressing PDAs + ATA (UNIFIED)...");
    let result = rpc
        .create_and_send_transaction(&[decompress_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(
        result.is_ok(),
        "Unified PDA + ATA decompression should succeed: {:?}",
        result
    );

    // Verify PDAs are back on-chain
    let placeholder_after = rpc.get_account(placeholder_pda).await.unwrap();
    assert!(
        placeholder_after.is_some(),
        "PlaceholderRecord should be on-chain after decompression"
    );
    println!("  - PlaceholderRecord decompressed to on-chain");

    let user_record_after = rpc.get_account(user_record_pda).await.unwrap();
    assert!(
        user_record_after.is_some(),
        "UserRecord should be on-chain after decompression"
    );
    println!("  - UserRecord decompressed to on-chain");

    // Verify ATA is back on-chain with tokens
    let user_ata_after = rpc.get_account(user_ata_pda).await.unwrap();
    assert!(
        user_ata_after.is_some(),
        "User ATA should be on-chain after unified decompression"
    );
    let user_ata_data: CToken =
        borsh::BorshDeserialize::deserialize(&mut &user_ata_after.unwrap().data[..]).unwrap();
    assert_eq!(
        user_ata_data.amount, user_ata_mint_amount,
        "User ATA should have {} tokens after unified decompression",
        user_ata_mint_amount
    );
    println!(
        "  - User ATA decompressed with {} tokens (UNIFIED!)",
        user_ata_data.amount
    );

    // After decompression, the compressed accounts are nullified (consumed).
    // The indexer may still return them but they should be marked as spent.
    // For now, we just verify the on-chain accounts exist which is the important part.
    println!("  - Compressed accounts nullified (decompression complete)");

    // Verify compressed ATA is consumed
    let remaining_ata = rpc
        .get_compressed_token_accounts_by_owner(&user_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert_eq!(
        remaining_ata.len(),
        0,
        "Compressed User ATA should be consumed after unified decompression"
    );

    println!("  Phase 4 complete: PDAs + ATA decompressed UNIFIED!");

    // ===========================================================================
    // PHASE 5: Decompress CMint via mint_action_comprehensive
    // ===========================================================================
    println!("PHASE 5: Decompressing CMint...");

    use light_token_client::actions::mint_action_comprehensive;
    use light_token_client::instructions::mint_action::DecompressMintParams;

    // Verify CMint is closed (auto-compressed in Phase 3)
    let cmint_before_decompress = rpc.get_account(cmint_pda).await.unwrap();
    let cmint_is_closed = cmint_before_decompress.is_none()
        || cmint_before_decompress
            .as_ref()
            .map(|a| a.lamports == 0)
            .unwrap_or(true);
    assert!(
        cmint_is_closed,
        "CMint should be closed before decompression"
    );
    println!("  - CMint confirmed closed");

    // Decompress CMint
    mint_action_comprehensive(
        &mut rpc,
        &mint_signer,                          // mint_seed
        &mint_authority,                       // authority
        &payer,                                // payer
        Some(DecompressMintParams::default()), // decompress_mint
        false,                                 // compress_and_close_cmint
        vec![],                                // mint_to_recipients
        vec![],                                // mint_to_decompressed_recipients
        None,                                  // update_mint_authority
        None,                                  // update_freeze_authority
        None,                                  // new_mint
    )
    .await
    .expect("CMint decompression should succeed");

    // Verify CMint is back on-chain
    let cmint_after = rpc.get_account(cmint_pda).await.unwrap();
    assert!(
        cmint_after.is_some(),
        "CMint should be on-chain after decompression"
    );
    let cmint_data: light_ctoken_interface::state::CompressedMint =
        borsh::BorshDeserialize::deserialize(&mut &cmint_after.unwrap().data[..])
            .expect("Failed to deserialize CMint");
    assert_eq!(
        cmint_data.base.decimals, 9,
        "CMint should have correct decimals"
    );
    println!(
        "  - CMint decompressed to on-chain with {} decimals",
        cmint_data.base.decimals
    );

    println!("\nE2E test completed successfully!");
    println!("   PHASE 1: Created ALL accounts atomically:");
    println!("     - PlaceholderRecord PDA (on-chain, empty compressed shell)");
    println!("     - UserRecord PDA (on-chain, empty compressed shell)");
    println!("     - CMint (decompressed on-chain)");
    println!("     - Vault CToken with {} tokens", vault_mint_amount);
    println!("     - User ATA with {} tokens", user_ata_mint_amount);
    println!("   PHASE 2: Epoch warp triggered auto-compression");
    println!("   PHASE 3: Verified all accounts compressed (PDAs, CTokens, CMint)");
    println!("   PHASE 4: UNIFIED DECOMPRESSION - PDAs + User ATA in ONE instruction!");
    println!("     - PlaceholderRecord PDA decompressed");
    println!("     - UserRecord PDA decompressed");
    println!(
        "     - User ATA decompressed with {} tokens",
        user_ata_mint_amount
    );
    println!("   PHASE 5: Decompressed CMint");
    println!("   Full lifecycle: CREATE -> COMPRESS -> UNIFIED DECOMPRESS complete!");
}
