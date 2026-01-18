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
use csdk_anchor_full_derived_test::{
    amm_test::{
        InitializeParams, AUTH_SEED, OBSERVATION_SEED, POOL_LP_MINT_SIGNER_SEED, POOL_SEED,
        POOL_VAULT_SEED,
    },
    csdk_anchor_full_derived_test::{ObservationStateSeeds, PoolStateSeeds, TokenAccountVariant},
};
use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
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
use light_token_interface::{instructions::mint_action::MintInstructionData, state::Token};
use light_token_sdk::token::{
    find_mint_address, get_associated_token_address_and_bump, CreateAssociatedTokenAccount,
    Decompress, DecompressMint, MintWithContext, COMPRESSIBLE_CONFIG_V1, LIGHT_TOKEN_CPI_AUTHORITY,
    LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const RENT_SPONSOR: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

// =============================================================================
// Assertion Helpers
// =============================================================================

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
    // ==========================================================================
    // PHASE 1: Setup
    // ==========================================================================
    let mut ctx = setup().await;

    // ==========================================================================
    // PHASE 2: Derive PDAs
    // ==========================================================================
    let pdas = derive_amm_pdas(
        &ctx.program_id,
        &ctx.amm_config.pubkey(),
        &ctx.token_0_mint,
        &ctx.token_1_mint,
        &ctx.creator.pubkey(),
    );

    println!("Derived PDAs:");
    println!("  pool_state: {}", pdas.pool_state);
    println!("  observation_state: {}", pdas.observation_state);
    println!("  authority: {}", pdas.authority);
    println!("  token_0_vault: {}", pdas.token_0_vault);
    println!("  token_1_vault: {}", pdas.token_1_vault);
    println!("  lp_mint_signer: {}", pdas.lp_mint_signer);
    println!("  lp_mint: {}", pdas.lp_mint);
    println!("  creator_lp_token: {}", pdas.creator_lp_token);

    // ==========================================================================
    // PHASE 3: Get create accounts proof
    // ==========================================================================
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

    // ==========================================================================
    // PHASE 4: Initialize Pool
    // ==========================================================================
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
        ctoken_compressible_config: COMPRESSIBLE_CONFIG_V1,
        ctoken_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        ctoken_cpi_authority: LIGHT_TOKEN_CPI_AUTHORITY,
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

    // ==========================================================================
    // PHASE 5: Verify Initial State (assert_after_initialize)
    // ==========================================================================
    assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.lp_mint).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.creator_lp_token).await;

    // Verify creator LP token balance (should have initial LP amount from initialize)
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
    println!("Initial LP balance: {}", initial_lp_balance);

    // ==========================================================================
    // PHASE 6: Deposit
    // ==========================================================================
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

    // Verify LP balance after deposit (assert_after_deposit)
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
    println!(
        "LP balance after deposit: {} (expected: {})",
        lp_token_data_after_deposit.amount, expected_balance_after_deposit
    );

    // ==========================================================================
    // PHASE 7: Withdraw
    // ==========================================================================
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

    // Verify LP balance after withdraw (assert_after_withdraw)
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
    println!(
        "LP balance after withdraw: {} (expected: {})",
        lp_token_data_after_withdraw.amount, expected_balance_after_withdraw
    );

    // ==========================================================================
    // PHASE 8: Advance Epochs (trigger auto-compression)
    // ==========================================================================
    println!("\nAdvancing epochs to trigger auto-compression...");
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();

    // Derive compressed addresses for verification
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
        light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
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

    println!("All accounts compressed successfully!");

    // ==========================================================================
    // PHASE 9: Decompress accounts
    // ==========================================================================
    println!("\nPhase 9: Decompressing all accounts...");

    // Fetch unified interfaces (hot/cold transparent) for PDAs
    let pool_interface = ctx
        .rpc
        .get_account_info_interface(&pdas.pool_state, &ctx.program_id)
        .await
        .expect("failed to get pool_state");
    assert!(pool_interface.is_cold, "pool_state should be cold");

    let observation_interface = ctx
        .rpc
        .get_account_info_interface(&pdas.observation_state, &ctx.program_id)
        .await
        .expect("failed to get observation_state");
    assert!(
        observation_interface.is_cold,
        "observation_state should be cold"
    );

    // Fetch token account interfaces for vaults
    let vault_0_interface = ctx
        .rpc
        .get_token_account_interface(&pdas.token_0_vault)
        .await
        .expect("failed to get token_0_vault");
    assert!(vault_0_interface.is_cold, "token_0_vault should be cold");

    let vault_1_interface = ctx
        .rpc
        .get_token_account_interface(&pdas.token_1_vault)
        .await
        .expect("failed to get token_1_vault");
    assert!(vault_1_interface.is_cold, "token_1_vault should be cold");

    // Fetch ATA interface for creator LP token
    let creator_lp_interface = ctx
        .rpc
        .get_ata_interface(&ctx.creator.pubkey(), &pdas.lp_mint)
        .await
        .expect("failed to get creator_lp_token");
    assert!(
        creator_lp_interface.is_cold(),
        "creator_lp_token should be cold"
    );
    assert_eq!(
        creator_lp_interface.amount(),
        expected_balance_after_withdraw,
        "Compressed LP token amount should match"
    );

    // Fetch mint interface for LP mint
    let lp_mint_interface = ctx
        .rpc
        .get_mint_interface(&pdas.lp_mint_signer)
        .await
        .expect("failed to get lp_mint");
    assert!(lp_mint_interface.is_cold(), "lp_mint should be cold");

    // Build RentFreeDecompressAccount list for program-owned accounts
    let program_owned_accounts = vec![
        RentFreeDecompressAccount::from_seeds(
            AccountInterface::from(&pool_interface),
            PoolStateSeeds {
                amm_config: ctx.amm_config.pubkey(),
                token_0_mint: ctx.token_0_mint,
                token_1_mint: ctx.token_1_mint,
            },
        )
        .expect("PoolState seed verification failed"),
        RentFreeDecompressAccount::from_seeds(
            AccountInterface::from(&observation_interface),
            ObservationStateSeeds {
                pool_state: pdas.pool_state,
            },
        )
        .expect("ObservationState seed verification failed"),
        RentFreeDecompressAccount::from_ctoken(
            AccountInterface::from(&vault_0_interface),
            TokenAccountVariant::Token0Vault {
                pool_state: pdas.pool_state,
                token_0_mint: ctx.token_0_mint,
            },
        )
        .expect("Token0Vault construction failed"),
        RentFreeDecompressAccount::from_ctoken(
            AccountInterface::from(&vault_1_interface),
            TokenAccountVariant::Token1Vault {
                pool_state: pdas.pool_state,
                token_1_mint: ctx.token_1_mint,
            },
        )
        .expect("Token1Vault construction failed"),
    ];
    for account in program_owned_accounts {
        // Create decompression instructions
        let all_instructions = create_load_accounts_instructions(
            &[account],
            &[], //std::slice::from_ref(&creator_lp_interface.inner), TODO decompress directly from ctoken program
            &[], // std::slice::from_ref(&lp_mint_interface), TODO decompress directly from ctoken program
            ctx.program_id,
            ctx.payer.pubkey(),
            ctx.config_pda,
            ctx.payer.pubkey(), // rent_sponsor
            &ctx.rpc,
        )
        .await
        .expect("create_load_accounts_instructions should succeed");

        println!(
            "  Generated {} decompression instructions",
            all_instructions.len()
        );

        // Execute decompression
        ctx.rpc
            .create_and_send_transaction(&all_instructions, &ctx.payer.pubkey(), &[&ctx.payer])
            .await
            .expect("Decompression should succeed");
    }

    // Decompress LP mint manually
    if lp_mint_interface.is_cold() {
        println!("  Decompressing LP mint...");
        let (compressed, mint_data) = lp_mint_interface
            .compressed()
            .expect("LP mint should have compressed data");

        // Get validity proof for the mint
        let proof_result = ctx
            .rpc
            .get_validity_proof(vec![compressed.hash], vec![], None)
            .await
            .expect("get_validity_proof should succeed")
            .value;

        let account_info = &proof_result.accounts[0];
        let state_tree = account_info.tree_info.tree;
        let input_queue = account_info.tree_info.queue;
        let output_queue = account_info
            .tree_info
            .next_tree_info
            .as_ref()
            .map(|n| n.queue)
            .unwrap_or(input_queue);

        let mint_instruction_data = MintInstructionData::try_from(mint_data.clone())
            .expect("MintInstructionData conversion should succeed");

        let decompress_mint_ix = DecompressMint {
            payer: ctx.payer.pubkey(),
            authority: ctx.payer.pubkey(),
            state_tree,
            input_queue,
            output_queue,
            compressed_mint_with_context: MintWithContext {
                leaf_index: account_info.leaf_index as u32,
                prove_by_index: account_info.root_index.proof_by_index(),
                root_index: account_info.root_index.root_index().unwrap_or_default(),
                address: lp_mint_interface.compressed_address,
                mint: Some(mint_instruction_data),
            },
            proof: ValidityProof(proof_result.proof.into()),
            rent_payment: 2,
            write_top_up: 766,
        }
        .instruction()
        .expect("DecompressMint instruction should succeed");

        ctx.rpc
            .create_and_send_transaction(&[decompress_mint_ix], &ctx.payer.pubkey(), &[&ctx.payer])
            .await
            .expect("LP mint decompression should succeed");
    }

    // Decompress creator LP token ATA manually
    if creator_lp_interface.is_cold() {
        println!("  Decompressing creator LP token ATA...");

        // First create the ATA (idempotent)
        let create_ata_ix = CreateAssociatedTokenAccount::new(
            ctx.payer.pubkey(),
            ctx.creator.pubkey(),
            pdas.lp_mint,
        )
        .idempotent()
        .instruction()
        .expect("CreateAssociatedTokenAccount instruction should succeed");

        ctx.rpc
            .create_and_send_transaction(&[create_ata_ix], &ctx.payer.pubkey(), &[&ctx.payer])
            .await
            .expect("Create ATA should succeed");

        // Get the compressed token account data
        let load_context = creator_lp_interface
            .inner
            .load_context
            .as_ref()
            .expect("ATA should have load_context");
        let compressed = &load_context.compressed;

        // Get validity proof
        let proof_result = ctx
            .rpc
            .get_validity_proof(vec![compressed.account.hash], vec![], None)
            .await
            .expect("get_validity_proof should succeed")
            .value;

        let account_info = &proof_result.accounts[0];

        // Build TokenData from the compressed token account
        use light_token_sdk::compat::TokenData;
        let token_data = TokenData {
            mint: compressed.token.mint,
            owner: compressed.token.owner,
            amount: compressed.token.amount,
            delegate: compressed.token.delegate,
            state: compressed.token.state,
            tlv: compressed.token.tlv.clone(),
        };

        // Get discriminator from compressed account data
        let discriminator = compressed
            .account
            .data
            .as_ref()
            .map(|d| d.discriminator)
            .unwrap_or([0, 0, 0, 0, 0, 0, 0, 4]); // ShaFlat default

        // Build Decompress instruction
        let decompress_ix = Decompress {
            token_data,
            discriminator,
            merkle_tree: account_info.tree_info.tree,
            queue: account_info.tree_info.queue,
            leaf_index: account_info.leaf_index as u32,
            root_index: account_info.root_index.root_index().unwrap_or_default(),
            destination: creator_lp_interface.inner.pubkey,
            payer: ctx.payer.pubkey(),
            signer: ctx.creator.pubkey(),
            validity_proof: ValidityProof(proof_result.proof.into()),
        }
        .instruction()
        .expect("Decompress instruction should succeed");

        ctx.rpc
            .create_and_send_transaction(
                &[decompress_ix],
                &ctx.payer.pubkey(),
                &[&ctx.payer, &ctx.creator],
            )
            .await
            .expect("ATA decompression should succeed");
    }

    // ==========================================================================
    // PHASE 10: Assert decompression success
    // ==========================================================================
    println!("\nPhase 10: Verifying decompression...");

    // All accounts should be back on-chain
    assert_onchain_exists(&mut ctx.rpc, &pdas.pool_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.observation_state).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.lp_mint).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_0_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.token_1_vault).await;
    assert_onchain_exists(&mut ctx.rpc, &pdas.creator_lp_token).await;

    // Verify LP token balance preserved after decompression
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
    println!(
        "  LP balance after decompression: {} (expected: {})",
        lp_token_after_decompression.amount, expected_balance_after_withdraw
    );

    // Verify compressed token accounts are consumed
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

    println!("\nAMM full lifecycle test completed successfully!");
    println!("  - Initialize: OK");
    println!("  - Deposit: OK");
    println!("  - Withdraw: OK");
    println!("  - Compression: OK");
    println!("  - Decompression: OK");
}
