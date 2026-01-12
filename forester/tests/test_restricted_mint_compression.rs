use std::{sync::Arc, time::Duration};

use anchor_spl::token_2022::spl_token_2022;
use forester::compressible::{
    AccountSubscriber, CompressibleAccountTracker, Compressor, LogSubscriber,
};
use forester_utils::{
    forester_epoch::get_epoch_phases,
    rpc_pool::{SolanaRpcPool, SolanaRpcPoolBuilder},
    utils::wait_for_indexer,
};
use light_client::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_ctoken_interface::state::TokenDataVersion;
use light_ctoken_sdk::spl_interface::CreateSplInterfacePda;
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
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use solana_system_interface::instruction as system_instruction;
use spl_token_2022::extension::ExtensionType;
use tokio::time::sleep;

/// Context returned from forester registration containing everything needed for compression testing
struct ForesterContext {
    forester_keypair: Keypair,
    rpc_pool: Arc<SolanaRpcPool<LightClient>>,
    epoch: forester_utils::forester_epoch::Epoch,
}

/// Register a forester for epoch 0 and wait for registration phase to complete
/// (Reused from test_compressible_ctoken.rs)
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

    // Create RPC pool
    let rpc_pool = Arc::new(
        SolanaRpcPoolBuilder::<LightClient>::new()
            .url("http://localhost:8899".to_string())
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .build()
            .await
            .expect("Failed to create RPC pool"),
    );

    // Construct Epoch struct
    use forester_utils::forester_epoch::Epoch;
    use light_registry::protocol_config::state::EpochState;

    let epoch_struct = Epoch {
        epoch,
        epoch_pda: solana_sdk::pubkey::Pubkey::default(),
        forester_epoch_pda: solana_sdk::pubkey::Pubkey::default(),
        phases,
        state: EpochState::Active,
        merkle_trees: vec![],
    };

    Ok(ForesterContext {
        forester_keypair,
        rpc_pool,
        epoch: epoch_struct,
    })
}

/// Test that a restricted mint CToken account is compressed by the forester
/// and can be retrieved from the indexer.
///
/// Flow:
/// 1. Create Token-2022 mint with PermanentDelegate (restricted extension)
/// 2. Create restricted SPL interface PDA (pool)
/// 3. Create CToken account with num_prepaid_epochs: 0 (immediately compressible)
/// 4. Register forester and run compression
/// 5. Verify account is closed on-chain
/// 6. Retrieve compressed token account from indexer
///
///
/// Indexer error:
/// 2026-01-06T19:33:11.101239Z ERROR photon_indexer::ingester: Failed to index block batch 103-103. Got error Parser error: Failed to parse token data: Custom { kind: InvalidData, error: "Not all bytes read" }
/// Photon wip branch: jorrit/fix-token-data-parsing-tlv (same error)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_restricted_mint_compression() {
    // 1. Start validator with indexer enabled
    spawn_validator(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: false,
        wait_time: 10,
        sbf_programs: vec![],
        limit_ledger_size: None,
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

    // 2. Setup tracker and subscribers
    let tracker = Arc::new(CompressibleAccountTracker::new());
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let shutdown_rx_log = shutdown_tx.subscribe();

    // Spawn account subscriber to track new/updated accounts
    let mut account_subscriber = AccountSubscriber::new(
        "ws://localhost:8900".to_string(),
        tracker.clone(),
        shutdown_rx,
    );
    let account_subscriber_handle = tokio::spawn(async move {
        account_subscriber
            .run()
            .await
            .expect("Account subscriber failed to run");
    });

    // Spawn log subscriber to detect compress_and_close operations
    let mut log_subscriber = LogSubscriber::new(
        "ws://localhost:8900".to_string(),
        tracker.clone(),
        shutdown_rx_log,
    );
    let log_subscriber_handle = tokio::spawn(async move {
        log_subscriber
            .run()
            .await
            .expect("Log subscriber failed to run");
    });
    sleep(Duration::from_secs(2)).await;

    // 3. Create Token-2022 mint with PermanentDelegate (restricted extension)
    let mint = Keypair::new();
    let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
        ExtensionType::PermanentDelegate,
    ])
    .unwrap();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(space)
        .await
        .unwrap();

    let mint_instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rent,
            space as u64,
            &spl_token_2022::ID,
        ),
        spl_token_2022::instruction::initialize_permanent_delegate(
            &spl_token_2022::ID,
            &mint.pubkey(),
            &payer.pubkey(),
        )
        .unwrap(),
        spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            9,
        )
        .unwrap(),
    ];

    rpc.create_and_send_transaction(&mint_instructions, &payer.pubkey(), &[&payer, &mint])
        .await
        .expect("Failed to create restricted mint");

    println!(
        "Created Token-2022 mint with PermanentDelegate: {}",
        mint.pubkey()
    );

    // 4. Create restricted SPL interface PDA (pool)
    let create_pool_ix = CreateSplInterfacePda::new(
        payer.pubkey(),
        mint.pubkey(),
        spl_token_2022::ID,
        true, // restricted = true for mints with restricted extensions
    )
    .instruction();

    rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Failed to create restricted SPL interface PDA");

    println!("Created restricted SPL interface PDA");

    // 5. Create CToken account with num_prepaid_epochs: 0 (immediately compressible)
    // The create_compressible_token_account function automatically detects restricted
    // extensions and sets compression_only: true
    let owner_keypair = Keypair::new();
    let ctoken_pubkey = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: owner_keypair.pubkey(),
            mint: mint.pubkey(),
            num_prepaid_epochs: 0, // Immediately compressible
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write: Some(100),
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .await
    .expect("Failed to create compressible token account");

    println!(
        "Created CToken account with num_prepaid_epochs=0: {}",
        ctoken_pubkey
    );
    sleep(Duration::from_millis(1000)).await;
    // 6. Verify tracker picked up the account
    assert_eq!(tracker.len(), 1, "Tracker should have 1 account");
    let accounts = tracker.get_compressible_accounts();
    assert_eq!(accounts.len(), 1);
    let account_state = &accounts[0];
    assert_eq!(account_state.pubkey, ctoken_pubkey);
    assert_eq!(account_state.account.mint, mint.pubkey().to_bytes());
    assert_eq!(
        account_state.account.owner,
        owner_keypair.pubkey().to_bytes()
    );
    println!("Tracker verified: account tracked correctly");

    // 7. Register forester
    let ctx = register_forester(&mut rpc)
        .await
        .expect("Failed to register forester");

    let rpc_from_pool = ctx.rpc_pool.get_connection().await.unwrap();
    let current_slot = rpc_from_pool.get_slot().await.unwrap();
    let ready_accounts = tracker.get_ready_to_compress(current_slot);
    assert_eq!(
        ready_accounts.len(),
        1,
        "Should have 1 account ready to compress"
    );
    assert_eq!(ready_accounts[0].pubkey, ctoken_pubkey);

    println!("Account ready to compress: {}", ctoken_pubkey);

    // 8. Run compression
    let (registered_forester_pda, _) = light_registry::utils::get_forester_epoch_pda_from_authority(
        &ctx.forester_keypair.pubkey(),
        ctx.epoch.epoch,
    );
    let compressor = Compressor::new(ctx.rpc_pool.clone(), tracker.clone(), ctx.forester_keypair);
    let compressor_handle = tokio::spawn(async move {
        compressor
            .compress_batch(&ready_accounts, registered_forester_pda)
            .await
    });
    sleep(Duration::from_millis(2000)).await;

    // 9. Wait for account to be closed on-chain
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(30);
    let mut account_closed = false;
    while start.elapsed() < timeout {
        let account = rpc_from_pool.get_account(ctoken_pubkey).await.unwrap();
        if account.is_none() || account.as_ref().map(|a| a.lamports) == Some(0) {
            account_closed = true;
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
    compressor_handle.abort();

    assert!(
        account_closed,
        "CToken account should be closed after compression"
    );
    println!("CToken account closed on-chain");

    // 10. Verify tracker updated
    assert_eq!(
        tracker.len(),
        0,
        "Tracker should have 0 accounts after compression"
    );

    // 11. Query compressed token account from indexer
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    let compressed_accounts: Vec<_> = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(
            &owner_keypair.pubkey(),
            Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions {
                mint: Some(mint.pubkey()),
                cursor: None,
                limit: None,
            }),
            None,
        )
        .await
        .expect("Failed to get compressed token accounts")
        .into();

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    let compressed_account = &compressed_accounts[0];
    assert_eq!(
        compressed_account.token_data.mint,
        mint.pubkey(),
        "Compressed account mint should match"
    );
    assert_eq!(
        compressed_account.token_data.owner,
        owner_keypair.pubkey(),
        "Compressed account owner should match"
    );

    println!(
        "Successfully retrieved compressed token account from indexer: mint={}, owner={}",
        mint.pubkey(),
        owner_keypair.pubkey()
    );

    // 12. Shutdown subscribers
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");
    account_subscriber_handle
        .await
        .expect("Account subscriber task panicked");
    log_subscriber_handle
        .await
        .expect("Log subscriber task panicked");

    println!("Test completed successfully!");
}
