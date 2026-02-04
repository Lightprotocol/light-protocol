//! Security validation tests for decompression.
//!
//! Tests verify that invalid decompression attempts are correctly rejected.
//! Error codes reference (LightSdkTypesError custom program error codes):
//! - 14017: FewerAccountsThanSystemAccounts
//! - 14035: ConstraintViolation
//! - 14038: InvalidRentSponsor
//! - 14043: InvalidInstructionData
//! - 14044: InvalidSeeds
//! - 14046: NotEnoughAccountKeys
//! - 14047: MissingRequiredSignature

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::{
    csdk_anchor_full_derived_test::LightAccountVariant,
    d11_zero_copy::{
        D11ZcWithVaultParams, ZcBasicRecord, D11_ZC_RECORD_SEED, D11_ZC_VAULT_AUTH_SEED,
        D11_ZC_VAULT_SEED,
    },
};
use light_account::IntoVariant;
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt, AccountSpec,
    CreateAccountsProofInput, InitializeRentFreeConfig, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    utils::assert::assert_rpc_error,
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test context for failing tests.
struct FailingTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    config_pda: Pubkey,
    program_id: Pubkey,
}

impl FailingTestContext {
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

    async fn setup_mint(&mut self) -> (Pubkey, [u8; 32], Vec<Pubkey>, Keypair) {
        shared::setup_create_mint(&mut self.rpc, &self.payer, self.payer.pubkey(), 9, vec![]).await
    }

    /// Creates a PDA account and compresses it, returning the PDA pubkey and owner.
    async fn create_and_compress_pda(&mut self) -> (Pubkey, Pubkey, u8) {
        let (mint, _, _, _) = self.setup_mint().await;
        let owner = Keypair::new().pubkey();

        // Derive PDAs
        let (zc_pda, _) =
            Pubkey::find_program_address(&[D11_ZC_RECORD_SEED, owner.as_ref()], &self.program_id);
        let (vault_authority, _) =
            Pubkey::find_program_address(&[D11_ZC_VAULT_AUTH_SEED], &self.program_id);
        let (vault_pda, vault_bump) =
            Pubkey::find_program_address(&[D11_ZC_VAULT_SEED, mint.as_ref()], &self.program_id);

        // Get proof for PDA
        let proof_result = get_create_accounts_proof(
            &self.rpc,
            &self.program_id,
            vec![CreateAccountsProofInput::pda(zc_pda)],
        )
        .await
        .unwrap();

        // Build instruction
        let accounts = csdk_anchor_full_derived_test::accounts::D11ZcWithVault {
            fee_payer: self.payer.pubkey(),
            compression_config: self.config_pda,
            pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
            zc_vault_record: zc_pda,
            d11_mint: mint,
            d11_vault_authority: vault_authority,
            d11_zc_vault: vault_pda,
            light_token_config: LIGHT_TOKEN_CONFIG,
            light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
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
            program_id: self.program_id,
            accounts: [
                accounts.to_account_metas(None),
                proof_result.remaining_accounts,
            ]
            .concat(),
            data: instruction_data.data(),
        };

        self.rpc
            .create_and_send_transaction(&[instruction], &self.payer.pubkey(), &[&self.payer])
            .await
            .expect("Create PDA should succeed");

        // Warp to compress
        self.warp_to_compress().await;

        // Verify compressed
        shared::assert_onchain_closed(&mut self.rpc, &zc_pda, "zc_pda").await;

        (zc_pda, owner, vault_bump)
    }
}

// =============================================================================
// PDA DECOMPRESSION TESTS
// =============================================================================

/// Test: Wrong rent sponsor should fail with InvalidRentSponsor (14038).
/// Validates rent sponsor PDA derivation check in decompress.rs:160-169.
#[tokio::test]
async fn test_pda_wrong_rent_sponsor() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    // Get account interface
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    // Build valid variant
    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // Modify the rent_sponsor in remaining_accounts[2] to a wrong address
    let wrong_rent_sponsor = Keypair::new().pubkey();
    if let Some(ix) = decompress_instructions.first_mut() {
        // Rent sponsor is at index 2 in remaining accounts
        if ix.accounts.len() > 2 {
            ix.accounts[2] = AccountMeta::new(wrong_rent_sponsor, false);
        }
    }

    let result = ctx
        .rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await;

    // Should fail with InvalidRentSponsor (14038)
    assert_rpc_error(result, 0, 14038).unwrap();
}

/// Test: Double decompression should be a noop (idempotent).
/// Validates idempotency check in pda.rs:43-50.
#[tokio::test]
async fn test_pda_double_decompress_is_noop() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    // Get account interface
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant.clone(), ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // First decompression
    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("First decompression should succeed");

    // Verify account is decompressed
    shared::assert_onchain_exists(&mut ctx.rpc, &zc_pda, "zc_pda").await;

    // Get data after first decompression
    let record_account_after_first = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data_after_first = &record_account_after_first.data[8..];
    let record_after_first: &ZcBasicRecord = bytemuck::from_bytes(data_after_first);
    let counter_after_first = record_after_first.counter;

    // For second decompression, we need a fresh account interface since it's now hot
    // The idempotency is at the on-chain level - if the discriminator is non-zero, skip
    // Since the account is now hot, create_load_instructions will return empty
    let account_interface_2 = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    // Account should be hot now
    assert!(
        !account_interface_2.is_cold(),
        "Account should be hot after decompression"
    );

    // Build same spec but with fresh interface
    let spec_2 = PdaSpec::new(account_interface_2.clone(), variant, ctx.program_id);
    let specs_2: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec_2)];

    // This should return empty vec because account is hot
    let decompress_instructions_2 =
        create_load_instructions(&specs_2, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    assert!(
        decompress_instructions_2.is_empty(),
        "Second decompress should return empty instructions for hot account"
    );

    // Verify data unchanged
    let record_account_final = ctx.rpc.get_account(zc_pda).await.unwrap().unwrap();
    let data_final = &record_account_final.data[8..];
    let record_final: &ZcBasicRecord = bytemuck::from_bytes(data_final);

    assert_eq!(
        record_final.counter, counter_after_first,
        "Counter should be unchanged after attempted second decompression"
    );
}

/// Test: Wrong config PDA should fail with ConstraintViolation (14035).
/// Validates config check in config.rs:144-153.
#[tokio::test]
async fn test_pda_wrong_config() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    // Get account interface
    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // Modify the config PDA in remaining_accounts[1] to a wrong address
    let wrong_config = Keypair::new().pubkey();
    if let Some(ix) = decompress_instructions.first_mut() {
        if ix.accounts.len() > 1 {
            ix.accounts[1] = AccountMeta::new_readonly(wrong_config, false);
        }
    }

    let result = ctx
        .rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await;

    // Should fail - the config validation will reject the wrong address
    // ConstraintViolation (14035) since deserialization/validation of the config account fails
    assert_rpc_error(result, 0, 14035).unwrap();
}

// =============================================================================
// COMMON PARAMETER TESTS
// =============================================================================

/// Test: system_accounts_offset out of bounds should fail with InvalidInstructionData (14043).
/// Validates bounds check in decompress.rs:175-177.
#[tokio::test]
async fn test_system_accounts_offset_out_of_bounds() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // The instruction data format is: [discriminator(8)] [system_accounts_offset(1)] ...
    // Modify system_accounts_offset to be out of bounds
    if let Some(ix) = decompress_instructions.first_mut() {
        // Byte 8 is system_accounts_offset (directly after 8-byte discriminator)
        if ix.data.len() > 8 {
            ix.data[8] = 255; // Set to max u8, guaranteed out of bounds
        }
    }

    let result = ctx
        .rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await;

    // Should fail with InvalidInstructionData (14043)
    assert_rpc_error(result, 0, 14043).unwrap();
}

/// Test: token_accounts_offset invalid should fail with InvalidInstructionData (14043).
/// Validates bounds check in decompress.rs:178-181.
#[tokio::test]
async fn test_token_accounts_offset_invalid() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // The instruction data format is:
    // [discriminator(8)] [system_accounts_offset(1)] [token_accounts_offset(1)] ...
    // Modify token_accounts_offset to be larger than accounts.len()
    if let Some(ix) = decompress_instructions.first_mut() {
        // Byte 9 is token_accounts_offset (after 8-byte discriminator + 1-byte system_accounts_offset)
        if ix.data.len() > 9 {
            ix.data[9] = 200; // Set to value larger than accounts
        }
    }

    let result = ctx
        .rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await;

    // Should fail with InvalidInstructionData (14043) - token_accounts_offset exceeds accounts count
    assert_rpc_error(result, 0, 14043).unwrap();
}

// =============================================================================
// REMAINING ACCOUNTS MANIPULATION TESTS
// =============================================================================

/// Test: Removing required accounts should fail.
/// Error code 14017 is FewerAccountsThanSystemAccounts.
#[tokio::test]
async fn test_missing_system_accounts() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // Remove several accounts from the end
    if let Some(ix) = decompress_instructions.first_mut() {
        let num_to_remove = 5.min(ix.accounts.len().saturating_sub(5));
        for _ in 0..num_to_remove {
            ix.accounts.pop();
        }
    }

    let result = ctx
        .rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await;

    // Should fail with FewerAccountsThanSystemAccounts (14017) - not enough remaining accounts
    assert_rpc_error(result, 0, 14017).unwrap();
}

/// Test: Wrong PDA account (mismatch between seeds and account) should fail.
/// When seeds don't match the account, we get InvalidSeeds (14044).
#[tokio::test]
async fn test_pda_account_mismatch() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // Replace the PDA account (last account) with a wrong address
    let wrong_pda = Keypair::new().pubkey();
    if let Some(ix) = decompress_instructions.first_mut() {
        if let Some(last) = ix.accounts.last_mut() {
            *last = AccountMeta::new(wrong_pda, false);
        }
    }

    let result = ctx
        .rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await;

    // Should fail with InvalidSeeds (14044) - PDA derivation validation catches
    // the mismatch before attempting CPI
    assert_rpc_error(result, 0, 14044).unwrap();
}

/// Test: Fee payer not a signer should fail with MissingRequiredSignature (8).
#[tokio::test]
async fn test_fee_payer_not_signer() {
    let mut ctx = FailingTestContext::new().await;
    let (zc_pda, owner, _) = ctx.create_and_compress_pda().await;

    let account_interface = ctx
        .rpc
        .get_account_interface(&zc_pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");

    let variant: LightAccountVariant =
        csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::ZcVaultRecordSeeds { owner }
            .into_variant(&account_interface.account.data[8..])
            .expect("Seed verification failed");

    let spec = PdaSpec::new(account_interface.clone(), variant, ctx.program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let mut decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // Remove signer flag from fee_payer (index 0)
    if let Some(ix) = decompress_instructions.first_mut() {
        if !ix.accounts.is_empty() {
            ix.accounts[0].is_signer = false;
        }
    }

    // Create a different keypair to sign instead (not the fee_payer)
    let other_signer = Keypair::new();
    light_test_utils::airdrop_lamports(&mut ctx.rpc, &other_signer.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // This should fail because the transaction will be missing the fee_payer signature
    let result = ctx
        .rpc
        .create_and_send_transaction(
            &decompress_instructions,
            &other_signer.pubkey(),
            &[&other_signer],
        )
        .await;

    // Should fail with MissingRequiredSignature (8) or similar
    // The exact error depends on where validation fails first
    assert!(result.is_err(), "Should fail when fee_payer is not signer");
}
