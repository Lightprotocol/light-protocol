/// Stress test: 20-iteration compression/decompression cycles for all account types.
///
/// Tests repeated cycles of:
/// 1. Decompress all accounts
/// 2. Assert cached state matches on-chain state
/// 3. Update cache from on-chain state
/// 4. Compress all accounts (warp forward)
mod shared;

use light_account_pinocchio::token::TokenDataWithSeeds;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterface, AccountSpec,
    ColdContext, CreateAccountsProofInput, PdaSpec,
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
    MinimalRecordSeeds, VaultSeeds, ZeroCopyRecord, ZeroCopyRecordSeeds, MINT_SIGNER_SEED_A,
    RECORD_SEED, VAULT_AUTH_SEED, VAULT_SEED,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Stores all derived PDAs
#[allow(dead_code)]
struct TestPdas {
    record: Pubkey,
    zc_record: Pubkey,
    ata: Pubkey,
    ata_owner: Pubkey,
    vault: Pubkey,
    vault_owner: Pubkey,
    mint: Pubkey,
}

/// Cached state for accounts that go through the compress/decompress cycle.
#[derive(Clone)]
struct CachedState {
    record: MinimalRecord,
    zc_record: ZeroCopyRecord,
    ata_token: Token,
    vault_token: Token,
    owner: [u8; 32],
}

/// Test context
struct StressTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    config_pda: Pubkey,
    program_id: Pubkey,
}

fn parse_token(data: &[u8]) -> Token {
    borsh::BorshDeserialize::deserialize(&mut &data[..]).unwrap()
}

/// Setup environment with larger queues for stress test
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

    // Mint signer PDA
    let (mint_signer, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_pda, _) = light_token::instruction::find_mint_address(&mint_signer);

    // Token vault PDA (uses the mint we're creating)
    let (vault_owner, _) = Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint_pda.as_ref()], &program_id);

    // ATA (uses the mint we're creating)
    let ata_owner = payer.pubkey();
    let (ata, _) = light_token::instruction::derive_token_ata(&ata_owner, &mint_pda);

    // Create all accounts in one instruction
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(record_pda),
            CreateAccountsProofInput::pda(zc_record_pda),
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

    // Account order per all/accounts.rs
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(record_pda, false),
        AccountMeta::new(zc_record_pda, false),
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
        ata,
        ata_owner,
        vault,
        vault_owner,
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

/// Re-read all on-chain accounts into the cache
async fn refresh_cache(
    rpc: &mut LightProgramTest,
    pdas: &TestPdas,
    owner: [u8; 32],
) -> CachedState {
    let record_account = rpc.get_account(pdas.record).await.unwrap().unwrap();
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..]).unwrap();

    let zc_account = rpc.get_account(pdas.zc_record).await.unwrap().unwrap();
    let zc_record: ZeroCopyRecord = *bytemuck::from_bytes(&zc_account.data[8..]);

    let ata_token = parse_token(&rpc.get_account(pdas.ata).await.unwrap().unwrap().data);
    let vault_token = parse_token(&rpc.get_account(pdas.vault).await.unwrap().unwrap().data);

    CachedState {
        record,
        zc_record,
        ata_token,
        vault_token,
        owner,
    }
}

/// Decompress all accounts
async fn decompress_all(ctx: &mut StressTestContext, pdas: &TestPdas, cached: &CachedState) {
    // PDA: MinimalRecord
    let record_interface = ctx
        .rpc
        .get_account_interface(&pdas.record, None)
        .await
        .expect("failed to get MinimalRecord interface")
        .value
        .expect("MinimalRecord interface should exist");
    assert!(record_interface.is_cold(), "MinimalRecord should be cold");

    let record_data: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_interface.account.data[8..])
            .expect("Failed to parse MinimalRecord");
    let record_variant = LightAccountVariant::MinimalRecord {
        seeds: MinimalRecordSeeds {
            owner: cached.owner,
        },
        data: record_data,
    };
    let record_spec = PdaSpec::new(record_interface, record_variant, ctx.program_id);

    // PDA: ZeroCopyRecord
    let zc_interface = ctx
        .rpc
        .get_account_interface(&pdas.zc_record, None)
        .await
        .expect("failed to get ZeroCopyRecord interface")
        .value
        .expect("ZeroCopyRecord interface should exist");
    assert!(zc_interface.is_cold(), "ZeroCopyRecord should be cold");

    let zc_data: ZeroCopyRecord =
        borsh::BorshDeserialize::deserialize(&mut &zc_interface.account.data[8..])
            .expect("Failed to parse ZeroCopyRecord");
    let zc_variant = LightAccountVariant::ZeroCopyRecord {
        seeds: ZeroCopyRecordSeeds {
            owner: cached.owner,
        },
        data: zc_data,
    };
    let zc_spec = PdaSpec::new(zc_interface, zc_variant, ctx.program_id);

    // ATA
    let ata_interface = ctx
        .rpc
        .get_associated_token_account_interface(&pdas.ata_owner, &pdas.mint, None)
        .await
        .expect("failed to get ATA interface")
        .value
        .expect("ATA interface should exist");
    assert!(ata_interface.is_cold(), "ATA should be cold");

    // Token PDA: Vault
    let vault_iface = ctx
        .rpc
        .get_token_account_interface(&pdas.vault, None)
        .await
        .expect("failed to get vault interface")
        .value
        .expect("vault interface should exist");
    assert!(vault_iface.is_cold(), "Vault should be cold");

    let vault_token_data: Token =
        borsh::BorshDeserialize::deserialize(&mut &vault_iface.account.data[..])
            .expect("Failed to parse vault Token");
    let vault_variant = LightAccountVariant::Vault(TokenDataWithSeeds {
        seeds: VaultSeeds {
            mint: pdas.mint.to_bytes(),
        },
        token_data: vault_token_data,
    });
    let vault_compressed = vault_iface
        .compressed()
        .expect("cold vault must have compressed data");
    let vault_interface = AccountInterface {
        key: vault_iface.key,
        account: vault_iface.account.clone(),
        cold: Some(ColdContext::Account(vault_compressed.account.clone())),
    };
    let vault_spec = PdaSpec::new(vault_interface, vault_variant, ctx.program_id);

    // Mint
    let mint_iface = ctx
        .rpc
        .get_mint_interface(&pdas.mint, None)
        .await
        .expect("failed to get mint interface")
        .value
        .expect("mint interface should exist");
    assert!(mint_iface.is_cold(), "Mint should be cold");
    let mint_ai = AccountInterface::from(mint_iface);

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![
        AccountSpec::Pda(record_spec),
        AccountSpec::Pda(zc_spec),
        AccountSpec::Ata(ata_interface),
        AccountSpec::Pda(vault_spec),
        AccountSpec::Mint(mint_ai),
    ];

    let decompress_ixs =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_ixs, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // Verify all decompressed accounts exist on-chain
    for (pda, name) in [
        (&pdas.record, "MinimalRecord"),
        (&pdas.zc_record, "ZeroCopyRecord"),
        (&pdas.ata, "ATA"),
        (&pdas.vault, "Vault"),
        (&pdas.mint, "Mint"),
    ] {
        shared::assert_onchain_exists(&mut ctx.rpc, pda, name).await;
    }
}

/// Compress all accounts by warping forward epochs
async fn compress_all(ctx: &mut StressTestContext, pdas: &TestPdas) {
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 100)
        .await
        .unwrap();

    for (pda, name) in [
        (&pdas.record, "MinimalRecord"),
        (&pdas.zc_record, "ZeroCopyRecord"),
        (&pdas.ata, "ATA"),
        (&pdas.vault, "Vault"),
        (&pdas.mint, "Mint"),
    ] {
        shared::assert_onchain_closed(&mut ctx.rpc, pda, name).await;
    }
}

/// Full-struct assertions for all accounts against cached state
async fn assert_all_state(
    rpc: &mut LightProgramTest,
    pdas: &TestPdas,
    cached: &CachedState,
    iteration: usize,
) {
    // MinimalRecord
    let account = rpc.get_account(pdas.record).await.unwrap().unwrap();
    let actual_record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    let expected_record = MinimalRecord {
        compression_info: shared::expected_compression_info(&actual_record.compression_info),
        ..cached.record.clone()
    };
    assert_eq!(
        actual_record, expected_record,
        "MinimalRecord mismatch at iteration {iteration}"
    );

    // ZeroCopyRecord
    let account = rpc.get_account(pdas.zc_record).await.unwrap().unwrap();
    let actual_zc: &ZeroCopyRecord = bytemuck::from_bytes(&account.data[8..]);
    let expected_zc = ZeroCopyRecord {
        compression_info: shared::expected_compression_info(&actual_zc.compression_info),
        ..cached.zc_record
    };
    assert_eq!(
        *actual_zc, expected_zc,
        "ZeroCopyRecord mismatch at iteration {iteration}"
    );

    // ATA
    let actual_ata = parse_token(&rpc.get_account(pdas.ata).await.unwrap().unwrap().data);
    let expected_ata = Token {
        extensions: actual_ata.extensions.clone(),
        ..cached.ata_token.clone()
    };
    assert_eq!(
        actual_ata, expected_ata,
        "ATA mismatch at iteration {iteration}"
    );

    // Vault
    let actual_vault = parse_token(&rpc.get_account(pdas.vault).await.unwrap().unwrap().data);
    let expected_vault = Token {
        extensions: actual_vault.extensions.clone(),
        ..cached.vault_token.clone()
    };
    assert_eq!(
        actual_vault, expected_vault,
        "Vault mismatch at iteration {iteration}"
    );

    // Mint
    let actual_mint: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(pdas.mint).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(
        actual_mint.base.decimals, 9,
        "Mint decimals mismatch at iteration {iteration}"
    );
}

#[tokio::test]
async fn test_stress_20_iterations() {
    let (mut ctx, pdas) = setup().await;

    // Verify initial creation
    for (pda, name) in [
        (&pdas.record, "MinimalRecord"),
        (&pdas.zc_record, "ZeroCopyRecord"),
        (&pdas.ata, "ATA"),
        (&pdas.vault, "Vault"),
        (&pdas.mint, "Mint"),
    ] {
        shared::assert_onchain_exists(&mut ctx.rpc, pda, name).await;
    }

    // Cache initial state
    let owner = {
        let account = ctx.rpc.get_account(pdas.record).await.unwrap().unwrap();
        let record: MinimalRecord =
            borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
        record.owner
    };
    let mut cached = refresh_cache(&mut ctx.rpc, &pdas, owner).await;

    // First compression
    compress_all(&mut ctx, &pdas).await;

    // Main loop: 20 iterations
    for i in 0..20 {
        println!("--- Iteration {i} ---");

        // Decompress all
        decompress_all(&mut ctx, &pdas, &cached).await;

        // Assert all cached state
        assert_all_state(&mut ctx.rpc, &pdas, &cached, i).await;

        // Update cache after decompression (compression_info changes)
        cached = refresh_cache(&mut ctx.rpc, &pdas, owner).await;

        // Compress all
        compress_all(&mut ctx, &pdas).await;

        println!("  iteration {i} complete");
    }

    println!("All 20 iterations completed successfully.");
}
