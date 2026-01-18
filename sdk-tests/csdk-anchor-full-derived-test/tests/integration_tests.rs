//! Integration tests for D6, D8, and D9 macro test instructions.
//!
//! These tests verify that the macro-generated code works correctly at runtime
//! by testing the full lifecycle: create account -> verify on-chain -> compress -> decompress.

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::RentFreeAccountVariant;
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
        S: IntoVariant<RentFreeAccountVariant>,
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

/// Tests D8PdaOnly: Only #[rentfree] fields (no token accounts)
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

/// Tests D8MultiRentfree: Multiple #[rentfree] fields of same type
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

/// Tests D8All: Multiple #[rentfree] fields of different types
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
    async fn decompress_one<S: IntoVariant<RentFreeAccountVariant>>(
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

/// Tests D5RentfreeToken: #[rentfree_token] attribute
/// NOTE: This test is skipped because token-only instructions (no #[rentfree] PDAs)
/// still require a CreateAccountsProof but get_create_accounts_proof fails with empty inputs.
#[tokio::test]
async fn test_d5_rentfree_token() {
    use csdk_anchor_full_derived_test::d5_markers::{
        D5RentfreeTokenParams, D5_VAULT_AUTH_SEED, D5_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

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
    let accounts = csdk_anchor_full_derived_test::accounts::D5RentfreeToken {
        fee_payer: ctx.payer.pubkey(),
        mint,
        vault_authority,
        d5_token_vault: vault,
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D5RentfreeToken {
        params: D5RentfreeTokenParams {
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
        .expect("D5RentfreeToken instruction should succeed");

    // Verify token vault exists
    ctx.assert_onchain_exists(&vault).await;

    // Note: Token vault decompression not tested - requires TokenAccountVariant
}

/// Tests D5AllMarkers: #[rentfree] + #[rentfree_token] combined
#[tokio::test]
async fn test_d5_all_markers() {
    use csdk_anchor_full_derived_test::d5_markers::{
        D5AllMarkersParams, D5_ALL_AUTH_SEED, D5_ALL_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

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
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
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

/// Tests D7CtokenConfig: ctoken_compressible_config/ctoken_rent_sponsor naming
/// Token-only instruction (no #[rentfree] PDAs) - verifies infrastructure field naming.
#[tokio::test]
async fn test_d7_ctoken_config() {
    use csdk_anchor_full_derived_test::d7_infra_names::{
        D7CtokenConfigParams, D7_CTOKEN_AUTH_SEED, D7_CTOKEN_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

    let mut ctx = TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (d7_ctoken_authority, _) =
        Pubkey::find_program_address(&[D7_CTOKEN_AUTH_SEED], &ctx.program_id);
    let (d7_ctoken_vault, _) =
        Pubkey::find_program_address(&[D7_CTOKEN_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof (no PDA accounts for token-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D7CtokenConfig {
        fee_payer: ctx.payer.pubkey(),
        mint,
        d7_ctoken_authority,
        d7_ctoken_vault,
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D7CtokenConfig {
        _params: D7CtokenConfigParams {
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
        .expect("D7CtokenConfig instruction should succeed");

    // Verify token vault exists
    ctx.assert_onchain_exists(&d7_ctoken_vault).await;

    // Note: Token vault decompression not tested - requires TokenAccountVariant
}

/// Tests D7AllNames: payer + ctoken_config/rent_sponsor naming combined
#[tokio::test]
async fn test_d7_all_names() {
    use csdk_anchor_full_derived_test::d7_infra_names::{
        D7AllNamesParams, D7_ALL_AUTH_SEED, D7_ALL_VAULT_SEED,
    };
    use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
    use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

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
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: CTOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        ctoken_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
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
