//! Integration tests for D10 token account macro features.
//!
//! Tests #[light_account(init, token, ...)] and #[light_account(init, associated_token, ...)]
//! automatic code generation for creating compressed token accounts.

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::d10_token_accounts::{
    D10SingleAtaParams, D10SingleVaultParams, D10_SINGLE_VAULT_AUTH_SEED, D10_SINGLE_VAULT_SEED,
};
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_macros::pubkey;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const RENT_SPONSOR_PUBKEY: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

/// Test context for D10 token account tests
struct D10TestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    #[allow(dead_code)]
    config_pda: Pubkey,
    program_id: Pubkey,
}

impl D10TestContext {
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
            RENT_SPONSOR_PUBKEY,
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

    async fn assert_onchain_exists(&mut self, account: &Pubkey) {
        assert!(
            self.rpc.get_account(*account).await.unwrap().is_some(),
            "Account {} should exist on-chain",
            account
        );
    }

    /// Setup a mint for token-based tests.
    /// Returns (mint_pubkey, compression_address, ata_pubkeys, mint_seed_keypair)
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

/// Tests D10SingleVault: #[light_account(init, token, ...)] automatic code generation.
/// The macro should generate CreateTokenAccountCpi in LightFinalize.
#[tokio::test]
async fn test_d10_single_vault() {
    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (d10_vault_authority, _auth_bump) =
        Pubkey::find_program_address(&[D10_SINGLE_VAULT_AUTH_SEED], &ctx.program_id);
    let (d10_single_vault, vault_bump) =
        Pubkey::find_program_address(&[D10_SINGLE_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof (no PDA accounts for token-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleVault {
        fee_payer: ctx.payer.pubkey(),
        d10_mint: mint,
        d10_vault_authority,
        d10_single_vault,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleVault {
        params: D10SingleVaultParams {
            create_accounts_proof: proof_result.create_accounts_proof,
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
        .expect("D10SingleVault instruction should succeed");

    // Verify token vault exists on-chain
    ctx.assert_onchain_exists(&d10_single_vault).await;
}

/// Tests D10SingleAta: #[light_account(init, associated_token, ...)] automatic code generation.
/// The macro should generate create_associated_token_account_idempotent in LightFinalize.
#[tokio::test]
async fn test_d10_single_ata() {
    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // The ATA owner will be the payer
    let ata_owner = ctx.payer.pubkey();

    // Derive the ATA address using Light Token SDK's derivation
    let (d10_single_ata, ata_bump) = light_token::token::derive_token_ata(&ata_owner, &mint);

    // Get proof (no PDA accounts for ATA-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAta {
        fee_payer: ctx.payer.pubkey(),
        d10_ata_mint: mint,
        d10_ata_owner: ata_owner,
        d10_single_ata,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleAta {
        params: D10SingleAtaParams {
            create_accounts_proof: proof_result.create_accounts_proof,
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
        .expect("D10SingleAta instruction should succeed");

    // Verify ATA exists on-chain
    ctx.assert_onchain_exists(&d10_single_ata).await;
}
