use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_ctoken_sdk::compressed_token::create_compressed_mint::{
    derive_cmint_compressed_address, find_cmint_address,
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

const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

/// 2 PDAs + 1 CMint + 1 Vault + 1 User ATA, all in one instruction with single proof.
/// After init: all accounts on-chain + parseable.
/// After warp: all cold (auto-compressed) with non-empty compressed data.
#[tokio::test]
async fn test_create_pdas_and_mint_auto() {
    use csdk_anchor_full_derived_test::instruction_accounts::{LP_MINT_SIGNER_SEED, VAULT_SEED};
    use csdk_anchor_full_derived_test::FullAutoWithMintParams;
    use light_ctoken_sdk::ctoken::{
        get_associated_ctoken_address_and_bump, CToken, COMPRESSIBLE_CONFIG_V1,
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
    fn parse_ctoken(data: &[u8]) -> CToken {
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

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let _program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

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

    let authority = Keypair::new();
    let mint_authority = Keypair::new();

    let owner = payer.pubkey();
    let category_id = 111u64;
    let session_id = 222u64;
    let vault_mint_amount = 100u64;
    let user_ata_mint_amount = 50u64;

    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[LP_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (cmint_pda, _) = find_cmint_address(&mint_signer_pda);
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, cmint_pda.as_ref()], &program_id);
    let (vault_authority_pda, _) = Pubkey::find_program_address(&[b"vault_authority"], &program_id);
    let (user_ata_pda, user_ata_bump) =
        get_associated_ctoken_address_and_bump(&payer.pubkey(), &cmint_pda);

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

    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

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
        derive_cmint_compressed_address(&mint_signer_pda, &address_tree_pubkey);

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
    let vault_data = parse_ctoken(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    assert_eq!(vault_data.owner, vault_authority_pda.to_bytes());
    assert_eq!(vault_data.amount, vault_mint_amount);

    let user_ata_data = parse_ctoken(&rpc.get_account(user_ata_pda).await.unwrap().unwrap().data);
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

    // PHASE 3: Decompress PDAs + vault via decompress_accounts_idempotent
    use anchor_lang::AnchorDeserialize;
    use csdk_anchor_full_derived_test::{
        CTokenAccountVariant, CompressedAccountVariant, GameSession, UserRecord,
    };
    use light_compressible_client::compressible_instruction;

    // Fetch and deserialize compressed PDA accounts
    let compressed_user = rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let c_user_record =
        UserRecord::deserialize(&mut &compressed_user.data.as_ref().unwrap().data[..]).unwrap();

    let compressed_game = rpc
        .get_compressed_account(game_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let c_game_session =
        GameSession::deserialize(&mut &compressed_game.data.as_ref().unwrap().data[..]).unwrap();

    // Fetch compressed vault token account
    let compressed_vault_accounts = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    let compressed_vault = &compressed_vault_accounts[0];
    let vault_ctoken_data = light_ctoken_sdk::compat::CTokenData {
        variant: CTokenAccountVariant::Vault,
        token_data: compressed_vault.token.clone(),
    };

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

    // Build decompress instruction
    let mut decompress_instruction = compressible_instruction::decompress_accounts_idempotent(
        &program_id,
        &compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[user_record_pda, game_session_pda, vault_pda],
        &[
            (
                compressed_user.clone(),
                CompressedAccountVariant::UserRecord(c_user_record),
            ),
            (
                compressed_game.clone(),
                CompressedAccountVariant::GameSession(c_game_session),
            ),
            (
                compressed_vault.account.clone(),
                CompressedAccountVariant::CTokenData(vault_ctoken_data),
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
            some_account: None,
            mint_authority: Some(mint_authority.pubkey()),
            user: Some(payer.pubkey()),
            mint: None,
            cmint: Some(cmint_pda),
            mint_signer: None,
            wallet: None,
        }
        .to_account_metas(None),
        rpc_result,
    )
    .unwrap();

    // Append SeedParams to instruction data
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::SeedParams;
    let seed_params = SeedParams {
        owner,
        category_id,
        session_id,
        placeholder_id: 0,
        counter: 0,
    };
    let seed_params_data = borsh::to_vec(&seed_params).unwrap();
    decompress_instruction
        .data
        .extend_from_slice(&seed_params_data);

    rpc.create_and_send_transaction(&[decompress_instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("PDA + vault decompression should succeed");

    // Assert PDAs are back on-chain
    assert_onchain_exists(&mut rpc, &user_record_pda).await;
    assert_onchain_exists(&mut rpc, &game_session_pda).await;

    // Assert vault is back on-chain with correct balance
    assert_onchain_exists(&mut rpc, &vault_pda).await;
    let vault_after = parse_ctoken(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    assert_eq!(vault_after.amount, vault_mint_amount);

    // Verify compressed vault token is consumed (no more compressed token accounts for vault)
    let remaining_vault = rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(remaining_vault.is_empty());
}
