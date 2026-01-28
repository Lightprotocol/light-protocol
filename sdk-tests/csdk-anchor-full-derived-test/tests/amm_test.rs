/// AMM Full Lifecycle Integration Test
///
/// Tests the complete AMM flow:
/// 1. Initialize pool with rent-free PDAs and LP mint
/// 2. Deposit tokens and receive LP tokens
/// 3. Withdraw tokens by burning LP tokens
/// 4. Advance epochs to trigger auto-compression
/// 5. Decompress all accounts
/// 6. Deposit after decompression to verify pool works
///
/// Also includes aggregator-style test flow using LightAmmInterface.
mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::amm_test::{
    InitializeParams, AUTH_SEED, OBSERVATION_SEED, POOL_LP_MINT_SIGNER_SEED, POOL_SEED,
    POOL_VAULT_SEED,
};
// SDK for AmmSdk-based approach
use csdk_anchor_full_derived_test_sdk::{AmmInstruction, AmmSdk};
use jupiter_amm_interface::{Amm, AmmContext, KeyedAccount, QuoteParams, SwapMode, SwapParams};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt,
    InitializeRentFreeConfig, LightAmmInterface, LightProgramInterface,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_macros::pubkey;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use light_token::instruction::{
    find_mint_address, get_associated_token_address_and_bump, COMPRESSIBLE_CONFIG_V1,
    LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR,
};
use light_token_interface::state::Token;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

async fn assert_onchain_exists(rpc: &mut LightProgramTest, pda: &Pubkey) {
    assert!(
        rpc.get_account(*pda).await.unwrap().is_some(),
        "Account {} should exist on-chain",
        pda
    );
}

async fn assert_onchain_closed(rpc: &mut LightProgramTest, pda: &Pubkey) {
    let acc = rpc.get_account(*pda).await.unwrap();
    assert!(
        acc.is_none() || acc.unwrap().lamports == 0,
        "Account {} should be closed",
        pda
    );
}

fn parse_token(data: &[u8]) -> Token {
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
    assert!(!accs.is_empty(), "Compressed token account should exist");
    assert_eq!(
        accs[0].token.amount, expected_amount,
        "Compressed token amount mismatch"
    );
}

/// Stores all AMM-related PDAs
struct AmmPdas {
    pool_state: Pubkey,
    #[allow(dead_code)]
    pool_state_bump: u8,
    observation_state: Pubkey,
    #[allow(dead_code)]
    observation_state_bump: u8,
    authority: Pubkey,
    #[allow(dead_code)]
    authority_bump: u8,
    token_0_vault: Pubkey,
    #[allow(dead_code)]
    token_0_vault_bump: u8,
    token_1_vault: Pubkey,
    #[allow(dead_code)]
    token_1_vault_bump: u8,
    lp_mint_signer: Pubkey,
    lp_mint_signer_bump: u8,
    lp_mint: Pubkey,
    creator_lp_token: Pubkey,
    creator_lp_token_bump: u8,
}

/// Context for AMM tests
struct AmmTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    config_pda: Pubkey,
    program_id: Pubkey,
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,
    creator: Keypair,
    creator_token_0: Pubkey,
    creator_token_1: Pubkey,
    amm_config: Keypair,
}

/// Setup the test environment with light mints
async fn setup() -> AmmTestContext {
    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Setup mock program data and compression config
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

    // Create creator keypair and fund
    let creator = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &creator.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Create two light mints (cmints) for token_0 and token_1
    // Using shared::setup_create_mint which creates both compressed mint and on-chain Mint account
    let (mint_a, _compression_addr_a, ata_pubkeys_a, _mint_seed_a) = shared::setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),                       // mint_authority
        9,                                    // decimals
        vec![(10_000_000, creator.pubkey())], // mint to creator
    )
    .await;

    let (mint_b, _compression_addr_b, ata_pubkeys_b, _mint_seed_b) = shared::setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),                       // mint_authority
        9,                                    // decimals
        vec![(10_000_000, creator.pubkey())], // mint to creator
    )
    .await;

    // Ensure proper ordering: token_0_mint.key() < token_1_mint.key()
    let (token_0_mint, token_1_mint, creator_token_0, creator_token_1) = if mint_a < mint_b {
        (mint_a, mint_b, ata_pubkeys_a[0], ata_pubkeys_b[0])
    } else {
        (mint_b, mint_a, ata_pubkeys_b[0], ata_pubkeys_a[0])
    };

    // Create amm_config account (simple funded account for this test)
    let amm_config = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &amm_config.pubkey(), 1_000_000)
        .await
        .unwrap();

    AmmTestContext {
        rpc,
        payer,
        config_pda,
        program_id,
        token_0_mint,
        token_1_mint,
        creator,
        creator_token_0,
        creator_token_1,
        amm_config,
    }
}

/// Derive all AMM PDAs
fn derive_amm_pdas(
    program_id: &Pubkey,
    amm_config: &Pubkey,
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
    creator: &Pubkey,
) -> AmmPdas {
    // Pool state: seeds = [POOL_SEED, amm_config, token_0_mint, token_1_mint]
    let (pool_state, pool_state_bump) = Pubkey::find_program_address(
        &[
            POOL_SEED.as_bytes(),
            amm_config.as_ref(),
            token_0_mint.as_ref(),
            token_1_mint.as_ref(),
        ],
        program_id,
    );

    // Authority: seeds = [AUTH_SEED]
    let (authority, authority_bump) =
        Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], program_id);

    // Observation: seeds = [OBSERVATION_SEED, pool_state]
    let (observation_state, observation_state_bump) = Pubkey::find_program_address(
        &[OBSERVATION_SEED.as_bytes(), pool_state.as_ref()],
        program_id,
    );

    // Vault 0: seeds = [POOL_VAULT_SEED, pool_state, token_0_mint]
    let (token_0_vault, token_0_vault_bump) = Pubkey::find_program_address(
        &[
            POOL_VAULT_SEED.as_bytes(),
            pool_state.as_ref(),
            token_0_mint.as_ref(),
        ],
        program_id,
    );

    // Vault 1: seeds = [POOL_VAULT_SEED, pool_state, token_1_mint]
    let (token_1_vault, token_1_vault_bump) = Pubkey::find_program_address(
        &[
            POOL_VAULT_SEED.as_bytes(),
            pool_state.as_ref(),
            token_1_mint.as_ref(),
        ],
        program_id,
    );

    // LP mint signer: seeds = [POOL_LP_MINT_SIGNER_SEED, pool_state]
    let (lp_mint_signer, lp_mint_signer_bump) =
        Pubkey::find_program_address(&[POOL_LP_MINT_SIGNER_SEED, pool_state.as_ref()], program_id);

    // LP mint: derived from lp_mint_signer using find_mint_address
    let (lp_mint, _) = find_mint_address(&lp_mint_signer);

    // Creator LP token ATA: using get_associated_token_address_and_bump
    let (creator_lp_token, creator_lp_token_bump) =
        get_associated_token_address_and_bump(creator, &lp_mint);

    AmmPdas {
        pool_state,
        pool_state_bump,
        observation_state,
        observation_state_bump,
        authority,
        authority_bump,
        token_0_vault,
        token_0_vault_bump,
        token_1_vault,
        token_1_vault_bump,
        lp_mint_signer,
        lp_mint_signer_bump,
        lp_mint,
        creator_lp_token,
        creator_lp_token_bump,
    }
}

/// AMM full lifecycle integration test
#[tokio::test]
async fn test_amm_full_lifecycle() {
    let mut ctx = setup().await;

    let pdas = derive_amm_pdas(
        &ctx.program_id,
        &ctx.amm_config.pubkey(),
        &ctx.token_0_mint,
        &ctx.token_1_mint,
        &ctx.creator.pubkey(),
    );

    let proof_inputs = AmmSdk::create_initialize_pool_proof_inputs(
        pdas.pool_state,
        pdas.observation_state,
        pdas.lp_mint,
    );
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, proof_inputs)
        .await
        .unwrap();

    let init_amount_0 = 1000u64;
    let init_amount_1 = 1000u64;
    let open_time = 0u64;

    let init_params = InitializeParams {
        init_amount_0,
        init_amount_1,
        open_time,
        create_accounts_proof: proof_result.create_accounts_proof,
        lp_mint_signer_bump: pdas.lp_mint_signer_bump,
        creator_lp_token_bump: pdas.creator_lp_token_bump,
        authority_bump: pdas.authority_bump,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::InitializePool {
        creator: ctx.creator.pubkey(),
        amm_config: ctx.amm_config.pubkey(),
        authority: pdas.authority,
        pool_state: pdas.pool_state,
        token_0_mint: ctx.token_0_mint,
        token_1_mint: ctx.token_1_mint,
        lp_mint_signer: pdas.lp_mint_signer,
        lp_mint: pdas.lp_mint,
        creator_token_0: ctx.creator_token_0,
        creator_token_1: ctx.creator_token_1,
        creator_lp_token: pdas.creator_lp_token,
        token_0_vault: pdas.token_0_vault,
        token_1_vault: pdas.token_1_vault,
        observation_state: pdas.observation_state,
        token_program: LIGHT_TOKEN_PROGRAM_ID,
        token_0_program: LIGHT_TOKEN_PROGRAM_ID,
        token_1_program: LIGHT_TOKEN_PROGRAM_ID,
        associated_token_program: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
        compression_config: ctx.config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        light_token_cpi_authority: LIGHT_TOKEN_CPI_AUTHORITY,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::InitializePool {
        params: init_params,
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
            &[&ctx.payer, &ctx.creator],
        )
        .await
        .expect("Initialize pool should succeed");

    assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.lp_mint).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.creator_lp_token).await;

    let lp_token_data = parse_token(
        &ctx.rpc
            .get_account(pdas.creator_lp_token)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let initial_lp_balance = lp_token_data.amount;
    assert!(
        initial_lp_balance > 0,
        "Creator should have received LP tokens"
    );

    // Deposit
    let deposit_amount = 500u64;

    let deposit_accounts = csdk_anchor_full_derived_test::accounts::Deposit {
        owner: ctx.creator.pubkey(),
        authority: pdas.authority,
        pool_state: pdas.pool_state,
        owner_lp_token: pdas.creator_lp_token,
        token_0_account: ctx.creator_token_0,
        token_1_account: ctx.creator_token_1,
        token_0_vault: pdas.token_0_vault,
        token_1_vault: pdas.token_1_vault,
        vault_0_mint: ctx.token_0_mint,
        vault_1_mint: ctx.token_1_mint,
        lp_mint: pdas.lp_mint,
        token_program: LIGHT_TOKEN_PROGRAM_ID,
        token_program_2022: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
    };

    let deposit_instruction_data = csdk_anchor_full_derived_test::instruction::Deposit {
        lp_token_amount: deposit_amount,
    };

    let deposit_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: deposit_accounts.to_account_metas(None),
        data: deposit_instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(
            &[deposit_instruction],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.creator],
        )
        .await
        .expect("Deposit should succeed");

    // Verify LP balance after deposit
    let lp_token_data_after_deposit = parse_token(
        &ctx.rpc
            .get_account(pdas.creator_lp_token)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_balance_after_deposit = initial_lp_balance + deposit_amount;
    assert_eq!(
        lp_token_data_after_deposit.amount, expected_balance_after_deposit,
        "LP balance should increase after deposit"
    );

    // Withdraw
    let withdraw_amount = 200u64;

    let withdraw_accounts = csdk_anchor_full_derived_test::accounts::Withdraw {
        owner: ctx.creator.pubkey(),
        authority: pdas.authority,
        pool_state: pdas.pool_state,
        owner_lp_token: pdas.creator_lp_token,
        token_0_account: ctx.creator_token_0,
        token_1_account: ctx.creator_token_1,
        token_0_vault: pdas.token_0_vault,
        token_1_vault: pdas.token_1_vault,
        vault_0_mint: ctx.token_0_mint,
        vault_1_mint: ctx.token_1_mint,
        lp_mint: pdas.lp_mint,
        token_program: LIGHT_TOKEN_PROGRAM_ID,
        token_program_2022: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
    };

    let withdraw_instruction_data = csdk_anchor_full_derived_test::instruction::Withdraw {
        lp_token_amount: withdraw_amount,
    };

    let withdraw_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: withdraw_accounts.to_account_metas(None),
        data: withdraw_instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(
            &[withdraw_instruction],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.creator],
        )
        .await
        .expect("Withdraw should succeed");

    let lp_token_data_after_withdraw = parse_token(
        &ctx.rpc
            .get_account(pdas.creator_lp_token)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_balance_after_withdraw = expected_balance_after_deposit - withdraw_amount;
    assert_eq!(
        lp_token_data_after_withdraw.amount, expected_balance_after_withdraw,
        "LP balance should decrease after withdraw"
    );

    // Advance epochs to trigger auto-compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();

    // Derive compressed addresses
    let address_tree_pubkey = ctx.rpc.get_address_tree_v2().tree;

    let pool_compressed_address = light_compressed_account::address::derive_address(
        &pdas.pool_state.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &ctx.program_id.to_bytes(),
    );
    let observation_compressed_address = light_compressed_account::address::derive_address(
        &pdas.observation_state.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &ctx.program_id.to_bytes(),
    );
    let mint_compressed_address =
        light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
            &pdas.lp_mint_signer,
            &address_tree_pubkey,
        );

    // Assert compression (assert_after_compression)
    assert_onchain_closed(&mut ctx.rpc, &pdas.pool_state).await;
    assert_onchain_closed(&mut ctx.rpc, &pdas.observation_state).await;
    assert_onchain_closed(&mut ctx.rpc, &pdas.lp_mint).await;
    assert_onchain_closed(&mut ctx.rpc, &pdas.token_0_vault).await;
    assert_onchain_closed(&mut ctx.rpc, &pdas.token_1_vault).await;
    assert_onchain_closed(&mut ctx.rpc, &pdas.creator_lp_token).await;

    // Verify compressed accounts exist with non-empty data
    assert_compressed_exists_with_data(&mut ctx.rpc, pool_compressed_address).await;
    assert_compressed_exists_with_data(&mut ctx.rpc, observation_compressed_address).await;
    assert_compressed_exists_with_data(&mut ctx.rpc, mint_compressed_address).await;

    // Verify compressed token accounts
    assert_compressed_token_exists(&mut ctx.rpc, &pdas.token_0_vault, 0).await;
    assert_compressed_token_exists(&mut ctx.rpc, &pdas.token_1_vault, 0).await;
    assert_compressed_token_exists(
        &mut ctx.rpc,
        &pdas.creator_lp_token,
        expected_balance_after_withdraw,
    )
    .await;

    let pool_interface = ctx
        .rpc
        .get_account_interface(&pdas.pool_state, &ctx.program_id)
        .await
        .expect("failed to get pool_state");
    assert!(pool_interface.is_cold(), "pool_state should be cold");

    // Create Program Interface SDK.
    let mut sdk = AmmSdk::from_keyed_accounts(&[pool_interface])
        .expect("ProgrammSdk::from_keyed_accounts should succeed");

    let accounts_to_fetch = sdk.get_accounts_for_instruction(AmmInstruction::Deposit);

    let keyed_accounts = ctx
        .rpc
        .get_multiple_account_interfaces(&accounts_to_fetch)
        .await
        .expect("get_multiple_account_interfaces should succeed");

    sdk.update_with_interfaces(&keyed_accounts)
        .expect("sdk.update should succeed");

    let specs = sdk.get_specs_for_instruction(AmmInstruction::Deposit);

    let creator_lp_interface = ctx
        .rpc
        .get_ata_interface(&ctx.creator.pubkey(), &pdas.lp_mint)
        .await
        .expect("failed to get creator_lp_token");

    // add ata
    use light_client::interface::AccountSpec;
    let mut all_specs = specs;
    all_specs.push(AccountSpec::Ata(creator_lp_interface));

    let decompress_ixs = create_load_instructions(
        &all_specs,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .expect("create_load_instructions should succeed");

    ctx.rpc
        .create_and_send_transaction(
            &decompress_ixs,
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.creator],
        )
        .await
        .expect("Decompression should succeed");

    assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.lp_mint).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.creator_lp_token).await;

    // Verify LP token balance
    let lp_token_after_decompression = parse_token(
        &ctx.rpc
            .get_account(pdas.creator_lp_token)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    assert_eq!(
        lp_token_after_decompression.amount, expected_balance_after_withdraw,
        "LP token balance should be preserved after decompression"
    );

    // Verify compressed token accounts
    let remaining_vault_0 = ctx
        .rpc
        .get_compressed_token_accounts_by_owner(&pdas.token_0_vault, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        remaining_vault_0.is_empty(),
        "Compressed token_0_vault should be consumed"
    );

    let remaining_vault_1 = ctx
        .rpc
        .get_compressed_token_accounts_by_owner(&pdas.token_1_vault, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        remaining_vault_1.is_empty(),
        "Compressed token_1_vault should be consumed"
    );

    let remaining_creator_lp = ctx
        .rpc
        .get_compressed_token_accounts_by_owner(&pdas.creator_lp_token, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        remaining_creator_lp.is_empty(),
        "Compressed creator_lp_token should be consumed"
    );
}

/// Aggregator-style test flow demonstrating LightAmmInterface usage.
///
/// This test simulates how a DEX aggregator (like Jupiter) would:
/// 1. Discover a pool via pool_state account
/// 2. Use get_swap_accounts() to know what to fetch/cache
/// 3. Detect cold accounts via swap_needs_loading()
/// 4. Use get_cold_swap_specs() to build load instructions
/// 5. Execute load + swap atomically
#[tokio::test]
async fn test_aggregator_flow() {
    let mut ctx = setup().await;

    let pdas = derive_amm_pdas(
        &ctx.program_id,
        &ctx.amm_config.pubkey(),
        &ctx.token_0_mint,
        &ctx.token_1_mint,
        &ctx.creator.pubkey(),
    );

    // Initialize pool (same as above)
    let proof_inputs = AmmSdk::create_initialize_pool_proof_inputs(
        pdas.pool_state,
        pdas.observation_state,
        pdas.lp_mint,
    );
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, proof_inputs)
        .await
        .unwrap();

    let init_params = InitializeParams {
        init_amount_0: 1000u64,
        init_amount_1: 1000u64,
        open_time: 0u64,
        create_accounts_proof: proof_result.create_accounts_proof,
        lp_mint_signer_bump: pdas.lp_mint_signer_bump,
        creator_lp_token_bump: pdas.creator_lp_token_bump,
        authority_bump: pdas.authority_bump,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::InitializePool {
        creator: ctx.creator.pubkey(),
        amm_config: ctx.amm_config.pubkey(),
        authority: pdas.authority,
        pool_state: pdas.pool_state,
        token_0_mint: ctx.token_0_mint,
        token_1_mint: ctx.token_1_mint,
        lp_mint_signer: pdas.lp_mint_signer,
        lp_mint: pdas.lp_mint,
        creator_token_0: ctx.creator_token_0,
        creator_token_1: ctx.creator_token_1,
        creator_lp_token: pdas.creator_lp_token,
        token_0_vault: pdas.token_0_vault,
        token_1_vault: pdas.token_1_vault,
        observation_state: pdas.observation_state,
        token_program: LIGHT_TOKEN_PROGRAM_ID,
        token_0_program: LIGHT_TOKEN_PROGRAM_ID,
        token_1_program: LIGHT_TOKEN_PROGRAM_ID,
        associated_token_program: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
        compression_config: ctx.config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        light_token_cpi_authority: LIGHT_TOKEN_CPI_AUTHORITY,
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: csdk_anchor_full_derived_test::instruction::InitializePool {
            params: init_params,
        }
        .data(),
    };

    ctx.rpc
        .create_and_send_transaction(
            &[instruction],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.creator],
        )
        .await
        .expect("Initialize pool should succeed");

    // ==========================================================================
    // AGGREGATOR FLOW: Pool is now hot - simulate aggregator discovering it
    // ==========================================================================

    // Step 1: Aggregator discovers pool via pool_state pubkey
    let pool_interface = ctx
        .rpc
        .get_account_interface(&pdas.pool_state, &ctx.program_id)
        .await
        .expect("failed to get pool_state");

    // Step 2: Create SDK from pool discovery
    let mut sdk = AmmSdk::from_keyed_accounts(&[pool_interface])
        .expect("AmmSdk::from_keyed_accounts should succeed");

    // Step 3: Use LightAmmInterface methods to get swap-relevant accounts
    let swap_accounts = sdk.get_swap_accounts();
    assert!(!swap_accounts.is_empty(), "Swap should require accounts");

    // Step 4: Fetch all swap accounts
    let keyed_accounts = ctx
        .rpc
        .get_multiple_account_interfaces(&swap_accounts)
        .await
        .expect("get_multiple_account_interfaces should succeed");

    sdk.update_with_interfaces(&keyed_accounts)
        .expect("sdk.update should succeed");

    // Step 5: Check if any swap accounts are cold (they're all hot at this point)
    assert!(
        !sdk.swap_needs_loading(),
        "Fresh pool should have all hot accounts"
    );

    // ==========================================================================
    // AGGREGATOR FLOW: Accounts go cold - simulate compression
    // ==========================================================================

    // Advance epochs to trigger auto-compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();

    // Re-fetch pool to check if cold
    let pool_interface = ctx
        .rpc
        .get_account_interface(&pdas.pool_state, &ctx.program_id)
        .await
        .expect("failed to get pool_state after compression");

    assert!(pool_interface.is_cold(), "pool_state should be cold now");

    // Re-initialize SDK with cold pool
    let mut sdk = AmmSdk::from_keyed_accounts(&[pool_interface])
        .expect("AmmSdk::from_keyed_accounts should succeed with cold account");

    // Fetch swap accounts again (now cold)
    let swap_accounts = sdk.get_swap_accounts();
    let keyed_accounts = ctx
        .rpc
        .get_multiple_account_interfaces(&swap_accounts)
        .await
        .expect("get_multiple_account_interfaces should succeed");

    sdk.update_with_interfaces(&keyed_accounts)
        .expect("sdk.update should succeed");

    // Step 6: Check cold status using LightAmmInterface
    assert!(
        sdk.swap_needs_loading(),
        "Compressed pool should need loading for swap"
    );

    // Step 7: Get cold specs for swap (lean, no redundant Account data)
    let cold_specs = sdk.get_cold_swap_specs();
    assert!(!cold_specs.is_empty(), "Should have cold specs for swap");

    // Verify cold specs have the expected keys
    let cold_keys: std::collections::HashSet<_> = cold_specs.iter().map(|s| s.key()).collect();
    assert!(
        cold_keys.contains(&pdas.pool_state),
        "Cold specs should include pool_state"
    );
    assert!(
        cold_keys.contains(&pdas.token_0_vault),
        "Cold specs should include token_0_vault"
    );
    assert!(
        cold_keys.contains(&pdas.token_1_vault),
        "Cold specs should include token_1_vault"
    );
    assert!(
        cold_keys.contains(&pdas.observation_state),
        "Cold specs should include observation_key"
    );

    // Step 8: Build load instructions from full specs
    let specs = sdk.get_swap_specs();
    let load_ixs = create_load_instructions(
        &specs,
        ctx.payer.pubkey(),
        ctx.config_pda,
        ctx.payer.pubkey(),
        &ctx.rpc,
    )
    .await
    .expect("create_load_instructions should succeed");

    assert!(
        !load_ixs.is_empty(),
        "Should have load instructions for cold accounts"
    );

    // Step 9: Execute load instructions
    ctx.rpc
        .create_and_send_transaction(&load_ixs, &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Load instructions should succeed");

    // Verify accounts are now on-chain
    assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state).await;

    // Step 10: Re-fetch and verify no longer needs loading
    let keyed_accounts = ctx
        .rpc
        .get_multiple_account_interfaces(&sdk.get_swap_accounts())
        .await
        .expect("get_multiple_account_interfaces should succeed");

    sdk.update_with_interfaces(&keyed_accounts)
        .expect("sdk.update should succeed");

    assert!(
        !sdk.swap_needs_loading(),
        "After decompression, swap should not need loading"
    );
}

/// Jupiter Amm trait test - exercises the actual Jupiter interface.
///
/// This test demonstrates how Jupiter would use the AmmSdk:
/// 1. Discover pool via KeyedAccount
/// 2. Use Amm::from_keyed_account() to create SDK
/// 3. Use Amm::get_accounts_to_update() and Amm::update()
/// 4. Use Amm::quote() to get a swap quote
/// 5. Use Amm::get_swap_and_account_metas() to build swap instruction
#[tokio::test]
async fn test_jupiter_amm_trait() {
    let mut ctx = setup().await;

    let pdas = derive_amm_pdas(
        &ctx.program_id,
        &ctx.amm_config.pubkey(),
        &ctx.token_0_mint,
        &ctx.token_1_mint,
        &ctx.creator.pubkey(),
    );

    // Initialize pool
    let proof_inputs = AmmSdk::create_initialize_pool_proof_inputs(
        pdas.pool_state,
        pdas.observation_state,
        pdas.lp_mint,
    );
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, proof_inputs)
        .await
        .unwrap();

    let init_params = InitializeParams {
        init_amount_0: 10_000u64,
        init_amount_1: 10_000u64,
        open_time: 0u64,
        create_accounts_proof: proof_result.create_accounts_proof,
        lp_mint_signer_bump: pdas.lp_mint_signer_bump,
        creator_lp_token_bump: pdas.creator_lp_token_bump,
        authority_bump: pdas.authority_bump,
    };

    let accounts = csdk_anchor_full_derived_test::accounts::InitializePool {
        creator: ctx.creator.pubkey(),
        amm_config: ctx.amm_config.pubkey(),
        authority: pdas.authority,
        pool_state: pdas.pool_state,
        token_0_mint: ctx.token_0_mint,
        token_1_mint: ctx.token_1_mint,
        lp_mint_signer: pdas.lp_mint_signer,
        lp_mint: pdas.lp_mint,
        creator_token_0: ctx.creator_token_0,
        creator_token_1: ctx.creator_token_1,
        creator_lp_token: pdas.creator_lp_token,
        token_0_vault: pdas.token_0_vault,
        token_1_vault: pdas.token_1_vault,
        observation_state: pdas.observation_state,
        token_program: LIGHT_TOKEN_PROGRAM_ID,
        token_0_program: LIGHT_TOKEN_PROGRAM_ID,
        token_1_program: LIGHT_TOKEN_PROGRAM_ID,
        associated_token_program: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
        compression_config: ctx.config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        light_token_cpi_authority: LIGHT_TOKEN_CPI_AUTHORITY,
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: csdk_anchor_full_derived_test::instruction::InitializePool {
            params: init_params,
        }
        .data(),
    };

    ctx.rpc
        .create_and_send_transaction(
            &[instruction],
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ctx.creator],
        )
        .await
        .expect("Initialize pool should succeed");

    // ==========================================================================
    // JUPITER AMM TRAIT FLOW
    // ==========================================================================

    // Step 1: Jupiter discovers pool - fetches pool_state account
    let pool_account = ctx
        .rpc
        .get_account(pdas.pool_state)
        .await
        .unwrap()
        .expect("Pool state should exist");

    // Step 2: Create KeyedAccount (Jupiter's input format)
    let keyed_account = KeyedAccount {
        key: pdas.pool_state,
        account: pool_account,
        params: None,
    };

    // Step 3: Use Amm::from_keyed_account() - Jupiter's entry point
    let amm_context = AmmContext {
        clock_ref: Default::default(),
    };
    let mut amm = AmmSdk::from_keyed_account(&keyed_account, &amm_context)
        .expect("Amm::from_keyed_account should succeed");

    // Verify identity methods
    assert_eq!(amm.label(), "LightAMM");
    assert_eq!(amm.program_id(), ctx.program_id);
    assert_eq!(amm.key(), pdas.pool_state);
    assert!(amm.is_active());

    // Verify reserve mints
    let reserve_mints = amm.get_reserve_mints();
    assert_eq!(reserve_mints.len(), 2);
    assert!(reserve_mints.contains(&ctx.token_0_mint));
    assert!(reserve_mints.contains(&ctx.token_1_mint));

    // Step 4: Get accounts to update (Jupiter calls this)
    let accounts_to_update = amm.get_accounts_to_update();
    assert!(!accounts_to_update.is_empty(), "Should have accounts to update");

    // Step 5: Fetch accounts and update (Jupiter's cache update)
    let fetched_accounts = ctx
        .rpc
        .get_multiple_accounts(&accounts_to_update)
        .await
        .unwrap();

    let account_map: jupiter_amm_interface::AccountMap = accounts_to_update
        .iter()
        .zip(fetched_accounts.iter())
        .filter_map(|(pubkey, opt_account)| {
            opt_account.as_ref().map(|account| (*pubkey, account.clone()))
        })
        .collect();

    Amm::update(&mut amm, &account_map).expect("Amm::update should succeed");

    // Step 6: Get a quote (Jupiter's quoting)
    // Note: The test AMM doesn't transfer tokens in initialize, so vaults have 0 balance
    // This tests the quote interface works, even with empty pools
    let quote_params = QuoteParams {
        amount: 100,
        input_mint: ctx.token_0_mint,
        output_mint: ctx.token_1_mint,
        swap_mode: SwapMode::ExactIn,
    };

    let quote = amm.quote(&quote_params).expect("Amm::quote should succeed");

    assert_eq!(quote.in_amount, 100, "Input amount should match");
    assert_eq!(quote.fee_mint, ctx.token_0_mint, "Fee mint should be input mint");
    // With 0/0 reserves, output is 0 (empty pool)
    assert_eq!(quote.out_amount, 0, "Empty pool should return 0 output");

    // Step 7: Build swap instruction (Jupiter's swap building)
    let jupiter_program_id = solana_pubkey::pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
    let swap_params = SwapParams {
        swap_mode: SwapMode::ExactIn,
        in_amount: quote.in_amount,
        out_amount: quote.out_amount,
        source_mint: ctx.token_0_mint,
        destination_mint: ctx.token_1_mint,
        source_token_account: ctx.creator_token_0,
        destination_token_account: ctx.creator_token_1,
        token_transfer_authority: ctx.creator.pubkey(),
        quote_mint_to_referrer: None,
        jupiter_program_id: &jupiter_program_id,
        missing_dynamic_accounts_as_default: false,
    };

    let swap_result = amm
        .get_swap_and_account_metas(&swap_params)
        .expect("Amm::get_swap_and_account_metas should succeed");

    // Verify swap instruction structure
    assert!(
        !swap_result.account_metas.is_empty(),
        "Swap should have account metas"
    );

    // Verify expected accounts are in the metas
    let account_keys: Vec<Pubkey> = swap_result
        .account_metas
        .iter()
        .map(|m| m.pubkey)
        .collect();

    assert!(
        account_keys.contains(&pdas.pool_state),
        "Swap accounts should include pool_state"
    );
    assert!(
        account_keys.contains(&pdas.token_0_vault),
        "Swap accounts should include token_0_vault"
    );
    assert!(
        account_keys.contains(&pdas.token_1_vault),
        "Swap accounts should include token_1_vault"
    );
    assert!(
        account_keys.contains(&pdas.observation_state),
        "Swap accounts should include observation_state"
    );

    // Step 8: Verify clone_amm works
    let cloned = amm.clone_amm();
    assert_eq!(cloned.key(), pdas.pool_state, "Cloned AMM should have same key");
    assert_eq!(cloned.label(), "LightAMM", "Cloned AMM should have same label");
}
