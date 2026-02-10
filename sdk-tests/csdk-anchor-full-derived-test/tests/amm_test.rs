/// AMM Full Lifecycle Integration Test
///
/// Tests the complete AMM flow:
/// 1. Initialize pool with rent-free PDAs and LP mint
/// 2. Deposit tokens and receive LP tokens
/// 3. Withdraw tokens by burning LP tokens
/// 4. Advance epochs to trigger auto-compression
/// 5. Decompress all accounts
/// 6. Deposit after decompression to verify pool works
mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::amm_test::{
    InitializeParams, AUTH_SEED, OBSERVATION_SEED, POOL_LP_MINT_SIGNER_SEED, POOL_SEED,
    POOL_VAULT_SEED,
};
// SDK for AmmSdk-based approach
use csdk_anchor_full_derived_test_sdk::{AmmInstruction, AmmSdk};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, CreateAccountsProofInput,
    InitializeRentFreeConfig, LightProgramInterface,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
use light_token::instruction::{
    find_mint_address, get_associated_token_address_and_bump, LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_RENT_SPONSOR,
};
use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
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
        csdk_anchor_full_derived_test::program_rent_sponsor(),
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

    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state, "pool state").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state, "observation state").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.lp_mint, "LP mint").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault, "token 0 vault").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault, "token 1 vault").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.creator_lp_token, "creator LP token").await;

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

    // Full-struct assertion for PoolState after init
    {
        use csdk_anchor_full_derived_test::amm_test::{
            Observation, ObservationState, PoolState, OBSERVATION_NUM,
        };
        let pool_account = ctx.rpc.get_account(pdas.pool_state).await.unwrap().unwrap();
        let pool_state: PoolState =
            anchor_lang::AccountDeserialize::try_deserialize(&mut &pool_account.data[..]).unwrap();
        let expected_pool = PoolState {
            compression_info: shared::expected_compression_info(&pool_state.compression_info),
            amm_config: ctx.amm_config.pubkey(),
            pool_creator: ctx.creator.pubkey(),
            token_0_vault: pdas.token_0_vault,
            token_1_vault: pdas.token_1_vault,
            lp_mint: pdas.lp_mint,
            token_0_mint: ctx.token_0_mint,
            token_1_mint: ctx.token_1_mint,
            token_0_program: LIGHT_TOKEN_PROGRAM_ID,
            token_1_program: LIGHT_TOKEN_PROGRAM_ID,
            observation_key: pdas.observation_state,
            auth_bump: pdas.authority_bump,
            status: 1,
            lp_mint_decimals: 9,
            mint_0_decimals: 9,
            mint_1_decimals: 9,
            lp_supply: initial_lp_balance,
            protocol_fees_token_0: 0,
            protocol_fees_token_1: 0,
            fund_fees_token_0: 0,
            fund_fees_token_1: 0,
            open_time: 0,
            recent_epoch: 0,
            padding: [0; 1],
        };
        assert_eq!(
            pool_state, expected_pool,
            "PoolState should match after init"
        );

        // ObservationState assertion
        let obs_account = ctx
            .rpc
            .get_account(pdas.observation_state)
            .await
            .unwrap()
            .unwrap();
        let obs_state: ObservationState =
            anchor_lang::AccountDeserialize::try_deserialize(&mut &obs_account.data[..]).unwrap();
        let expected_obs = ObservationState {
            compression_info: shared::expected_compression_info(&obs_state.compression_info),
            initialized: false,
            observation_index: 0,
            pool_id: Pubkey::default(),
            observations: [Observation::default(); OBSERVATION_NUM],
            padding: [0; 4],
        };
        assert_eq!(
            obs_state, expected_obs,
            "ObservationState should match after init"
        );
    }

    // Full-struct Token assertions after init
    {
        let token_0_vault_data = parse_token(
            &ctx.rpc
                .get_account(pdas.token_0_vault)
                .await
                .unwrap()
                .unwrap()
                .data,
        );
        let expected_token_0 = Token {
            mint: ctx.token_0_mint.into(),
            owner: pdas.authority.into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: token_0_vault_data.extensions.clone(),
        };
        assert_eq!(
            token_0_vault_data, expected_token_0,
            "token_0_vault should match after init"
        );

        let token_1_vault_data = parse_token(
            &ctx.rpc
                .get_account(pdas.token_1_vault)
                .await
                .unwrap()
                .unwrap()
                .data,
        );
        let expected_token_1 = Token {
            mint: ctx.token_1_mint.into(),
            owner: pdas.authority.into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: token_1_vault_data.extensions.clone(),
        };
        assert_eq!(
            token_1_vault_data, expected_token_1,
            "token_1_vault should match after init"
        );
    }

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
    shared::assert_onchain_closed(&mut ctx.rpc, &pdas.pool_state, "pool_state").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &pdas.observation_state, "observation_state").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &pdas.lp_mint, "lp_mint").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &pdas.token_0_vault, "token_0_vault").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &pdas.token_1_vault, "token_1_vault").await;
    shared::assert_onchain_closed(&mut ctx.rpc, &pdas.creator_lp_token, "creator_lp_token").await;

    // Verify compressed accounts exist with non-empty data
    shared::assert_compressed_exists_with_data(&mut ctx.rpc, pool_compressed_address, "pool_state")
        .await;
    shared::assert_compressed_exists_with_data(
        &mut ctx.rpc,
        observation_compressed_address,
        "observation_state",
    )
    .await;
    shared::assert_compressed_exists_with_data(&mut ctx.rpc, mint_compressed_address, "lp_mint")
        .await;

    // Verify compressed token accounts
    shared::assert_compressed_token_exists(&mut ctx.rpc, &pdas.token_0_vault, 0, "token_0_vault")
        .await;
    shared::assert_compressed_token_exists(&mut ctx.rpc, &pdas.token_1_vault, 0, "token_1_vault")
        .await;
    shared::assert_compressed_token_exists(
        &mut ctx.rpc,
        &pdas.creator_lp_token,
        expected_balance_after_withdraw,
        "creator_lp_token",
    )
    .await;

    let pool_interface = ctx
        .rpc
        .get_account_interface(&pdas.pool_state, None)
        .await
        .expect("failed to get pool_state")
        .value
        .expect("pool_state should exist");
    assert!(pool_interface.is_cold(), "pool_state should be cold");

    // Create SDK from pool state data.
    let sdk = AmmSdk::new(pdas.pool_state, pool_interface.data())
        .expect("AmmSdk::new should succeed");

    // Fetch all instruction accounts and filter cold ones.
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

    let mut all_specs = sdk.load_specs(&cold_accounts).expect("load_specs should succeed");

    let creator_lp_account = ctx
        .rpc
        .get_account_interface(&pdas.creator_lp_token, None)
        .await
        .expect("failed to get creator_lp_token")
        .value
        .expect("creator_lp_token should exist");

    use light_client::interface::{AccountSpec, TokenAccountInterface};
    let creator_lp_interface = TokenAccountInterface::try_from(creator_lp_account)
        .expect("should convert to TokenAccountInterface");

    all_specs.push(AccountSpec::Ata(creator_lp_interface));

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

    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state, "pool_state").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state, "observation_state").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.lp_mint, "lp_mint").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault, "token_0_vault").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault, "token_1_vault").await;
    shared::assert_onchain_exists(&mut ctx.rpc, &pdas.creator_lp_token, "creator_lp_token").await;

    // Full-struct assertion for PoolState after decompression
    {
        use csdk_anchor_full_derived_test::amm_test::{
            Observation, ObservationState, PoolState, OBSERVATION_NUM,
        };
        let pool_account = ctx.rpc.get_account(pdas.pool_state).await.unwrap().unwrap();
        let pool_state: PoolState =
            anchor_lang::AccountDeserialize::try_deserialize(&mut &pool_account.data[..]).unwrap();
        let expected_pool = PoolState {
            compression_info: shared::expected_compression_info(&pool_state.compression_info),
            amm_config: ctx.amm_config.pubkey(),
            pool_creator: ctx.creator.pubkey(),
            token_0_vault: pdas.token_0_vault,
            token_1_vault: pdas.token_1_vault,
            lp_mint: pdas.lp_mint,
            token_0_mint: ctx.token_0_mint,
            token_1_mint: ctx.token_1_mint,
            token_0_program: LIGHT_TOKEN_PROGRAM_ID,
            token_1_program: LIGHT_TOKEN_PROGRAM_ID,
            observation_key: pdas.observation_state,
            auth_bump: pdas.authority_bump,
            status: 1,
            lp_mint_decimals: 9,
            mint_0_decimals: 9,
            mint_1_decimals: 9,
            lp_supply: initial_lp_balance,
            protocol_fees_token_0: 0,
            protocol_fees_token_1: 0,
            fund_fees_token_0: 0,
            fund_fees_token_1: 0,
            open_time: 0,
            recent_epoch: 0,
            padding: [0; 1],
        };
        assert_eq!(
            pool_state, expected_pool,
            "PoolState should match after decompression"
        );

        let obs_account = ctx
            .rpc
            .get_account(pdas.observation_state)
            .await
            .unwrap()
            .unwrap();
        let obs_state: ObservationState =
            anchor_lang::AccountDeserialize::try_deserialize(&mut &obs_account.data[..]).unwrap();
        let expected_obs = ObservationState {
            compression_info: shared::expected_compression_info(&obs_state.compression_info),
            initialized: false,
            observation_index: 0,
            pool_id: Pubkey::default(),
            observations: [Observation::default(); OBSERVATION_NUM],
            padding: [0; 4],
        };
        assert_eq!(
            obs_state, expected_obs,
            "ObservationState should match after decompression"
        );
    }

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

    // Verify token account owners after decompression using full struct comparison
    let token_0_vault_data = parse_token(
        &ctx.rpc
            .get_account(pdas.token_0_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_token_0_vault = Token {
        mint: ctx.token_0_mint.into(),
        owner: pdas.authority.into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: token_0_vault_data.extensions.clone(),
    };
    assert_eq!(
        token_0_vault_data, expected_token_0_vault,
        "token_0_vault should match expected after decompression"
    );

    let token_1_vault_data = parse_token(
        &ctx.rpc
            .get_account(pdas.token_1_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    );
    let expected_token_1_vault = Token {
        mint: ctx.token_1_mint.into(),
        owner: pdas.authority.into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: token_1_vault_data.extensions.clone(),
    };
    assert_eq!(
        token_1_vault_data, expected_token_1_vault,
        "token_1_vault should match expected after decompression"
    );

    let expected_creator_lp_token = Token {
        mint: pdas.lp_mint.into(),
        owner: ctx.creator.pubkey().into(),
        amount: expected_balance_after_withdraw,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: lp_token_after_decompression.extensions.clone(),
    };
    assert_eq!(
        lp_token_after_decompression, expected_creator_lp_token,
        "creator_lp_token should match expected after decompression"
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
