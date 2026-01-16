use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_compressible_client::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_macros::pubkey;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMintMetadata,
};
use light_token_sdk::compressed_token::create_compressed_mint::{
    derive_mint_compressed_address, find_mint_address,
};
use light_token_sdk::token::find_mint_address as find_cmint_address;
use light_token_types::CPI_AUTHORITY_PDA;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

/// 2 PDAs + 1 CMint + 1 Vault + 1 User ATA, all in one instruction with single proof.
/// After init: all accounts on-chain + parseable.
/// After warp: all cold (auto-compressed) with non-empty compressed data.
#[tokio::test]
async fn test_create_pdas_and_mint_auto() {
    use csdk_anchor_full_derived_test::instruction_accounts::{LP_MINT_SIGNER_SEED, VAULT_SEED};
    use csdk_anchor_full_derived_test::FullAutoWithMintParams;
    use light_token_interface::state::Token;
    use light_token_sdk::token::{
        get_associated_token_address_and_bump, COMPRESSIBLE_CONFIG_V1,
        RENT_SPONSOR as CTOKEN_RENT_SPONSOR,
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
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
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

    // PHASE 3: Decompress PDAs + vault via build_decompress_idempotent
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        GameSessionSeeds, TokenAccountVariant, UserRecordSeeds,
    };
    use light_compressible_client::{
        compressible_instruction, AccountInterface, RentFreeDecompressAccount,
    };

    // Fetch compressed PDA accounts
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

    // Fetch compressed vault token account
    let compressed_vault_accounts = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    let compressed_vault = &compressed_vault_accounts[0];

    // Get validity proof for PDAs + vault
    let rpc_result = rpc
        .get_validity_proof(
            vec![
                compressed_user.hash,
                compressed_game.hash,
                compressed_vault.account.hash,
            ],
            vec![],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build RentFreeDecompressAccount using from_seeds and from_ctoken helpers
    let decompress_accounts = vec![
        RentFreeDecompressAccount::from_seeds(
            AccountInterface::cold(user_record_pda, compressed_user.clone()),
            UserRecordSeeds {
                authority: authority.pubkey(),
                mint_authority: mint_authority.pubkey(),
                owner,
                category_id,
            },
        )
        .expect("UserRecord seed verification failed"),
        RentFreeDecompressAccount::from_seeds(
            AccountInterface::cold(game_session_pda, compressed_game.clone()),
            GameSessionSeeds {
                fee_payer: payer.pubkey(),
                authority: authority.pubkey(),
                session_id,
            },
        )
        .expect("GameSession seed verification failed"),
        RentFreeDecompressAccount::from_ctoken(
            AccountInterface::cold(vault_pda, compressed_vault.account.clone()),
            TokenAccountVariant::Vault { cmint: cmint_pda },
        )
        .expect("CToken variant construction failed"),
    ];

    // Build decompress instruction
    // No SeedParams needed - data.* seeds from unpacked account, ctx.* from variant idx
    let decompress_instruction = compressible_instruction::build_decompress_idempotent(
        &program_id,
        decompress_accounts,
        compressible_instruction::decompress::accounts(payer.pubkey(), config_pda, payer.pubkey()),
        rpc_result,
    )
    .unwrap()
    .expect("Should have cold accounts to decompress");

    rpc.create_and_send_transaction(&[decompress_instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("PDA + vault decompression should succeed");

    // Assert PDAs are back on-chain
    assert_onchain_exists(&mut rpc, &user_record_pda).await;
    assert_onchain_exists(&mut rpc, &game_session_pda).await;

    // Assert vault is back on-chain with correct balance
    assert_onchain_exists(&mut rpc, &vault_pda).await;
    let vault_after = parse_token(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    assert_eq!(vault_after.amount, vault_mint_amount);

    // Verify compressed vault token is consumed (no more compressed token accounts for vault)
    let remaining_vault = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(remaining_vault.is_empty());

    // PHASE 4: Decompress user ATA via new high-performance API pattern
    use light_compressible_client::{
        build_decompress_token_accounts, decompress_cmint, decompress_token_accounts,
        parse_token_account_interface,
    };

    // Step 1: Fetch raw account interface (Account bytes always present)
    let account_interface = rpc
        .get_ata_account_interface(&cmint_pda, &payer.pubkey())
        .await
        .expect("get_ata_account_interface should succeed");

    // Verify raw bytes are present (even for cold accounts)
    assert_eq!(account_interface.account.data.len(), 165);

    // Step 2: Parse into TokenAccountInterface (sync, no RPC)
    let parsed = parse_token_account_interface(&account_interface)
        .expect("parse_token_account_interface should succeed");

    // Verify it's cold (compressed)
    assert!(parsed.is_cold, "ATA should be cold after warp");
    assert!(
        parsed.decompression_context.is_some(),
        "Cold ATA should have decompression_context"
    );

    // Amount accessible via TokenData
    assert_eq!(parsed.amount(), user_ata_mint_amount);

    // Step 3: Get proof and build instructions (sync after proof)
    let cold_hash = parsed.hash().expect("Cold ATA should have hash");
    let proof = rpc
        .get_validity_proof(vec![cold_hash], vec![], None)
        .await
        .expect("get_validity_proof should succeed")
        .value;

    // Step 4: Build decompress instructions (sync)
    let ata_instructions = build_decompress_token_accounts(&[parsed], payer.pubkey(), Some(proof))
        .expect("build_decompress_token_accounts should succeed");

    assert!(!ata_instructions.is_empty(), "Should have instructions");

    rpc.create_and_send_transaction(&ata_instructions, &payer.pubkey(), &[&payer])
        .await
        .expect("ATA decompression should succeed");

    // Assert user ATA is back on-chain with correct balance
    assert_onchain_exists(&mut rpc, &user_ata_pda).await;
    let user_ata_after = parse_token(&rpc.get_account(user_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user_ata_after.amount, user_ata_mint_amount);

    // Verify idempotency: calling again should return empty vec
    let account_interface_again = rpc
        .get_ata_account_interface(&cmint_pda, &payer.pubkey())
        .await
        .expect("get_ata_account_interface should succeed");

    let parsed_again = parse_token_account_interface(&account_interface_again)
        .expect("parse_token_account_interface should succeed");

    assert!(
        !parsed_again.is_cold,
        "ATA should be hot after decompression"
    );
    assert!(
        parsed_again.decompression_context.is_none(),
        "Hot ATA should not have decompression_context"
    );

    // Using async wrapper (alternative pattern)
    let ata_instructions_again = decompress_token_accounts(&[parsed_again], payer.pubkey(), &rpc)
        .await
        .expect("decompress_token_accounts should succeed");
    assert!(
        ata_instructions_again.is_empty(),
        "Should return empty vec when already decompressed"
    );

    // PHASE 5: Decompress CMint via decompress_cmint (lean wrapper)
    let mint_interface = rpc
        .get_mint_interface(&mint_signer_pda)
        .await
        .expect("get_mint_interface should succeed");

    // Verify it's cold (compressed)
    assert!(mint_interface.is_cold(), "Mint should be cold after warp");

    // Decompress using lean wrapper (fetches proof internally)
    let mint_instructions = decompress_cmint(&mint_interface, payer.pubkey(), &rpc)
        .await
        .expect("decompress_cmint should succeed");

    if !mint_instructions.is_empty() {
        rpc.create_and_send_transaction(&mint_instructions, &payer.pubkey(), &[&payer])
            .await
            .expect("Mint decompression should succeed");
    }

    // Assert CMint is back on-chain
    assert_onchain_exists(&mut rpc, &cmint_pda).await;

    // Verify calling again returns empty vec (idempotent)
    let mint_interface_again = rpc
        .get_mint_interface(&mint_signer_pda)
        .await
        .expect("get_mint_interface should succeed");
    assert!(
        mint_interface_again.is_hot(),
        "Mint should be hot after decompression"
    );
    let mint_instructions_again = decompress_cmint(&mint_interface_again, payer.pubkey(), &rpc)
        .await
        .expect("decompress_cmint should succeed");
    assert!(
        mint_instructions_again.is_empty(),
        "Should return empty vec when mint already decompressed"
    );
}
