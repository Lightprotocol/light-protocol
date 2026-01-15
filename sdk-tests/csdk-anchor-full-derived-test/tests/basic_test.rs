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
        DecompressAtasParams, PackedAtaAccountData, PackedAtaTokenData, PackedAtaVariant,
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

    // Build PackedAtaAccountData - PACKED format (indices only, ~14 bytes per ATA)
    let mut compressed_accounts = Vec::with_capacity(atas.len());

    for (i, ((_, _, amount), _)) in atas.iter().zip(wallets.iter()).enumerate() {
        let compressed = &compressed_atas[i];
        let root_index = proof_result.accounts[i]
            .root_index
            .root_index()
            .unwrap_or(0);
        let (wallet_idx, mint_idx, ata_idx) = ata_indices[i];

        // PACKED: no pubkeys, just indices + values
        compressed_accounts.push(PackedAtaAccountData {
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
            data: PackedAtaVariant::Standard(PackedAtaTokenData {
                wallet_index: wallet_idx,
                mint_index: mint_idx,
                ata_index: ata_idx,
                amount: *amount,
                has_delegate: false,
                delegate_index: 0,
                is_frozen: false,
            }),
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

/// Test decompressing compressed mints with PACKED instruction data.
/// Creates 2 compressed mints, then decompresses mint1 using packed format.
#[tokio::test]
async fn test_decompress_cmints() {
    use csdk_anchor_full_derived_test::instruction_accounts::DecompressCMintsParams;
    use light_ctoken_sdk::compressed_token::create_compressed_mint::derive_cmint_compressed_address;
    use light_ctoken_sdk::ctoken::{
        find_cmint_address, CreateCMint, CreateCMintParams, COMPRESSIBLE_CONFIG_V1,
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
    // Test decompress_cmints with PACKED instruction data
    // Client: pack pubkeys to indices. On-chain: unpack indices to pubkeys.
    // =========================================================================
    use csdk_anchor_full_derived_test::instruction_accounts::{
        PackedMintAccountData, PackedMintTokenData, PackedMintVariant,
    };
    use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
    use light_sdk::instruction::PackedAccounts;

    // CLIENT-SIDE VALIDATION: at most 1 mint
    if 1 > 1 {
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

    // Build PackedAccounts - client packs all pubkeys to indices
    let mut packed_accounts = PackedAccounts::default();

    // Insert system accounts at fixed indices 0-5
    let ctoken_program_idx =
        packed_accounts.insert_or_get_read_only(light_sdk_types::C_TOKEN_PROGRAM_ID.into());
    let light_system_idx =
        packed_accounts.insert_or_get_read_only(light_sdk_types::LIGHT_SYSTEM_PROGRAM_ID.into());
    let cpi_authority_idx =
        packed_accounts.insert_or_get_read_only(light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY);
    let registered_program_idx =
        packed_accounts.insert_or_get_read_only(light_sdk_types::REGISTERED_PROGRAM_PDA.into());
    let acc_compression_authority_idx = packed_accounts
        .insert_or_get_read_only(light_sdk_types::ACCOUNT_COMPRESSION_AUTHORITY_PDA.into());
    let acc_compression_program_idx = packed_accounts
        .insert_or_get_read_only(light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID.into());

    // Verify system accounts are at expected indices
    assert_eq!(ctoken_program_idx, 0);
    assert_eq!(light_system_idx, 1);
    assert_eq!(cpi_authority_idx, 2);
    assert_eq!(registered_program_idx, 3);
    assert_eq!(acc_compression_authority_idx, 4);
    assert_eq!(acc_compression_program_idx, 5);

    // Pack tree infos from validity proof (after system accounts)
    let packed_tree_infos = decompress_proof_result.pack_tree_infos(&mut packed_accounts);
    let packed_tree_info = &packed_tree_infos
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];

    // Add output queue
    let output_queue_pubkey = compressed_mint1
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(compressed_mint1.tree_info.queue);
    let output_state_tree_index = packed_accounts.insert_or_get(output_queue_pubkey);

    // Pack mint-specific pubkeys (only actual Solana accounts, not raw data)
    let mint_seed_index = packed_accounts.insert_or_get_read_only(mint_signer1.pubkey());
    let cmint_pda_index = packed_accounts.insert_or_get(cmint1_pda); // writable for decompression

    // Authority pubkeys - pack if present
    let has_mint_authority = mint1_data.base.mint_authority.is_some();
    let mint_authority_index = if let Some(auth) = mint1_data.base.mint_authority {
        packed_accounts.insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
    } else {
        0
    };
    let has_freeze_authority = mint1_data.base.freeze_authority.is_some();
    let freeze_authority_index = if let Some(auth) = mint1_data.base.freeze_authority {
        packed_accounts.insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
    } else {
        0
    };

    // Build packed mint data
    // Note: compressed_address is raw data (not a Solana account), kept as [u8; 32]
    let packed_mint_data = PackedMintTokenData {
        mint_seed_index,
        cmint_pda_index,
        compressed_address: cmint1_compressed_address, // raw data, not an index
        leaf_index: packed_tree_info.leaf_index,
        prove_by_index: packed_tree_info.prove_by_index,
        root_index: packed_tree_info.root_index,
        supply: mint1_data.base.supply,
        decimals: mint1_data.base.decimals,
        version: mint1_data.metadata.version,
        cmint_decompressed: mint1_data.metadata.cmint_decompressed,
        has_mint_authority,
        mint_authority_index,
        has_freeze_authority,
        freeze_authority_index,
        rent_payment: 2,
        write_top_up: 5000,
        extensions: None, // No extensions for this test
    };

    // Build compressed account data with packed struct types
    let compressed_accounts = vec![PackedMintAccountData {
        meta: CompressedAccountMetaNoLamportsNoAddress {
            tree_info: *packed_tree_info,
            output_state_tree_index,
        },
        data: PackedMintVariant::Standard(packed_mint_data),
    }];

    // Get remaining accounts from PackedAccounts (returns tuple, take first element)
    let (remaining_accounts, _, _) = packed_accounts.to_account_metas();

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

// ============================================================================
// Decompress Unified - Comprehensive E2E Tests
// ============================================================================

/// Helper to create a compressed mint using the existing CreateCMint SDK
async fn create_standalone_mint(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    authority: &Keypair,
    address_tree: Pubkey,
    output_queue: Pubkey,
) -> (Keypair, Pubkey, [u8; 32]) {
    use light_ctoken_sdk::ctoken::{CreateCMint, CreateCMintParams};

    let mint_signer = Keypair::new();
    let (cmint_pda, _) = find_cmint_address(&mint_signer.pubkey());
    let compressed_address = derive_cmint_compressed_address(&mint_signer.pubkey(), &address_tree);

    let proof_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compressed_address,
                tree: address_tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let params = CreateCMintParams {
        decimals: 9,
        address_merkle_tree_root_index: proof_result.addresses[0].root_index,
        mint_authority: authority.pubkey(),
        proof: proof_result.proof.0.unwrap(),
        compression_address: compressed_address,
        mint: cmint_pda,
        freeze_authority: None,
        extensions: None,
    };

    let ix = CreateCMint::new(
        params,
        mint_signer.pubkey(),
        payer.pubkey(),
        address_tree,
        output_queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[payer, &mint_signer, authority])
        .await
        .expect("Create mint should succeed");

    (mint_signer, cmint_pda, compressed_address)
}

/// Helper to build decompress_unified params for given mints and ATAs
async fn build_unified_decompress_params(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    authority: &Keypair,
    mints: &[(Keypair, Pubkey, [u8; 32])], // (signer, cmint_pda, compressed_address)
    atas: &[(Keypair, Pubkey, Pubkey, u64)], // (wallet, ata_pda, mint_pda, amount)
    program_id: Pubkey,
) -> Result<Instruction, String> {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        DecompressUnifiedAccountData, DecompressUnifiedParams, DecompressVariant,
        PackedAtaTokenData, PackedMintTokenData,
    };
    use light_ctoken_sdk::ctoken::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};
    use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

    // Collect all hashes for proof
    let mut hashes = Vec::new();
    let mut mint_compressed_data = Vec::new();
    let mut ata_compressed_data = Vec::new();

    for (_, _, compressed_address) in mints {
        let compressed = rpc
            .get_compressed_account(*compressed_address, None)
            .await
            .map_err(|e| format!("Failed to get compressed mint: {}", e))?
            .value
            .ok_or("Compressed mint not found")?;
        hashes.push(compressed.hash);
        mint_compressed_data.push(compressed);
    }

    for (_, ata_pda, _, expected_amount) in atas {
        let accounts = rpc
            .get_compressed_token_accounts_by_owner(ata_pda, None, None)
            .await
            .map_err(|e| format!("Failed to get compressed ATA: {}", e))?
            .value
            .items;
        if accounts.is_empty() {
            return Err(format!("No compressed ATA found for {}", ata_pda));
        }
        let compressed = accounts.into_iter().next().unwrap();
        if compressed.token.amount != *expected_amount {
            return Err(format!(
                "ATA amount mismatch: expected {}, got {}",
                expected_amount, compressed.token.amount
            ));
        }
        hashes.push(compressed.account.hash);
        ata_compressed_data.push(compressed);
    }

    // Get validity proof
    let proof_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .map_err(|e| format!("Failed to get validity proof: {}", e))?
        .value;

    // Build PackedAccounts
    let mut packed = PackedAccounts::default();

    // System accounts [0-5]
    packed.insert_or_get_read_only(light_sdk_types::C_TOKEN_PROGRAM_ID.into());
    packed.insert_or_get_read_only(light_sdk_types::LIGHT_SYSTEM_PROGRAM_ID.into());
    packed.insert_or_get_read_only(light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY);
    packed.insert_or_get_read_only(light_sdk_types::REGISTERED_PROGRAM_PDA.into());
    packed.insert_or_get_read_only(light_sdk_types::ACCOUNT_COMPRESSION_AUTHORITY_PDA.into());
    packed.insert_or_get_read_only(light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID.into());

    // CPI context if mixing types
    let has_mint = !mints.is_empty();
    let has_ata = !atas.is_empty();
    if has_mint && has_ata {
        // Need CPI context account - get from state tree info
        let state_tree_info = rpc.get_random_state_tree_info().unwrap();
        if let Some(cpi_ctx) = state_tree_info.cpi_context {
            packed.insert_or_get(cpi_ctx);
        }
    }

    // Pack tree infos
    let packed_tree_infos = proof_result.pack_tree_infos(&mut packed);

    // Build compressed_accounts vec
    let mut compressed_accounts = Vec::new();
    let mut proof_idx = 0;

    // Add mints
    for (i, (mint_signer, cmint_pda, compressed_address)) in mints.iter().enumerate() {
        let compressed = &mint_compressed_data[i];
        let mint_data: light_ctoken_interface::state::CompressedMint =
            borsh::BorshDeserialize::deserialize(&mut &compressed.data.as_ref().unwrap().data[..])
                .map_err(|e| format!("Failed to parse mint data: {}", e))?;

        let packed_tree_info = &packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos[proof_idx];

        let output_queue = compressed
            .tree_info
            .next_tree_info
            .as_ref()
            .map(|n| n.queue)
            .unwrap_or(compressed.tree_info.queue);
        let output_state_tree_index = packed.insert_or_get(output_queue);

        let mint_seed_index = packed.insert_or_get_read_only(mint_signer.pubkey());
        let cmint_pda_index = packed.insert_or_get(*cmint_pda);

        let has_mint_authority = mint_data.base.mint_authority.is_some();
        let mint_authority_index = if let Some(auth) = mint_data.base.mint_authority {
            packed.insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
        } else {
            0
        };
        let has_freeze_authority = mint_data.base.freeze_authority.is_some();
        let freeze_authority_index = if let Some(auth) = mint_data.base.freeze_authority {
            packed.insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
        } else {
            0
        };

        compressed_accounts.push(DecompressUnifiedAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info: *packed_tree_info,
                output_state_tree_index,
            },
            data: DecompressVariant::Mint(PackedMintTokenData {
                mint_seed_index,
                cmint_pda_index,
                compressed_address: *compressed_address,
                leaf_index: packed_tree_info.leaf_index,
                prove_by_index: packed_tree_info.prove_by_index,
                root_index: packed_tree_info.root_index,
                supply: mint_data.base.supply,
                decimals: mint_data.base.decimals,
                version: mint_data.metadata.version,
                cmint_decompressed: mint_data.metadata.cmint_decompressed,
                has_mint_authority,
                mint_authority_index,
                has_freeze_authority,
                freeze_authority_index,
                rent_payment: 2,
                write_top_up: 5000,
                extensions: None,
            }),
        });
        proof_idx += 1;
    }

    // Add ATAs
    for (i, (wallet, ata_pda, mint_pda, amount)) in atas.iter().enumerate() {
        let compressed = &ata_compressed_data[i];
        let packed_tree_info = &packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos[proof_idx];

        let output_queue = compressed
            .account
            .tree_info
            .next_tree_info
            .as_ref()
            .map(|n| n.queue)
            .unwrap_or(compressed.account.tree_info.queue);
        let output_state_tree_index = packed.insert_or_get(output_queue);

        let wallet_idx = packed.insert_or_get_config(wallet.pubkey(), true, false);
        let mint_idx = packed.insert_or_get_read_only(*mint_pda);
        let ata_idx = packed.insert_or_get(*ata_pda);

        compressed_accounts.push(DecompressUnifiedAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info: *packed_tree_info,
                output_state_tree_index,
            },
            data: DecompressVariant::Ata(PackedAtaTokenData {
                wallet_index: wallet_idx,
                mint_index: mint_idx,
                ata_index: ata_idx,
                amount: *amount,
                has_delegate: false,
                delegate_index: 0,
                is_frozen: false,
            }),
        });
        proof_idx += 1;
    }

    let params = DecompressUnifiedParams {
        proof: proof_result.proof,
        compressed_accounts,
        system_accounts_offset: 0,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::DecompressUnified {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        system_program: solana_sdk::system_program::ID,
    };

    let (remaining_accounts, _, _) = packed.to_account_metas();

    Ok(Instruction {
        program_id,
        accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
        data: csdk_anchor_full_derived_test::instruction::DecompressUnified { params }.data(),
    })
}

/// Shared context for unified decompression tests.
/// Mirrors create_pdas_and_mint_auto but stores all keypairs for later use.
struct UnifiedTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    authority: Keypair,
    mint_authority: Keypair,
    user1: Keypair,
    user2: Keypair,
    program_id: Pubkey,
    config_pda: Pubkey,
    mint_signer_pda: Pubkey,
    cmint_pda: Pubkey,
    mint_compressed_address: [u8; 32],
    user1_ata_pda: Pubkey,
    user2_ata_pda: Pubkey,
    user1_ata_amount: u64,
    user2_ata_amount: u64,
}

impl UnifiedTestContext {
    /// Setup and create all accounts via create_pdas_and_mint_auto
    async fn new() -> Self {
        use csdk_anchor_full_derived_test::instruction_accounts::{
            LP_MINT_SIGNER_SEED, VAULT_SEED,
        };
        use csdk_anchor_full_derived_test::FullAutoWithMintParams;
        use light_ctoken_sdk::ctoken::{
            get_associated_ctoken_address_and_bump, COMPRESSIBLE_CONFIG_V1,
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
        let authority = Keypair::new();
        let mint_authority = Keypair::new();
        let user1 = Keypair::new();
        let user2 = Keypair::new();

        let address_tree_pubkey = rpc.get_address_tree_v2().tree;
        let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
        let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

        // Initialize compression config
        let config_instruction =
            csdk_anchor_full_derived_test::instruction::InitializeCompressionConfig {
                rent_sponsor: RENT_SPONSOR,
                compression_authority: payer.pubkey(),
                rent_config: light_compressible::rent::RentConfig::default(),
                write_top_up: 5_000,
                address_space: vec![address_tree_pubkey],
            };
        let config_accounts =
            csdk_anchor_full_derived_test::accounts::InitializeCompressionConfig {
                payer: payer.pubkey(),
                config: config_pda,
                program_data: program_data_pda,
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

        // Setup PDAs
        let owner = payer.pubkey();
        let category_id = 999u64;
        let session_id = 888u64;
        let vault_mint_amount = 100u64;
        let user1_ata_amount = 50u64;
        let user2_ata_amount = 75u64;

        let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
            &[LP_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
            &program_id,
        );
        let (cmint_pda, _) = find_cmint_address(&mint_signer_pda);
        let (vault_pda, vault_bump) =
            Pubkey::find_program_address(&[VAULT_SEED, cmint_pda.as_ref()], &program_id);
        let (vault_authority_pda, _) =
            Pubkey::find_program_address(&[b"vault_authority"], &program_id);
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

        // Get validity proof
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

        // Create all accounts
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
                user1_ata_mint_amount: user1_ata_amount,
                user2_ata_bump,
                user2_ata_mint_amount: user2_ata_amount,
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
        .expect("create_pdas_and_mint_auto should succeed");

        Self {
            rpc,
            payer,
            authority,
            mint_authority,
            user1,
            user2,
            program_id,
            config_pda,
            mint_signer_pda,
            cmint_pda,
            mint_compressed_address,
            user1_ata_pda,
            user2_ata_pda,
            user1_ata_amount,
            user2_ata_amount,
        }
    }

    /// Warp forward to compress all accounts
    async fn warp_to_compress(&mut self) {
        self.rpc
            .warp_slot_forward(SLOTS_PER_EPOCH * 30)
            .await
            .unwrap();
    }

    /// Assert ATA is on-chain with expected amount
    async fn assert_ata_on_chain(&mut self, ata: &Pubkey, expected_amount: u64) {
        use light_ctoken_sdk::ctoken::CToken;
        let acc = self.rpc.get_account(*ata).await.unwrap();
        assert!(acc.is_some(), "ATA {} should be on-chain", ata);
        let ctoken: CToken =
            borsh::BorshDeserialize::deserialize(&mut &acc.unwrap().data[..]).unwrap();
        assert_eq!(
            ctoken.amount, expected_amount,
            "ATA amount mismatch for {}",
            ata
        );
    }

    /// Assert ATA is compressed (not on-chain)
    async fn assert_ata_compressed(&mut self, ata: &Pubkey) {
        let accounts = self
            .rpc
            .get_compressed_token_accounts_by_owner(ata, None, None)
            .await
            .unwrap()
            .value
            .items;
        assert!(!accounts.is_empty(), "ATA {} should be compressed", ata);
    }

    /// Assert ATA is consumed (no compressed account)
    async fn assert_ata_consumed(&mut self, ata: &Pubkey) {
        let accounts = self
            .rpc
            .get_compressed_token_accounts_by_owner(ata, None, None)
            .await
            .unwrap()
            .value
            .items;
        assert!(
            accounts.is_empty(),
            "ATA {} compressed account should be consumed",
            ata
        );
    }

    /// Assert mint is on-chain
    async fn assert_mint_on_chain(&mut self, mint: &Pubkey) {
        let acc = self.rpc.get_account(*mint).await.unwrap();
        assert!(acc.is_some(), "Mint {} should be on-chain", mint);
    }

    /// Assert mint is compressed (has data)
    async fn assert_mint_compressed(&mut self) {
        let compressed = self
            .rpc
            .get_compressed_account(self.mint_compressed_address, None)
            .await
            .unwrap()
            .value
            .unwrap();
        assert!(
            !compressed.data.as_ref().unwrap().data.is_empty(),
            "Mint should be compressed with data"
        );
    }
}

/// E2E test: decompress_unified with single ATA only
#[tokio::test]
async fn test_decompress_unified_ata_only() {
    let mut ctx = UnifiedTestContext::new().await;

    // Copy values needed for later
    let user1_ata_pda = ctx.user1_ata_pda;
    let cmint_pda = ctx.cmint_pda;
    let user1_ata_amount = ctx.user1_ata_amount;
    let program_id = ctx.program_id;

    // Warp to compress
    ctx.warp_to_compress().await;
    ctx.assert_ata_compressed(&user1_ata_pda).await;

    // Build and execute decompress_unified for user1 ATA only
    let ix = build_unified_decompress_params(
        &mut ctx.rpc,
        &ctx.payer,
        &ctx.mint_authority,
        &[], // No mints
        &[(
            ctx.user1.insecure_clone(),
            user1_ata_pda,
            cmint_pda,
            user1_ata_amount,
        )],
        program_id,
    )
    .await
    .expect("Build ATA-only params");

    // Note: authority must sign even for ATA-only (it's in the accounts struct)
    ctx.rpc
        .create_and_send_transaction(
            &[ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority, &ctx.user1],
        )
        .await
        .expect("decompress_unified (1 ATA) should succeed");

    // Verify
    ctx.assert_ata_on_chain(&user1_ata_pda, user1_ata_amount)
        .await;
    ctx.assert_ata_consumed(&user1_ata_pda).await;

    println!("test_decompress_unified_ata_only PASSED");
}

/// E2E test: decompress_unified with 2 ATAs
#[tokio::test]
async fn test_decompress_unified_two_atas() {
    let mut ctx = UnifiedTestContext::new().await;

    // Copy values needed for later
    let user1_ata_pda = ctx.user1_ata_pda;
    let user2_ata_pda = ctx.user2_ata_pda;
    let cmint_pda = ctx.cmint_pda;
    let user1_ata_amount = ctx.user1_ata_amount;
    let user2_ata_amount = ctx.user2_ata_amount;
    let program_id = ctx.program_id;

    // Warp to compress
    ctx.warp_to_compress().await;
    ctx.assert_ata_compressed(&user1_ata_pda).await;
    ctx.assert_ata_compressed(&user2_ata_pda).await;

    // Build and execute decompress_unified for BOTH ATAs
    let ix = build_unified_decompress_params(
        &mut ctx.rpc,
        &ctx.payer,
        &ctx.mint_authority,
        &[], // No mints
        &[
            (
                ctx.user1.insecure_clone(),
                user1_ata_pda,
                cmint_pda,
                user1_ata_amount,
            ),
            (
                ctx.user2.insecure_clone(),
                user2_ata_pda,
                cmint_pda,
                user2_ata_amount,
            ),
        ],
        program_id,
    )
    .await
    .expect("Build 2-ATAs params");

    // Note: authority must sign even for ATA-only (it's in the accounts struct)
    ctx.rpc
        .create_and_send_transaction(
            &[ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority, &ctx.user1, &ctx.user2],
        )
        .await
        .expect("decompress_unified (2 ATAs) should succeed");

    // Verify both ATAs
    ctx.assert_ata_on_chain(&user1_ata_pda, user1_ata_amount)
        .await;
    ctx.assert_ata_on_chain(&user2_ata_pda, user2_ata_amount)
        .await;
    ctx.assert_ata_consumed(&user1_ata_pda).await;
    ctx.assert_ata_consumed(&user2_ata_pda).await;

    println!("test_decompress_unified_two_atas PASSED");
}

/// E2E test: Verify mixed ATA + Mint fails due to ctoken limitation.
/// Both ATA decompress and Mint decompress modify on-chain state, so neither
/// can be in CPI context write mode. This makes batched ATA + Mint decompression
/// via CPI context impossible with the current protocol design.
#[tokio::test]
async fn test_decompress_unified_ata_and_mint_not_supported() {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        DecompressUnifiedAccountData, DecompressUnifiedParams, DecompressVariant,
        PackedAtaTokenData, PackedMintTokenData,
    };
    use light_ctoken_sdk::ctoken::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};
    use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

    let mut ctx = UnifiedTestContext::new().await;

    // Copy values needed for later
    let user1_ata_pda = ctx.user1_ata_pda;
    let cmint_pda = ctx.cmint_pda;
    let user1_ata_amount = ctx.user1_ata_amount;
    let program_id = ctx.program_id;
    let mint_signer_pda = ctx.mint_signer_pda;
    let mint_compressed_address = ctx.mint_compressed_address;

    // Warp to compress
    ctx.warp_to_compress().await;
    ctx.assert_ata_compressed(&user1_ata_pda).await;
    ctx.assert_mint_compressed().await;

    // Get compressed data for both
    let compressed_mint = ctx
        .rpc
        .get_compressed_account(mint_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let mint_data: light_ctoken_interface::state::CompressedMint =
        borsh::BorshDeserialize::deserialize(&mut &compressed_mint.data.as_ref().unwrap().data[..])
            .unwrap();

    let compressed_ata_accounts = ctx
        .rpc
        .get_compressed_token_accounts_by_owner(&user1_ata_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    let compressed_ata = &compressed_ata_accounts[0];

    // Get validity proof for both
    let proof_result = ctx
        .rpc
        .get_validity_proof(
            vec![compressed_mint.hash, compressed_ata.account.hash],
            vec![],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build PackedAccounts
    let mut packed = PackedAccounts::default();

    // System accounts [0-5]
    packed.insert_or_get_read_only(light_sdk_types::C_TOKEN_PROGRAM_ID.into());
    packed.insert_or_get_read_only(light_sdk_types::LIGHT_SYSTEM_PROGRAM_ID.into());
    packed.insert_or_get_read_only(light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY);
    packed.insert_or_get_read_only(light_sdk_types::REGISTERED_PROGRAM_PDA.into());
    packed.insert_or_get_read_only(light_sdk_types::ACCOUNT_COMPRESSION_AUTHORITY_PDA.into());
    packed.insert_or_get_read_only(light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID.into());

    // CPI context at index 6 (required for mixed types)
    let state_tree_info = ctx.rpc.get_random_state_tree_info().unwrap();
    let cpi_context_index = packed.insert_or_get(state_tree_info.cpi_context.unwrap());
    assert_eq!(cpi_context_index, 6, "CPI context should be at index 6");

    // Pack tree infos
    let packed_tree_infos = proof_result.pack_tree_infos(&mut packed);

    // Mint output queue
    let mint_output_queue = compressed_mint
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(compressed_mint.tree_info.queue);
    let mint_output_state_tree_index = packed.insert_or_get(mint_output_queue);

    // Mint-specific indices
    let mint_seed_index = packed.insert_or_get_read_only(mint_signer_pda);
    let cmint_pda_index = packed.insert_or_get(cmint_pda);

    let has_mint_authority = mint_data.base.mint_authority.is_some();
    let mint_authority_index = if let Some(auth) = mint_data.base.mint_authority {
        packed.insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
    } else {
        0
    };
    let has_freeze_authority = mint_data.base.freeze_authority.is_some();
    let freeze_authority_index = if let Some(auth) = mint_data.base.freeze_authority {
        packed.insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
    } else {
        0
    };

    // ATA output queue
    let ata_output_queue = compressed_ata
        .account
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(compressed_ata.account.tree_info.queue);
    let ata_output_state_tree_index = packed.insert_or_get(ata_output_queue);

    // ATA-specific indices
    let wallet_index = packed.insert_or_get_config(ctx.user1.pubkey(), true, false);
    let mint_idx_for_ata = packed.insert_or_get_read_only(cmint_pda);
    let ata_index = packed.insert_or_get(user1_ata_pda);

    // Build compressed_accounts vec - MINT FIRST for CPI context ordering
    let mint_tree_info = &packed_tree_infos
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];
    let ata_tree_info = &packed_tree_infos
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[1];

    let compressed_accounts = vec![
        // Mint first
        DecompressUnifiedAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info: *mint_tree_info,
                output_state_tree_index: mint_output_state_tree_index,
            },
            data: DecompressVariant::Mint(PackedMintTokenData {
                mint_seed_index,
                cmint_pda_index,
                compressed_address: mint_compressed_address,
                leaf_index: mint_tree_info.leaf_index,
                prove_by_index: mint_tree_info.prove_by_index,
                root_index: mint_tree_info.root_index,
                supply: mint_data.base.supply,
                decimals: mint_data.base.decimals,
                version: mint_data.metadata.version,
                cmint_decompressed: mint_data.metadata.cmint_decompressed,
                has_mint_authority,
                mint_authority_index,
                has_freeze_authority,
                freeze_authority_index,
                rent_payment: 2,
                write_top_up: 5000,
                extensions: None,
            }),
        },
        // ATA second
        DecompressUnifiedAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info: *ata_tree_info,
                output_state_tree_index: ata_output_state_tree_index,
            },
            data: DecompressVariant::Ata(PackedAtaTokenData {
                wallet_index,
                mint_index: mint_idx_for_ata,
                ata_index,
                amount: user1_ata_amount,
                has_delegate: false,
                delegate_index: 0,
                is_frozen: false,
            }),
        },
    ];

    let params = DecompressUnifiedParams {
        proof: proof_result.proof,
        compressed_accounts,
        system_accounts_offset: 0,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::DecompressUnified {
        fee_payer: ctx.payer.pubkey(),
        authority: ctx.mint_authority.pubkey(),
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        system_program: solana_sdk::system_program::ID,
    };

    let (remaining_accounts, _, _) = packed.to_account_metas();

    let ix = Instruction {
        program_id,
        accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
        data: csdk_anchor_full_derived_test::instruction::DecompressUnified { params }.data(),
    };

    // This SHOULD FAIL - both ATA decompress and Mint decompress modify on-chain state,
    // so neither can be in CPI context write mode. The ctoken program blocks:
    // - Transfer2 with compressions when writing to CPI context (error 18001)
    // - DecompressMint when writing to CPI context (error 6035)
    let result = ctx
        .rpc
        .create_and_send_transaction(
            &[ix],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.mint_authority, &ctx.user1],
        )
        .await;

    assert!(
        result.is_err(),
        "Mixed ATA + Mint should fail - both require on-chain state changes incompatible with CPI context write mode"
    );

    println!("test_decompress_unified_ata_and_mint_not_supported: Correctly rejected mixed types");
}

/// Test decompress_unified with mint-only scenario
/// Uses create_pdas_and_mint_auto to setup, then tests mint decompression via unified instruction
#[tokio::test]
async fn test_decompress_unified_mint_only() {
    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 2_000_000_000)
        .await
        .unwrap();

    let address_tree = rpc.get_address_tree_v2().tree;
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Create a mint
    println!("📦 Creating mint...");
    let (mint_signer, cmint_pda, compressed_address) = create_standalone_mint(
        &mut rpc,
        &payer,
        &authority,
        address_tree,
        state_tree_info.queue,
    )
    .await;

    // Warp to compress
    println!("⏰ Warping to compress...");
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // Verify mint is compressed
    let compressed = rpc
        .get_compressed_account(compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        !compressed.data.as_ref().unwrap().data.is_empty(),
        "Mint should be compressed"
    );

    // Test decompress_unified with mint only
    println!("\n🧪 Testing mint-only decompression via decompress_unified...");
    let ix = build_unified_decompress_params(
        &mut rpc,
        &payer,
        &authority,
        &[(mint_signer, cmint_pda, compressed_address)],
        &[],
        program_id,
    )
    .await
    .expect("Build params should succeed");

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("Decompress unified (mint-only) should succeed");

    // Verify mint is on-chain
    let acc = rpc.get_account(cmint_pda).await.unwrap();
    assert!(acc.is_some(), "CMint should be on-chain");

    println!("✅ decompress_unified mint-only: PASSED");
}

/// Test that passing 2 mints in decompress_unified fails with appropriate error
#[tokio::test]
async fn test_decompress_unified_two_mints_fails() {
    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 2_000_000_000)
        .await
        .unwrap();

    let address_tree = rpc.get_address_tree_v2().tree;
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Create 2 mints
    println!("📦 Creating 2 mints for error test...");
    let mint1 = create_standalone_mint(
        &mut rpc,
        &payer,
        &authority,
        address_tree,
        state_tree_info.queue,
    )
    .await;

    let mint2 = create_standalone_mint(
        &mut rpc,
        &payer,
        &authority,
        address_tree,
        state_tree_info.queue,
    )
    .await;

    // Warp to compress
    println!("⏰ Warping to compress...");
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // Try to decompress both mints - should fail
    println!("\n🧪 Testing 2 mints (should fail)...");
    let result = build_unified_decompress_params(
        &mut rpc,
        &payer,
        &authority,
        &[
            (mint1.0.insecure_clone(), mint1.1, mint1.2),
            (mint2.0.insecure_clone(), mint2.1, mint2.2),
        ],
        &[],
        program_id,
    )
    .await;

    match result {
        Ok(ix) => {
            let tx_result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &authority])
                .await;

            assert!(tx_result.is_err(), "Transaction with 2 mints should fail");
        }
        Err(_) => {
            assert!(true);
        }
    }
}

/// E2E test: cPDA + cToken vault decompression works together.
/// cPDAs only modify compressed state (can write to CPI context), then vault executes.
/// Uses vault (program-owned cToken) instead of user ATA because vault doesn't require wallet signing.
#[tokio::test]
async fn test_decompress_cpda_and_vault() {
    use anchor_lang::AnchorDeserialize;
    use csdk_anchor_full_derived_test::instruction_accounts::VAULT_SEED;
    use csdk_anchor_full_derived_test::{
        CTokenAccountVariant, CompressedAccountVariant, UserRecord,
    };
    use light_compressible_client::compressible_instruction;
    use light_ctoken_sdk::ctoken::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

    // Use UnifiedTestContext which creates PDAs + CMint + vault + ATAs
    let mut ctx = UnifiedTestContext::new().await;

    // Calculate UserRecord PDA
    let user_record_pda = {
        let (pda, _) = Pubkey::find_program_address(
            &[
                b"user_record",
                ctx.authority.pubkey().as_ref(),
                ctx.mint_authority.pubkey().as_ref(),
                ctx.payer.pubkey().as_ref(),
                999u64.to_le_bytes().as_ref(),
            ],
            &ctx.program_id,
        );
        pda
    };

    // Calculate vault PDA
    let vault_pda = {
        let (pda, _) =
            Pubkey::find_program_address(&[VAULT_SEED, ctx.cmint_pda.as_ref()], &ctx.program_id);
        pda
    };

    let address_tree_pubkey = ctx.rpc.get_address_tree_v2().tree;
    let user_compressed_address = light_compressed_account::address::derive_address(
        &user_record_pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &ctx.program_id.to_bytes(),
    );

    // Copy values before warp
    let cmint_pda = ctx.cmint_pda;
    let config_pda = ctx.config_pda;
    let program_id = ctx.program_id;
    let vault_mint_amount = 100u64; // matches UnifiedTestContext

    // Warp to compress
    ctx.warp_to_compress().await;

    // Verify both are compressed
    let compressed_user = ctx
        .rpc
        .get_compressed_account(user_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert!(
        !compressed_user.data.as_ref().unwrap().data.is_empty(),
        "UserRecord should be compressed"
    );

    let compressed_vault_accounts = ctx
        .rpc
        .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !compressed_vault_accounts.is_empty(),
        "Vault should be compressed"
    );
    let compressed_vault = &compressed_vault_accounts[0];

    // Decompress both together via decompress_accounts_idempotent
    let c_user_record =
        UserRecord::deserialize(&mut &compressed_user.data.as_ref().unwrap().data[..]).unwrap();

    let vault_ctoken_data = light_ctoken_sdk::compat::CTokenData {
        variant: CTokenAccountVariant::Vault,
        token_data: compressed_vault.token.clone(),
    };

    let decompress_proof = ctx
        .rpc
        .get_validity_proof(
            vec![compressed_user.hash, compressed_vault.account.hash],
            vec![],
            None,
        )
        .await
        .unwrap()
        .value;

    let decompress_instruction = compressible_instruction::decompress_accounts_idempotent(
        &program_id,
        &compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[user_record_pda, vault_pda],
        &[
            (
                compressed_user.clone(),
                CompressedAccountVariant::UserRecord(c_user_record),
            ),
            (
                compressed_vault.account.clone(),
                CompressedAccountVariant::CTokenData(vault_ctoken_data),
            ),
        ],
        &csdk_anchor_full_derived_test::accounts::DecompressAccountsIdempotent {
            fee_payer: ctx.payer.pubkey(),
            config: config_pda,
            rent_sponsor: ctx.payer.pubkey(),
            ctoken_rent_sponsor: Some(CTOKEN_RENT_SPONSOR),
            ctoken_config: Some(COMPRESSIBLE_CONFIG_V1),
            ctoken_program: Some(C_TOKEN_PROGRAM_ID.into()),
            ctoken_cpi_authority: Some(light_ctoken_sdk::ctoken::CTOKEN_CPI_AUTHORITY),
            cmint_authority: None,
            authority: Some(ctx.authority.pubkey()),
            some_account: None,
            mint_authority: Some(ctx.mint_authority.pubkey()),
            user: Some(ctx.payer.pubkey()),
            mint: None,
            cmint: Some(cmint_pda),
        }
        .to_account_metas(None),
        decompress_proof,
    )
    .unwrap();

    // Add SeedParams
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::SeedParams;
    let seed_params = SeedParams {
        owner: ctx.payer.pubkey(),
        category_id: 999,
        session_id: 888,
        placeholder_id: 0,
        counter: 0,
    };
    let mut final_ix = decompress_instruction;
    final_ix
        .data
        .extend_from_slice(&borsh::to_vec(&seed_params).unwrap());

    ctx.rpc
        .create_and_send_transaction(&[final_ix], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("cPDA + vault decompression should succeed");

    // Verify both are on-chain
    let user_account = ctx.rpc.get_account(user_record_pda).await.unwrap();
    assert!(user_account.is_some(), "UserRecord should be on-chain");

    let vault_account = ctx.rpc.get_account(vault_pda).await.unwrap();
    assert!(vault_account.is_some(), "Vault should be on-chain");

    // Verify vault balance
    use light_ctoken_sdk::ctoken::CToken;
    let vault_data: CToken =
        borsh::BorshDeserialize::deserialize(&mut &vault_account.unwrap().data[..]).unwrap();
    assert_eq!(vault_data.amount, vault_mint_amount);

    println!("test_decompress_cpda_and_vault: SUCCESS");
}
