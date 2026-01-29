//! Integration tests for D11 zero-copy (AccountLoader) macro features.
//!
//! Tests `#[light_account(init, zero_copy)]` automatic code generation.
//! Each test validates the full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
// Import generated variant/seeds types from the program module
use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::LightAccountVariant;
use csdk_anchor_full_derived_test::d11_zero_copy::{
    // mixed_zc_borsh
    D11MixedZcBorshParams,
    // multiple_zc
    D11MultipleZcParams,
    // with_ata
    D11ZcWithAtaParams,
    // with_ctx_seeds
    D11ZcWithCtxSeedsParams,
    // with_mint_to
    D11ZcWithMintToParams,
    // with_params_seeds
    D11ZcWithParamsSeedsParams,
    // with_vault
    D11ZcWithVaultParams,
    // State types
    ZcBasicRecord,
    ZcWithParamsRecord,
    ZcWithSeedsRecord,
    D11_BORSH_MIXED_SEED,
    D11_MINT_VAULT_AUTH_SEED,
    D11_MINT_VAULT_SEED,
    D11_MINT_ZC_RECORD_SEED,
    D11_ZC1_SEED,
    D11_ZC2_SEED,
    D11_ZC_ATA_RECORD_SEED,
    D11_ZC_CTX_SEED,
    D11_ZC_MIXED_SEED,
    D11_ZC_PARAMS_SEED,
    D11_ZC_RECORD_SEED,
    D11_ZC_VAULT_AUTH_SEED,
    D11_ZC_VAULT_SEED,
};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt, AccountSpec,
    CreateAccountsProofInput, InitializeRentFreeConfig, PdaSpec,
};
use light_compressed_account::address::derive_address;
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    ProgramTestConfig, Rpc,
};
use light_sdk::interface::{CompressionState, IntoVariant};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test context for D11 zero-copy tests.
struct D11TestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    config_pda: Pubkey,
    program_id: Pubkey,
}

impl D11TestContext {
    async fn new() -> Self {
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
            csdk_anchor_full_derived_test::program_rent_sponsor(),
            payer.pubkey(),
        )
        .build();

        rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
            .await
            .expect("Initialize config should succeed");

        Self {
            rpc,
            payer,
            config_pda,
            program_id,
        }
    }

    async fn warp_to_compress(&mut self) {
        self.rpc
            .warp_slot_forward(SLOTS_PER_EPOCH * 30)
            .await
            .unwrap();
    }

    fn get_compressed_address(&self, pda: &Pubkey) -> [u8; 32] {
        let address_tree_pubkey = self.rpc.get_address_tree_v2().tree;
        derive_address(
            &pda.to_bytes(),
            &address_tree_pubkey.to_bytes(),
            &self.program_id.to_bytes(),
        )
    }

    /// Setup a mint for token-based tests.
    async fn setup_mint(&mut self) -> (Pubkey, [u8; 32], Vec<Pubkey>, Keypair) {
        shared::setup_create_mint(
            &mut self.rpc,
            &self.payer,
            self.payer.pubkey(), // mint_authority
            9,                   // decimals
            vec![],              // no recipients initially
        )
        .await
    }
}

/// Test 1: D11ZcWithVault - Zero-copy + Token Vault
/// Tests `#[light_account(init, zero_copy)]` combined with token vault creation.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_zc_with_vault() {
    let mut ctx = D11TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    let owner = Keypair::new().pubkey();

    // Derive PDAs
    let (zc_pda, _) =
        Pubkey::find_program_address(&[D11_ZC_RECORD_SEED, owner.as_ref()], &ctx.program_id);
    let (vault_authority, _) =
        Pubkey::find_program_address(&[D11_ZC_VAULT_AUTH_SEED], &ctx.program_id);
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[D11_ZC_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof for PDA
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(zc_pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11ZcWithVault {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        zc_vault_record: zc_pda,
        d11_mint: mint,
        d11_vault_authority: vault_authority,
        d11_zc_vault: vault_pda,
        light_token_compressible_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11ZcWithVault {
        params: D11ZcWithVaultParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            vault_bump,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D11ZcWithVault instruction should succeed");

    // PHASE 1: Verify PDAs exist on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &vault_pda, "vault_pda").await;

    // Verify zero-copy record data
    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..]; // Skip discriminator
    let record: &ZcBasicRecord = bytemuck::from_bytes(data);
    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(record.counter, 0, "Record counter should be 0");

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify zc_pda is closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    // Note: vault_pda is a token account and doesn't get compressed

    // PHASE 3: Verify compressed account exists
    let compressed_address = ctx.get_compressed_address(&zc_pda);
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address,
        "compressed_account",
    )
    .await;

    // PHASE 4: Decompress account
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    // Build variant using IntoVariant
    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    // Build PdaSpec and create decompress instructions
    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain with correct data
    let record_account = ctx
        .rpc
        .get_account(zc_pda)
        .await
        .unwrap()
        .expect("Account should exist after decompression");

    let data = &record_account.data[8..];
    let record: &ZcBasicRecord = bytemuck::from_bytes(data);
    assert_eq!(
        record.owner, owner,
        "Record owner should match after decompression"
    );
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed after decompression"
    );
}

/// Test 2: D11ZcWithAta - Zero-copy + ATA
/// Tests `#[light_account(init, zero_copy)]` combined with ATA creation.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_zc_with_ata() {
    let mut ctx = D11TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    let owner = Keypair::new().pubkey();
    let ata_owner = ctx.payer.pubkey();

    // Derive PDAs
    let (zc_pda, _) =
        Pubkey::find_program_address(&[D11_ZC_ATA_RECORD_SEED, owner.as_ref()], &ctx.program_id);
    let (ata_pda, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // Get proof for PDA
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(zc_pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11ZcWithAta {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        zc_ata_record: zc_pda,
        d11_ata_mint: mint,
        d11_ata_owner: ata_owner,
        d11_user_ata: ata_pda,
        light_token_compressible_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11ZcWithAta {
        params: D11ZcWithAtaParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            ata_bump,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D11ZcWithAta instruction should succeed");

    // PHASE 1: Verify PDAs exist on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &ata_pda, "ata_pda").await;

    // Verify zero-copy record data
    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcBasicRecord = bytemuck::from_bytes(data);
    assert_eq!(record.owner, owner, "Record owner should match");

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify zc_pda is closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    // PHASE 3: Verify compressed account exists
    let compressed_address = ctx.get_compressed_address(&zc_pda);
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address,
        "compressed_account",
    )
    .await;

    // PHASE 4: Decompress account
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcAtaRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcBasicRecord = bytemuck::from_bytes(data);
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed"
    );
}

/// Test 3: D11MultipleZc - Multiple zero-copy PDAs
/// Tests `#[light_account(init, zero_copy)]` with multiple AccountLoader fields.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_multiple_zc() {
    let mut ctx = D11TestContext::new().await;

    let owner = Keypair::new().pubkey();

    // Derive PDAs
    let (zc_pda_1, _) =
        Pubkey::find_program_address(&[D11_ZC1_SEED, owner.as_ref()], &ctx.program_id);
    let (zc_pda_2, _) =
        Pubkey::find_program_address(&[D11_ZC2_SEED, owner.as_ref()], &ctx.program_id);

    // Get proof for PDAs
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(zc_pda_1),
            CreateAccountsProofInput::pda(zc_pda_2),
        ],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11MultipleZc {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        zc_record_1: zc_pda_1,
        zc_record_2: zc_pda_2,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11MultipleZc {
        params: D11MultipleZcParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D11MultipleZc instruction should succeed");

    // PHASE 1: Verify both PDAs exist on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda_1, "zc_pda_1").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda_2, "zc_pda_2").await;

    // Verify zero-copy record data for both
    let record_1_account = ctx.rpc.get_account(zc_pda_1).await.unwrap().unwrap();
    let data_1 = &record_1_account.data[8..];
    let record_1: &ZcBasicRecord = bytemuck::from_bytes(data_1);
    assert_eq!(record_1.owner, owner, "Record 1 owner should match");
    assert_eq!(record_1.counter, 1, "Record 1 counter should be 1");

    let record_2_account = ctx.rpc.get_account(zc_pda_2).await.unwrap().unwrap();
    let data_2 = &record_2_account.data[8..];
    let record_2: &ZcBasicRecord = bytemuck::from_bytes(data_2);
    assert_eq!(record_2.owner, owner, "Record 2 owner should match");
    assert_eq!(record_2.counter, 2, "Record 2 counter should be 2");

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify both PDAs are closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda_1, "zc_pda_1").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda_2, "zc_pda_2").await;

    // PHASE 3: Verify both compressed accounts exist
    let compressed_address_1 = ctx.get_compressed_address(&zc_pda_1);
    let compressed_address_2 = ctx.get_compressed_address(&zc_pda_2);
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address_1,
        "compressed_account_1",
    )
    .await;
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address_2,
        "compressed_account_2",
    )
    .await;

    // PHASE 4: Decompress first account
    let account_interface_1 = ctx
        .rpc
        .get_account_interface(&zc_pda_1, &ctx.program_id)
        .await
        .expect("failed to get account interface 1");
    assert!(account_interface_1.is_cold(), "Account 1 should be cold");

    let variant_1: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcRecord1Seeds { owner }
            .into_variant(&account_interface_1.account.data[8..])
            .expect("Seed verification failed for record 1");

    let spec_1 = PdaSpec::new(account_interface_1.clone(), variant_1, ctx.program_id);
    let specs_1: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec_1)];

    let decompress_instructions_1 =
        create_load_instructions(&specs_1, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed for record 1");

    ctx.rpc
        .create_and_send_transaction(
            &decompress_instructions_1,
            &ctx.payer.pubkey(),
            &[&ctx.payer],
        )
        .await
        .expect("Decompression of record 1 should succeed");

    // Decompress second account
    let account_interface_2 = ctx
        .rpc
        .get_account_interface(&zc_pda_2, &ctx.program_id)
        .await
        .expect("failed to get account interface 2");
    assert!(account_interface_2.is_cold(), "Account 2 should be cold");

    let variant_2: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcRecord2Seeds { owner }
            .into_variant(&account_interface_2.account.data[8..])
            .expect("Seed verification failed for record 2");

    let spec_2 = PdaSpec::new(account_interface_2.clone(), variant_2, ctx.program_id);
    let specs_2: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec_2)];

    let decompress_instructions_2 =
        create_load_instructions(&specs_2, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed for record 2");

    ctx.rpc
        .create_and_send_transaction(
            &decompress_instructions_2,
            &ctx.payer.pubkey(),
            &[&ctx.payer],
        )
        .await
        .expect("Decompression of record 2 should succeed");

    // PHASE 5: Verify both accounts are back on-chain with correct data
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda_1, "zc_pda_1").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda_2, "zc_pda_2").await;

    let record_1_account = ctx.rpc.get_account(zc_pda_1).await.unwrap().unwrap();
    let data_1 = &record_1_account.data[8..];
    let record_1: &ZcBasicRecord = bytemuck::from_bytes(data_1);
    assert_eq!(record_1.counter, 1, "Record 1 counter should still be 1");
    assert_eq!(
        record_1.compression_info.state,
        CompressionState::Decompressed,
        "Record 1 state should be Decompressed"
    );

    let record_2_account = ctx.rpc.get_account(zc_pda_2).await.unwrap().unwrap();
    let data_2 = &record_2_account.data[8..];
    let record_2: &ZcBasicRecord = bytemuck::from_bytes(data_2);
    assert_eq!(record_2.counter, 2, "Record 2 counter should still be 2");
    assert_eq!(
        record_2.compression_info.state,
        CompressionState::Decompressed,
        "Record 2 state should be Decompressed"
    );
}

/// Test 4: D11MixedZcBorsh - Mixed zero-copy and Borsh accounts
/// Tests `#[light_account(init, zero_copy)]` alongside regular `#[light_account(init)]`.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_mixed_zc_borsh() {
    let mut ctx = D11TestContext::new().await;

    let owner = Keypair::new().pubkey();

    // Derive PDAs
    let (zc_pda, _) =
        Pubkey::find_program_address(&[D11_ZC_MIXED_SEED, owner.as_ref()], &ctx.program_id);
    let (borsh_pda, _) =
        Pubkey::find_program_address(&[D11_BORSH_MIXED_SEED, owner.as_ref()], &ctx.program_id);

    // Get proof for PDAs
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(zc_pda),
            CreateAccountsProofInput::pda(borsh_pda),
        ],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11MixedZcBorsh {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        zc_mixed_record: zc_pda,
        borsh_record: borsh_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11MixedZcBorsh {
        params: D11MixedZcBorshParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D11MixedZcBorsh instruction should succeed");

    // PHASE 1: Verify both PDAs exist on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &borsh_pda, "borsh_pda").await;

    // Verify zero-copy record data
    let zc_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let zc_data = &zc_account.data[8..];
    let zc_record: &ZcBasicRecord = bytemuck::from_bytes(zc_data);
    assert_eq!(zc_record.owner, owner, "ZC record owner should match");
    assert_eq!(zc_record.counter, 100, "ZC record counter should be 100");

    // Verify Borsh record data
    let borsh_account = ctx.rpc.get_account(borsh_pda).await.unwrap().unwrap();
    let borsh_record: csdk_anchor_full_derived_test::SinglePubkeyRecord =
        anchor_lang::AccountDeserialize::try_deserialize(&mut &borsh_account.data[..]).unwrap();
    assert_eq!(borsh_record.owner, owner, "Borsh record owner should match");
    assert_eq!(
        borsh_record.counter, 200,
        "Borsh record counter should be 200"
    );

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify both PDAs are closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &borsh_pda, "borsh_pda").await;

    // PHASE 3: Verify both compressed accounts exist
    let compressed_address_zc = ctx.get_compressed_address(&zc_pda);
    let compressed_address_borsh = ctx.get_compressed_address(&borsh_pda);
    shared::assert_compressed_exists_with_data(&mut ctx.rpc, compressed_address_zc, "zc_record")
        .await;
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address_borsh,
        "borsh_record",
    )
    .await;

    // PHASE 4: Decompress zero-copy account
    let account_interface_zc = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get zc account interface");

    let variant_zc: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcMixedRecordSeeds { owner }
            .into_variant(&account_interface_zc.account.data[8..])
            .expect("Seed verification failed for zc record");

    let spec_zc = PdaSpec::new(account_interface_zc.clone(), variant_zc, ctx.program_id);
    let specs_zc: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec_zc)];

    let decompress_instructions_zc =
        create_load_instructions(&specs_zc, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed for zc record");

    ctx.rpc
        .create_and_send_transaction(
            &decompress_instructions_zc,
            &ctx.payer.pubkey(),
            &[&ctx.payer],
        )
        .await
        .expect("Decompression of zc record should succeed");

    // Decompress borsh account
    let account_interface_borsh = ctx
        .rpc
        .get_account_interface(&borsh_pda, &ctx.program_id)
        .await
        .expect("failed to get borsh account interface");

    let variant_borsh: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::BorshRecordSeeds { owner }
            .into_variant(&account_interface_borsh.account.data[8..])
            .expect("Seed verification failed for borsh record");

    let spec_borsh = PdaSpec::new(
        account_interface_borsh.clone(),
        variant_borsh,
        ctx.program_id,
    );
    let specs_borsh: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec_borsh)];

    let decompress_instructions_borsh =
        create_load_instructions(&specs_borsh, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed for borsh record");

    ctx.rpc
        .create_and_send_transaction(
            &decompress_instructions_borsh,
            &ctx.payer.pubkey(),
            &[&ctx.payer],
        )
        .await
        .expect("Decompression of borsh record should succeed");

    // PHASE 5: Verify both accounts are back on-chain with correct data
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &borsh_pda, "borsh_pda").await;

    let zc_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let zc_data = &zc_account.data[8..];
    let zc_record: &ZcBasicRecord = bytemuck::from_bytes(zc_data);
    assert_eq!(zc_record.counter, 100, "ZC counter should still be 100");
    assert_eq!(
        zc_record.compression_info.state,
        CompressionState::Decompressed,
        "ZC state should be Decompressed"
    );
}

/// Test 5: D11ZcWithCtxSeeds - Zero-copy with ctx.accounts.* seeds
/// Tests `#[light_account(init, zero_copy)]` with context account seeds.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_zc_with_ctx_seeds() {
    let mut ctx = D11TestContext::new().await;

    let owner = Keypair::new().pubkey();
    let authority = Keypair::new();

    // Derive PDA using authority as seed
    let (zc_pda, _) = Pubkey::find_program_address(
        &[D11_ZC_CTX_SEED, authority.pubkey().as_ref()],
        &ctx.program_id,
    );

    // Get proof for PDA
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(zc_pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11ZcWithCtxSeeds {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        authority: authority.pubkey(),
        zc_ctx_record: zc_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11ZcWithCtxSeeds {
        params: D11ZcWithCtxSeedsParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(
            &[instruction],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &authority],
        )
        .await
        .expect("D11ZcWithCtxSeeds instruction should succeed");

    // PHASE 1: Verify PDA exists on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    // Verify zero-copy record data
    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcWithSeedsRecord = bytemuck::from_bytes(data);
    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(
        record.authority,
        authority.pubkey(),
        "Record authority should match"
    );
    assert_eq!(record.value, 42, "Record value should be 42");

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify PDA is closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    // PHASE 3: Verify compressed account exists
    let compressed_address = ctx.get_compressed_address(&zc_pda);
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address,
        "compressed_account",
    )
    .await;

    // PHASE 4: Decompress account
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    // Note: The seeds struct uses authority from ctx.accounts, which is stored in the record
    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcCtxRecordSeeds {
            authority: authority.pubkey(),
        }
        .into_variant(&account_interface.account.data[8..])
        .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain with correct data
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcWithSeedsRecord = bytemuck::from_bytes(data);
    assert_eq!(record.value, 42, "Record value should still be 42");
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed"
    );
}

/// Test 6: D11ZcWithParamsSeeds - Zero-copy with params-only seeds
/// Tests `#[light_account(init, zero_copy)]` with params seeds not on struct.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_zc_with_params_seeds() {
    let mut ctx = D11TestContext::new().await;

    let owner = Keypair::new().pubkey();
    let category_id: u64 = 12345;

    // Derive PDA using owner and category_id as seeds
    let (zc_pda, _) = Pubkey::find_program_address(
        &[
            D11_ZC_PARAMS_SEED,
            owner.as_ref(),
            &category_id.to_le_bytes(),
        ],
        &ctx.program_id,
    );

    // Get proof for PDA
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(zc_pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11ZcWithParamsSeeds {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        zc_params_record: zc_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11ZcWithParamsSeeds {
        params: D11ZcWithParamsSeedsParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            category_id,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D11ZcWithParamsSeeds instruction should succeed");

    // PHASE 1: Verify PDA exists on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    // Verify zero-copy record data
    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcWithParamsRecord = bytemuck::from_bytes(data);
    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(
        record.data, category_id,
        "Record data should match category_id"
    );

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify PDA is closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    // PHASE 3: Verify compressed account exists
    let compressed_address = ctx.get_compressed_address(&zc_pda);
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address,
        "compressed_account",
    )
    .await;

    // PHASE 4: Decompress account
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcParamsRecordSeeds {
            owner,
            category_id,
        }
        .into_variant(&account_interface.account.data[8..])
        .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain with correct data
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcWithParamsRecord = bytemuck::from_bytes(data);
    assert_eq!(
        record.data, category_id,
        "Record data should still match category_id"
    );
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed"
    );
}

/// Test 7: D11ZcWithMintTo - Zero-copy + Vault + MintTo
/// Tests `#[light_account(init, zero_copy)]` combined with vault and token minting.
/// Full lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify decompressed
#[tokio::test]
async fn test_d11_zc_with_mint_to() {
    let mut ctx = D11TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    let owner = Keypair::new().pubkey();
    let mint_amount: u64 = 1_000_000_000; // 1 token with 9 decimals

    // Derive PDAs
    let (zc_pda, _) =
        Pubkey::find_program_address(&[D11_MINT_ZC_RECORD_SEED, owner.as_ref()], &ctx.program_id);
    let (vault_authority, _) =
        Pubkey::find_program_address(&[D11_MINT_VAULT_AUTH_SEED], &ctx.program_id);
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[D11_MINT_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof for PDA
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(zc_pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D11ZcWithMintTo {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        zc_mint_record: zc_pda,
        d11_mint: mint,
        mint_authority: ctx.payer.pubkey(),
        d11_vault_authority: vault_authority,
        d11_mint_vault: vault_pda,
        light_token_compressible_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D11ZcWithMintTo {
        params: D11ZcWithMintToParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            vault_bump,
            mint_amount,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D11ZcWithMintTo instruction should succeed");

    // PHASE 1: Verify PDAs exist on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &vault_pda, "vault_pda").await;

    // Verify zero-copy record data
    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcBasicRecord = bytemuck::from_bytes(data);
    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(
        record.counter, mint_amount,
        "Record counter should match mint_amount"
    );

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.warp_to_compress().await;

    // Verify zc_pda is closed (compressed by forester)
    shared::assert_onchain_closed(&mut ctx.rpc, &zc_pda, "zc_pda").await;
    // Note: vault_pda is a token account and doesn't get compressed

    // PHASE 3: Verify compressed account exists
    let compressed_address = ctx.get_compressed_address(&zc_pda);
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        compressed_address,
        "compressed_account",
    )
    .await;

    // PHASE 4: Decompress account
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");
    assert!(
        account_interface.is_cold(),
        "Account should be cold (compressed)"
    );

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcMintRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 5: Verify account is back on-chain with correct data
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    let record_account = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data = &record_account.data[8..];
    let record: &ZcBasicRecord = bytemuck::from_bytes(data);
    assert_eq!(
        record.counter, mint_amount,
        "Record counter should still match mint_amount"
    );
    assert_eq!(
        record.compression_info.state,
        CompressionState::Decompressed,
        "state should be Decompressed after decompression"
    );
}
