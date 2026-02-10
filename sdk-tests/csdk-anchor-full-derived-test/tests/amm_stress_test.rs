/// AMM Stress Test: 100-Iteration Compression/Decompression Cycles
///
/// Tests repeated cycles of:
/// 1. Decompress all accounts
/// 2. Assert cached state matches on-chain state
/// 3. Perform randomized operations (deposit, withdraw, swap)
/// 4. Update cache from on-chain state
/// 5. Compress all accounts
mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::amm_test::{
    InitializeParams, ObservationState, PoolState, TradeDirection, AUTH_SEED, OBSERVATION_SEED,
    POOL_LP_MINT_SIGNER_SEED, POOL_SEED, POOL_VAULT_SEED,
};
use csdk_anchor_full_derived_test_sdk::{AmmInstruction, AmmSdk};
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountSpec, CreateAccountsProofInput,
    InitializeRentFreeConfig, LightProgramInterface, TokenAccountInterface,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    ProgramTestConfig, Rpc,
};
use light_token::instruction::{
    find_mint_address, get_associated_token_address_and_bump, LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_RENT_SPONSOR,
};
use light_token_interface::state::token::Token;
use rand::{prelude::*, rngs::StdRng, SeedableRng};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn parse_token(data: &[u8]) -> Token {
    borsh::BorshDeserialize::deserialize(&mut &data[..]).unwrap()
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

/// Cached state for all AMM accounts
#[derive(Clone, Debug)]
struct CachedState {
    pool_state: PoolState,
    obs_state: ObservationState,
    creator_lp_token: Token,
    token_0_vault: Token,
    token_1_vault: Token,
}

/// Setup the test environment with light mints
async fn setup() -> AmmTestContext {
    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    )
    .with_decoders(vec![
        Box::new(csdk_anchor_full_derived_test::CsdkTestInstructionDecoder),
        Box::new(csdk_anchor_full_derived_test::CsdkAnchorFullDerivedTestInstructionDecoder),
    ]);
    // Use larger queues (batch_size=500) to avoid queue full errors during 100 iterations.
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::e2e_test_default());
    config.v2_address_tree_config =
        Some(InitAddressTreeAccountsInstructionData::e2e_test_default());

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

    let creator = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &creator.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let (mint_a, _compression_addr_a, ata_pubkeys_a, _mint_seed_a) = shared::setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(10_000_000, creator.pubkey())],
    )
    .await;

    let (mint_b, _compression_addr_b, ata_pubkeys_b, _mint_seed_b) = shared::setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(10_000_000, creator.pubkey())],
    )
    .await;

    let (token_0_mint, token_1_mint, creator_token_0, creator_token_1) = if mint_a < mint_b {
        (mint_a, mint_b, ata_pubkeys_a[0], ata_pubkeys_b[0])
    } else {
        (mint_b, mint_a, ata_pubkeys_b[0], ata_pubkeys_a[0])
    };

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
    let (pool_state, pool_state_bump) = Pubkey::find_program_address(
        &[
            POOL_SEED.as_bytes(),
            amm_config.as_ref(),
            token_0_mint.as_ref(),
            token_1_mint.as_ref(),
        ],
        program_id,
    );

    let (authority, authority_bump) =
        Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], program_id);

    let (observation_state, observation_state_bump) = Pubkey::find_program_address(
        &[OBSERVATION_SEED.as_bytes(), pool_state.as_ref()],
        program_id,
    );

    let (token_0_vault, token_0_vault_bump) = Pubkey::find_program_address(
        &[
            POOL_VAULT_SEED.as_bytes(),
            pool_state.as_ref(),
            token_0_mint.as_ref(),
        ],
        program_id,
    );

    let (token_1_vault, token_1_vault_bump) = Pubkey::find_program_address(
        &[
            POOL_VAULT_SEED.as_bytes(),
            pool_state.as_ref(),
            token_1_mint.as_ref(),
        ],
        program_id,
    );

    let (lp_mint_signer, lp_mint_signer_bump) =
        Pubkey::find_program_address(&[POOL_LP_MINT_SIGNER_SEED, pool_state.as_ref()], program_id);

    let (lp_mint, _) = find_mint_address(&lp_mint_signer);

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

// --- Instruction builders ---

fn build_deposit_ix(ctx: &AmmTestContext, pdas: &AmmPdas, amount: u64) -> Instruction {
    let accounts = csdk_anchor_full_derived_test::accounts::Deposit {
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
    Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: csdk_anchor_full_derived_test::instruction::Deposit {
            lp_token_amount: amount,
        }
        .data(),
    }
}

fn build_withdraw_ix(ctx: &AmmTestContext, pdas: &AmmPdas, amount: u64) -> Instruction {
    let accounts = csdk_anchor_full_derived_test::accounts::Withdraw {
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
    Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: csdk_anchor_full_derived_test::instruction::Withdraw {
            lp_token_amount: amount,
        }
        .data(),
    }
}

fn build_swap_ix(ctx: &AmmTestContext, pdas: &AmmPdas, direction: TradeDirection) -> Instruction {
    let (
        input_vault,
        output_vault,
        input_mint,
        output_mint,
        input_token_account,
        output_token_account,
    ) = match direction {
        TradeDirection::ZeroForOne => (
            pdas.token_0_vault,
            pdas.token_1_vault,
            ctx.token_0_mint,
            ctx.token_1_mint,
            ctx.creator_token_0,
            ctx.creator_token_1,
        ),
        TradeDirection::OneForZero => (
            pdas.token_1_vault,
            pdas.token_0_vault,
            ctx.token_1_mint,
            ctx.token_0_mint,
            ctx.creator_token_1,
            ctx.creator_token_0,
        ),
    };
    let accounts = csdk_anchor_full_derived_test::accounts::Swap {
        payer: ctx.creator.pubkey(),
        authority: pdas.authority,
        pool_state: pdas.pool_state,
        input_token_account,
        output_token_account,
        input_vault,
        output_vault,
        input_token_program: LIGHT_TOKEN_PROGRAM_ID,
        output_token_program: LIGHT_TOKEN_PROGRAM_ID,
        input_token_mint: input_mint,
        output_token_mint: output_mint,
        observation_state: pdas.observation_state,
    };
    Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: csdk_anchor_full_derived_test::instruction::Swap {
            amount_in: 100,
            minimum_amount_out: 0,
            direction,
        }
        .data(),
    }
}

// --- Lifecycle helpers ---

/// Initialize the AMM pool
async fn initialize_pool(ctx: &mut AmmTestContext, pdas: &AmmPdas) {
    let proof_result = get_create_accounts_proof(
        &ctx.rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(pdas.pool_state),
            CreateAccountsProofInput::pda(pdas.observation_state),
            CreateAccountsProofInput::mint(pdas.lp_mint_signer),
        ],
    )
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
        pda_rent_sponsor: csdk_anchor_full_derived_test::program_rent_sponsor(),
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
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

    for (pda, name) in [
        (&pdas.pool_state, "pool_state"),
        (&pdas.observation_state, "observation_state"),
        (&pdas.lp_mint, "lp_mint"),
        (&pdas.token_0_vault, "token_0_vault"),
        (&pdas.token_1_vault, "token_1_vault"),
        (&pdas.creator_lp_token, "creator_lp_token"),
    ] {
        shared::assert_onchain_exists(&mut ctx.rpc, pda, name).await;
    }
}

/// Re-read all on-chain accounts into the cache
async fn refresh_cache(rpc: &mut LightProgramTest, pdas: &AmmPdas) -> CachedState {
    let pool_account = rpc.get_account(pdas.pool_state).await.unwrap().unwrap();
    let pool_state: PoolState =
        anchor_lang::AccountDeserialize::try_deserialize(&mut &pool_account.data[..]).unwrap();

    let obs_account = rpc
        .get_account(pdas.observation_state)
        .await
        .unwrap()
        .unwrap();
    let obs_state: ObservationState =
        anchor_lang::AccountDeserialize::try_deserialize(&mut &obs_account.data[..]).unwrap();

    let creator_lp_token = parse_token(
        &rpc.get_account(pdas.creator_lp_token)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let token_0_vault = parse_token(
        &rpc.get_account(pdas.token_0_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let token_1_vault = parse_token(
        &rpc.get_account(pdas.token_1_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    );

    CachedState {
        pool_state,
        obs_state,
        creator_lp_token,
        token_0_vault,
        token_1_vault,
    }
}

/// Decompress all AMM accounts using the SDK interface
async fn decompress_all(ctx: &mut AmmTestContext, pdas: &AmmPdas) {
    let pool_interface = ctx
        .rpc
        .get_account_interface(&pdas.pool_state, None)
        .await
        .expect("failed to get pool_state")
        .value
        .expect("pool_state should exist");
    assert!(pool_interface.is_cold(), "pool_state should be cold");

    let sdk = AmmSdk::new(pdas.pool_state, pool_interface.data())
        .expect("AmmSdk::new should succeed");

    let pubkeys = sdk.instruction_accounts(&AmmInstruction::Deposit);
    let account_interfaces = ctx
        .rpc
        .get_multiple_account_interfaces(pubkeys.iter().collect(), None)
        .await
        .expect("get_multiple_account_interfaces should succeed");
    let cold_accounts: Vec<_> = account_interfaces
        .value
        .into_iter()
        .flatten()
        .filter(|a| a.is_cold())
        .collect();

    let specs = sdk.load_specs(&cold_accounts).expect("load_specs should succeed");

    let creator_lp_interface: TokenAccountInterface = ctx
        .rpc
        .get_account_interface(&pdas.creator_lp_token, None)
        .await
        .expect("failed to get creator_lp_token")
        .value
        .expect("creator_lp_token should exist")
        .try_into()
        .expect("should convert to TokenAccountInterface");

    // Creator's token_0 and token_1 ATAs also get compressed during epoch warp
    let creator_token_0_interface: TokenAccountInterface = ctx
        .rpc
        .get_account_interface(&ctx.creator_token_0, None)
        .await
        .expect("failed to get creator_token_0")
        .value
        .expect("creator_token_0 should exist")
        .try_into()
        .expect("should convert to TokenAccountInterface");

    let creator_token_1_interface: TokenAccountInterface = ctx
        .rpc
        .get_account_interface(&ctx.creator_token_1, None)
        .await
        .expect("failed to get creator_token_1")
        .value
        .expect("creator_token_1 should exist")
        .try_into()
        .expect("should convert to TokenAccountInterface");

    let mint_0_account_iface = ctx
        .rpc
        .get_account_interface(&ctx.token_0_mint, None)
        .await
        .expect("failed to get token_0_mint")
        .value
        .expect("token_0_mint should exist");

    let mint_1_account_iface = ctx
        .rpc
        .get_account_interface(&ctx.token_1_mint, None)
        .await
        .expect("failed to get token_1_mint")
        .value
        .expect("token_1_mint should exist");

    let mut all_specs = specs;
    all_specs.push(AccountSpec::Ata(creator_lp_interface));
    all_specs.push(AccountSpec::Ata(creator_token_0_interface));
    all_specs.push(AccountSpec::Ata(creator_token_1_interface));
    all_specs.push(AccountSpec::Mint(mint_0_account_iface));
    all_specs.push(AccountSpec::Mint(mint_1_account_iface));

    let decompress_ixs =
        create_load_instructions(&all_specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
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

    for (pda, name) in [
        (&pdas.pool_state, "pool_state"),
        (&pdas.observation_state, "observation_state"),
        (&pdas.lp_mint, "lp_mint"),
        (&pdas.token_0_vault, "token_0_vault"),
        (&pdas.token_1_vault, "token_1_vault"),
        (&pdas.creator_lp_token, "creator_lp_token"),
        (&ctx.creator_token_0, "creator_token_0"),
        (&ctx.creator_token_1, "creator_token_1"),
        (&ctx.token_0_mint, "token_0_mint"),
        (&ctx.token_1_mint, "token_1_mint"),
    ] {
        shared::assert_onchain_exists(&mut ctx.rpc, pda, name).await;
    }
}

/// Compress all AMM accounts by warping forward epochs
async fn compress_all(ctx: &mut AmmTestContext, pdas: &AmmPdas, cached: &CachedState) {
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 100)
        .await
        .unwrap();

    for (pda, name) in [
        (&pdas.pool_state, "pool_state"),
        (&pdas.observation_state, "observation_state"),
        (&pdas.lp_mint, "lp_mint"),
        (&pdas.token_0_vault, "token_0_vault"),
        (&pdas.token_1_vault, "token_1_vault"),
        (&pdas.creator_lp_token, "creator_lp_token"),
    ] {
        shared::assert_onchain_closed(&mut ctx.rpc, pda, name).await;
    }

    shared::assert_compressed_token_exists(&mut ctx.rpc, &pdas.token_0_vault, 0, "token_0_vault")
        .await;
    shared::assert_compressed_token_exists(&mut ctx.rpc, &pdas.token_1_vault, 0, "token_1_vault")
        .await;
    shared::assert_compressed_token_exists(
        &mut ctx.rpc,
        &pdas.creator_lp_token,
        cached.creator_lp_token.amount,
        "creator_lp_token",
    )
    .await;
}

/// Full-struct assertions for all accounts against cached state
async fn assert_all_state(
    rpc: &mut LightProgramTest,
    pdas: &AmmPdas,
    cached: &CachedState,
    iteration: usize,
) {
    // PoolState
    let pool_account = rpc.get_account(pdas.pool_state).await.unwrap().unwrap();
    let actual_pool: PoolState =
        anchor_lang::AccountDeserialize::try_deserialize(&mut &pool_account.data[..]).unwrap();
    let expected_pool = PoolState {
        compression_info: shared::expected_compression_info(&actual_pool.compression_info),
        ..cached.pool_state.clone()
    };
    assert_eq!(
        actual_pool, expected_pool,
        "PoolState mismatch at iteration {iteration}"
    );

    // ObservationState
    let obs_account = rpc
        .get_account(pdas.observation_state)
        .await
        .unwrap()
        .unwrap();
    let actual_obs: ObservationState =
        anchor_lang::AccountDeserialize::try_deserialize(&mut &obs_account.data[..]).unwrap();
    let expected_obs = ObservationState {
        compression_info: shared::expected_compression_info(&actual_obs.compression_info),
        ..cached.obs_state.clone()
    };
    assert_eq!(
        actual_obs, expected_obs,
        "ObservationState mismatch at iteration {iteration}"
    );

    // Token accounts
    let actual_lp = parse_token(
        &rpc.get_account(pdas.creator_lp_token)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_lp = Token {
        extensions: actual_lp.extensions.clone(),
        ..cached.creator_lp_token.clone()
    };
    assert_eq!(
        actual_lp, expected_lp,
        "creator_lp_token mismatch at iteration {iteration}"
    );

    let actual_v0 = parse_token(
        &rpc.get_account(pdas.token_0_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_v0 = Token {
        extensions: actual_v0.extensions.clone(),
        ..cached.token_0_vault.clone()
    };
    assert_eq!(
        actual_v0, expected_v0,
        "token_0_vault mismatch at iteration {iteration}"
    );

    let actual_v1 = parse_token(
        &rpc.get_account(pdas.token_1_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_v1 = Token {
        extensions: actual_v1.extensions.clone(),
        ..cached.token_1_vault.clone()
    };
    assert_eq!(
        actual_v1, expected_v1,
        "token_1_vault mismatch at iteration {iteration}"
    );
}

// --- Main test ---

#[tokio::test]
async fn test_amm_stress_100_iterations() {
    let mut ctx = setup().await;

    let pdas = derive_amm_pdas(
        &ctx.program_id,
        &ctx.amm_config.pubkey(),
        &ctx.token_0_mint,
        &ctx.token_1_mint,
        &ctx.creator.pubkey(),
    );

    // 1. Initialize pool
    initialize_pool(&mut ctx, &pdas).await;
    let mut cached = refresh_cache(&mut ctx.rpc, &pdas).await;

    // 2. First compression
    compress_all(&mut ctx, &pdas, &cached).await;
    let seed = thread_rng().next_u64();
    println!("Seed: {seed}");
    let mut rng = StdRng::seed_from_u64(seed);

    // 3. Loop iterations
    for i in 0..20 {
        println!("--- Iteration {i} ---");

        // --- DECOMPRESS ---
        decompress_all(&mut ctx, &pdas).await;

        // --- ASSERT ALL CACHED STATE ---
        assert_all_state(&mut ctx.rpc, &pdas, &cached, i).await;

        // Update cache after decompression (compression_info changes)
        cached = refresh_cache(&mut ctx.rpc, &pdas).await;

        // --- RANDOM OPERATIONS (1-3) ---
        let num_ops = rng.gen_range(1..=3);
        for j in 0..num_ops {
            match rng.gen_range(0..3u32) {
                0 => {
                    // Deposit
                    let amount = rng.gen_range(1..=500u64);
                    println!("  op {j}: deposit {amount}");
                    let ix = build_deposit_ix(&ctx, &pdas, amount);
                    ctx.rpc
                        .create_and_send_transaction(
                            &[ix],
                            &ctx.payer.pubkey(),
                            &[&ctx.payer, &ctx.creator],
                        )
                        .await
                        .expect("Deposit failed");
                    cached = refresh_cache(&mut ctx.rpc, &pdas).await;
                }
                1 => {
                    // Withdraw (skip if balance is 0)
                    if cached.creator_lp_token.amount > 0 {
                        let max = cached.creator_lp_token.amount.min(500);
                        let amount = rng.gen_range(1..=max);
                        println!("  op {j}: withdraw {amount}");
                        let ix = build_withdraw_ix(&ctx, &pdas, amount);
                        ctx.rpc
                            .create_and_send_transaction(
                                &[ix],
                                &ctx.payer.pubkey(),
                                &[&ctx.payer, &ctx.creator],
                            )
                            .await
                            .expect("Withdraw failed");
                        cached = refresh_cache(&mut ctx.rpc, &pdas).await;
                    } else {
                        println!("  op {j}: withdraw skipped (balance 0)");
                    }
                }
                _ => {
                    // Swap (no-op in this AMM, no actual state change)
                    let direction = if rng.gen_bool(0.5) {
                        TradeDirection::ZeroForOne
                    } else {
                        TradeDirection::OneForZero
                    };
                    println!("  op {j}: swap {direction:?}");
                    let ix = build_swap_ix(&ctx, &pdas, direction);
                    ctx.rpc
                        .create_and_send_transaction(
                            &[ix],
                            &ctx.payer.pubkey(),
                            &[&ctx.payer, &ctx.creator],
                        )
                        .await
                        .expect("Swap failed");
                    cached = refresh_cache(&mut ctx.rpc, &pdas).await;
                }
            }
        }

        // --- COMPRESS ---
        compress_all(&mut ctx, &pdas, &cached).await;

        println!(
            "  iteration {i} complete (lp_balance={})",
            cached.creator_lp_token.amount
        );
    }

    println!("All 100 iterations completed successfully.");
}
