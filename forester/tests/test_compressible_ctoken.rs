use std::{sync::Arc, time::Duration};

use forester::{
    compressible::{AccountSubscriber, CompressibleAccountTracker, Compressor},
    slot_tracker::SlotTracker,
};
use forester_utils::{
    forester_epoch::get_epoch_phases,
    rpc_pool::{SolanaRpcPool, SolanaRpcPoolBuilder},
};
use light_client::{
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_token_sdk::instructions::create_compressed_mint;
use light_ctoken_types::state::TokenDataVersion;
use light_registry::{
    protocol_config::state::ProtocolConfigPda,
    sdk::{
        create_finalize_registration_instruction, create_register_forester_epoch_pda_instruction,
        create_register_forester_instruction,
    },
    utils::{get_forester_pda, get_protocol_config_pda_address},
    ForesterConfig,
};
use light_token_client::actions::{
    create_compressible_token_account, CreateCompressibleTokenAccountInputs,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use tokio::{sync::oneshot, time::sleep};

/// Context returned from forester registration containing everything needed for compression testing
struct ForesterContext {
    forester_keypair: Keypair,
    rpc_pool: Arc<SolanaRpcPool<LightClient>>,
    epoch: u64,
    active_phase_end_slot: u64,
    epoch_phases: forester_utils::forester_epoch::EpochPhases,
}

/// Register a forester for epoch 0 and wait for registration phase to complete
async fn register_forester<R: Rpc>(
    rpc: &mut R,
) -> Result<ForesterContext, Box<dyn std::error::Error>> {
    // Create forester keypair
    let forester_keypair = Keypair::new();
    let forester_pubkey = forester_keypair.pubkey();

    // Get governance authority
    let governance_authority =
        Keypair::try_from(light_program_test::accounts::test_keypairs::PAYER_KEYPAIR.as_ref())
            .expect("Failed to load governance authority");
    let governance_pubkey = governance_authority.pubkey();

    // Airdrop to governance authority
    rpc.airdrop_lamports(&governance_pubkey, 1_000_000_000)
        .await?;

    // Get protocol config to calculate phase timing
    let protocol_config_pda_address = get_protocol_config_pda_address().0;
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda_address)
        .await?
        .ok_or("Protocol config not found")?
        .config;

    // Airdrop to forester for transaction fees
    rpc.airdrop_lamports(&forester_pubkey, 10_000_000_000)
        .await?;

    // Register base forester
    let (forester_pda, _) = get_forester_pda(&forester_pubkey);

    let register_ix = create_register_forester_instruction(
        &governance_pubkey,
        &governance_pubkey,
        &forester_pubkey,
        ForesterConfig::default(),
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

    // Calculate epoch info
    let current_slot = rpc.get_slot().await?;
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    println!("current_epoch {:?}", current_epoch);
    let phases = get_epoch_phases(&protocol_config, current_epoch);
    let register_phase_start = phases.registration.start;
    let active_phase_start = phases.active.start;
    println!("phases {:?}", phases);
    println!("current_slot {}", current_slot);

    // Wait for registration phase
    while rpc.get_slot().await? < register_phase_start {
        sleep(Duration::from_millis(400)).await;
    }

    // Register for epoch 0
    let epoch = 0u64;
    let register_epoch_ix =
        create_register_forester_epoch_pda_instruction(&forester_pubkey, &forester_pubkey, epoch);

    let (blockhash, _) = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[register_epoch_ix],
        Some(&forester_pubkey),
        &[&forester_keypair],
        blockhash,
    );
    rpc.process_transaction(tx).await?;

    println!("Registered for epoch {}", epoch);

    println!(
        "Waiting for active phase (current slot: {}, active phase starts at: {})...",
        current_slot, active_phase_start
    );

    // Wait for active phase
    while rpc.get_slot().await? < active_phase_start {
        sleep(Duration::from_millis(400)).await;
    }

    println!("Active phase reached");

    // Finalize registration
    let finalize_ix =
        create_finalize_registration_instruction(&forester_pubkey, &forester_pubkey, epoch);

    let (blockhash, _) = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[finalize_ix],
        Some(&forester_pubkey),
        &[&forester_keypair],
        blockhash,
    );
    rpc.process_transaction(tx).await?;

    println!("Finalized forester registration");

    // Get updated slot and create RPC pool
    let current_slot = rpc.get_slot().await?;
    let active_phase_end_slot = current_slot + 10000; // Far future so compression can run

    let rpc_pool = Arc::new(
        SolanaRpcPoolBuilder::<LightClient>::new()
            .url("http://localhost:8899".to_string())
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .build()
            .await
            .expect("Failed to create RPC pool"),
    );

    Ok(ForesterContext {
        forester_keypair,
        rpc_pool,
        epoch,
        active_phase_end_slot,
        epoch_phases: phases,
    })
}

/// Test that compressible token accounts are tracked and compressed automatically
///
/// 1. Start AccountSubscriber with CompressibleAccountTracker
/// 2. Create two compressible token accounts: one with 2 epochs rent, one with 0 epochs rent
/// 3. Assert subscriber picked up both accounts (tracker.len() == 2)
/// 4. Register forester and run compression pipeline
/// 5. Assert account with 0 epochs is compressed and closed on-chain
/// 6. Assert tracker is updated and now has only 1 account (the one with 2 epochs rent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_ctoken_compression() {
    // Start validator and RPC client
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: false,
        wait_time: 10,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    })
    .await;
    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");
    let payer = rpc.get_payer().insecure_clone();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .expect("Failed to airdrop lamports");
    // Setup tracker and subscriber
    let tracker = Arc::new(CompressibleAccountTracker::new());
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let mut subscriber = AccountSubscriber::new(
        "ws://localhost:8900".to_string(),
        tracker.clone(),
        shutdown_rx,
    );
    let subscriber_handle = tokio::spawn(async move {
        subscriber.run().await.expect("Subscriber failed to run");
    });
    sleep(Duration::from_secs(2)).await;
    // Create mint
    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2().tree;
    let mint = Pubkey::from(create_compressed_mint::derive_compressed_mint_address(
        &mint_seed.pubkey(),
        &address_tree,
    ));
    // Create first account with 2 epochs rent
    let owner_keypair = Keypair::new();
    let token_account_pubkey = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: owner_keypair.pubkey(),
            mint,
            num_prepaid_epochs: 2,
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write: Some(100),
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .await
    .expect("Failed to create compressible token account");
    // Verify tracker has the account
    assert_eq!(tracker.len(), 1, "Tracker should have 1 account");
    let accounts = tracker.get_compressible_accounts();
    assert_eq!(accounts.len(), 1);
    let account_state = &accounts[0];
    assert_eq!(account_state.pubkey, token_account_pubkey);
    assert_eq!(account_state.account.mint, mint.to_bytes());
    assert_eq!(
        account_state.account.owner,
        owner_keypair.pubkey().to_bytes()
    );
    assert_eq!(account_state.account.amount, 0);
    assert!(account_state.lamports > 0);
    let lamports = account_state.lamports;
    // Test lamports update
    rpc.airdrop_lamports(&account_state.pubkey, 10_000_000)
        .await
        .expect("Failed to airdrop to token account");
    let accounts = tracker.get_compressible_accounts();
    assert_eq!(accounts[0].lamports, lamports + 10_000_000);
    // Create second account with 0 epochs rent
    let token_account_pubkey_2 = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: Keypair::new().pubkey(),
            mint,
            num_prepaid_epochs: 0,
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write: Some(100),
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .await
    .expect("Failed to create second compressible token account");
    assert_eq!(tracker.len(), 2, "Tracker should have 2 accounts");
    // Register forester and run compression
    let ctx = register_forester(&mut rpc)
        .await
        .expect("Failed to register forester");
    let rpc_from_pool = ctx.rpc_pool.get_connection().await.unwrap();
    let current_slot = rpc_from_pool.get_slot().await.unwrap();
    let ready_accounts = tracker.get_ready_to_compress(current_slot);
    assert_eq!(ready_accounts.len(), 1, "Should have 1 account ready");
    assert_eq!(ready_accounts[0].pubkey, token_account_pubkey_2);
    let slot_tracker = Arc::new(SlotTracker::new(current_slot, Duration::from_secs(10)));
    let epoch = ctx.epoch;
    let active_phase_end_slot = ctx.active_phase_end_slot;
    let epoch_phases = ctx.epoch_phases;
    let mut compressor = Compressor::new(
        ctx.rpc_pool.clone(),
        tracker.clone(),
        ctx.forester_keypair,
        slot_tracker.clone(),
        10,
    );
    let compressor_handle = tokio::spawn(async move {
        compressor
            .run_for_epoch(epoch, active_phase_end_slot, epoch_phases, 50, 100)
            .await
    });
    sleep(Duration::from_millis(2000)).await;
    // Wait for account to be closed
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(30);
    let mut account_closed = false;
    while start.elapsed() < timeout {
        let account = rpc_from_pool
            .get_account(token_account_pubkey_2)
            .await
            .unwrap();
        if account.is_none() || account.as_ref().map(|a| a.lamports) == Some(0) {
            account_closed = true;
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
    compressor_handle.abort();
    assert!(account_closed, "Account should be closed");
    // Verify compression succeeded
    let account_after_compression = rpc_from_pool
        .get_account(token_account_pubkey_2)
        .await
        .unwrap();
    assert!(
        account_after_compression.is_none() || account_after_compression.unwrap().lamports == 0
    );
    assert_eq!(
        tracker.len(),
        1,
        "Tracker should have 1 account after compression"
    );
    let remaining_accounts = tracker.get_compressible_accounts();
    assert_eq!(remaining_accounts.len(), 1);
    assert_eq!(remaining_accounts[0].pubkey, token_account_pubkey);
    // Shutdown
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");
    subscriber_handle.await.expect("Subscriber task panicked");
}
