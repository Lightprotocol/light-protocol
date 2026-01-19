//! Integration tests for D6, D8, and D9 macro test instructions.
//!
//! These tests verify that the macro-generated code works correctly at runtime
//! by testing the full lifecycle: create account -> verify on-chain -> compress -> decompress.

#![allow(clippy::useless_asref)] // Testing that macro handles .as_ref() patterns

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::LightAccountVariant;
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_compressible_client::{
    create_load_accounts_instructions, get_create_accounts_proof, AccountInterface,
    AccountInterfaceExt, CreateAccountsProofInput, InitializeRentFreeConfig,
    RentFreeDecompressAccount,
};
use light_macros::pubkey;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::compressible::IntoVariant;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

/// Test context shared across instruction tests
#[allow(dead_code)]
struct TestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    config_pda: Pubkey,
    program_id: Pubkey,
}

impl TestContext {
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
            RENT_SPONSOR,
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

    async fn assert_onchain_exists(&mut self, pda: &Pubkey) {
        assert!(
            self.rpc.get_account(*pda).await.unwrap().is_some(),
            "Account {} should exist on-chain",
            pda
        );
    }

    async fn assert_onchain_closed(&mut self, pda: &Pubkey) {
        let acc = self.rpc.get_account(*pda).await.unwrap();
        assert!(
            acc.is_none() || acc.unwrap().lamports == 0,
            "Account {} should be closed",
            pda
        );
    }

    async fn assert_compressed_exists(&mut self, addr: [u8; 32]) {
        let acc = self
            .rpc
            .get_compressed_account(addr, None)
            .await
            .unwrap()
            .value
            .unwrap();
        assert_eq!(acc.address.unwrap(), addr);
        assert!(!acc.data.as_ref().unwrap().data.is_empty());
    }

    /// Runs the full compression/decompression lifecycle for a single PDA.
    async fn assert_lifecycle<S>(&mut self, pda: &Pubkey, seeds: S)
    where
        S: IntoVariant<LightAccountVariant>,
    {
        // Warp to trigger compression
        self.rpc
            .warp_slot_forward(SLOTS_PER_EPOCH * 30)
            .await
            .unwrap();
        self.assert_onchain_closed(pda).await;

        // Get account interface
        let account_interface = self
            .rpc
            .get_account_info_interface(pda, &self.program_id)
            .await
            .expect("failed to get account interface");
        assert!(
            account_interface.is_cold,
            "Account should be cold after compression"
        );

        // Build decompression request
        let program_owned_accounts = vec![RentFreeDecompressAccount::from_seeds(
            AccountInterface::from(&account_interface),
            seeds,
        )
        .expect("Seed verification failed")];

        // Create and execute decompression
        let decompress_instructions = create_load_accounts_instructions(
            &program_owned_accounts,
            &[],
            &[],
            self.program_id,
            self.payer.pubkey(),
            self.config_pda,
            self.payer.pubkey(),
            &self.rpc,
        )
        .await
        .expect("create_load_accounts_instructions should succeed");

        self.rpc
            .create_and_send_transaction(
                &decompress_instructions,
                &self.payer.pubkey(),
                &[&self.payer],
            )
            .await
            .expect("Decompression should succeed");

        // Verify account is back on-chain
        self.assert_onchain_exists(pda).await;
    }

    /// Setup a mint for token-based tests.
    /// Returns (mint_pubkey, compression_address, ata_pubkeys, mint_seed_keypair)
    #[allow(dead_code)]
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

// =============================================================================
// D6 Account Types Tests
// =============================================================================

/// Tests D6Account: Direct Account<'info, T> type
#[tokio::test]
async fn test_d6_account() {
    use csdk_anchor_full_derived_test::d6_account_types::D6AccountParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d6_account", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D6Account {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d6_account_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D6Account {
        params: D6AccountParams {
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
        .expect("D6Account instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D6AccountRecordSeeds;
    ctx.assert_lifecycle(&pda, D6AccountRecordSeeds { owner })
        .await;
}

/// Tests D6Boxed: Box<Account<'info, T>> type
#[tokio::test]
async fn test_d6_boxed() {
    use csdk_anchor_full_derived_test::d6_account_types::D6BoxedParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d6_boxed", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D6Boxed {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d6_boxed_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D6Boxed {
        params: D6BoxedParams {
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
        .expect("D6Boxed instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D6BoxedRecordSeeds;
    ctx.assert_lifecycle(&pda, D6BoxedRecordSeeds { owner })
        .await;
}

// =============================================================================
// D8 Builder Paths Tests
// =============================================================================

/// Tests D8PdaOnly: Only #[light_account(init)] fields (no token accounts)
#[tokio::test]
async fn test_d8_pda_only() {
    use csdk_anchor_full_derived_test::d8_builder_paths::D8PdaOnlyParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d8_pda_only", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D8PdaOnly {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d8_pda_only_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8PdaOnly {
        params: D8PdaOnlyParams {
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
        .expect("D8PdaOnly instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D8PdaOnlyRecordSeeds;
    ctx.assert_lifecycle(&pda, D8PdaOnlyRecordSeeds { owner })
        .await;
}

/// Tests D8MultiRentfree: Multiple #[light_account(init)] fields of same type
#[tokio::test]
async fn test_d8_multi_rentfree() {
    use csdk_anchor_full_derived_test::d8_builder_paths::D8MultiRentfreeParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let id1 = 111u64;
    let id2 = 222u64;

    // Derive PDAs
    let (pda1, _) = Pubkey::find_program_address(
        &[b"d8_multi_1", owner.as_ref(), id1.to_le_bytes().as_ref()],
        &ctx.program_id,
    );
    let (pda2, _) = Pubkey::find_program_address(
        &[b"d8_multi_2", owner.as_ref(), id2.to_le_bytes().as_ref()],
        &ctx.program_id,
    );

    // Get proof for both PDAs
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(pda1),
            CreateAccountsProofInput::pda(pda2),
        ],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D8MultiRentfree {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d8_multi_record1: pda1,
        d8_multi_record2: pda2,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8MultiRentfree {
        params: D8MultiRentfreeParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            id1,
            id2,
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
        .expect("D8MultiRentfree instruction should succeed");

    // Verify both accounts exist on-chain
    ctx.assert_onchain_exists(&pda1).await;
    ctx.assert_onchain_exists(&pda2).await;

    // Full lifecycle: compression + decompression (multi-PDA, one at a time)
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        D8MultiRecord1Seeds, D8MultiRecord2Seeds,
    };

    // Warp to trigger compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();
    ctx.assert_onchain_closed(&pda1).await;
    ctx.assert_onchain_closed(&pda2).await;

    // Decompress first account
    let interface1 = ctx
        .rpc
        .get_account_info_interface(&pda1, &ctx.program_id)
        .await
        .unwrap();
    let program_owned_accounts = vec![RentFreeDecompressAccount::from_seeds(
        AccountInterface::from(&interface1),
        D8MultiRecord1Seeds { owner, id1 },
    )
    .unwrap()];
    let decompress_instructions = create_load_accounts_instructions(
        &program_owned_accounts,
        &[],
        &[],
        ctx.program_id,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .unwrap();
    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .unwrap();
    ctx.assert_onchain_exists(&pda1).await;

    // Decompress second account
    let interface2 = ctx
        .rpc
        .get_account_info_interface(&pda2, &ctx.program_id)
        .await
        .unwrap();
    let program_owned_accounts = vec![RentFreeDecompressAccount::from_seeds(
        AccountInterface::from(&interface2),
        D8MultiRecord2Seeds { owner, id2 },
    )
    .unwrap()];
    let decompress_instructions = create_load_accounts_instructions(
        &program_owned_accounts,
        &[],
        &[],
        ctx.program_id,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .unwrap();
    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .unwrap();
    ctx.assert_onchain_exists(&pda2).await;
}

/// Tests D8All: Multiple #[light_account(init)] fields of different types
#[tokio::test]
async fn test_d8_all() {
    use csdk_anchor_full_derived_test::d8_builder_paths::D8AllParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDAs
    let (pda_single, _) =
        Pubkey::find_program_address(&[b"d8_all_single", owner.as_ref()], &ctx.program_id);
    let (pda_multi, _) =
        Pubkey::find_program_address(&[b"d8_all_multi", owner.as_ref()], &ctx.program_id);

    // Get proof for both PDAs
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(pda_single),
            CreateAccountsProofInput::pda(pda_multi),
        ],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D8All {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d8_all_single: pda_single,
        d8_all_multi: pda_multi,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8All {
        params: D8AllParams {
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
        .expect("D8All instruction should succeed");

    // Verify both accounts exist on-chain
    ctx.assert_onchain_exists(&pda_single).await;
    ctx.assert_onchain_exists(&pda_multi).await;

    // Full lifecycle: compression + decompression (multi-PDA, one at a time)
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        D8AllMultiSeeds, D8AllSingleSeeds,
    };

    // Warp to trigger compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();
    ctx.assert_onchain_closed(&pda_single).await;
    ctx.assert_onchain_closed(&pda_multi).await;

    // Decompress first account (single type)
    let interface_single = ctx
        .rpc
        .get_account_info_interface(&pda_single, &ctx.program_id)
        .await
        .unwrap();
    let program_owned_accounts = vec![RentFreeDecompressAccount::from_seeds(
        AccountInterface::from(&interface_single),
        D8AllSingleSeeds { owner },
    )
    .unwrap()];
    let decompress_instructions = create_load_accounts_instructions(
        &program_owned_accounts,
        &[],
        &[],
        ctx.program_id,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .unwrap();
    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .unwrap();
    ctx.assert_onchain_exists(&pda_single).await;

    // Decompress second account (multi type)
    let interface_multi = ctx
        .rpc
        .get_account_info_interface(&pda_multi, &ctx.program_id)
        .await
        .unwrap();
    let program_owned_accounts = vec![RentFreeDecompressAccount::from_seeds(
        AccountInterface::from(&interface_multi),
        D8AllMultiSeeds { owner },
    )
    .unwrap()];
    let decompress_instructions = create_load_accounts_instructions(
        &program_owned_accounts,
        &[],
        &[],
        ctx.program_id,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .unwrap();
    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .unwrap();
    ctx.assert_onchain_exists(&pda_multi).await;
}

// =============================================================================
// D9 Seeds Tests
// =============================================================================

/// Tests D9Literal: Literal seed expression
#[tokio::test]
async fn test_d9_literal() {
    use csdk_anchor_full_derived_test::d9_seeds::D9LiteralParams;

    let mut ctx = TestContext::new().await;

    // Derive PDA (literal seeds only)
    let (pda, _) = Pubkey::find_program_address(&[b"d9_literal_record"], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9Literal {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d9_literal_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9Literal {
        _params: D9LiteralParams {
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
        .expect("D9Literal instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9LiteralRecordSeeds;
    ctx.assert_lifecycle(&pda, D9LiteralRecordSeeds {}).await;
}

/// Tests D9Constant: Constant seed expression
#[tokio::test]
async fn test_d9_constant() {
    use csdk_anchor_full_derived_test::{d9_seeds::D9ConstantParams, D9_CONSTANT_SEED};

    let mut ctx = TestContext::new().await;

    // Derive PDA using constant
    let (pda, _) = Pubkey::find_program_address(&[D9_CONSTANT_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9Constant {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d9_constant_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9Constant {
        _params: D9ConstantParams {
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
        .expect("D9Constant instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9ConstantRecordSeeds;
    ctx.assert_lifecycle(&pda, D9ConstantRecordSeeds {}).await;
}

/// Tests D9CtxAccount: Context account seed expression
#[tokio::test]
async fn test_d9_ctx_account() {
    use csdk_anchor_full_derived_test::d9_seeds::D9CtxAccountParams;

    let mut ctx = TestContext::new().await;
    let authority = Keypair::new();

    // Derive PDA using authority key
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_ctx", authority.pubkey().as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9CtxAccount {
        fee_payer: ctx.payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: ctx.config_pda,
        d9_ctx_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9CtxAccount {
        _params: D9CtxAccountParams {
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
        .expect("D9CtxAccount instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9CtxRecordSeeds;
    ctx.assert_lifecycle(
        &pda,
        D9CtxRecordSeeds {
            authority: authority.pubkey(),
        },
    )
    .await;
}

/// Tests D9Param: Param seed expression (Pubkey)
#[tokio::test]
async fn test_d9_param() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ParamParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using param
    let (pda, _) = Pubkey::find_program_address(&[b"d9_param", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9Param {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d9_param_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9Param {
        params: D9ParamParams {
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
        .expect("D9Param instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9ParamRecordSeeds;
    ctx.assert_lifecycle(&pda, D9ParamRecordSeeds { owner })
        .await;
}

/// Tests D9ParamBytes: Param bytes seed expression (u64)
#[tokio::test]
async fn test_d9_param_bytes() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ParamBytesParams;

    let mut ctx = TestContext::new().await;
    let id = 12345u64;

    // Derive PDA using param bytes
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_param_bytes", id.to_le_bytes().as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ParamBytes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d9_param_bytes_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ParamBytes {
        _params: D9ParamBytesParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            id,
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
        .expect("D9ParamBytes instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9ParamBytesRecordSeeds;
    ctx.assert_lifecycle(&pda, D9ParamBytesRecordSeeds { id })
        .await;
}

/// Tests D9Mixed: Mixed seed expression types
#[tokio::test]
async fn test_d9_mixed() {
    use csdk_anchor_full_derived_test::d9_seeds::D9MixedParams;

    let mut ctx = TestContext::new().await;
    let authority = Keypair::new();
    let owner = Keypair::new().pubkey();

    // Derive PDA using mixed seeds
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_mixed", authority.pubkey().as_ref(), owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9Mixed {
        fee_payer: ctx.payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: ctx.config_pda,
        d9_mixed_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9Mixed {
        params: D9MixedParams {
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
        .expect("D9Mixed instruction should succeed");

    // Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9MixedRecordSeeds;
    ctx.assert_lifecycle(
        &pda,
        D9MixedRecordSeeds {
            authority: authority.pubkey(),
            owner,
        },
    )
    .await;
}

// =============================================================================
// D7 Infrastructure Names Tests
// =============================================================================

/// Tests D7Payer: "payer" field name variant
#[tokio::test]
async fn test_d7_payer() {
    use csdk_anchor_full_derived_test::d7_infra_names::D7PayerParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d7_payer", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D7Payer {
        payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d7_payer_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D7Payer {
        params: D7PayerParams {
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
        .expect("D7Payer instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D7PayerRecordSeeds;
    ctx.assert_lifecycle(&pda, D7PayerRecordSeeds { owner })
        .await;
}

/// Tests D7Creator: "creator" field name variant
#[tokio::test]
async fn test_d7_creator() {
    use csdk_anchor_full_derived_test::d7_infra_names::D7CreatorParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d7_creator", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D7Creator {
        creator: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d7_creator_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D7Creator {
        params: D7CreatorParams {
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
        .expect("D7Creator instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D7CreatorRecordSeeds;
    ctx.assert_lifecycle(&pda, D7CreatorRecordSeeds { owner })
        .await;
}

// =============================================================================
// D9 Additional Seeds Tests
// =============================================================================

/// Tests D9FunctionCall: Function call seed expression
#[tokio::test]
async fn test_d9_function_call() {
    use csdk_anchor_full_derived_test::d9_seeds::D9FunctionCallParams;

    let mut ctx = TestContext::new().await;
    let key_a = Keypair::new().pubkey();
    let key_b = Keypair::new().pubkey();

    // Derive PDA using max_key (same as in instruction)
    let max_key = csdk_anchor_full_derived_test::max_key(&key_a, &key_b);
    let (pda, _) = Pubkey::find_program_address(&[b"d9_func", max_key.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9FunctionCall {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d9_func_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9FunctionCall {
        params: D9FunctionCallParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            key_a,
            key_b,
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
        .expect("D9FunctionCall instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;

    // Full lifecycle: compression + decompression
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D9FuncRecordSeeds;
    ctx.assert_lifecycle(&pda, D9FuncRecordSeeds { key_a, key_b })
        .await;
}

/// Tests D9All: All 6 seed expression types
#[tokio::test]
async fn test_d9_all() {
    use csdk_anchor_full_derived_test::{d9_seeds::D9AllParams, D9_ALL_SEED};

    let mut ctx = TestContext::new().await;
    let authority = Keypair::new();
    let owner = Keypair::new().pubkey();
    let id = 42u64;
    let key_a = Keypair::new().pubkey();
    let key_b = Keypair::new().pubkey();

    // Derive all 6 PDAs
    let (pda_lit, _) = Pubkey::find_program_address(&[b"d9_all_lit"], &ctx.program_id);
    let (pda_const, _) = Pubkey::find_program_address(&[D9_ALL_SEED], &ctx.program_id);
    let (pda_ctx, _) = Pubkey::find_program_address(
        &[b"d9_all_ctx", authority.pubkey().as_ref()],
        &ctx.program_id,
    );
    let (pda_param, _) =
        Pubkey::find_program_address(&[b"d9_all_param", owner.as_ref()], &ctx.program_id);
    let (pda_bytes, _) = Pubkey::find_program_address(
        &[b"d9_all_bytes", id.to_le_bytes().as_ref()],
        &ctx.program_id,
    );
    let max_key = csdk_anchor_full_derived_test::max_key(&key_a, &key_b);
    let (pda_func, _) =
        Pubkey::find_program_address(&[b"d9_all_func", max_key.as_ref()], &ctx.program_id);

    // Get proof for all 6 PDAs
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(pda_lit),
            CreateAccountsProofInput::pda(pda_const),
            CreateAccountsProofInput::pda(pda_ctx),
            CreateAccountsProofInput::pda(pda_param),
            CreateAccountsProofInput::pda(pda_bytes),
            CreateAccountsProofInput::pda(pda_func),
        ],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9All {
        fee_payer: ctx.payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: ctx.config_pda,
        d9_all_lit: pda_lit,
        d9_all_const: pda_const,
        d9_all_ctx: pda_ctx,
        d9_all_param: pda_param,
        d9_all_bytes: pda_bytes,
        d9_all_func: pda_func,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9All {
        params: D9AllParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            id,
            key_a,
            key_b,
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
        .expect("D9All instruction should succeed");

    // Verify all 6 accounts exist
    ctx.assert_onchain_exists(&pda_lit).await;
    ctx.assert_onchain_exists(&pda_const).await;
    ctx.assert_onchain_exists(&pda_ctx).await;
    ctx.assert_onchain_exists(&pda_param).await;
    ctx.assert_onchain_exists(&pda_bytes).await;
    ctx.assert_onchain_exists(&pda_func).await;

    // Full lifecycle: compression + decompression (6 PDAs, one at a time)
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        D9AllBytesSeeds, D9AllConstSeeds, D9AllCtxSeeds, D9AllFuncSeeds, D9AllLitSeeds,
        D9AllParamSeeds,
    };

    // Warp to trigger compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();
    ctx.assert_onchain_closed(&pda_lit).await;
    ctx.assert_onchain_closed(&pda_const).await;
    ctx.assert_onchain_closed(&pda_ctx).await;
    ctx.assert_onchain_closed(&pda_param).await;
    ctx.assert_onchain_closed(&pda_bytes).await;
    ctx.assert_onchain_closed(&pda_func).await;

    // Helper to decompress a single account
    async fn decompress_one<S: IntoVariant<LightAccountVariant>>(
        ctx: &mut TestContext,
        pda: &Pubkey,
        seeds: S,
    ) {
        let interface = ctx
            .rpc
            .get_account_info_interface(pda, &ctx.program_id)
            .await
            .unwrap();
        let program_owned_accounts =
            vec![
                RentFreeDecompressAccount::from_seeds(AccountInterface::from(&interface), seeds)
                    .unwrap(),
            ];
        let decompress_instructions = create_load_accounts_instructions(
            &program_owned_accounts,
            &[],
            &[],
            ctx.program_id,
            ctx.payer.pubkey(),
            ctx.config_pda,
            ctx.payer.pubkey(),
            &ctx.rpc,
        )
        .await
        .unwrap();
        ctx.rpc
            .create_and_send_transaction(
                &decompress_instructions,
                &ctx.payer.pubkey(),
                &[&ctx.payer],
            )
            .await
            .unwrap();
        ctx.assert_onchain_exists(pda).await;
    }

    // Decompress all 6 accounts one at a time
    decompress_one(&mut ctx, &pda_lit, D9AllLitSeeds {}).await;
    decompress_one(&mut ctx, &pda_const, D9AllConstSeeds {}).await;
    decompress_one(
        &mut ctx,
        &pda_ctx,
        D9AllCtxSeeds {
            authority: authority.pubkey(),
        },
    )
    .await;
    decompress_one(&mut ctx, &pda_param, D9AllParamSeeds { owner }).await;
    decompress_one(&mut ctx, &pda_bytes, D9AllBytesSeeds { id }).await;
    decompress_one(&mut ctx, &pda_func, D9AllFuncSeeds { key_a, key_b }).await;
}

// =============================================================================
// Full Lifecycle Test (compression + decompression)
// =============================================================================

/// Tests the full lifecycle with compression and decompression
#[tokio::test]
async fn test_d8_pda_only_full_lifecycle() {
    use csdk_anchor_full_derived_test::{
        csdk_anchor_full_derived_test::D8PdaOnlyRecordSeeds, d8_builder_paths::D8PdaOnlyParams,
    };
    use light_compressible::rent::SLOTS_PER_EPOCH;
    use light_compressible_client::{
        create_load_accounts_instructions, AccountInterface, AccountInterfaceExt,
        RentFreeDecompressAccount,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d8_pda_only", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build and send instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D8PdaOnly {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        d8_pda_only_record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8PdaOnly {
        params: D8PdaOnlyParams {
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
        .expect("D8PdaOnly instruction should succeed");

    // PHASE 1: Verify account exists on-chain
    ctx.assert_onchain_exists(&pda).await;

    // PHASE 2: Warp to trigger auto-compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();

    // Verify account is compressed (on-chain closed)
    ctx.assert_onchain_closed(&pda).await;

    // Derive compressed address
    let address_tree_pubkey = ctx.rpc.get_address_tree_v2().tree;
    let compressed_address = light_compressed_account::address::derive_address(
        &pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &ctx.program_id.to_bytes(),
    );

    // Verify compressed account exists with data
    ctx.assert_compressed_exists(compressed_address).await;

    // PHASE 3: Decompress account
    let account_interface = ctx
        .rpc
        .get_account_info_interface(&pda, &ctx.program_id)
        .await
        .expect("failed to get account interface");
    assert!(account_interface.is_cold, "Account should be cold");

    let program_owned_accounts = vec![RentFreeDecompressAccount::from_seeds(
        AccountInterface::from(&account_interface),
        D8PdaOnlyRecordSeeds { owner },
    )
    .expect("Seed verification failed")];

    let decompress_instructions = create_load_accounts_instructions(
        &program_owned_accounts,
        &[],
        &[],
        ctx.program_id,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .expect("create_load_accounts_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(&decompress_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Verify account is back on-chain
    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D5 Markers Token Tests (require mint setup)
// =============================================================================

/// Tests D5LightToken: #[light_account(token)] attribute
/// NOTE: This test is skipped because token-only instructions (no #[light_account(init)] PDAs)
/// still require a CreateAccountsProof but get_create_accounts_proof fails with empty inputs.
#[tokio::test]
async fn test_d5_light_token() {
    use csdk_anchor_full_derived_test::d5_markers::{
        D5LightTokenParams, D5_VAULT_AUTH_SEED, D5_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

    let mut ctx = TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (vault_authority, _) = Pubkey::find_program_address(&[D5_VAULT_AUTH_SEED], &ctx.program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[D5_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof (no PDA accounts for token-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D5LightToken {
        fee_payer: ctx.payer.pubkey(),
        mint,
        vault_authority,
        d5_token_vault: vault,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D5LightToken {
        params: D5LightTokenParams {
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
        .expect("D5LightToken instruction should succeed");

    // Verify token vault exists
    ctx.assert_onchain_exists(&vault).await;

    // Note: Token vault decompression not tested - requires TokenAccountVariant
}

/// Tests D5AllMarkers: #[light_account(init)] + #[light_account(token)] combined
#[tokio::test]
async fn test_d5_all_markers() {
    use csdk_anchor_full_derived_test::d5_markers::{
        D5AllMarkersParams, D5_ALL_AUTH_SEED, D5_ALL_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (d5_all_authority, _) = Pubkey::find_program_address(&[D5_ALL_AUTH_SEED], &ctx.program_id);
    let (d5_all_record, _) =
        Pubkey::find_program_address(&[b"d5_all_record", owner.as_ref()], &ctx.program_id);
    let (d5_all_vault, _) =
        Pubkey::find_program_address(&[D5_ALL_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof for PDA record
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(d5_all_record)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D5AllMarkers {
        fee_payer: ctx.payer.pubkey(),
        mint,
        compression_config: ctx.config_pda,
        d5_all_authority,
        d5_all_record,
        d5_all_vault,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D5AllMarkers {
        params: D5AllMarkersParams {
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
        .expect("D5AllMarkers instruction should succeed");

    // Verify both PDA record and token vault exist
    ctx.assert_onchain_exists(&d5_all_record).await;
    ctx.assert_onchain_exists(&d5_all_vault).await;

    // Full lifecycle: compression + decompression (PDA only)
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D5AllRecordSeeds;
    ctx.assert_lifecycle(&d5_all_record, D5AllRecordSeeds { owner })
        .await;
    // Note: Token vault decompression not tested - requires TokenAccountVariant
}

// =============================================================================
// D7 Infrastructure Names Token Tests (require mint setup)
// =============================================================================

/// Tests D7LightTokenConfig: light_token_compressible_config/light_token_rent_sponsor naming
/// Token-only instruction (no #[light_account(init)] PDAs) - verifies infrastructure field naming.
#[tokio::test]
async fn test_d7_light_token_config() {
    use csdk_anchor_full_derived_test::d7_infra_names::{
        D7LightTokenConfigParams, D7_LIGHT_TOKEN_AUTH_SEED, D7_LIGHT_TOKEN_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{
        COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR,
    };

    let mut ctx = TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (d7_light_token_authority, _) =
        Pubkey::find_program_address(&[D7_LIGHT_TOKEN_AUTH_SEED], &ctx.program_id);
    let (d7_light_token_vault, _) =
        Pubkey::find_program_address(&[D7_LIGHT_TOKEN_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof (no PDA accounts for token-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D7LightTokenConfig {
        fee_payer: ctx.payer.pubkey(),
        mint,
        d7_light_token_authority,
        d7_light_token_vault,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D7LightTokenConfig {
        _params: D7LightTokenConfigParams {
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
        .expect("D7LightTokenConfig instruction should succeed");

    // Verify token vault exists
    ctx.assert_onchain_exists(&d7_light_token_vault).await;

    // Note: Token vault decompression not tested - requires TokenAccountVariant
}

/// Tests D7AllNames: payer + light_token_config/rent_sponsor naming combined
#[tokio::test]
async fn test_d7_all_names() {
    use csdk_anchor_full_derived_test::d7_infra_names::{
        D7AllNamesParams, D7_ALL_AUTH_SEED, D7_ALL_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (d7_all_authority, _) = Pubkey::find_program_address(&[D7_ALL_AUTH_SEED], &ctx.program_id);
    let (d7_all_record, _) =
        Pubkey::find_program_address(&[b"d7_all_record", owner.as_ref()], &ctx.program_id);
    let (d7_all_vault, _) =
        Pubkey::find_program_address(&[D7_ALL_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof for PDA record
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(d7_all_record)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D7AllNames {
        payer: ctx.payer.pubkey(),
        mint,
        compression_config: ctx.config_pda,
        d7_all_authority,
        d7_all_record,
        d7_all_vault,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D7AllNames {
        params: D7AllNamesParams {
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
        .expect("D7AllNames instruction should succeed");

    // Verify both PDA record and token vault exist
    ctx.assert_onchain_exists(&d7_all_record).await;
    ctx.assert_onchain_exists(&d7_all_vault).await;

    // Full lifecycle: compression + decompression (PDA only)
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::D7AllRecordSeeds;
    ctx.assert_lifecycle(&d7_all_record, D7AllRecordSeeds { owner })
        .await;
    // Note: Token vault decompression not tested - requires TokenAccountVariant
}

// =============================================================================
// D9 Qualified Paths Tests
// =============================================================================

/// Tests D9QualifiedBare: Bare constant (no path prefix)
#[tokio::test]
async fn test_d9_qualified_bare() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9QualifiedBareParams,
        instructions::d9_seeds::qualified_paths::D9_QUALIFIED_LOCAL,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA using bare constant
    let (pda, _) = Pubkey::find_program_address(&[D9_QUALIFIED_LOCAL], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9QualifiedBare {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9QualifiedBare {
        _params: D9QualifiedBareParams {
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
        .expect("D9QualifiedBare instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9QualifiedSelf: self:: prefix path qualification
#[tokio::test]
async fn test_d9_qualified_self() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9QualifiedSelfParams,
        instructions::d9_seeds::qualified_paths::D9_QUALIFIED_LOCAL,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA using self:: prefix (same constant as bare)
    let (pda, _) = Pubkey::find_program_address(&[D9_QUALIFIED_LOCAL], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9QualifiedSelf {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9QualifiedSelf {
        _params: D9QualifiedSelfParams {
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
        .expect("D9QualifiedSelf instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9QualifiedCrate: crate:: prefix path qualification
#[tokio::test]
async fn test_d9_qualified_crate() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9QualifiedCrateParams,
        instructions::d9_seeds::qualified_paths::D9_QUALIFIED_CRATE,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA using crate:: qualified constant
    let (pda, _) = Pubkey::find_program_address(&[D9_QUALIFIED_CRATE], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9QualifiedCrate {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9QualifiedCrate {
        _params: D9QualifiedCrateParams {
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
        .expect("D9QualifiedCrate instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9QualifiedDeep: Deeply nested crate path
#[tokio::test]
async fn test_d9_qualified_deep() {
    use csdk_anchor_full_derived_test::{d9_seeds::D9QualifiedDeepParams, D9_CONSTANT_SEED};

    let mut ctx = TestContext::new().await;

    // Derive PDA using deeply nested crate path
    let (pda, _) = Pubkey::find_program_address(&[D9_CONSTANT_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9QualifiedDeep {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9QualifiedDeep {
        _params: D9QualifiedDeepParams {
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
        .expect("D9QualifiedDeep instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9QualifiedMixed: Mixed qualified and bare paths in same seeds
#[tokio::test]
async fn test_d9_qualified_mixed() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9QualifiedMixedParams,
        instructions::d9_seeds::qualified_paths::D9_QUALIFIED_LOCAL, D9_CONSTANT_SEED,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using mixed paths
    let (pda, _) = Pubkey::find_program_address(
        &[D9_QUALIFIED_LOCAL, D9_CONSTANT_SEED, owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9QualifiedMixed {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9QualifiedMixed {
        params: D9QualifiedMixedParams {
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
        .expect("D9QualifiedMixed instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 Method Chains Tests
// =============================================================================

/// Tests D9MethodAsRef: constant.as_ref()
#[tokio::test]
async fn test_d9_method_as_ref() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9MethodAsRefParams, instructions::d9_seeds::method_chains::D9_METHOD_BYTES,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[D9_METHOD_BYTES.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MethodAsRef {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MethodAsRef {
        _params: D9MethodAsRefParams {
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
        .expect("D9MethodAsRef instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9MethodAsBytes: string_constant.as_bytes()
#[tokio::test]
async fn test_d9_method_as_bytes() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9MethodAsBytesParams, instructions::d9_seeds::method_chains::D9_METHOD_STR,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[D9_METHOD_STR.as_bytes()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MethodAsBytes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MethodAsBytes {
        _params: D9MethodAsBytesParams {
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
        .expect("D9MethodAsBytes instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9MethodQualifiedAsBytes: crate::path::CONST.as_bytes()
#[tokio::test]
async fn test_d9_method_qualified_as_bytes() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9MethodQualifiedAsBytesParams,
        instructions::d9_seeds::method_chains::D9_METHOD_STR,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[D9_METHOD_STR.as_bytes()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MethodQualifiedAsBytes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MethodQualifiedAsBytes {
        _params: D9MethodQualifiedAsBytesParams {
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
        .expect("D9MethodQualifiedAsBytes instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9MethodToLeBytes: params.field.to_le_bytes().as_ref()
#[tokio::test]
async fn test_d9_method_to_le_bytes() {
    use csdk_anchor_full_derived_test::d9_seeds::D9MethodToLeBytesParams;

    let mut ctx = TestContext::new().await;
    let id = 12345u64;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_le", id.to_le_bytes().as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MethodToLeBytes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MethodToLeBytes {
        _params: D9MethodToLeBytesParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            id,
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
        .expect("D9MethodToLeBytes instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9MethodToBeBytes: params.field.to_be_bytes().as_ref()
#[tokio::test]
async fn test_d9_method_to_be_bytes() {
    use csdk_anchor_full_derived_test::d9_seeds::D9MethodToBeBytesParams;

    let mut ctx = TestContext::new().await;
    let id = 67890u64;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_be", id.to_be_bytes().as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MethodToBeBytes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MethodToBeBytes {
        _params: D9MethodToBeBytesParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            id,
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
        .expect("D9MethodToBeBytes instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9MethodMixed: Mixed methods in seeds
#[tokio::test]
async fn test_d9_method_mixed() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9MethodMixedParams, instructions::d9_seeds::method_chains::D9_METHOD_STR,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let id = 11111u64;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[
            D9_METHOD_STR.as_bytes(),
            owner.as_ref(),
            id.to_le_bytes().as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MethodMixed {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MethodMixed {
        params: D9MethodMixedParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            id,
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
        .expect("D9MethodMixed instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 Array Bumps Tests
// =============================================================================

/// Tests D9BumpLiteral: Literal seed with bump
#[tokio::test]
async fn test_d9_bump_literal() {
    use csdk_anchor_full_derived_test::d9_seeds::D9BumpLiteralParams;

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[b"d9_bump_lit"], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9BumpLiteral {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9BumpLiteral {
        _params: D9BumpLiteralParams {
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
        .expect("D9BumpLiteral instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9BumpConstant: Constant seed with bump
#[tokio::test]
async fn test_d9_bump_constant() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9BumpConstantParams, instructions::d9_seeds::array_bumps::D9_BUMP_SEED,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[D9_BUMP_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9BumpConstant {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9BumpConstant {
        _params: D9BumpConstantParams {
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
        .expect("D9BumpConstant instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9BumpQualified: Qualified path with bump
#[tokio::test]
async fn test_d9_bump_qualified() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9BumpQualifiedParams, instructions::d9_seeds::array_bumps::D9_BUMP_STR,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[D9_BUMP_STR.as_bytes()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9BumpQualified {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9BumpQualified {
        _params: D9BumpQualifiedParams {
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
        .expect("D9BumpQualified instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9BumpParam: Param seed with bump
#[tokio::test]
async fn test_d9_bump_param() {
    use csdk_anchor_full_derived_test::d9_seeds::D9BumpParamParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_bump_param", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9BumpParam {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9BumpParam {
        params: D9BumpParamParams {
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
        .expect("D9BumpParam instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9BumpCtx: Ctx account seed with bump
#[tokio::test]
async fn test_d9_bump_ctx() {
    use csdk_anchor_full_derived_test::d9_seeds::D9BumpCtxParams;

    let mut ctx = TestContext::new().await;
    let authority = Keypair::new();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_bump_ctx", authority.pubkey().as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9BumpCtx {
        fee_payer: ctx.payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9BumpCtx {
        _params: D9BumpCtxParams {
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
        .expect("D9BumpCtx instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9BumpMixed: Multiple seeds with bump
#[tokio::test]
async fn test_d9_bump_mixed() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9BumpMixedParams, instructions::d9_seeds::array_bumps::D9_BUMP_SEED,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let id = 54321u64;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"d9_bump_mix",
            D9_BUMP_SEED,
            owner.as_ref(),
            id.to_le_bytes().as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9BumpMixed {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9BumpMixed {
        params: D9BumpMixedParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            id,
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
        .expect("D9BumpMixed instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 Complex Mixed Tests
// =============================================================================

/// Tests D9ComplexThree: 3 seeds - literal + constant + param
#[tokio::test]
async fn test_d9_complex_three() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9ComplexThreeParams, instructions::d9_seeds::complex_mixed::D9_COMPLEX_PREFIX,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_complex3", D9_COMPLEX_PREFIX, owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexThree {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexThree {
        params: D9ComplexThreeParams {
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
        .expect("D9ComplexThree instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexFour: 4 seeds - version + namespace + param + bytes
#[tokio::test]
async fn test_d9_complex_four() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9ComplexFourParams,
        instructions::d9_seeds::complex_mixed::{D9_COMPLEX_NAMESPACE, D9_COMPLEX_V1},
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let id = 99999u64;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[
            D9_COMPLEX_V1,
            D9_COMPLEX_NAMESPACE.as_bytes(),
            owner.as_ref(),
            id.to_le_bytes().as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexFour {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexFour {
        params: D9ComplexFourParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            id,
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
        .expect("D9ComplexFour instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexFive: 5 seeds with ctx account
#[tokio::test]
async fn test_d9_complex_five() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9ComplexFiveParams,
        instructions::d9_seeds::complex_mixed::{D9_COMPLEX_NAMESPACE, D9_COMPLEX_V1},
    };

    let mut ctx = TestContext::new().await;
    let authority = Keypair::new();
    let owner = Keypair::new().pubkey();
    let id = 88888u64;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[
            D9_COMPLEX_V1,
            D9_COMPLEX_NAMESPACE.as_bytes(),
            authority.pubkey().as_ref(),
            owner.as_ref(),
            id.to_le_bytes().as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexFive {
        fee_payer: ctx.payer.pubkey(),
        authority: authority.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexFive {
        params: D9ComplexFiveParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            id,
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
        .expect("D9ComplexFive instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexQualifiedMix: Qualified paths mixed with local
#[tokio::test]
async fn test_d9_complex_qualified_mix() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9ComplexQualifiedMixParams,
        instructions::d9_seeds::complex_mixed::{D9_COMPLEX_PREFIX, D9_COMPLEX_V1},
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[D9_COMPLEX_V1, D9_COMPLEX_PREFIX, owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexQualifiedMix {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexQualifiedMix {
        params: D9ComplexQualifiedMixParams {
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
        .expect("D9ComplexQualifiedMix instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexFunc: Function call combined with other seeds
#[tokio::test]
async fn test_d9_complex_func() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9ComplexFuncParams, instructions::d9_seeds::complex_mixed::D9_COMPLEX_V1,
    };

    let mut ctx = TestContext::new().await;
    let key_a = Keypair::new().pubkey();
    let key_b = Keypair::new().pubkey();
    let id = 77777u64;

    // Derive PDA using max_key
    let max_key = csdk_anchor_full_derived_test::max_key(&key_a, &key_b);
    let (pda, _) = Pubkey::find_program_address(
        &[D9_COMPLEX_V1, max_key.as_ref(), id.to_le_bytes().as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexFunc {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexFunc {
        params: D9ComplexFuncParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            key_a,
            key_b,
            id,
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
        .expect("D9ComplexFunc instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexAllQualified: All paths being fully qualified
#[tokio::test]
async fn test_d9_complex_all_qualified() {
    use csdk_anchor_full_derived_test::{
        d9_seeds::D9ComplexAllQualifiedParams,
        instructions::d9_seeds::complex_mixed::{D9_COMPLEX_NAMESPACE, D9_COMPLEX_V1},
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[
            D9_COMPLEX_V1,
            D9_COMPLEX_NAMESPACE.as_bytes(),
            owner.as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexAllQualified {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexAllQualified {
        params: D9ComplexAllQualifiedParams {
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
        .expect("D9ComplexAllQualified instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexProgramId: Program ID as seed
#[tokio::test]
async fn test_d9_complex_program_id() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ComplexProgramIdParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using program ID
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_progid", ctx.program_id.as_ref(), owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexProgramId {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexProgramId {
        params: D9ComplexProgramIdParams {
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
        .expect("D9ComplexProgramId instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ComplexIdFunc: id() function call as seed
#[tokio::test]
async fn test_d9_complex_id_func() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ComplexIdFuncParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using id() function (same result as program ID)
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_idfunc", ctx.program_id.as_ref(), owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ComplexIdFunc {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ComplexIdFunc {
        params: D9ComplexIdFuncParams {
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
        .expect("D9ComplexIdFunc instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 Edge Cases Tests
// =============================================================================

/// Tests D9EdgeEmpty: Empty literal placeholder
#[tokio::test]
async fn test_d9_edge_empty() {
    use csdk_anchor_full_derived_test::d9_seeds::D9EdgeEmptyParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[&b"d9_edge_empty"[..], &b"_"[..], owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeEmpty {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeEmpty {
        params: D9EdgeEmptyParams {
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
        .expect("D9EdgeEmpty instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9EdgeSingleByte: Single byte constant
#[tokio::test]
async fn test_d9_edge_single_byte() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        edge_cases::D9_SINGLE_BYTE, D9EdgeSingleByteParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[D9_SINGLE_BYTE], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeSingleByte {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeSingleByte {
        _params: D9EdgeSingleByteParams {
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
        .expect("D9EdgeSingleByte instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9EdgeSingleLetter: Single letter constant name
#[tokio::test]
async fn test_d9_edge_single_letter() {
    use csdk_anchor_full_derived_test::d9_seeds::{edge_cases::A, D9EdgeSingleLetterParams};

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[A], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeSingleLetter {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeSingleLetter {
        _params: D9EdgeSingleLetterParams {
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
        .expect("D9EdgeSingleLetter instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9EdgeDigits: Constant name with digits
#[tokio::test]
async fn test_d9_edge_digits() {
    use csdk_anchor_full_derived_test::d9_seeds::{edge_cases::SEED_123, D9EdgeDigitsParams};

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[SEED_123], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeDigits {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeDigits {
        _params: D9EdgeDigitsParams {
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
        .expect("D9EdgeDigits instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9EdgeUnderscore: Leading underscore constant
#[tokio::test]
async fn test_d9_edge_underscore() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        edge_cases::_UNDERSCORE_CONST, D9EdgeUnderscoreParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[_UNDERSCORE_CONST], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeUnderscore {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeUnderscore {
        _params: D9EdgeUnderscoreParams {
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
        .expect("D9EdgeUnderscore instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9EdgeManyLiterals: Many literals in seeds
#[tokio::test]
async fn test_d9_edge_many_literals() {
    use csdk_anchor_full_derived_test::d9_seeds::D9EdgeManyLiteralsParams;

    let mut ctx = TestContext::new().await;

    // Derive PDA with 5 byte literals
    let (pda, _) = Pubkey::find_program_address(&[b"a", b"b", b"c", b"d", b"e"], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeManyLiterals {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeManyLiterals {
        _params: D9EdgeManyLiteralsParams {
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
        .expect("D9EdgeManyLiterals instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9EdgeMixed: Mixed edge cases
#[tokio::test]
async fn test_d9_edge_mixed() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        edge_cases::{A, SEED_123, _UNDERSCORE_CONST},
        D9EdgeMixedParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[A, SEED_123, _UNDERSCORE_CONST, owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9EdgeMixed {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9EdgeMixed {
        params: D9EdgeMixedParams {
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
        .expect("D9EdgeMixed instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 External Paths Tests
// =============================================================================

/// Tests D9ExternalSdkTypes: External crate (light_sdk_types)
#[tokio::test]
async fn test_d9_external_sdk_types() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ExternalSdkTypesParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using external constant
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"d9_ext_sdk",
            light_sdk_types::constants::CPI_AUTHORITY_PDA_SEED,
            owner.as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ExternalSdkTypes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ExternalSdkTypes {
        params: D9ExternalSdkTypesParams {
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
        .expect("D9ExternalSdkTypes instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ExternalCtoken: External crate (light_token_types)
#[tokio::test]
async fn test_d9_external_ctoken() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ExternalCtokenParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using external constant
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"d9_ext_ctoken",
            light_token_interface::POOL_SEED,
            owner.as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ExternalCtoken {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ExternalCtoken {
        params: D9ExternalCtokenParams {
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
        .expect("D9ExternalCtoken instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ExternalMixed: Multiple external crates mixed
#[tokio::test]
async fn test_d9_external_mixed() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ExternalMixedParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA using mixed external constants
    let (pda, _) = Pubkey::find_program_address(
        &[
            light_sdk_types::constants::CPI_AUTHORITY_PDA_SEED,
            light_token_interface::POOL_SEED,
            owner.as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ExternalMixed {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ExternalMixed {
        params: D9ExternalMixedParams {
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
        .expect("D9ExternalMixed instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ExternalWithLocal: External with local constant
#[tokio::test]
async fn test_d9_external_with_local() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        external_paths::D9_EXTERNAL_LOCAL, D9ExternalWithLocalParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[
            D9_EXTERNAL_LOCAL,
            light_sdk_types::constants::RENT_SPONSOR_SEED,
            owner.as_ref(),
        ],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ExternalWithLocal {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ExternalWithLocal {
        params: D9ExternalWithLocalParams {
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
        .expect("D9ExternalWithLocal instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ExternalBump: External constant with bump
#[tokio::test]
async fn test_d9_external_bump() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ExternalBumpParams;

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[light_token_interface::COMPRESSED_MINT_SEED, owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ExternalBump {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ExternalBump {
        params: D9ExternalBumpParams {
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
        .expect("D9ExternalBump instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ExternalReexport: Re-exported external constant
#[tokio::test]
async fn test_d9_external_reexport() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        external_paths::REEXPORTED_SEED, D9ExternalReexportParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA using re-exported constant
    let (pda, _) = Pubkey::find_program_address(&[REEXPORTED_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ExternalReexport {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ExternalReexport {
        _params: D9ExternalReexportParams {
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
        .expect("D9ExternalReexport instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 Nested Seeds Tests
// =============================================================================

/// Tests D9NestedSimple: Simple nested struct access
#[tokio::test]
async fn test_d9_nested_simple() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        nested_seeds::InnerNested, D9NestedSimpleParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_nested_simple", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9NestedSimple {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9NestedSimple {
        params: D9NestedSimpleParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            nested: InnerNested { owner, id: 0 },
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
        .expect("D9NestedSimple instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9NestedDouble: Double nested struct access
#[tokio::test]
async fn test_d9_nested_double() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        nested_seeds::{InnerNested, OuterNested},
        D9NestedDoubleParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_nested_double", owner.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9NestedDouble {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9NestedDouble {
        params: D9NestedDoubleParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            outer: OuterNested {
                array: [0; 16],
                nested: InnerNested { owner, id: 0 },
            },
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
        .expect("D9NestedDouble instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9NestedArrayField: Nested array field access
#[tokio::test]
async fn test_d9_nested_array_field() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        nested_seeds::{InnerNested, OuterNested},
        D9NestedArrayFieldParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let array = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_nested_array", array.as_ref()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9NestedArrayField {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9NestedArrayField {
        params: D9NestedArrayFieldParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            outer: OuterNested {
                array,
                nested: InnerNested { owner, id: 0 },
            },
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
        .expect("D9NestedArrayField instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ArrayIndex: Array indexing params.arrays[2].as_slice()
#[tokio::test]
async fn test_d9_array_index() {
    use csdk_anchor_full_derived_test::d9_seeds::D9ArrayIndexParams;

    let mut ctx = TestContext::new().await;

    // Create 2D array with deterministic values
    let mut arrays = [[0u8; 16]; 10];
    arrays[2] = [42u8; 16]; // The indexed array

    // Derive PDA using the indexed array
    let (pda, _) =
        Pubkey::find_program_address(&[b"d9_array_idx", arrays[2].as_slice()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ArrayIndex {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ArrayIndex {
        _params: D9ArrayIndexParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            arrays,
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
        .expect("D9ArrayIndex instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9NestedBytes: Nested struct with bytes conversion
#[tokio::test]
async fn test_d9_nested_bytes() {
    use csdk_anchor_full_derived_test::d9_seeds::{nested_seeds::InnerNested, D9NestedBytesParams};

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let id = 123456u64;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_nested_bytes", id.to_le_bytes().as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9NestedBytes {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9NestedBytes {
        params: D9NestedBytesParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            nested: InnerNested { owner, id },
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
        .expect("D9NestedBytes instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9NestedCombined: Multiple nested seeds combined
#[tokio::test]
async fn test_d9_nested_combined() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        nested_seeds::{InnerNested, OuterNested},
        D9NestedCombinedParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();
    let array = [7u8; 16];

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"d9_nested_combined", array.as_ref(), owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9NestedCombined {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9NestedCombined {
        params: D9NestedCombinedParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            outer: OuterNested {
                array,
                nested: InnerNested { owner, id: 0 },
            },
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
        .expect("D9NestedCombined instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

// =============================================================================
// D9 Const Patterns Tests
// =============================================================================

/// Tests D9AssocConst: Associated constant
#[tokio::test]
async fn test_d9_assoc_const() {
    use csdk_anchor_full_derived_test::d9_seeds::{const_patterns::SeedHolder, D9AssocConstParams};

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[SeedHolder::SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9AssocConst {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9AssocConst {
        _params: D9AssocConstParams {
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
        .expect("D9AssocConst instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9AssocConstMethod: Associated constant with method
#[tokio::test]
async fn test_d9_assoc_const_method() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::SeedHolder, D9AssocConstMethodParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[SeedHolder::NAMESPACE.as_bytes()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9AssocConstMethod {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9AssocConstMethod {
        _params: D9AssocConstMethodParams {
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
        .expect("D9AssocConstMethod instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9MultiAssocConst: Multiple associated constants
#[tokio::test]
async fn test_d9_multi_assoc_const() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::{AnotherHolder, SeedHolder},
        D9MultiAssocConstParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[SeedHolder::SEED, AnotherHolder::PREFIX, owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9MultiAssocConst {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9MultiAssocConst {
        params: D9MultiAssocConstParams {
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
        .expect("D9MultiAssocConst instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ConstFn: Const fn call
#[tokio::test]
async fn test_d9_const_fn() {
    use csdk_anchor_full_derived_test::d9_seeds::{const_patterns::const_seed, D9ConstFnParams};

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[const_seed()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ConstFn {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ConstFn {
        _params: D9ConstFnParams {
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
        .expect("D9ConstFn instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ConstFnGeneric: Const fn with generic
#[tokio::test]
async fn test_d9_const_fn_generic() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::identity_seed, D9ConstFnGenericParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[identity_seed::<12>(b"generic_seed")], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ConstFnGeneric {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ConstFnGeneric {
        _params: D9ConstFnGenericParams {
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
        .expect("D9ConstFnGeneric instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9TraitAssocConst: Trait associated constant
#[tokio::test]
async fn test_d9_trait_assoc_const() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::{HasSeed, SeedHolder},
        D9TraitAssocConstParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[<SeedHolder as HasSeed>::TRAIT_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9TraitAssocConst {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9TraitAssocConst {
        _params: D9TraitAssocConstParams {
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
        .expect("D9TraitAssocConst instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9Static: Static variable
#[tokio::test]
async fn test_d9_static() {
    use csdk_anchor_full_derived_test::d9_seeds::{const_patterns::STATIC_SEED, D9StaticParams};

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[&STATIC_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9Static {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9Static {
        _params: D9StaticParams {
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
        .expect("D9Static instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9QualifiedConstFn: Qualified const fn
#[tokio::test]
async fn test_d9_qualified_const_fn() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::const_seed, D9QualifiedConstFnParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[const_seed()], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9QualifiedConstFn {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9QualifiedConstFn {
        _params: D9QualifiedConstFnParams {
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
        .expect("D9QualifiedConstFn instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9FullyQualifiedAssoc: Fully qualified associated constant
#[tokio::test]
async fn test_d9_fully_qualified_assoc() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::SeedHolder, D9FullyQualifiedAssocParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(&[SeedHolder::SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9FullyQualifiedAssoc {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9FullyQualifiedAssoc {
        _params: D9FullyQualifiedAssocParams {
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
        .expect("D9FullyQualifiedAssoc instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9FullyQualifiedTrait: Fully qualified trait associated constant
#[tokio::test]
async fn test_d9_fully_qualified_trait() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::{HasSeed, SeedHolder},
        D9FullyQualifiedTraitParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[<SeedHolder as HasSeed>::TRAIT_SEED], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9FullyQualifiedTrait {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9FullyQualifiedTrait {
        _params: D9FullyQualifiedTraitParams {
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
        .expect("D9FullyQualifiedTrait instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9FullyQualifiedGeneric: Fully qualified const fn with generic
#[tokio::test]
async fn test_d9_fully_qualified_generic() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::identity_seed, D9FullyQualifiedGenericParams,
    };

    let mut ctx = TestContext::new().await;

    // Derive PDA
    let (pda, _) =
        Pubkey::find_program_address(&[identity_seed::<10>(b"fq_generic")], &ctx.program_id);

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9FullyQualifiedGeneric {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9FullyQualifiedGeneric {
        _params: D9FullyQualifiedGenericParams {
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
        .expect("D9FullyQualifiedGeneric instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}

/// Tests D9ConstCombined: Combined const patterns
#[tokio::test]
async fn test_d9_const_combined() {
    use csdk_anchor_full_derived_test::d9_seeds::{
        const_patterns::{const_seed, SeedHolder},
        D9ConstCombinedParams,
    };

    let mut ctx = TestContext::new().await;
    let owner = Keypair::new().pubkey();

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[SeedHolder::SEED, const_seed(), owner.as_ref()],
        &ctx.program_id,
    );

    // Get proof
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(pda)],
    )
    .await
    .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D9ConstCombined {
        fee_payer: ctx.payer.pubkey(),
        compression_config: ctx.config_pda,
        record: pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9ConstCombined {
        params: D9ConstCombinedParams {
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
        .expect("D9ConstCombined instruction should succeed");

    ctx.assert_onchain_exists(&pda).await;
}
