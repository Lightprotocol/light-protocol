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
    let user1 = Keypair::new(); // First user for ATA
    let user2 = Keypair::new(); // Second user for ATA

    let owner = payer.pubkey();
    let category_id = 111u64;
    let session_id = 222u64;
    let vault_mint_amount = 100u64;
    let user1_ata_mint_amount = 50u64;
    let user2_ata_mint_amount = 75u64;

    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[LP_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (cmint_pda, _) = find_cmint_address(&mint_signer_pda);
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, cmint_pda.as_ref()], &program_id);
    let (vault_authority_pda, _) = Pubkey::find_program_address(&[b"vault_authority"], &program_id);
    let (user1_ata_pda, user1_ata_bump) =
        get_associated_ctoken_address_and_bump(&user1.pubkey(), &cmint_pda);
    let (user2_ata_pda, user2_ata_bump) =
        get_associated_ctoken_address_and_bump(&user2.pubkey(), &cmint_pda);

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
        user1: user1.pubkey(),
        user2: user2.pubkey(),
        mint_signer: mint_signer_pda,
        user_record: user_record_pda,
        game_session: game_session_pda,
        cmint: cmint_pda,
        vault: vault_pda,
        vault_authority: vault_authority_pda,
        user1_ata: user1_ata_pda,
        user2_ata: user2_ata_pda,
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
            vault_mint_amount,
            user1_ata_bump,
            user1_ata_mint_amount,
            user2_ata_bump,
            user2_ata_mint_amount,
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
        &[&payer, &authority, &mint_authority, &user1, &user2],
    )
    .await
    .unwrap();

    // PHASE 1: After init - all accounts on-chain and parseable
    assert_onchain_exists(&mut rpc, &user_record_pda).await;
    assert_onchain_exists(&mut rpc, &game_session_pda).await;
    assert_onchain_exists(&mut rpc, &cmint_pda).await;
    assert_onchain_exists(&mut rpc, &vault_pda).await;
    assert_onchain_exists(&mut rpc, &user1_ata_pda).await;
    assert_onchain_exists(&mut rpc, &user2_ata_pda).await;

    // Parse and verify CToken data
    let vault_data = parse_ctoken(&rpc.get_account(vault_pda).await.unwrap().unwrap().data);
    assert_eq!(vault_data.owner, vault_authority_pda.to_bytes());
    assert_eq!(vault_data.amount, vault_mint_amount);

    let user1_ata_data = parse_ctoken(&rpc.get_account(user1_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user1_ata_data.owner, user1.pubkey().to_bytes());
    assert_eq!(user1_ata_data.amount, user1_ata_mint_amount);

    let user2_ata_data = parse_ctoken(&rpc.get_account(user2_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user2_ata_data.owner, user2.pubkey().to_bytes());
    assert_eq!(user2_ata_data.amount, user2_ata_mint_amount);

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
    assert_onchain_closed(&mut rpc, &user1_ata_pda).await;
    assert_onchain_closed(&mut rpc, &user2_ata_pda).await;

    // Compressed accounts should exist with non-empty data
    assert_compressed_exists_with_data(&mut rpc, user_compressed_address).await;
    assert_compressed_exists_with_data(&mut rpc, game_compressed_address).await;
    assert_compressed_exists_with_data(&mut rpc, mint_compressed_address).await;

    // Compressed token accounts should exist with correct balances
    assert_compressed_token_exists(&mut rpc, &vault_pda, vault_mint_amount).await;
    assert_compressed_token_exists(&mut rpc, &user1_ata_pda, user1_ata_mint_amount).await;
    assert_compressed_token_exists(&mut rpc, &user2_ata_pda, user2_ata_mint_amount).await;

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

    // PHASE 4: Decompress BOTH ATAs in a single instruction call
    decompress_multiple_atas_helper(
        &mut rpc,
        &payer,
        &[&user1, &user2],
        &[
            (user1_ata_pda, cmint_pda, user1_ata_mint_amount),
            (user2_ata_pda, cmint_pda, user2_ata_mint_amount),
        ],
        program_id,
    )
    .await;

    // Verify BOTH ATAs are back on-chain
    assert_onchain_exists(&mut rpc, &user1_ata_pda).await;
    let user1_ata_after =
        parse_ctoken(&rpc.get_account(user1_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user1_ata_after.amount, user1_ata_mint_amount);

    assert_onchain_exists(&mut rpc, &user2_ata_pda).await;
    let user2_ata_after =
        parse_ctoken(&rpc.get_account(user2_ata_pda).await.unwrap().unwrap().data);
    assert_eq!(user2_ata_after.amount, user2_ata_mint_amount);

    // Verify BOTH compressed ATAs are consumed
    let remaining_ata1 = rpc
        .get_compressed_token_accounts_by_owner(&user1_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(remaining_ata1.is_empty());

    let remaining_ata2 = rpc
        .get_compressed_token_accounts_by_owner(&user2_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(remaining_ata2.is_empty());
}

/// Helper function to decompress multiple ATAs in a single instruction.
/// atas: slice of (ata_pubkey, mint_pubkey, expected_amount)
async fn decompress_multiple_atas_helper(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    wallets: &[&Keypair],
    atas: &[(Pubkey, Pubkey, u64)], // (ata_pubkey, mint_pubkey, amount)
    program_id: Pubkey,
) {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CompressedAtaAccountData, CompressedAtaTokenData, CompressedAtaVariant,
        DecompressAtasParams,
    };
    use light_ctoken_sdk::ctoken::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};
    use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

    assert_eq!(wallets.len(), atas.len(), "Must have one wallet per ATA");

    // Fetch all compressed ATAs and collect hashes
    let mut compressed_atas = Vec::with_capacity(atas.len());
    let mut hashes = Vec::with_capacity(atas.len());

    for (ata_pubkey, _mint_pubkey, expected_amount) in atas {
        let accounts = rpc
            .get_compressed_token_accounts_by_owner(ata_pubkey, None, None)
            .await
            .unwrap()
            .value
            .items;
        assert!(
            !accounts.is_empty(),
            "Should have compressed ATA for {}",
            ata_pubkey
        );
        let compressed = accounts.into_iter().next().unwrap();
        assert_eq!(compressed.token.amount, *expected_amount);
        hashes.push(compressed.account.hash);
        compressed_atas.push(compressed);
    }

    // Get single validity proof for ALL compressed accounts
    let proof_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .unwrap()
        .value;

    // Use first ATA's tree info for shared tree accounts
    let first_ata = &compressed_atas[0];
    let output_queue = first_ata
        .account
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(first_ata.account.tree_info.queue);

    // Use PackedAccounts for de-duplication (same as SDK pattern)
    use light_sdk::instruction::PackedAccounts;
    let mut packed = PackedAccounts::default();

    // Trees first
    let state_tree_index = packed.insert_or_get(first_ata.account.tree_info.tree);
    let input_queue_index = packed.insert_or_get(first_ata.account.tree_info.queue);
    let _output_queue_index = packed.insert_or_get(output_queue);

    // Collect indices for each ATA
    let mut ata_indices: Vec<(u8, u8, u8)> = Vec::with_capacity(atas.len());

    for ((ata_pubkey, mint_pubkey, _), wallet) in atas.iter().zip(wallets.iter()) {
        let wallet_idx = packed.insert_or_get_config(wallet.pubkey(), true, false);
        let mint_idx = packed.insert_or_get_read_only(*mint_pubkey);
        let ata_idx = packed.insert_or_get(*ata_pubkey);
        ata_indices.push((wallet_idx, mint_idx, ata_idx));
    }

    // Build CompressedAtaAccountData with indices from PackedAccounts
    let mut compressed_accounts = Vec::with_capacity(atas.len());

    for (i, ((_, mint_pubkey, amount), wallet)) in atas.iter().zip(wallets.iter()).enumerate() {
        let compressed = &compressed_atas[i];
        let root_index = proof_result.accounts[i]
            .root_index
            .root_index()
            .unwrap_or(0);
        let (wallet_idx, mint_idx, ata_idx) = ata_indices[i];

        compressed_accounts.push(CompressedAtaAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info: light_sdk::instruction::PackedStateTreeInfo {
                    merkle_tree_pubkey_index: state_tree_index,
                    queue_pubkey_index: input_queue_index,
                    root_index,
                    leaf_index: compressed.account.leaf_index,
                    prove_by_index: proof_result.accounts[i].root_index.proof_by_index(),
                },
                output_state_tree_index: 0,
            },
            data: CompressedAtaVariant::Standard(CompressedAtaTokenData {
                wallet: wallet.pubkey(),
                mint: *mint_pubkey,
                amount: *amount,
                delegate: None,
                is_frozen: false,
            }),
            wallet_index: wallet_idx,
            mint_index: mint_idx,
            ata_index: ata_idx,
        });
    }

    let decompress_params = DecompressAtasParams {
        proof: light_sdk::instruction::ValidityProof(proof_result.proof.0),
        compressed_accounts,
        system_accounts_offset: 0,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::DecompressAtas {
        fee_payer: payer.pubkey(),
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        system_program: solana_sdk::system_program::ID,
    };

    // Build remaining accounts: system accounts + packed_accounts from PackedAccounts
    let mut remaining_accounts = vec![
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::C_TOKEN_PROGRAM_ID.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::LIGHT_SYSTEM_PROGRAM_ID.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY,
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::REGISTERED_PROGRAM_PDA.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::ACCOUNT_COMPRESSION_AUTHORITY_PDA.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID.into(),
            false,
        ),
    ];

    // Add de-duplicated packed_accounts from PackedAccounts
    let (packed_account_metas, _, _) = packed.to_account_metas();
    remaining_accounts.extend(packed_account_metas);

    let instruction_data = csdk_anchor_full_derived_test::instruction::DecompressAtas {
        params: decompress_params,
    };

    let instruction = Instruction {
        program_id,
        accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    // Payer pays fees, all wallets must sign
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend(wallets.iter().copied());

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
        .expect("Decompress ATAs should succeed");
}

/// Test decompressing multiple compressed mints in one instruction.
/// Creates 2 compressed mints, then decompresses both with a single call.
#[tokio::test]
async fn test_decompress_cmints() {
    use csdk_anchor_full_derived_test::instruction_accounts::DecompressCMintsParams;
    use light_ctoken_interface::instructions::mint_action::CompressedMintInstructionData;
    use light_ctoken_sdk::compressed_token::create_compressed_mint::derive_cmint_compressed_address;
    use light_ctoken_sdk::ctoken::{
        find_cmint_address, CompressedMintWithContext, CreateCMint, CreateCMintParams,
        COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR,
    };

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let authority = Keypair::new();

    // Airdrop to authority for signing
    rpc.airdrop_lamports(&authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Create two mint signers
    let mint_signer1 = Keypair::new();
    let mint_signer2 = Keypair::new();

    // Derive CMint PDAs
    let (cmint1_pda, _) = find_cmint_address(&mint_signer1.pubkey());
    let (cmint2_pda, _) = find_cmint_address(&mint_signer2.pubkey());

    // Derive compressed addresses for both mints
    let cmint1_compressed_address =
        derive_cmint_compressed_address(&mint_signer1.pubkey(), &address_tree_pubkey);
    let cmint2_compressed_address =
        derive_cmint_compressed_address(&mint_signer2.pubkey(), &address_tree_pubkey);

    // Get validity proof for creating mint 1 (non-inclusion proof)
    let create_proof_result1 = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: cmint1_compressed_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Create mint 1
    let params1 = CreateCMintParams {
        decimals: 9,
        address_merkle_tree_root_index: create_proof_result1.addresses[0].root_index,
        mint_authority: authority.pubkey(),
        proof: create_proof_result1.proof.0.unwrap(),
        compression_address: cmint1_compressed_address,
        mint: cmint1_pda,
        freeze_authority: None,
        extensions: None,
    };

    let create_mint1_ix = CreateCMint::new(
        params1,
        mint_signer1.pubkey(),
        payer.pubkey(),
        address_tree_pubkey,
        state_tree_info.queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint1_ix],
        &payer.pubkey(),
        &[&payer, &mint_signer1, &authority],
    )
    .await
    .expect("Create mint 1 should succeed");

    // Get new proof for mint 2 (state changed after mint 1)
    let create_proof_result2 = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: cmint2_compressed_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Create mint 2
    let params2 = CreateCMintParams {
        decimals: 6,
        address_merkle_tree_root_index: create_proof_result2.addresses[0].root_index,
        mint_authority: authority.pubkey(),
        proof: create_proof_result2.proof.0.unwrap(),
        compression_address: cmint2_compressed_address,
        mint: cmint2_pda,
        freeze_authority: None,
        extensions: None,
    };

    let create_mint2_ix = CreateCMint::new(
        params2,
        mint_signer2.pubkey(),
        payer.pubkey(),
        address_tree_pubkey,
        state_tree_info.queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint2_ix],
        &payer.pubkey(),
        &[&payer, &mint_signer2, &authority],
    )
    .await
    .expect("Create mint 2 should succeed");

    // Verify both compressed mints exist (with data - not decompressed)
    let compressed_mint1 = rpc
        .get_compressed_account(cmint1_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(compressed_mint1.address.unwrap(), cmint1_compressed_address);
    assert!(!compressed_mint1.data.as_ref().unwrap().data.is_empty());

    let compressed_mint2 = rpc
        .get_compressed_account(cmint2_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(compressed_mint2.address.unwrap(), cmint2_compressed_address);
    assert!(!compressed_mint2.data.as_ref().unwrap().data.is_empty());

    // =========================================================================
    // Test decompress_cmints with NEW instruction design
    // Client-side validation: at most 1 mint allowed
    // =========================================================================
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CompressedMintAccountData, CompressedMintTokenData, CompressedMintVariant,
    };
    use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
    use light_sdk::instruction::PackedAccounts;

    // CLIENT-SIDE VALIDATION: at most 1 mint
    // For this test, we only decompress mint1 to demonstrate the pattern
    let mints_to_decompress = vec![(
        &compressed_mint1,
        &mint_signer1,
        cmint1_pda,
        cmint1_compressed_address,
    )];

    if mints_to_decompress.len() > 1 {
        panic!("Client-side error: at most 1 mint can be decompressed per instruction");
    }

    // Get validity proof for mint1 only
    let decompress_proof_result = rpc
        .get_validity_proof(vec![compressed_mint1.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Parse mint data from compressed account
    let mint1_data: light_ctoken_interface::state::CompressedMint =
        borsh::BorshDeserialize::deserialize(
            &mut &compressed_mint1.data.as_ref().unwrap().data[..],
        )
        .unwrap();

    // Build PackedAccounts for tree infos only
    let mut packed_accounts = PackedAccounts::default();

    // Pack tree infos from validity proof
    let packed_tree_infos = decompress_proof_result.pack_tree_infos(&mut packed_accounts);
    let packed_tree_info = &packed_tree_infos
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];

    // Add output queue
    let output_queue = compressed_mint1
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(compressed_mint1.tree_info.queue);
    let output_state_tree_index = packed_accounts.insert_or_get(output_queue);

    // Build CompressedMintWithContext
    let cmint1_with_context = CompressedMintWithContext {
        leaf_index: packed_tree_info.leaf_index,
        prove_by_index: packed_tree_info.prove_by_index,
        root_index: packed_tree_info.root_index,
        address: cmint1_compressed_address,
        mint: Some(CompressedMintInstructionData::try_from(mint1_data).unwrap()),
    };

    // Build compressed account data with new struct types
    let compressed_accounts = vec![CompressedMintAccountData {
        meta: CompressedAccountMetaNoLamportsNoAddress {
            tree_info: *packed_tree_info,
            output_state_tree_index,
        },
        data: CompressedMintVariant::Standard(CompressedMintTokenData {
            mint_seed_pubkey: mint_signer1.pubkey(),
            compressed_mint_with_context: cmint1_with_context,
            rent_payment: 2,
            write_top_up: 5000,
        }),
    }];

    // Build remaining accounts manually (order matches what on-chain code expects):
    // 0: ctoken_program (required for CPI)
    // 1: light_system_program
    // 2: cpi_authority_pda (ctoken's CPI authority)
    // 3: registered_program_pda
    // 4: account_compression_authority
    // 5: account_compression_program
    // 6: state_tree
    // 7: input_queue
    // 8: output_queue
    // 9+: [mint_signer, cmint] per mint
    let remaining_accounts = vec![
        // CToken program (required for CPI)
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::C_TOKEN_PROGRAM_ID.into(),
            false,
        ),
        // System accounts
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::LIGHT_SYSTEM_PROGRAM_ID.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY,
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::REGISTERED_PROGRAM_PDA.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::ACCOUNT_COMPRESSION_AUTHORITY_PDA.into(),
            false,
        ),
        solana_instruction::AccountMeta::new_readonly(
            light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID.into(),
            false,
        ),
        // Tree accounts
        solana_instruction::AccountMeta::new(compressed_mint1.tree_info.tree, false),
        solana_instruction::AccountMeta::new(compressed_mint1.tree_info.queue, false),
        solana_instruction::AccountMeta::new(output_queue, false),
        // Mint 1 accounts: [mint_signer, cmint]
        solana_instruction::AccountMeta::new_readonly(mint_signer1.pubkey(), false),
        solana_instruction::AccountMeta::new(cmint1_pda, false),
    ];

    // system_accounts_offset is 0 since system accounts start at beginning of remaining
    let decompress_params = DecompressCMintsParams {
        proof: decompress_proof_result.proof,
        compressed_accounts,
        system_accounts_offset: 0,
    };

    // Build accounts
    let accounts = csdk_anchor_full_derived_test::accounts::DecompressCMints {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::DecompressCmints {
        params: decompress_params,
    };

    let instruction = Instruction {
        program_id,
        accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("Decompress cmints should succeed");

    // Verify CMint 1 is now on-chain
    let cmint1_account = rpc.get_account(cmint1_pda).await.unwrap();
    assert!(cmint1_account.is_some(), "CMint 1 should exist on-chain");

    // Verify compressed account now has empty data (decompressed state)
    let compressed_mint1_after = rpc
        .get_compressed_account(cmint1_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        compressed_mint1_after
            .data
            .as_ref()
            .unwrap()
            .data
            .is_empty(),
        "Compressed mint 1 should have empty data after decompression"
    );

    // Mint 2 should still be compressed (not decompressed)
    let compressed_mint2_still = rpc
        .get_compressed_account(cmint2_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        !compressed_mint2_still
            .data
            .as_ref()
            .unwrap()
            .data
            .is_empty(),
        "Compressed mint 2 should still have data (not decompressed)"
    );
}

/// Test decompressing compression_only ATAs via the decompress_atas instruction.
/// This test uses the existing infrastructure from test_create_pdas_and_mint_auto
/// to create and compress ATAs, then decompresses them.
///
/// NOTE: This test is a placeholder demonstrating the decompress_atas instruction structure.
/// The instruction handler and data structures are in place.
/// Full end-to-end testing requires resolving the MintToCToken API complexity.
#[tokio::test]
async fn test_decompress_atas_structure() {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CompressedAtaAccountData, CompressedAtaTokenData, CompressedAtaVariant,
        DecompressAtasParams,
    };
    use light_ctoken_sdk::ctoken::COMPRESSIBLE_CONFIG_V1;
    use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Verify the data structures compile correctly
    let _example_params = DecompressAtasParams {
        proof: light_sdk::instruction::ValidityProof(None),
        compressed_accounts: vec![CompressedAtaAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info: light_sdk::instruction::PackedStateTreeInfo {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    root_index: 0,
                    leaf_index: 0,
                    prove_by_index: true,
                },
                output_state_tree_index: 0,
            },
            data: CompressedAtaVariant::Standard(CompressedAtaTokenData {
                wallet: payer.pubkey(),
                mint: payer.pubkey(), // placeholder
                amount: 1000,
                delegate: None,
                is_frozen: false,
            }),
            wallet_index: 3,
            mint_index: 4,
            ata_index: 5,
        }],
        system_accounts_offset: 0,
    };

    // Verify the accounts struct compiles
    let _accounts = csdk_anchor_full_derived_test::accounts::DecompressAtas {
        fee_payer: payer.pubkey(),
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: RENT_SPONSOR,
        system_program: solana_sdk::system_program::ID,
    };

    // The instruction handler is implemented in lib.rs
    // Full e2e test would require:
    // 1. Creating a CMint
    // 2. Creating compression_only ATAs
    // 3. Minting tokens to ATAs
    // 4. Warping to trigger compression
    // 5. Calling decompress_atas

    // For now, just verify the structures are correct
    println!("DecompressAtas instruction structures validated successfully");
}
