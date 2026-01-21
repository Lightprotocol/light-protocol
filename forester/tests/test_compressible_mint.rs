use std::{sync::Arc, time::Duration};

use borsh::BorshDeserialize;
use forester::compressible::{
    mint::{MintAccountTracker, MintCompressor},
    traits::CompressibleTracker,
    AccountSubscriber, SubscriptionConfig,
};
use forester_utils::{rpc_pool::SolanaRpcPoolBuilder, utils::wait_for_indexer};
use light_client::{
    indexer::{AddressWithTree, Indexer},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_token::instruction::{
    derive_mint_compressed_address, find_mint_address, CreateMint, CreateMintParams,
};
use light_token_interface::state::{BaseMint, Mint, MintMetadata, ACCOUNT_TYPE_MINT};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use tokio::{
    sync::{broadcast, oneshot},
    time::sleep,
};

/// Build an expected Mint for assertion comparison.
///
/// Takes known values from test setup plus runtime values extracted from the on-chain account.
fn build_expected_mint(
    mint_authority: &Pubkey,
    decimals: u8,
    mint_pda: &Pubkey,
    mint_signer: &[u8; 32],
    bump: u8,
    compression: light_compressible::compression_info::CompressionInfo,
) -> Mint {
    Mint {
        base: BaseMint {
            mint_authority: Some((*mint_authority).into()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: 1,
            mint_decompressed: true,
            mint: (*mint_pda).into(),
            mint_signer: *mint_signer,
            bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression,
        extensions: None,
    }
}

/// Helper to create a compressed mint with decompression.
/// Returns (mint_pda, compression_address, mint_seed, bump).
async fn create_decompressed_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
) -> (Pubkey, [u8; 32], Keypair, u8) {
    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address
    let compression_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);

    let (mint_pda, bump) = find_mint_address(&mint_seed.pubkey());

    // Get validity proof for the address
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build params - rent_payment = 0 makes the mint immediately compressible (no auto-decompress period)
    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint: mint_pda,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: 0, // Immediately compressible for testing
        write_top_up: 0,
    };

    // Create instruction
    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .expect("CreateMint should succeed");

    (mint_pda, compression_address, mint_seed, bump)
}

/// Test that Mint bootstrap discovers decompressed mints
///
/// This test:
/// 1. Creates a compressed mint with decompression (CreateMint auto-decompresses)
/// 2. Runs bootstrap to discover the decompressed mint
/// 3. Verifies the mint is tracked correctly
///
/// Run with: cargo test -p forester --test test_compressible_mint -- --nocapture
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_mint_bootstrap() {
    // Start validator
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 45,
        sbf_programs: vec![],
        upgradeable_programs: vec![],
        limit_ledger_size: None,
        use_surfpool: true,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    let payer = rpc.get_payer().insecure_clone();

    // Airdrop to payer
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .expect("Failed to airdrop lamports");

    // Wait for indexer to be ready before making validity proof requests
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Create a decompressed mint
    let (mint_pda, compression_address, mint_seed, bump) =
        create_decompressed_mint(&mut rpc, &payer, payer.pubkey(), 9).await;

    println!("Created decompressed mint at: {}", mint_pda);
    println!("Compression address: {:?}", compression_address);

    // Verify mint exists on-chain and matches expected structure
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(mint_account.is_some(), "Mint should exist after creation");

    // Verify mint is decompressed using single assert_eq against expected Mint
    let mint_data = mint_account.unwrap();
    let mint = Mint::deserialize(&mut &mint_data.data[..]).expect("Failed to deserialize Mint");

    // Extract runtime-specific values from deserialized mint
    let compression = mint.compression;
    let metadata_version = mint.metadata.version;

    // Build expected Mint
    let expected_mint = Mint {
        base: BaseMint {
            mint_authority: Some(payer.pubkey().to_bytes().into()),
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: metadata_version,
            mint_decompressed: true,
            mint: mint_pda.to_bytes().into(),
            mint_signer: mint_seed.pubkey().to_bytes(),
            bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression,
        extensions: None,
    };

    assert_eq!(mint, expected_mint, "Mint should match expected state");
=======
    let mint_data = mint_account.unwrap();
    let mint = Mint::deserialize(&mut &mint_data.data[..]).expect("Failed to deserialize Mint");

    // Build expected mint using known values plus runtime compression info
    let expected_mint = build_expected_mint(
        &payer.pubkey(),
        9,
        &mint_pda,
        &mint_seed.pubkey().to_bytes(),
        bump,
        mint.compression,
    );
>>>>>>> d6299d718 (feat: add hex dependency and update existing hex usage in Cargo.toml files; refactor mint compression logic to handle batching and improve error handling; enhance test cases for mint creation and compression)

    assert_eq!(mint, expected_mint, "Mint should match expected structure");

    // Wait for indexer
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Create tracker and run bootstrap
    let tracker = Arc::new(MintAccountTracker::new());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let tracker_clone = tracker.clone();
    let rpc_url = "http://localhost:8899".to_string();

    println!("Starting Mint bootstrap...");
    let bootstrap_handle = tokio::spawn(async move {
        if let Err(e) = forester::compressible::mint::bootstrap_mint_accounts(
            rpc_url,
            tracker_clone,
            Some(shutdown_rx),
        )
        .await
        {
            tracing::error!("Mint bootstrap failed: {:?}", e);
            panic!("Mint bootstrap failed: {:?}", e);
        }
    });

    // Wait for bootstrap to find the account
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(60);
    let mut iteration = 0;

    while start.elapsed() < timeout {
        if !tracker.is_empty() {
            println!("Bootstrap found {} Mint accounts", tracker.len());
            break;
        }
        iteration += 1;
        if iteration % 20 == 0 {
            println!(
                "Bootstrap polling: {} iterations, {:.1}s elapsed",
                iteration,
                start.elapsed().as_secs_f64()
            );
        }
        sleep(Duration::from_millis(500)).await;
    }

    // Verify bootstrap found the mint
    assert!(
        !tracker.is_empty(),
        "Bootstrap should have found at least 1 decompressed Mint"
    );

    // Verify account data
    let current_slot = rpc.get_slot().await.unwrap();
    let ready_accounts = tracker.get_ready_to_compress(current_slot);

    println!(
        "Tracked {} Mints, {} ready to compress",
        tracker.len(),
        ready_accounts.len()
    );

    // Cleanup
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(5), bootstrap_handle).await;

    println!("Mint bootstrap test completed successfully!");
}

/// Test that MintCompressor can compress decompressed mints
///
/// This test creates a mint with rent_payment=0 (immediately compressible),
/// then verifies the compressor can close the on-chain mint account.
///
/// Run with: cargo test -p forester --test test_compressible_mint -- --nocapture
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_mint_compression() {
    // Start validator
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 45,
        sbf_programs: vec![],
        upgradeable_programs: vec![],
        limit_ledger_size: None,
        use_surfpool: true,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    let payer = rpc.get_payer().insecure_clone();

    // Airdrop to payer
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .expect("Failed to airdrop lamports");

    // Wait for indexer to be ready before making validity proof requests
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Create a decompressed mint
    let (mint_pda, compression_address, mint_seed, bump) =
        create_decompressed_mint(&mut rpc, &payer, payer.pubkey(), 9).await;

    println!("Created decompressed mint at: {}", mint_pda);

    // Verify mint exists
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(mint_account.is_some(), "Mint should exist");

    // Verify mint is decompressed using single assert_eq against expected Mint
    let mint_data = mint_account.clone().unwrap();
    let mint = Mint::deserialize(&mut &mint_data.data[..]).expect("Failed to deserialize Mint");

    // Extract runtime-specific values from deserialized mint
    let compression = mint.compression;
    let metadata_version = mint.metadata.version;

    // Build expected Mint
    let expected_mint = Mint {
        base: BaseMint {
            mint_authority: Some(payer.pubkey().to_bytes().into()),
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: metadata_version,
            mint_decompressed: true,
            mint: mint_pda.to_bytes().into(),
            mint_signer: mint_seed.pubkey().to_bytes(),
            bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression,
        extensions: None,
    };

    assert_eq!(mint, expected_mint, "Mint should match expected state");

    // Wait for indexer after mint creation
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Create tracker and add the mint manually
    let tracker = Arc::new(MintAccountTracker::new());

    // Update tracker from the actual account
    let mint_account_data = mint_account.unwrap();
    tracker
        .update_from_account(
            mint_pda,
            &mint_account_data.data,
            mint_account_data.lamports,
        )
        .expect("Failed to update tracker");

    assert_eq!(tracker.len(), 1, "Tracker should have 1 mint");

    // Create RPC pool with indexer URL
    let rpc_pool = Arc::new(
        SolanaRpcPoolBuilder::<LightClient>::new()
            .url("http://localhost:8899".to_string())
            .photon_url(Some("http://127.0.0.1:8784".to_string()))
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .build()
            .await
            .expect("Failed to create RPC pool"),
    );

    // Get ready accounts - with rent_payment=0, the mint is immediately compressible
    let current_slot = rpc.get_slot().await.unwrap();
    let ready_accounts = tracker.get_ready_to_compress(current_slot);
    println!("Ready to compress: {} mints", ready_accounts.len());

    assert!(
        !ready_accounts.is_empty(),
        "Mint should be ready to compress with rent_payment=0"
    );

    // Create compressor and compress
    let compressor = MintCompressor::new(rpc_pool.clone(), tracker.clone(), payer.insecure_clone());

    println!("Compressing Mint...");
    let compress_result = compressor.compress_batch(&ready_accounts).await;

    let signatures = compress_result.expect("Compression should succeed");
    let signature = signatures
        .last()
        .expect("Should have at least one signature");
    println!("Compression transaction sent: {}", signature);

    // Wait for account to be closed
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(30);
    let mut account_closed = false;

    while start.elapsed() < timeout {
        let mint_after = rpc
            .get_account(mint_pda)
            .await
            .expect("Failed to query mint account");
        if mint_after.is_none() {
            account_closed = true;
            println!("Mint account closed successfully!");
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    assert!(
        account_closed,
        "Mint account should be closed after compression"
    );

    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Verify compressed mint still exists in the merkle tree
    let compressed_after = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;
    assert!(
        compressed_after.is_some(),
        "Compressed mint should still exist after compression"
    );

    // Test Photon API: get_compressed_mint
    println!("Testing Photon get_compressed_mint API...");
    let mint_response = rpc
        .get_compressed_mint(compression_address, None)
        .await
        .expect("get_compressed_mint should succeed");

    let compressed_mint = mint_response
        .value
        .expect("Compressed mint should be returned by get_compressed_mint");

    assert_eq!(compressed_mint.mint.decimals, 9, "Decimals should match");
    assert_eq!(
        compressed_mint.mint.mint_authority,
        Some(payer.pubkey()),
        "Mint authority should be payer"
    );
    println!(
        "Photon get_compressed_mint verified: decimals={}, supply={}",
        compressed_mint.mint.decimals, compressed_mint.mint.supply
    );

    // Test Photon API: get_compressed_mint_by_pda
    let mint_by_pda = rpc
        .get_compressed_mint_by_pda(&mint_pda, None)
        .await
        .expect("get_compressed_mint_by_pda should succeed");
    assert!(
        mint_by_pda.value.is_some(),
        "Should find compressed mint by PDA"
    );
    println!("Photon get_compressed_mint_by_pda verified!");

    println!("Mint compression test completed successfully!");
}

/// Test AccountSubscriber for Mint accounts
///
/// This test verifies the full subscription flow:
/// 1. Start AccountSubscriber with MintAccountTracker
/// 2. Create two decompressed mints: one with rent, one immediately compressible
/// 3. Assert subscriber picks up both accounts (tracker.len() == 2)
/// 4. Run MintCompressor to compress the immediately compressible mint
/// 5. Assert account is closed and tracker is updated via direct removal
///
/// Run with: cargo test -p forester --test test_compressible_mint test_compressible_mint_subscription -- --nocapture
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_compressible_mint_subscription() {
    // Start validator with prover enabled (needed for validity proofs)
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 45,
        sbf_programs: vec![],
        upgradeable_programs: vec![],
        limit_ledger_size: None,
        use_surfpool: true,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    let payer = rpc.get_payer().insecure_clone();

    // Airdrop to payer
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .expect("Failed to airdrop lamports");

    // Wait for indexer to be ready
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Setup tracker and subscribers
    let tracker = Arc::new(MintAccountTracker::new());
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // Spawn account subscriber to track new/updated mint accounts
    // Use oneshot channel to surface failures immediately
    let mut account_subscriber = AccountSubscriber::new(
        "ws://localhost:8900".to_string(),
        tracker.clone(),
        SubscriptionConfig::mint(),
        shutdown_rx,
    );
    let (subscriber_result_tx, mut subscriber_result_rx) =
        oneshot::channel::<Result<(), anyhow::Error>>();
    let account_subscriber_handle = tokio::spawn(async move {
        let result = account_subscriber.run().await;
        let _ = subscriber_result_tx.send(result);
    });

    // Give subscribers time to connect
    sleep(Duration::from_secs(2)).await;

    // Create first decompressed mint (immediately compressible with rent_payment=0)
    let (mint_pda_1, compression_address_1, _mint_seed_1, _bump_1) =
        create_decompressed_mint(&mut rpc, &payer, payer.pubkey(), 9).await;
    println!("Created first decompressed mint at: {}", mint_pda_1);

    // Wait for subscriber to pick up the account
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(30);
    while start.elapsed() < timeout {
        // Check for early subscriber failure
        if let Ok(result) = subscriber_result_rx.try_recv() {
            result.expect("Account subscriber failed early");
        }
        if tracker.len() >= 1 {
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }

    // Verify tracker picked up the first mint
    assert_eq!(
        tracker.len(),
        1,
        "Tracker should have 1 mint after first creation"
    );
    println!("Tracker detected first mint via subscription");

    // Create second decompressed mint
    let (mint_pda_2, _compression_address_2, _mint_seed_2, _bump_2) =
        create_decompressed_mint(&mut rpc, &payer, payer.pubkey(), 6).await;
    println!("Created second decompressed mint at: {}", mint_pda_2);

    // Wait for subscriber to pick up the second account
    let start = tokio::time::Instant::now();
    while start.elapsed() < timeout {
        // Check for early subscriber failure
        if let Ok(result) = subscriber_result_rx.try_recv() {
            result.expect("Account subscriber failed early");
        }
        if tracker.len() >= 2 {
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }

    // Verify tracker has both mints
    assert_eq!(
        tracker.len(),
        2,
        "Tracker should have 2 mints after second creation"
    );
    println!("Tracker detected second mint via subscription");

    // Create RPC pool for compressor
    let rpc_pool = Arc::new(
        SolanaRpcPoolBuilder::<LightClient>::new()
            .url("http://localhost:8899".to_string())
            .photon_url(Some("http://127.0.0.1:8784".to_string()))
            .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
            .build()
            .await
            .expect("Failed to create RPC pool"),
    );

    // Get ready-to-compress accounts
    let current_slot = rpc.get_slot().await.unwrap();
    let ready_accounts = tracker.get_ready_to_compress(current_slot);
    println!(
        "Ready to compress: {} mints (current_slot: {})",
        ready_accounts.len(),
        current_slot
    );

    // Both mints should be ready (rent_payment=0)
    assert_eq!(
        ready_accounts.len(),
        2,
        "Both mints should be ready to compress"
    );

    // Compress just the first mint
    let compressor = MintCompressor::new(rpc_pool.clone(), tracker.clone(), payer.insecure_clone());

    // Compress only the first mint
    let first_mint_state = ready_accounts
        .iter()
        .find(|m| m.pubkey == mint_pda_1)
        .expect("First mint should be in ready accounts")
        .clone();

    println!("Compressing first mint: {}", mint_pda_1);
    let signatures = compressor
        .compress_batch(&[first_mint_state])
        .await
        .expect("Compression should succeed");
    let signature = signatures
        .last()
        .expect("Should have at least one signature");

    println!("Compression tx sent: {}", signature);

    // Wait for mint account to be closed
    let start = tokio::time::Instant::now();
    let mut account_closed = false;
    while start.elapsed() < timeout {
        let mint_after = rpc
            .get_account(mint_pda_1)
            .await
            .expect("Failed to query mint account");
        if mint_after.is_none() {
            account_closed = true;
            println!("First mint account closed successfully!");
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
    assert!(account_closed, "First mint account should be closed");

    // Verify tracker was updated (compress_batch removes from tracker after successful compression)
    assert_eq!(
        tracker.len(),
        1,
        "Tracker should have 1 mint after compression"
    );
    println!("Tracker updated: now has {} mint(s)", tracker.len());

    // Verify the remaining mint is the second one
    let remaining_accounts = tracker.get_ready_to_compress(current_slot);
    assert_eq!(remaining_accounts.len(), 1);
    assert_eq!(
        remaining_accounts[0].pubkey, mint_pda_2,
        "Remaining mint should be the second one"
    );

    // Verify compressed mint still exists in merkle tree
    let compressed_after = rpc
        .get_compressed_account(compression_address_1, None)
        .await
        .unwrap()
        .value;
    assert!(
        compressed_after.is_some(),
        "Compressed mint should still exist after compression"
    );

    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    // Test Photon API: get_compressed_mint by address
    println!("Testing Photon get_compressed_mint API...");
    let mint_response = rpc
        .get_compressed_mint(compression_address_1, None)
        .await
        .expect("get_compressed_mint should succeed");

    let compressed_mint = mint_response
        .value
        .expect("Compressed mint should be returned by get_compressed_mint");

    // Verify mint data matches what we created
    assert_eq!(
        compressed_mint.mint.decimals, 9,
        "Decimals should match what we created"
    );
    assert_eq!(
        compressed_mint.mint.mint_authority,
        Some(payer.pubkey()),
        "Mint authority should be payer"
    );
    assert!(
        !compressed_mint.mint.mint_decompressed,
        "Mint should NOT be marked as decompressed after compression"
    );
    println!(
        "get_compressed_mint verified: decimals={}, supply={}",
        compressed_mint.mint.decimals, compressed_mint.mint.supply
    );

    // Test Photon API: get_compressed_mint_by_pda
    println!("Testing Photon get_compressed_mint_by_pda API...");
    let mint_by_pda = rpc
        .get_compressed_mint_by_pda(&mint_pda_1, None)
        .await
        .expect("get_compressed_mint_by_pda should succeed");

    assert!(
        mint_by_pda.value.is_some(),
        "Compressed mint should be found by PDA"
    );
    assert_eq!(
        mint_by_pda.value.as_ref().unwrap().mint.decimals,
        compressed_mint.mint.decimals,
        "Mint found by PDA should match mint found by address"
    );
    println!("get_compressed_mint_by_pda verified!");

    // Test Photon API: get_compressed_mints_by_authority
    println!("Testing Photon get_compressed_mints_by_authority API...");
    let mints_by_authority = rpc
        .get_compressed_mints_by_authority(
            &payer.pubkey(),
            light_client::indexer::MintAuthorityType::Either,
            None,
            None,
        )
        .await
        .expect("get_compressed_mints_by_authority should succeed");

    // We compressed mint_pda_1 (payer is authority), and mint_pda_2 is still decompressed
    // So we should have exactly 1 compressed mint with payer as authority
    assert!(
        !mints_by_authority.value.items.is_empty(),
        "Should find at least 1 compressed mint by authority"
    );
    println!(
        "get_compressed_mints_by_authority found {} mints for authority {}",
        mints_by_authority.value.items.len(),
        payer.pubkey()
    );

    // Verify the first mint in the list is the one we compressed
    let found_mint = mints_by_authority
        .value
        .items
        .iter()
        .find(|m| m.mint.decimals == 9);
    assert!(
        found_mint.is_some(),
        "Should find the mint with 9 decimals in authority query results"
    );
    println!("Photon API tests completed successfully!");

    // Shutdown subscribers
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");
    account_subscriber_handle
        .await
        .expect("Account subscriber task panicked");
    // Check if subscriber returned an error (if not already consumed by try_recv)
    if let Ok(result) = subscriber_result_rx.try_recv() {
        result.expect("Account subscriber failed");
    }

    println!("Mint subscription test completed successfully!");
}
