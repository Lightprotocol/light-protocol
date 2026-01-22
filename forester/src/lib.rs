pub type Result<T> = anyhow::Result<T>;

pub mod api_server;
pub mod cli;
pub mod compressible;
pub mod config;
pub mod epoch_manager;
pub mod errors;
pub mod forester_status;
pub mod health_check;
pub mod helius_priority_fee_types;
pub mod metrics;
pub mod pagerduty;
pub mod processor;
pub mod pubsub_client;
pub mod queue_helpers;
pub mod rollover;
pub mod slot_tracker;
pub mod smart_transaction;
pub mod telemetry;
pub mod tree_data_sync;
pub mod utils;

use std::{sync::Arc, time::Duration};

pub use config::{ForesterConfig, ForesterEpochInfo};
use forester_utils::{
    forester_epoch::TreeAccounts, rate_limiter::RateLimiter, rpc_pool::SolanaRpcPoolBuilder,
};
use itertools::Itertools;
use light_client::{
    indexer::Indexer,
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::TreeType;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::debug;

use crate::{
    epoch_manager::{run_service, WorkReport},
    metrics::QUEUE_LENGTH,
    processor::tx_cache::ProcessedHashCache,
    queue_helpers::{
        fetch_queue_item_data, print_address_v2_queue_info, print_state_v2_input_queue_info,
        print_state_v2_output_queue_info,
    },
    slot_tracker::SlotTracker,
    utils::{get_protocol_config_with_retry, get_slot_with_retry, retry_with_backoff, RetryConfig},
};

pub async fn run_queue_info(
    config: Arc<ForesterConfig>,
    trees: &[TreeAccounts],
    queue_type: TreeType,
) -> Result<()> {
    let mut rpc = LightClient::new(LightClientConfig {
        url: config.external_services.rpc_url.to_string(),
        photon_url: config.external_services.indexer_url.clone(),
        api_key: config.external_services.photon_api_key.clone(),
        commitment_config: None,
        fetch_active_tree: false,
    })
    .await?;
    let trees: Vec<_> = trees
        .iter()
        .filter(|t| t.tree_type == queue_type && !t.is_rolledover)
        .sorted_by_key(|t| t.merkle_tree.to_string())
        .cloned()
        .collect();

    for tree_data in trees {
        match tree_data.tree_type {
            TreeType::StateV1 => {
                let queue_length = fetch_queue_item_data(&mut rpc, &tree_data.queue, 0)
                    .await?
                    .len();
                QUEUE_LENGTH
                    .with_label_values(&[
                        &*queue_type.to_string(),
                        &tree_data.merkle_tree.to_string(),
                    ])
                    .set(queue_length as i64);

                println!(
                    "{:?} queue {} length: {}",
                    queue_type, tree_data.queue, queue_length
                );
            }
            TreeType::AddressV1 => {
                let queue_length = fetch_queue_item_data(&mut rpc, &tree_data.queue, 0)
                    .await?
                    .len();
                QUEUE_LENGTH
                    .with_label_values(&[
                        &*queue_type.to_string(),
                        &tree_data.merkle_tree.to_string(),
                    ])
                    .set(queue_length as i64);

                println!(
                    "{:?} queue {} length: {}",
                    queue_type, tree_data.queue, queue_length
                );
            }
            TreeType::StateV2 => {
                println!("\n=== StateV2 {} ===", tree_data.merkle_tree);

                println!("\n1. APPEND OPERATIONS:");
                let append_unprocessed =
                    print_state_v2_output_queue_info(&mut rpc, &tree_data.queue).await?;

                println!("\n2. NULLIFY OPERATIONS:");
                let nullify_unprocessed =
                    print_state_v2_input_queue_info(&mut rpc, &tree_data.merkle_tree).await?;

                println!("===========================================\n");

                QUEUE_LENGTH
                    .with_label_values(&["StateV2.Append", &tree_data.queue.to_string()])
                    .set(append_unprocessed as i64);

                QUEUE_LENGTH
                    .with_label_values(&["StateV2.Nullify", &tree_data.merkle_tree.to_string()])
                    .set(nullify_unprocessed as i64);
            }
            TreeType::AddressV2 => {
                println!("\n=== AddressV2 {} ===", tree_data.merkle_tree);
                let queue_length =
                    print_address_v2_queue_info(&mut rpc, &tree_data.merkle_tree).await?;
                println!("===========================================\n");
                QUEUE_LENGTH
                    .with_label_values(&["AddressV2", &tree_data.merkle_tree.to_string()])
                    .set(queue_length as i64);
            }
            TreeType::Unknown => {
                // Virtual tree type for compression, no queue to monitor
            }
        };
    }
    Ok(())
}

pub async fn run_pipeline<R: Rpc + Indexer>(
    config: Arc<ForesterConfig>,
    rpc_rate_limiter: Option<RateLimiter>,
    send_tx_rate_limiter: Option<RateLimiter>,
    shutdown_service: oneshot::Receiver<()>,
    shutdown_compressible: Option<tokio::sync::broadcast::Receiver<()>>,
    shutdown_bootstrap: Option<oneshot::Receiver<()>>,
    work_report_sender: mpsc::Sender<WorkReport>,
) -> Result<()> {
    let mut builder = SolanaRpcPoolBuilder::<R>::default()
        .url(config.external_services.rpc_url.to_string())
        .photon_url(config.external_services.indexer_url.clone())
        .api_key(config.external_services.photon_api_key.clone())
        .commitment(CommitmentConfig::processed())
        .max_size(config.rpc_pool_config.max_size)
        .connection_timeout_secs(config.rpc_pool_config.connection_timeout_secs)
        .idle_timeout_secs(config.rpc_pool_config.idle_timeout_secs)
        .max_retries(config.rpc_pool_config.max_retries)
        .initial_retry_delay_ms(config.rpc_pool_config.initial_retry_delay_ms)
        .max_retry_delay_ms(config.rpc_pool_config.max_retry_delay_ms);

    if let Some(limiter) = rpc_rate_limiter {
        builder = builder.rpc_rate_limiter(limiter);
    }

    if let Some(limiter) = send_tx_rate_limiter {
        builder = builder.send_tx_rate_limiter(limiter);
    }

    let arc_pool = Arc::new(builder.build().await?);
    let arc_pool_clone = Arc::clone(&arc_pool);

    let (protocol_config, slot) = {
        let mut rpc = arc_pool.get_connection().await?;
        let protocol_config = get_protocol_config_with_retry(&mut *rpc).await;
        let slot = get_slot_with_retry(&mut *rpc).await;
        (protocol_config, slot)
    };
    let slot_tracker = SlotTracker::new(
        slot,
        Duration::from_secs(config.general_config.slot_update_interval_seconds),
    );
    let arc_slot_tracker = Arc::new(slot_tracker);
    let arc_slot_tracker_clone = arc_slot_tracker.clone();
    let slot_tracker_handle = tokio::spawn(async move {
        loop {
            match arc_pool_clone.get_connection().await {
                Ok(mut rpc) => {
                    SlotTracker::run(arc_slot_tracker_clone.clone(), &mut *rpc).await;
                    // If SlotTracker::run returns, the connection likely failed
                    tracing::warn!("SlotTracker connection lost, reconnecting...");
                }
                Err(e) => {
                    tracing::error!("Failed to get RPC connection for SlotTracker: {:?}", e);
                }
            }
            // Wait before retrying
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    let tx_cache = Arc::new(Mutex::new(ProcessedHashCache::new(
        config.transaction_config.tx_cache_ttl_seconds,
    )));

    let ops_cache = Arc::new(Mutex::new(ProcessedHashCache::new(
        config.transaction_config.ops_cache_ttl_seconds,
    )));

    let (compressible_tracker, pda_tracker, mint_tracker) = if let Some(compressible_config) =
        &config.compressible_config
    {
        // Validate on-chain CompressibleConfig at startup (fail fast on misconfiguration)
        compressible::validate_compressible_config(&config.external_services.rpc_url).await?;

        if let Some(shutdown_rx) = shutdown_compressible {
            // Create all shutdown receivers upfront (before any are moved)
            let shutdown_rx_ctoken = shutdown_rx.resubscribe();
            let shutdown_rx_mint = shutdown_rx.resubscribe();
            let shutdown_rx_mint_bootstrap = shutdown_rx.resubscribe();
            // Keep original for PDA subscriptions (will resubscribe per-program)
            let shutdown_rx_pda_base = shutdown_rx;

            // Create ctoken tracker
            let ctoken_tracker = Arc::new(compressible::CTokenAccountTracker::new());
            let tracker_clone = ctoken_tracker.clone();
            let ws_url = compressible_config.ws_url.clone();

            // Spawn account subscriber for ctokens
            tokio::spawn(async move {
                let mut subscriber = compressible::AccountSubscriber::new(
                    ws_url,
                    tracker_clone,
                    compressible::SubscriptionConfig::ctoken(),
                    shutdown_rx_ctoken,
                );
                if let Err(e) = subscriber.run().await {
                    tracing::error!("Compressible subscriber error: {:?}", e);
                }
            });

            // Spawn bootstrap task for ctokens with shutdown support
            if let Some(mut shutdown_bootstrap_rx) = shutdown_bootstrap {
                let tracker_clone = ctoken_tracker.clone();
                let rpc_url = config.external_services.rpc_url.clone();

                tokio::spawn(async move {
                    let retry_config = RetryConfig::new("CToken bootstrap")
                        .with_max_attempts(3)
                        .with_initial_delay(Duration::from_secs(5));

                    let bootstrap_future = retry_with_backoff(retry_config, || {
                        let rpc_url = rpc_url.clone();
                        let tracker = tracker_clone.clone();
                        async move {
                            compressible::bootstrap_ctoken_accounts(rpc_url, tracker, None).await
                        }
                    });

                    tokio::select! {
                        result = bootstrap_future => {
                            match result {
                                Ok(()) => tracing::info!("CToken bootstrap complete"),
                                Err(e) => tracing::error!("CToken bootstrap failed after retries: {:?}", e),
                            }
                        }
                        _ = &mut shutdown_bootstrap_rx => {
                            tracing::info!("CToken bootstrap interrupted by shutdown signal");
                        }
                    }
                });
            }

            // Create PDA tracker if there are PDA programs configured
            let pda_tracker = if !compressible_config.pda_programs.is_empty() {
                let pda_tracker = Arc::new(compressible::pda::PdaAccountTracker::new(
                    compressible_config.pda_programs.clone(),
                ));

                // Spawn account subscribers for each PDA program
                for pda_config in &compressible_config.pda_programs {
                    let pda_tracker_sub = pda_tracker.clone();
                    let ws_url_pda = compressible_config.ws_url.clone();
                    let shutdown_rx_pda = shutdown_rx_pda_base.resubscribe();
                    let program_id = pda_config.program_id;
                    let discriminator = pda_config.discriminator;
                    let program_name = format!(
                        "pda-{}",
                        program_id.to_string().chars().take(8).collect::<String>()
                    );

                    tokio::spawn(async move {
                        let mut subscriber = compressible::AccountSubscriber::new(
                            ws_url_pda,
                            pda_tracker_sub,
                            compressible::SubscriptionConfig::pda(
                                program_id,
                                discriminator,
                                program_name.clone(),
                            ),
                            shutdown_rx_pda,
                        );
                        if let Err(e) = subscriber.run().await {
                            tracing::error!("PDA subscriber error for {}: {:?}", program_name, e);
                        }
                    });
                }

                // Spawn bootstrap task for PDAs with shutdown support
                let pda_tracker_clone = pda_tracker.clone();
                let rpc_url = config.external_services.rpc_url.clone();
                let mut shutdown_rx_pda_bootstrap = shutdown_rx_pda_base.resubscribe();

                tokio::spawn(async move {
                    let retry_config = RetryConfig::new("PDA bootstrap")
                        .with_max_attempts(3)
                        .with_initial_delay(Duration::from_secs(5));

                    let bootstrap_future = retry_with_backoff(retry_config, || {
                        let rpc_url = rpc_url.clone();
                        let tracker = pda_tracker_clone.clone();
                        async move {
                            compressible::pda::bootstrap_pda_accounts(rpc_url, tracker, None).await
                        }
                    });

                    tokio::select! {
                        result = bootstrap_future => {
                            match result {
                                Ok(()) => tracing::info!("PDA bootstrap complete"),
                                Err(e) => tracing::error!("PDA bootstrap failed after retries: {:?}", e),
                            }
                        }
                        _ = shutdown_rx_pda_bootstrap.recv() => {
                            tracing::info!("PDA bootstrap interrupted by shutdown signal");
                        }
                    }
                });

                Some(pda_tracker)
            } else {
                None
            };

            // Create Mint tracker and spawn subscriptions + bootstrap
            let mint_tracker = {
                let mint_tracker = Arc::new(compressible::mint::MintAccountTracker::new());

                // Spawn account subscriber for mints
                let mint_tracker_sub = mint_tracker.clone();
                let ws_url_mint = compressible_config.ws_url.clone();

                tokio::spawn(async move {
                    let mut subscriber = compressible::AccountSubscriber::new(
                        ws_url_mint,
                        mint_tracker_sub,
                        compressible::SubscriptionConfig::mint(),
                        shutdown_rx_mint,
                    );
                    if let Err(e) = subscriber.run().await {
                        tracing::error!("Mint subscriber error: {:?}", e);
                    }
                });

                // Spawn bootstrap task for Mints with shutdown support
                let mint_tracker_clone = mint_tracker.clone();
                let rpc_url = config.external_services.rpc_url.clone();
                let mut shutdown_rx_mint_bootstrap = shutdown_rx_mint_bootstrap;

                tokio::spawn(async move {
                    let retry_config = RetryConfig::new("Mint bootstrap")
                        .with_max_attempts(3)
                        .with_initial_delay(Duration::from_secs(5));

                    let bootstrap_future = retry_with_backoff(retry_config, || {
                        let rpc_url = rpc_url.clone();
                        let tracker = mint_tracker_clone.clone();
                        async move {
                            compressible::mint::bootstrap_mint_accounts(rpc_url, tracker, None)
                                .await
                        }
                    });

                    tokio::select! {
                        result = bootstrap_future => {
                            match result {
                                Ok(()) => tracing::info!("Mint bootstrap complete"),
                                Err(e) => tracing::error!("Mint bootstrap failed after retries: {:?}", e),
                            }
                        }
                        _ = shutdown_rx_mint_bootstrap.recv() => {
                            tracing::info!("Mint bootstrap interrupted by shutdown signal");
                        }
                    }
                });

                Some(mint_tracker)
            };

            (Some(ctoken_tracker), pda_tracker, mint_tracker)
        } else {
            tracing::warn!("Compressible config enabled but no shutdown receiver provided");
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    debug!("Starting Forester pipeline");
    let result = run_service(
        config,
        Arc::new(protocol_config),
        arc_pool,
        shutdown_service,
        work_report_sender,
        arc_slot_tracker,
        tx_cache,
        ops_cache,
        compressible_tracker,
        pda_tracker,
        mint_tracker,
    )
    .await;

    // Stop the SlotTracker task to prevent panic during shutdown
    tracing::debug!("Stopping SlotTracker task");
    slot_tracker_handle.abort();

    result
}
