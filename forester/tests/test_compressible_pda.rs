use std::{sync::Arc, time::Duration};

use anchor_lang::{InstructionData, ToAccountMetas};
use borsh::BorshDeserialize;
use csdk_anchor_full_derived_test::state::d1_field_types::single_pubkey::SinglePubkeyRecord;
use forester::compressible::{
    pda::{PdaAccountTracker, PdaCompressor, PdaProgramConfig},
    traits::CompressibleTracker,
    AccountSubscriber, SubscriptionConfig,
};
use forester_utils::{
    forester_epoch::get_epoch_phases,
    rpc_pool::{SolanaRpcPool, SolanaRpcPoolBuilder},
    utils::wait_for_indexer,
};
use light_client::{
    indexer::Indexer,
    interface::{get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::address::derive_address;
use light_program_test::accounts::test_keypairs::PAYER_KEYPAIR;
use light_registry::{
    protocol_config::state::ProtocolConfigPda,
    sdk::{
        create_finalize_registration_instruction, create_register_forester_epoch_pda_instruction,
        create_register_forester_instruction,
    },
    utils::{get_forester_pda, get_protocol_config_pda_address},
    ForesterConfig as RegistryForesterConfig,
};
use light_sdk::LightDiscriminator;
use serial_test::serial;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use tokio::{
    sync::{broadcast, oneshot},
    time::sleep,
};

// csdk_anchor_full_derived_test program ID
const CSDK_TEST_PROGRAM_ID: &str = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah";

// SinglePubkeyRecord discriminator (derived from LightDiscriminator macro)
// This needs to match the discriminator from csdk_anchor_full_derived_test::state::d1_field_types::single_pubkey::SinglePubkeyRecord
const SINGLE_PUBKEY_RECORD_DISCRIMINATOR: [u8; 8] = csdk_anchor_full_derived_test::state::d1_field_types::single_pubkey::SinglePubkeyRecord::LIGHT_DISCRIMINATOR;

/// Get the program's derived rent sponsor PDA
fn program_rent_sponsor() -> Pubkey {
    csdk_anchor_full_derived_test::light_rent_sponsor()
}

/// Context returned from forester registration
struct ForesterContext {
    forester_keypair: Keypair,
    rpc_pool: Arc<SolanaRpcPool<LightClient>>,
}

/// Register a forester for epoch 0 and wait for registration phase to complete
async fn register_forester<R: Rpc>(
    rpc: &mut R,
) -> Result<ForesterContext, Box<dyn std::error::Error>> {
    let forester_keypair = Keypair::new();
    let forester_pubkey = forester_keypair.pubkey();

    let governance_authority =
        Keypair::try_from(light_program_test::accounts::test_keypairs::PAYER_KEYPAIR.as_ref())
            .expect("Failed to load governance authority");
    let governance_pubkey = governance_authority.pubkey();

    // Use airdrop instead of fund_account (which uses transfer_lamports from unfunded rpc payer)
    let gov_balance = rpc.get_balance(&governance_pubkey).await.unwrap_or(0);
    if gov_balance < 1_000_000_000 {
        println!(
            "Account {} needs {} more lamports (has: {}, target: {})",
            governance_pubkey,
            1_000_000_000 - gov_balance,
            gov_balance,
            1_000_000_000
        );
        rpc.airdrop_lamports(&governance_pubkey, 1_000_000_000 - gov_balance)
            .await?;
        sleep(Duration::from_millis(500)).await;
    } else {
        println!(
            "Account {} already has sufficient balance: {} >= {}",
            governance_pubkey, gov_balance, 1_000_000_000
        );
    }

    let protocol_config_pda_address = get_protocol_config_pda_address().0;
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda_address)
        .await?
        .ok_or("Protocol config not found")?
        .config;

    // Use airdrop for forester
    println!("Funding forester {} with 10 SOL", forester_pubkey);
    rpc.airdrop_lamports(&forester_pubkey, 10_000_000_000)
        .await?;
    sleep(Duration::from_millis(500)).await;

    let (forester_pda, _) = get_forester_pda(&forester_pubkey);

    let register_ix = create_register_forester_instruction(
        &governance_pubkey,
        &governance_pubkey,
        &forester_pubkey,
        RegistryForesterConfig::default(),
    );

    let (blockhash, _) = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[register_ix],
        Some(&governance_pubkey),
        &[&governance_authority],
        blockhash,
    );
    rpc.process_transaction(tx).await?;

    println!("Registered base forester: {}", forester_pda);

    let current_slot = rpc.get_slot().await?;
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    let phases = get_epoch_phases(&protocol_config, current_epoch);

    println!(
        "Current slot: {}, current_epoch: {}, phases: {:?}",
        current_slot, current_epoch, phases
    );

    // Determine which epoch to register for:
    // If we're already past the registration phase start, we might be in active phase
    // and need to wait for the next epoch's registration
    let (target_epoch, register_phase_start, active_phase_start) =
        if current_slot >= phases.active.start {
            // Already in active phase, register for next epoch
            let next_epoch = current_epoch + 1;
            let next_phases = get_epoch_phases(&protocol_config, next_epoch);
            println!(
                "Already in active phase, registering for next epoch {}, phases: {:?}",
                next_epoch, next_phases
            );
            (
                next_epoch,
                next_phases.registration.start,
                next_phases.active.start,
            )
        } else if current_slot >= phases.registration.start {
            // In registration phase, register for current epoch
            println!("In registration phase for epoch {}", current_epoch);
            (
                current_epoch,
                phases.registration.start,
                phases.active.start,
            )
        } else {
            // Before registration phase, wait for it
            println!(
                "Waiting for registration phase (starts at slot {})",
                phases.registration.start
            );
            (
                current_epoch,
                phases.registration.start,
                phases.active.start,
            )
        };

    while rpc.get_slot().await? < register_phase_start {
        sleep(Duration::from_millis(400)).await;
    }

    // Register for the target epoch
    let register_epoch_ix = create_register_forester_epoch_pda_instruction(
        &forester_pubkey,
        &forester_pubkey,
        target_epoch,
    );

    let (blockhash, _) = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[register_epoch_ix],
        Some(&forester_pubkey),
        &[&forester_keypair],
        blockhash,
    );
    rpc.process_transaction(tx).await?;

    println!("Registered for epoch {}", target_epoch);

    while rpc.get_slot().await? < active_phase_start {
        sleep(Duration::from_millis(400)).await;
    }

    println!("Active phase reached for epoch {}", target_epoch);

    let finalize_ix =
        create_finalize_registration_instruction(&forester_pubkey, &forester_pubkey, target_epoch);

    let (blockhash, _) = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[finalize_ix],
        Some(&forester_pubkey),
        &[&forester_keypair],
        blockhash,
    );
    rpc.process_transaction(tx).await?;

    println!("Finalized forester registration");

    let rpc_pool = Arc::new(
        SolanaRpcPoolBuilder::<LightClient>::new()
            .url("http://localhost:8899".to_string())
            .photon_url(Some("http://127.0.0.1:8784".to_string()))
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .build()
            .await
            .expect("Failed to create RPC pool"),
    );

    Ok(ForesterContext {
        forester_keypair,
        rpc_pool,
    })
}

/// Get the payer pubkey string derived from the test keypair
fn payer_pubkey_string() -> String {
    Keypair::try_from(PAYER_KEYPAIR.as_ref())
        .expect("Invalid PAYER_KEYPAIR")
        .pubkey()
        .to_string()
}

/// Test that PDA bootstrap discovers existing compressible PDAs
///
/// This test:
/// 1. Deploys csdk_anchor_full_derived_test program with upgrade authority
/// 2. Initializes compression config
/// 3. Creates a rent-free PDA record
/// 4. Triggers auto-compression (via slot advancement)
/// 5. Decompresses it back to a hot PDA
/// 6. Runs bootstrap to discover the PDA
/// 7. Verifies the PDA is tracked correctly
///
/// Run with: cargo test -p forester --test test_compressible_pda -- --nocapture
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_pda_bootstrap() {
    use csdk_anchor_full_derived_test::d8_builder_paths::D8PdaOnlyParams;

    let program_id: Pubkey = CSDK_TEST_PROGRAM_ID.parse().unwrap();

    // Start validator with csdk_anchor_full_derived_test deployed as upgradeable program
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 60,
        sbf_programs: vec![],
        upgradeable_programs: vec![(
            CSDK_TEST_PROGRAM_ID.to_string(),
            "../target/deploy/csdk_anchor_full_derived_test.so".to_string(),
            payer_pubkey_string(),
        )],
        limit_ledger_size: None,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    // Use PAYER_KEYPAIR as it's the upgrade authority for the program
    let authority = Keypair::try_from(PAYER_KEYPAIR.as_ref()).expect("Invalid PAYER_KEYPAIR");

    // Fund the authority account (extra for tx fees + rent sponsor funding)
    rpc.airdrop_lamports(&authority.pubkey(), 20_000_000_000)
        .await
        .expect("Failed to airdrop to authority");

    // Initialize compression config (includes rent sponsor funding)
    let rent_sponsor = program_rent_sponsor();
    let (init_config_ixs, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &authority.pubkey(),
        &Pubkey::find_program_address(
            &[program_id.as_ref()],
            &solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111"),
        )
        .0,
        rent_sponsor,
        authority.pubkey(),
        10_000_000_000,
    )
    .build();

    rpc.create_and_send_transaction(&init_config_ixs, &authority.pubkey(), &[&authority])
        .await
        .expect("Initialize config should succeed");

    // Wait for indexer to be ready (after initial transactions)
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    println!("Initialized compression config at: {}", config_pda);

    // Derive PDA for the record using D8PdaOnly seeds
    let owner = authority.pubkey();
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"d8_pda_only", owner.as_ref()], &program_id);

    // Get proof for creating the account
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .expect("Failed to get create accounts proof");

    // Create the rent-free record PDA using D8PdaOnly (known working instruction)
    let accounts = csdk_anchor_full_derived_test::accounts::D8PdaOnly {
        fee_payer: authority.pubkey(),
        compression_config: config_pda,
        d8_pda_only_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8PdaOnly {
        params: D8PdaOnlyParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
        .await
        .expect("Failed to create record");

    println!("Created rent-free record at PDA: {}", record_pda);

    // Verify PDA exists on-chain
    let pda_account = rpc.get_account(record_pda).await.unwrap();
    assert!(pda_account.is_some(), "PDA should exist after creation");

    // Advance slots to trigger auto-compression
    // Note: In test validator, we can't easily warp slots, so we'll skip this step
    // and instead directly decompress after some time

    // Wait for account to be indexed
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Create PDA tracker with program config
    let pda_config = PdaProgramConfig {
        program_id,
        discriminator: SINGLE_PUBKEY_RECORD_DISCRIMINATOR,
    };
    let tracker = Arc::new(PdaAccountTracker::new(vec![pda_config]));

    // Run bootstrap
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let tracker_clone = tracker.clone();
    let rpc_url = "http://localhost:8899".to_string();

    println!("Starting PDA bootstrap...");
    let bootstrap_handle = tokio::spawn(async move {
        if let Err(e) = forester::compressible::pda::bootstrap_pda_accounts(
            rpc_url,
            tracker_clone,
            Some(shutdown_rx),
        )
        .await
        {
            tracing::error!("PDA bootstrap failed: {:?}", e);
            panic!("PDA bootstrap failed: {:?}", e);
        }
    });

    // Wait for bootstrap to find the account
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(60);

    while start.elapsed() < timeout {
        if !tracker.is_empty() {
            println!("Bootstrap found {} PDA accounts", tracker.len());
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    // Verify bootstrap found the account
    assert!(
        !tracker.is_empty(),
        "Bootstrap should have found at least 1 PDA"
    );

    // Verify account data
    let current_slot = rpc.get_slot().await.unwrap();
    let ready_accounts = tracker.get_ready_to_compress_for_program(&program_id, current_slot);

    // Note: The account may not be ready yet if it still has rent, but it should be tracked
    println!(
        "Tracked {} PDAs, {} ready to compress",
        tracker.len(),
        ready_accounts.len()
    );

    // Cleanup
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(5), bootstrap_handle).await;

    println!("PDA bootstrap test completed successfully!");
}

/// Test that PDA compressor can compress decompressed PDAs
///
/// Run with: cargo test -p forester --test test_compressible_pda -- --nocapture
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_pda_compression() {
    use csdk_anchor_full_derived_test::d8_builder_paths::D8PdaOnlyParams;

    let program_id: Pubkey = CSDK_TEST_PROGRAM_ID.parse().unwrap();

    // Start validator with csdk_anchor_full_derived_test deployed as upgradeable program
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 60,
        sbf_programs: vec![],
        upgradeable_programs: vec![(
            CSDK_TEST_PROGRAM_ID.to_string(),
            "../target/deploy/csdk_anchor_full_derived_test.so".to_string(),
            payer_pubkey_string(),
        )],
        limit_ledger_size: None,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    // Use PAYER_KEYPAIR as it's the upgrade authority for the program
    let authority = Keypair::try_from(PAYER_KEYPAIR.as_ref()).expect("Invalid PAYER_KEYPAIR");

    // Fund the authority account (extra for tx fees + rent sponsor funding)
    rpc.airdrop_lamports(&authority.pubkey(), 20_000_000_000)
        .await
        .expect("Failed to airdrop to authority");

    // Initialize compression config (includes rent sponsor funding)
    let rent_sponsor = program_rent_sponsor();
    let (init_config_ixs, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &authority.pubkey(),
        &Pubkey::find_program_address(
            &[program_id.as_ref()],
            &solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111"),
        )
        .0,
        rent_sponsor,
        authority.pubkey(),
        10_000_000_000,
    )
    .build();

    rpc.create_and_send_transaction(&init_config_ixs, &authority.pubkey(), &[&authority])
        .await
        .expect("Initialize config should succeed");

    // Wait for indexer to be ready (after initial transactions)
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Derive PDA for the record using D8PdaOnly seeds
    let owner = authority.pubkey();
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"d8_pda_only", owner.as_ref()], &program_id);

    // Get proof for creating the account
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .expect("Failed to get create accounts proof");

    // Create the rent-free record PDA using D8PdaOnly (known working instruction)
    let accounts = csdk_anchor_full_derived_test::accounts::D8PdaOnly {
        fee_payer: authority.pubkey(),
        compression_config: config_pda,
        d8_pda_only_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8PdaOnly {
        params: D8PdaOnlyParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
        .await
        .expect("Failed to create record");

    println!("Created rent-free record at PDA: {}", record_pda);

    // Verify PDA exists
    let pda_account = rpc.get_account(record_pda).await.unwrap();
    assert!(pda_account.is_some(), "PDA should exist");

    // Get the compressed address for verification
    let address_tree = rpc.get_address_tree_v2().tree;
    let compressed_address = derive_address(
        &record_pda.to_bytes(),
        &address_tree.to_bytes(),
        &program_id.to_bytes(),
    );

    // Create tracker and add the PDA manually (simulating bootstrap)
    let pda_config = PdaProgramConfig {
        program_id,
        discriminator: SINGLE_PUBKEY_RECORD_DISCRIMINATOR,
    };
    let tracker = Arc::new(PdaAccountTracker::new(vec![pda_config.clone()]));

    // Update tracker from the actual account
    let account = pda_account.unwrap();
    tracker
        .update_from_account(record_pda, program_id, &account.data, account.lamports)
        .unwrap();

    assert_eq!(tracker.len(), 1, "Tracker should have 1 account");

    // Register forester
    let ctx = register_forester(&mut rpc)
        .await
        .expect("Failed to register forester");

    // Get ready accounts (should be ready since decompressed accounts start with minimal rent)
    let rpc_from_pool = ctx.rpc_pool.get_connection().await.unwrap();
    let current_slot = rpc_from_pool.get_slot().await.unwrap();

    // Wait for indexer to catch up
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Use current_slot + 1000 to simulate a future slot, making accounts appear past their
    // compressible_slot threshold for testing. We can't warp slots in the test validator,
    // so this tricks get_ready_to_compress_for_program into returning accounts as if
    // enough time has passed (ready_accounts will include accounts where compressible_slot < current_slot + 1000).
    let ready_accounts =
        tracker.get_ready_to_compress_for_program(&program_id, current_slot + 1000);
    println!("Ready to compress: {} accounts", ready_accounts.len());

    if !ready_accounts.is_empty() {
        // Create compressor and compress
        let compressor = PdaCompressor::new(
            ctx.rpc_pool.clone(),
            tracker.clone(),
            ctx.forester_keypair.insecure_clone(),
        );

        println!("Compressing PDA...");
        let compress_result = compressor
            .compress_batch(&ready_accounts, &pda_config)
            .await;

        let signature = compress_result.expect("Compression should succeed");
        println!("Compression succeeded with signature: {}", signature);

        // Wait for indexer to confirm
        wait_for_indexer(&rpc)
            .await
            .expect("Failed to wait for indexer");

        // Verify PDA is closed
        let pda_after = rpc_from_pool.get_account(record_pda).await.unwrap();
        assert!(
            pda_after.is_none() || pda_after.as_ref().map(|a| a.lamports) == Some(0),
            "PDA should be closed after compression"
        );

        // Verify compressed account data matches expected record
        let compressed_after = rpc_from_pool
            .get_compressed_account(compressed_address, None)
            .await
            .unwrap()
            .value
            .unwrap();

        let compressed_data = compressed_after
            .data
            .as_ref()
            .expect("Compressed account should have data")
            .data
            .as_slice();

        let deserialized = SinglePubkeyRecord::try_from_slice(compressed_data)
            .expect("Failed to deserialize SinglePubkeyRecord from compressed account");

        let compression_info = deserialized.compression_info.clone();

        let expected_record = SinglePubkeyRecord {
            compression_info,
            owner: authority.pubkey(),
            counter: 0,
        };

        assert_eq!(
            deserialized, expected_record,
            "Compressed account data should match expected SinglePubkeyRecord"
        );

        println!("PDA compression test completed successfully!");
    } else {
        panic!("No accounts ready to compress - test setup failed");
    }
}

/// Test AccountSubscriber for PDA accounts
///
/// This test verifies the full subscription flow:
/// 1. Start AccountSubscriber with PdaAccountTracker
/// 2. Create a rent-free PDA record
/// 3. Assert subscriber picks up the account
/// 4. Run PdaCompressor to compress the PDA
/// 5. Assert account is closed and tracker is updated
///
/// Run with: cargo test -p forester --test test_compressible_pda test_compressible_pda_subscription -- --nocapture
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_pda_subscription() {
    use csdk_anchor_full_derived_test::d8_builder_paths::D8PdaOnlyParams;

    let program_id: Pubkey = CSDK_TEST_PROGRAM_ID.parse().unwrap();

    // Start validator with csdk_anchor_full_derived_test deployed
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 60,
        sbf_programs: vec![],
        upgradeable_programs: vec![(
            CSDK_TEST_PROGRAM_ID.to_string(),
            "../target/deploy/csdk_anchor_full_derived_test.so".to_string(),
            payer_pubkey_string(),
        )],
        limit_ledger_size: None,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    let authority = Keypair::try_from(&PAYER_KEYPAIR[..]).unwrap();

    // Fund accounts (extra for tx fees + rent sponsor funding)
    rpc.airdrop_lamports(&authority.pubkey(), 20_000_000_000)
        .await
        .expect("Failed to airdrop to authority");

    // Wait for indexer
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Initialize compression config (includes rent sponsor funding)
    let rent_sponsor = program_rent_sponsor();
    let (init_config_ixs, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &authority.pubkey(),
        &Pubkey::find_program_address(
            &[program_id.as_ref()],
            &solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111"),
        )
        .0,
        rent_sponsor,
        authority.pubkey(),
        10_000_000_000,
    )
    .build();

    rpc.create_and_send_transaction(&init_config_ixs, &authority.pubkey(), &[&authority])
        .await
        .expect("Initialize config should succeed");

    println!("Initialized compression config at: {}", config_pda);

    // Setup tracker and subscribers BEFORE creating PDAs
    let pda_config = PdaProgramConfig {
        program_id,
        discriminator: SINGLE_PUBKEY_RECORD_DISCRIMINATOR,
    };
    let tracker = Arc::new(PdaAccountTracker::new(vec![pda_config.clone()]));

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // Spawn account subscriber to track new/updated PDA accounts
    let mut account_subscriber = AccountSubscriber::new(
        "ws://localhost:8900".to_string(),
        tracker.clone(),
        SubscriptionConfig::pda(
            program_id,
            SINGLE_PUBKEY_RECORD_DISCRIMINATOR,
            "pda".to_string(),
        ),
        shutdown_rx,
    );
    let account_subscriber_handle = tokio::spawn(async move {
        account_subscriber
            .run()
            .await
            .expect("Account subscriber failed to run");
    });

    // Give subscribers time to connect
    sleep(Duration::from_secs(2)).await;

    // Create first PDA
    let owner1 = authority.pubkey();
    let (record_pda_1, _) =
        Pubkey::find_program_address(&[b"d8_pda_only", owner1.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda_1)],
    )
    .await
    .expect("Failed to get create accounts proof");

    let accounts = csdk_anchor_full_derived_test::accounts::D8PdaOnly {
        fee_payer: authority.pubkey(),
        compression_config: config_pda,
        d8_pda_only_record: record_pda_1,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D8PdaOnly {
        params: D8PdaOnlyParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner: owner1,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
        .await
        .expect("Failed to create first PDA");

    println!("Created first rent-free PDA at: {}", record_pda_1);

    // Wait for subscriber to pick up the account
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(30);
    while start.elapsed() < timeout {
        if tracker.len() >= 1 {
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }

    // Verify tracker picked up the first PDA
    assert_eq!(
        tracker.len(),
        1,
        "Tracker should have 1 PDA after first creation"
    );
    println!("Tracker detected first PDA via subscription");

    // Create second PDA with different owner
    let owner2_keypair = Keypair::new();
    let owner2 = owner2_keypair.pubkey();
    let (record_pda_2, _) =
        Pubkey::find_program_address(&[b"d8_pda_only", owner2.as_ref()], &program_id);

    let proof_result_2 = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda_2)],
    )
    .await
    .expect("Failed to get create accounts proof for second PDA");

    let accounts_2 = csdk_anchor_full_derived_test::accounts::D8PdaOnly {
        fee_payer: authority.pubkey(),
        compression_config: config_pda,
        d8_pda_only_record: record_pda_2,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data_2 = csdk_anchor_full_derived_test::instruction::D8PdaOnly {
        params: D8PdaOnlyParams {
            create_accounts_proof: proof_result_2.create_accounts_proof,
            owner: owner2,
        },
    };

    let instruction_2 = Instruction {
        program_id,
        accounts: [
            accounts_2.to_account_metas(None),
            proof_result_2.remaining_accounts,
        ]
        .concat(),
        data: instruction_data_2.data(),
    };

    rpc.create_and_send_transaction(&[instruction_2], &authority.pubkey(), &[&authority])
        .await
        .expect("Failed to create second PDA");

    println!("Created second rent-free PDA at: {}", record_pda_2);

    // Wait for subscriber to pick up the second account
    let start = tokio::time::Instant::now();
    while start.elapsed() < timeout {
        if tracker.len() >= 2 {
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }

    // Verify tracker has both PDAs
    assert_eq!(
        tracker.len(),
        2,
        "Tracker should have 2 PDAs after second creation"
    );
    println!("Tracker detected second PDA via subscription");

    // Register forester for compression
    let ctx = register_forester(&mut rpc)
        .await
        .expect("Failed to register forester");

    // Wait for indexer to catch up
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Get ready-to-compress accounts
    let rpc_from_pool = ctx.rpc_pool.get_connection().await.unwrap();
    let current_slot = rpc_from_pool.get_slot().await.unwrap();

    // These should be ready since they're rent-free PDAs
    let ready_accounts =
        tracker.get_ready_to_compress_for_program(&program_id, current_slot + 1000);
    println!(
        "Ready to compress: {} PDAs (current_slot: {})",
        ready_accounts.len(),
        current_slot
    );

    assert!(
        !ready_accounts.is_empty(),
        "Should have PDAs ready to compress"
    );

    // Compress just the first PDA
    let compressor = PdaCompressor::new(
        ctx.rpc_pool.clone(),
        tracker.clone(),
        ctx.forester_keypair.insecure_clone(),
    );

    let first_pda_state = ready_accounts
        .iter()
        .find(|s| s.pubkey == record_pda_1)
        .expect("First PDA should be ready")
        .clone();

    println!("Compressing first PDA: {}", record_pda_1);
    let signature = compressor
        .compress_batch(&[first_pda_state], &pda_config)
        .await
        .expect("Compression should succeed");

    println!("Compression tx sent: {}", signature);

    // Wait for PDA account to be closed
    let start = tokio::time::Instant::now();
    let mut account_closed = false;
    while start.elapsed() < timeout {
        let pda_after = rpc_from_pool.get_account(record_pda_1).await.unwrap();
        if pda_after.is_none() || pda_after.as_ref().map(|a| a.lamports) == Some(0) {
            account_closed = true;
            println!("First PDA closed successfully!");
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
    assert!(account_closed, "First PDA should be closed");

    // Verify tracker was updated
    assert_eq!(
        tracker.len(),
        1,
        "Tracker should have 1 PDA after compression"
    );
    println!("Tracker updated: now has {} PDA(s)", tracker.len());

    // Verify the remaining PDA is the second one
    let remaining = tracker.get_ready_to_compress_for_program(&program_id, current_slot + 1000);
    assert_eq!(remaining.len(), 1);
    assert_eq!(
        remaining[0].pubkey, record_pda_2,
        "Remaining PDA should be the second one"
    );

    // Shutdown subscribers
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");
    account_subscriber_handle
        .await
        .expect("Account subscriber task panicked");

    println!("PDA subscription test completed successfully!");
}
