/// Stress test: 20-iteration compression/decompression cycles for all account types.
///
/// Each iteration randomly selects a subset of accounts to decompress, leaving the rest
/// compressed. Tests that hot/cold accounts coexist correctly across repeated cycles.
mod shared;

use light_account::LightDiscriminator;
use light_account_pinocchio::token::TokenDataWithSeeds;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterface, AccountSpec,
    CreateAccountsProofInput, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use light_token_interface::state::{token::Token, Mint};
use pinocchio_light_program_test::{
    all::accounts::CreateAllParams, discriminators, LightAccountVariant, MinimalRecord,
    MinimalRecordSeeds, OneByteRecord, OneByteRecordSeeds, VaultSeeds, ZeroCopyRecord,
    ZeroCopyRecordSeeds, MINT_SIGNER_SEED_A, RECORD_SEED, VAULT_AUTH_SEED, VAULT_SEED,
};
use rand::{seq::SliceRandom, thread_rng, Rng};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Which accounts are hot (decompressed / on-chain) this iteration.
#[derive(Debug, Clone)]
struct HotSet {
    record: bool,
    zc_record: bool,
    one_byte: bool,
    /// Mint must be true whenever ata or vault is true.
    mint: bool,
    ata: bool,
    vault: bool,
}

impl HotSet {
    /// Random subset. Ensures Mint is hot when ATA or Vault is, and at least
    /// one account is always hot.
    fn random(rng: &mut impl Rng) -> Self {
        let ata = rng.gen_bool(0.7);
        let vault = rng.gen_bool(0.7);
        // Mint must precede ATA/Vault, so force it hot when either is selected.
        let mint = ata || vault || rng.gen_bool(0.7);
        let mut hot = Self {
            record: rng.gen_bool(0.7),
            zc_record: rng.gen_bool(0.7),
            one_byte: rng.gen_bool(0.7),
            mint,
            ata,
            vault,
        };
        // Guarantee at least one account is hot.
        if !hot.record && !hot.zc_record && !hot.one_byte && !hot.mint {
            hot.record = true;
        }
        hot
    }
}

/// Stores all derived PDAs.
struct TestPdas {
    record: Pubkey,
    zc_record: Pubkey,
    one_byte: Pubkey,
    ata: Pubkey,
    ata_owner: Pubkey,
    vault: Pubkey,
    mint: Pubkey,
}

/// Cached state for accounts that go through the compress/decompress cycle.
#[derive(Clone)]
struct CachedState {
    record: MinimalRecord,
    zc_record: ZeroCopyRecord,
    ob_record: OneByteRecord,
    ata_token: Token,
    vault_token: Token,
    owner: [u8; 32],
}

/// Test context.
struct StressTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    config_pda: Pubkey,
    program_id: Pubkey,
}

fn parse_token(data: &[u8]) -> Token {
    borsh::BorshDeserialize::deserialize(&mut &data[..]).unwrap()
}

/// Setup environment with larger queues for stress test.
async fn setup() -> (StressTestContext, TestPdas) {
    let program_id = Pubkey::new_from_array(pinocchio_light_program_test::ID);
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("pinocchio_light_program_test", program_id)]),
    )
    .with_light_protocol_events();
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::e2e_test_default());
    config.v2_address_tree_config =
        Some(InitAddressTreeAccountsInstructionData::e2e_test_default());

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = light_account::derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = light_client::interface::InitializeRentFreeConfig::new(
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

    let owner = Keypair::new().pubkey();
    let authority = Keypair::new();

    // Derive all PDAs
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"minimal_record", owner.as_ref()], &program_id);
    let (zc_record_pda, _) =
        Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);
    let (one_byte_pda, _) =
        Pubkey::find_program_address(&[b"one_byte_record", owner.as_ref()], &program_id);

    let (mint_signer, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_pda, _) = light_token::instruction::find_mint_address(&mint_signer);

    let (vault_owner, _) = Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint_pda.as_ref()], &program_id);

    let ata_owner = payer.pubkey();
    let ata = light_token::instruction::derive_token_ata(&ata_owner, &mint_pda);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(record_pda),
            CreateAccountsProofInput::pda(zc_record_pda),
            CreateAccountsProofInput::pda(one_byte_pda),
            CreateAccountsProofInput::mint(mint_signer),
        ],
    )
    .await
    .unwrap();

    let params = CreateAllParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        owner: owner.to_bytes(),
        mint_signer_bump,
        token_vault_bump: vault_bump,
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(record_pda, false),
        AccountMeta::new(zc_record_pda, false),
        AccountMeta::new(one_byte_pda, false),
        AccountMeta::new_readonly(mint_signer, false),
        AccountMeta::new(mint_pda, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(vault_owner, false),
        AccountMeta::new_readonly(ata_owner, false),
        AccountMeta::new(ata, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_CONFIG, false),
        AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(light_token_types::CPI_AUTHORITY_PDA.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_ALL, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateAll should succeed");

    let pdas = TestPdas {
        record: record_pda,
        zc_record: zc_record_pda,
        one_byte: one_byte_pda,
        ata,
        ata_owner,
        vault,
        mint: mint_pda,
    };

    let ctx = StressTestContext {
        rpc,
        payer,
        config_pda,
        program_id,
    };

    (ctx, pdas)
}

/// Read on-chain state for all accounts in `hot`, keep old values for the rest.
async fn refresh_cache_partial(
    rpc: &mut LightProgramTest,
    pdas: &TestPdas,
    hot: &HotSet,
    old: &CachedState,
) -> CachedState {
    let record = if hot.record {
        let data = rpc.get_account(pdas.record).await.unwrap().unwrap().data;
        borsh::BorshDeserialize::deserialize(&mut &data[8..]).unwrap()
    } else {
        old.record.clone()
    };

    let zc_record = if hot.zc_record {
        let data = rpc.get_account(pdas.zc_record).await.unwrap().unwrap().data;
        *bytemuck::from_bytes(&data[8..])
    } else {
        old.zc_record
    };

    let ob_record = if hot.one_byte {
        let data = rpc.get_account(pdas.one_byte).await.unwrap().unwrap().data;
        let disc_len = OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
        borsh::BorshDeserialize::deserialize(&mut &data[disc_len..]).unwrap()
    } else {
        old.ob_record.clone()
    };

    let ata_token = if hot.ata {
        parse_token(&rpc.get_account(pdas.ata).await.unwrap().unwrap().data)
    } else {
        old.ata_token.clone()
    };

    let vault_token = if hot.vault {
        parse_token(&rpc.get_account(pdas.vault).await.unwrap().unwrap().data)
    } else {
        old.vault_token.clone()
    };

    CachedState {
        record,
        zc_record,
        ob_record,
        ata_token,
        vault_token,
        owner: old.owner,
    }
}

/// Decompress only the accounts listed in `hot`. Mint is always placed first in the
/// specs vec; everything else is shuffled.
async fn decompress_subset(
    ctx: &mut StressTestContext,
    pdas: &TestPdas,
    cached: &CachedState,
    hot: &HotSet,
) {
    let mut specs: Vec<AccountSpec<LightAccountVariant>> = Vec::new();

    // Mint first (ATA and Vault depend on it).
    if hot.mint {
        let mint_iface = ctx
            .rpc
            .get_mint_interface(&pdas.mint, None)
            .await
            .expect("failed to get mint interface")
            .value
            .expect("mint interface should exist");
        assert!(mint_iface.is_cold(), "Mint should be cold");
        specs.push(AccountSpec::Mint(AccountInterface::from(mint_iface)));
    }

    // Remaining specs, shuffled.
    let mut rest: Vec<AccountSpec<LightAccountVariant>> = Vec::new();

    if hot.record {
        let iface = ctx
            .rpc
            .get_account_interface(&pdas.record, None)
            .await
            .expect("failed to get MinimalRecord interface")
            .value
            .expect("MinimalRecord interface should exist");
        assert!(iface.is_cold(), "MinimalRecord should be cold");
        let data: MinimalRecord =
            borsh::BorshDeserialize::deserialize(&mut &iface.account.data[8..])
                .expect("Failed to parse MinimalRecord");
        let variant = LightAccountVariant::MinimalRecord {
            seeds: MinimalRecordSeeds {
                owner: cached.owner,
            },
            data,
        };
        rest.push(AccountSpec::Pda(PdaSpec::new(
            iface,
            variant,
            ctx.program_id,
        )));
    }

    if hot.zc_record {
        let iface = ctx
            .rpc
            .get_account_interface(&pdas.zc_record, None)
            .await
            .expect("failed to get ZeroCopyRecord interface")
            .value
            .expect("ZeroCopyRecord interface should exist");
        assert!(iface.is_cold(), "ZeroCopyRecord should be cold");
        let data: ZeroCopyRecord =
            borsh::BorshDeserialize::deserialize(&mut &iface.account.data[8..])
                .expect("Failed to parse ZeroCopyRecord");
        let variant = LightAccountVariant::ZeroCopyRecord {
            seeds: ZeroCopyRecordSeeds {
                owner: cached.owner,
            },
            data,
        };
        rest.push(AccountSpec::Pda(PdaSpec::new(
            iface,
            variant,
            ctx.program_id,
        )));
    }

    if hot.one_byte {
        let iface = ctx
            .rpc
            .get_account_interface(&pdas.one_byte, None)
            .await
            .expect("failed to get OneByteRecord interface")
            .value
            .expect("OneByteRecord interface should exist");
        assert!(iface.is_cold(), "OneByteRecord should be cold");
        let data: OneByteRecord =
            borsh::BorshDeserialize::deserialize(&mut &iface.account.data[8..])
                .expect("Failed to parse OneByteRecord");
        let variant = LightAccountVariant::OneByteRecord {
            seeds: OneByteRecordSeeds {
                owner: cached.owner,
            },
            data,
        };
        rest.push(AccountSpec::Pda(PdaSpec::new(
            iface,
            variant,
            ctx.program_id,
        )));
    }

    if hot.ata {
        let iface = ctx
            .rpc
            .get_associated_token_account_interface(&pdas.ata_owner, &pdas.mint, None)
            .await
            .expect("failed to get ATA interface")
            .value
            .expect("ATA interface should exist");
        assert!(iface.is_cold(), "ATA should be cold");
        rest.push(AccountSpec::Ata(Box::new(iface)));
    }

    if hot.vault {
        let iface = ctx
            .rpc
            .get_token_account_interface(&pdas.vault, None)
            .await
            .expect("failed to get vault interface")
            .value
            .expect("vault interface should exist");
        assert!(iface.is_cold(), "Vault should be cold");
        let token_data: Token = borsh::BorshDeserialize::deserialize(&mut &iface.account.data[..])
            .expect("Failed to parse vault Token");
        let variant = LightAccountVariant::Vault(TokenDataWithSeeds {
            seeds: VaultSeeds {
                mint: pdas.mint.to_bytes(),
            },
            token_data,
        });
        let compressed = iface
            .compressed()
            .expect("cold vault must have compressed data");
        let vault_interface = AccountInterface {
            key: iface.key,
            account: iface.account.clone(),
            cold: Some(compressed.account.clone()),
        };
        rest.push(AccountSpec::Pda(PdaSpec::new(
            vault_interface,
            variant,
            ctx.program_id,
        )));
    }

    rest.shuffle(&mut thread_rng());
    specs.extend(rest);

    if specs.is_empty() {
        return;
    }

    let ixs = create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
        .await
        .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&ixs, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // Assert hot accounts are now on-chain.
    for (flag, pda, name) in [
        (hot.record, &pdas.record, "MinimalRecord"),
        (hot.zc_record, &pdas.zc_record, "ZeroCopyRecord"),
        (hot.one_byte, &pdas.one_byte, "OneByteRecord"),
        (hot.mint, &pdas.mint, "Mint"),
        (hot.ata, &pdas.ata, "ATA"),
        (hot.vault, &pdas.vault, "Vault"),
    ] {
        if flag {
            shared::assert_onchain_exists(&mut ctx.rpc, pda, name).await;
        }
    }
}

/// Compress all accounts by warping forward. Everything goes cold regardless of what was hot.
async fn compress_all(ctx: &mut StressTestContext, pdas: &TestPdas) {
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 100)
        .await
        .unwrap();

    for (pda, name) in [
        (&pdas.record, "MinimalRecord"),
        (&pdas.zc_record, "ZeroCopyRecord"),
        (&pdas.one_byte, "OneByteRecord"),
        (&pdas.ata, "ATA"),
        (&pdas.vault, "Vault"),
        (&pdas.mint, "Mint"),
    ] {
        shared::assert_onchain_closed(&mut ctx.rpc, pda, name).await;
    }
}

/// Assert on-chain state only for accounts in `hot`.
async fn assert_hot_state(
    rpc: &mut LightProgramTest,
    pdas: &TestPdas,
    cached: &CachedState,
    hot: &HotSet,
    iteration: usize,
) {
    if hot.record {
        let account = rpc.get_account(pdas.record).await.unwrap().unwrap();
        let actual: MinimalRecord =
            borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
        let expected = MinimalRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            ..cached.record.clone()
        };
        assert_eq!(
            actual, expected,
            "MinimalRecord mismatch at iteration {iteration}"
        );
    }

    if hot.zc_record {
        let account = rpc.get_account(pdas.zc_record).await.unwrap().unwrap();
        let actual: &ZeroCopyRecord = bytemuck::from_bytes(&account.data[8..]);
        let expected = ZeroCopyRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            ..cached.zc_record
        };
        assert_eq!(
            *actual, expected,
            "ZeroCopyRecord mismatch at iteration {iteration}"
        );
    }

    if hot.one_byte {
        let account = rpc.get_account(pdas.one_byte).await.unwrap().unwrap();
        let disc_len = OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
        let actual: OneByteRecord =
            borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
        let expected = OneByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            ..cached.ob_record.clone()
        };
        assert_eq!(
            actual, expected,
            "OneByteRecord mismatch at iteration {iteration}"
        );
    }

    if hot.ata {
        let actual = parse_token(&rpc.get_account(pdas.ata).await.unwrap().unwrap().data);
        let expected = Token {
            extensions: actual.extensions.clone(),
            ..cached.ata_token.clone()
        };
        assert_eq!(actual, expected, "ATA mismatch at iteration {iteration}");
    }

    if hot.vault {
        let actual = parse_token(&rpc.get_account(pdas.vault).await.unwrap().unwrap().data);
        let expected = Token {
            extensions: actual.extensions.clone(),
            ..cached.vault_token.clone()
        };
        assert_eq!(actual, expected, "Vault mismatch at iteration {iteration}");
    }

    if hot.mint {
        let actual: Mint = borsh::BorshDeserialize::deserialize(
            &mut &rpc.get_account(pdas.mint).await.unwrap().unwrap().data[..],
        )
        .unwrap();
        assert_eq!(
            actual.base.decimals, 9,
            "Mint decimals mismatch at iteration {iteration}"
        );
    }
}

#[tokio::test]
async fn test_stress_20_iterations() {
    let (mut ctx, pdas) = setup().await;

    // Verify initial creation
    for (pda, name) in [
        (&pdas.record, "MinimalRecord"),
        (&pdas.zc_record, "ZeroCopyRecord"),
        (&pdas.one_byte, "OneByteRecord"),
        (&pdas.ata, "ATA"),
        (&pdas.vault, "Vault"),
        (&pdas.mint, "Mint"),
    ] {
        shared::assert_onchain_exists(&mut ctx.rpc, pda, name).await;
    }

    // Read initial state — all accounts are on-chain right after creation.
    let record_data = ctx
        .rpc
        .get_account(pdas.record)
        .await
        .unwrap()
        .unwrap()
        .data;
    let owner: [u8; 32] = {
        let r: MinimalRecord =
            borsh::BorshDeserialize::deserialize(&mut &record_data[8..]).unwrap();
        r.owner
    };
    let zc_data = ctx
        .rpc
        .get_account(pdas.zc_record)
        .await
        .unwrap()
        .unwrap()
        .data;
    let ob_data = ctx
        .rpc
        .get_account(pdas.one_byte)
        .await
        .unwrap()
        .unwrap()
        .data;
    let disc_len = OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    let mut cached = CachedState {
        record: borsh::BorshDeserialize::deserialize(&mut &record_data[8..]).unwrap(),
        zc_record: *bytemuck::from_bytes(&zc_data[8..]),
        ob_record: borsh::BorshDeserialize::deserialize(&mut &ob_data[disc_len..]).unwrap(),
        ata_token: parse_token(&ctx.rpc.get_account(pdas.ata).await.unwrap().unwrap().data),
        vault_token: parse_token(&ctx.rpc.get_account(pdas.vault).await.unwrap().unwrap().data),
        owner,
    };

    // First compression — all accounts go cold.
    compress_all(&mut ctx, &pdas).await;

    let mut rng = thread_rng();

    for i in 0..20 {
        let hot = HotSet::random(&mut rng);
        println!("--- Iteration {i}: hot={hot:?} ---");

        decompress_subset(&mut ctx, &pdas, &cached, &hot).await;
        assert_hot_state(&mut ctx.rpc, &pdas, &cached, &hot, i).await;

        // Update cache only for accounts that were decompressed this iteration.
        cached = refresh_cache_partial(&mut ctx.rpc, &pdas, &hot, &cached).await;

        compress_all(&mut ctx, &pdas).await;

        println!("  iteration {i} complete");
    }

    println!("All 20 iterations completed successfully.");
}
