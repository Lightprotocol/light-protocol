//! Integration tests for non-idempotent ATA creation.
//!
//! Verifies that without the `associated_token::idempotent` flag:
//! - First ATA creation succeeds.
//! - Second ATA creation for the same (owner, mint) pair fails.

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::d10_token_accounts::D10SingleAtaNonIdempotentParams;
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

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

    async fn setup_mint(&mut self) -> (Pubkey, [u8; 32], Vec<Pubkey>, Keypair) {
        shared::setup_create_mint(&mut self.rpc, &self.payer, self.payer.pubkey(), 9, vec![]).await
    }
}

/// First non-idempotent ATA creation should succeed.
#[tokio::test]
async fn test_d10_ata_non_idempotent_first_creation_succeeds() {
    let mut ctx = D10TestContext::new().await;

    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;
    let ata_owner = ctx.payer.pubkey();
    let d10_non_idem_ata = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAtaNonIdempotent {
        fee_payer: ctx.payer.pubkey(),
        d10_non_idem_ata_mint: mint,
        d10_non_idem_ata_owner: ata_owner,
        d10_non_idem_ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleAtaNonIdempotent {
        params: D10SingleAtaNonIdempotentParams {
            create_accounts_proof: proof_result.create_accounts_proof,
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
        .expect("First non-idempotent ATA creation should succeed");

    shared::assert_onchain_exists(&mut ctx.rpc, &d10_non_idem_ata, "d10_non_idem_ata").await;
}

/// Second non-idempotent ATA creation should fail because the account already exists.
#[tokio::test]
async fn test_d10_ata_non_idempotent_second_creation_fails() {
    let mut ctx = D10TestContext::new().await;

    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;
    let ata_owner = ctx.payer.pubkey();
    let d10_non_idem_ata = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    let build_ix = |proof_result: light_client::interface::CreateAccountsProofResult| {
        let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAtaNonIdempotent {
            fee_payer: ctx.payer.pubkey(),
            d10_non_idem_ata_mint: mint,
            d10_non_idem_ata_owner: ata_owner,
            d10_non_idem_ata,
            light_token_config: LIGHT_TOKEN_CONFIG,
            light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
            light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
            system_program: solana_sdk::system_program::ID,
        };
        let instruction_data =
            csdk_anchor_full_derived_test::instruction::D10SingleAtaNonIdempotent {
                params: D10SingleAtaNonIdempotentParams {
                    create_accounts_proof: proof_result.create_accounts_proof,
                },
            };
        Instruction {
            program_id: ctx.program_id,
            accounts: [
                accounts.to_account_metas(None),
                proof_result.remaining_accounts,
            ]
            .concat(),
            data: instruction_data.data(),
        }
    };

    // First creation succeeds
    let proof_result_1 = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();
    ctx.rpc
        .create_and_send_transaction(
            &[build_ix(proof_result_1)],
            &ctx.payer.pubkey(),
            &[&ctx.payer],
        )
        .await
        .expect("First non-idempotent ATA creation should succeed");

    shared::assert_onchain_exists(&mut ctx.rpc, &d10_non_idem_ata, "d10_non_idem_ata").await;

    // Second creation must fail (ATA already exists, strict mode)
    let proof_result_2 = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();
    let result = ctx
        .rpc
        .create_and_send_transaction(
            &[build_ix(proof_result_2)],
            &ctx.payer.pubkey(),
            &[&ctx.payer],
        )
        .await;

    assert!(
        result.is_err(),
        "Second non-idempotent ATA creation should fail because ATA already exists"
    );
}
