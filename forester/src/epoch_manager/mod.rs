mod compression;
pub(crate) mod context;
mod monitor;
mod pipeline;
pub(crate) mod processor_pool;
mod registration;
mod reporting;
pub(crate) mod tracker;
mod v1;
mod v2;

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use forester_utils::{forester_epoch::TreeAccounts, rpc_pool::SolanaRpcPool};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::TreeType;
use light_registry::protocol_config::state::ProtocolConfig;
use solana_sdk::{address_lookup_table::AddressLookupTableAccount, signature::Signer};
use tokio::{
    sync::{mpsc, oneshot, watch, Mutex},
    task::JoinHandle,
    time::{sleep, Instant, MissedTickBehavior},
};
use tracing::{debug, error, info, info_span};

use self::{context::ForesterContext, processor_pool::ProcessorPool, tracker::EpochTracker};
use crate::{
    compressible::CTokenAccountTracker,
    errors::InitializationError,
    logging::ServiceHeartbeat,
    processor::tx_cache::ProcessedHashCache,
    queue_helpers::QueueItemData,
    slot_tracker::SlotTracker,
    tree_data_sync::{fetch_protocol_group_authority, fetch_trees},
    ForesterConfig, Result,
};

// ── Public re-exports (preserve existing public API) ─────────────────────

/// Timing for a single circuit type (circuit inputs + proof generation)
#[derive(Copy, Clone, Debug, Default)]
pub struct CircuitMetrics {
    /// Time spent building circuit inputs
    pub circuit_inputs_duration: std::time::Duration,
    /// Time spent generating ZK proofs (pure prover server time)
    pub proof_generation_duration: std::time::Duration,
    /// Total round-trip time (submit to result, includes queue wait)
    pub round_trip_duration: std::time::Duration,
}

impl CircuitMetrics {
    pub fn total(&self) -> std::time::Duration {
        self.circuit_inputs_duration + self.proof_generation_duration
    }
}

impl std::ops::AddAssign for CircuitMetrics {
    fn add_assign(&mut self, rhs: Self) {
        self.circuit_inputs_duration += rhs.circuit_inputs_duration;
        self.proof_generation_duration += rhs.proof_generation_duration;
        self.round_trip_duration += rhs.round_trip_duration;
    }
}

/// Timing breakdown by circuit type
#[derive(Copy, Clone, Debug, Default)]
pub struct ProcessingMetrics {
    /// State append circuit (output queue processing)
    pub append: CircuitMetrics,
    /// State nullify circuit (input queue processing)
    pub nullify: CircuitMetrics,
    /// Address append circuit
    pub address_append: CircuitMetrics,
    /// Time spent sending transactions (overlapped with proof gen)
    pub tx_sending_duration: std::time::Duration,
}

impl ProcessingMetrics {
    pub fn total(&self) -> std::time::Duration {
        self.append.total()
            + self.nullify.total()
            + self.address_append.total()
            + self.tx_sending_duration
    }

    pub fn total_circuit_inputs(&self) -> std::time::Duration {
        self.append.circuit_inputs_duration
            + self.nullify.circuit_inputs_duration
            + self.address_append.circuit_inputs_duration
    }

    pub fn total_proof_generation(&self) -> std::time::Duration {
        self.append.proof_generation_duration
            + self.nullify.proof_generation_duration
            + self.address_append.proof_generation_duration
    }

    pub fn total_round_trip(&self) -> std::time::Duration {
        self.append.round_trip_duration
            + self.nullify.round_trip_duration
            + self.address_append.round_trip_duration
    }
}

impl std::ops::AddAssign for ProcessingMetrics {
    fn add_assign(&mut self, rhs: Self) {
        self.append += rhs.append;
        self.nullify += rhs.nullify;
        self.address_append += rhs.address_append;
        self.tx_sending_duration += rhs.tx_sending_duration;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WorkReport {
    pub epoch: u64,
    pub processed_items: usize,
    pub metrics: ProcessingMetrics,
}

#[derive(Debug, Clone)]
pub struct WorkItem {
    pub tree_account: TreeAccounts,
    pub queue_item_data: QueueItemData,
}

impl WorkItem {
    pub fn is_address_tree(&self) -> bool {
        self.tree_account.tree_type == TreeType::AddressV1
    }
    pub fn is_state_tree(&self) -> bool {
        self.tree_account.tree_type == TreeType::StateV1
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum MerkleProofType {
    AddressProof(light_client::indexer::NewAddressProofWithContext),
    StateProof(light_client::indexer::MerkleProof),
}

// ── EpochManager ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct EpochManager<R: Rpc + Indexer> {
    pub(crate) ctx: ForesterContext<R>,
    pub(crate) epoch_tracker: EpochTracker,
    pub(crate) processor_pool: ProcessorPool<R>,
    pub(crate) trees: Arc<Mutex<Vec<TreeAccounts>>>,
    // External integrations
    pub(crate) work_report_sender: mpsc::Sender<WorkReport>,
    pub(crate) tx_cache: Arc<Mutex<ProcessedHashCache>>,
    pub(crate) ops_cache: Arc<Mutex<ProcessedHashCache>>,
    pub(crate) compressible_tracker: Option<Arc<CTokenAccountTracker>>,
    pub(crate) pda_tracker: Option<Arc<crate::compressible::pda::PdaAccountTracker>>,
    pub(crate) mint_tracker: Option<Arc<crate::compressible::mint::MintAccountTracker>>,
    pub(crate) shutdown_tx: watch::Sender<bool>,
}

impl<R: Rpc + Indexer> Clone for EpochManager<R> {
    fn clone(&self) -> Self {
        Self {
            ctx: self.ctx.clone(),
            epoch_tracker: self.epoch_tracker.clone(),
            processor_pool: self.processor_pool.clone(),
            trees: self.trees.clone(),
            work_report_sender: self.work_report_sender.clone(),
            tx_cache: self.tx_cache.clone(),
            ops_cache: self.ops_cache.clone(),
            compressible_tracker: self.compressible_tracker.clone(),
            pda_tracker: self.pda_tracker.clone(),
            mint_tracker: self.mint_tracker.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

impl<R: Rpc + Indexer> EpochManager<R> {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        config: Arc<ForesterConfig>,
        protocol_config: Arc<ProtocolConfig>,
        rpc_pool: Arc<SolanaRpcPool<R>>,
        work_report_sender: mpsc::Sender<WorkReport>,
        trees: Vec<TreeAccounts>,
        slot_tracker: Arc<SlotTracker>,
        tx_cache: Arc<Mutex<ProcessedHashCache>>,
        ops_cache: Arc<Mutex<ProcessedHashCache>>,
        compressible_tracker: Option<Arc<CTokenAccountTracker>>,
        pda_tracker: Option<Arc<crate::compressible::pda::PdaAccountTracker>>,
        mint_tracker: Option<Arc<crate::compressible::mint::MintAccountTracker>>,
        address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
        heartbeat: Arc<ServiceHeartbeat>,
        run_id: String,
    ) -> Result<Self> {
        let authority = Arc::new(config.payer_keypair.insecure_clone());
        let ctx = ForesterContext {
            config,
            protocol_config,
            rpc_pool,
            authority,
            slot_tracker,
            address_lookup_tables,
            heartbeat,
            run_id: Arc::<str>::from(run_id),
        };
        Ok(Self {
            ctx,
            epoch_tracker: EpochTracker::new(),
            processor_pool: ProcessorPool::new(),
            trees: Arc::new(Mutex::new(trees)),
            work_report_sender,
            tx_cache,
            ops_cache,
            compressible_tracker,
            pda_tracker,
            mint_tracker,
            shutdown_tx: watch::channel(false).0,
        })
    }

    pub(crate) fn request_shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let tx = Arc::new(tx);

        let mut monitor_handle = tokio::spawn({
            let self_clone = self.clone();
            let tx_clone = tx.clone();
            async move { self_clone.monitor_epochs(tx_clone).await }
        });

        let current_previous_handle = tokio::spawn({
            let self_clone = self.clone();
            let tx_clone = tx.clone();
            async move {
                self_clone
                    .process_current_and_previous_epochs(tx_clone)
                    .await
            }
        });

        let tree_discovery_handle = tokio::spawn({
            let self_clone = self.clone();
            async move { self_clone.discover_trees_periodically().await }
        });

        let balance_check_handle = tokio::spawn({
            let self_clone = self.clone();
            async move { self_clone.check_sol_balance_periodically().await }
        });

        let _guard = scopeguard::guard(
            (
                monitor_handle.abort_handle(),
                current_previous_handle,
                tree_discovery_handle,
                balance_check_handle,
            ),
            |(monitor, h2, h3, h4)| {
                info!(
                    event = "background_tasks_aborting",
                    run_id = %self.ctx.run_id,
                    "Aborting EpochManager background tasks"
                );
                monitor.abort();
                h2.abort();
                h3.abort();
                h4.abort();
            },
        );

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut epoch_tasks = tokio::task::JoinSet::new();
        let result = loop {
            if *shutdown_rx.borrow_and_update() {
                info!(
                    event = "epoch_manager_shutdown_requested",
                    run_id = %self.ctx.run_id,
                    "Stopping EpochManager after shutdown request"
                );
                break Ok(());
            }

            tokio::select! {
                _ = shutdown_rx.changed() => {}
                Some(join_result) = epoch_tasks.join_next() => {
                    match join_result {
                        Ok(Ok(())) => debug!(
                            event = "epoch_processing_completed",
                            run_id = %self.ctx.run_id,
                            "Epoch processed successfully"
                        ),
                        Ok(Err(e)) => error!(
                            event = "epoch_processing_failed",
                            run_id = %self.ctx.run_id,
                            error = ?e,
                            "Error processing epoch"
                        ),
                        Err(join_error) => {
                            if join_error.is_panic() {
                                error!(
                                    event = "epoch_processing_panicked",
                                    run_id = %self.ctx.run_id,
                                    error = %join_error,
                                    "Epoch processing panicked"
                                );
                            }
                        }
                    }
                }
                epoch_opt = rx.recv() => {
                    match epoch_opt {
                        Some(epoch) => {
                            debug!(
                                event = "epoch_queued_for_processing",
                                run_id = %self.ctx.run_id,
                                epoch,
                                "Received epoch from monitor"
                            );
                            let self_clone = self.clone();
                            epoch_tasks.spawn(async move {
                                self_clone.process_epoch(epoch).await
                            });
                        }
                        None => {
                            error!(
                                event = "epoch_monitor_channel_closed",
                                run_id = %self.ctx.run_id,
                                "Epoch monitor channel closed unexpectedly"
                            );
                            break Err(anyhow!(
                                "Epoch monitor channel closed - forester cannot function without it"
                            ));
                        }
                    }
                }
                result = &mut monitor_handle => {
                    match result {
                        Ok(Ok(())) => {
                            error!(
                                event = "epoch_monitor_exited_unexpected_ok",
                                run_id = %self.ctx.run_id,
                                "Epoch monitor exited unexpectedly with Ok(())"
                            );
                        }
                        Ok(Err(e)) => {
                            error!(
                                event = "epoch_monitor_exited_with_error",
                                run_id = %self.ctx.run_id,
                                error = ?e,
                                "Epoch monitor exited with error"
                            );
                        }
                        Err(e) => {
                            error!(
                                event = "epoch_monitor_task_failed",
                                run_id = %self.ctx.run_id,
                                error = ?e,
                                "Epoch monitor task panicked or was cancelled"
                            );
                        }
                    }
                    if let Some(pagerduty_key) = &self.ctx.config.external_services.pagerduty_routing_key {
                        let _ = crate::pagerduty::send_pagerduty_alert(
                            pagerduty_key,
                            &format!("Forester epoch monitor died unexpectedly on {}", self.ctx.config.payer_keypair.pubkey()),
                            "critical",
                            "epoch_monitor_dead",
                        ).await;
                    }
                    break Err(anyhow!("Epoch monitor exited unexpectedly - forester cannot function without it"));
                }
            }
        };

        result
    }
}

// ── run_service (top-level entry point) ──────────────────────────────────

pub fn generate_run_id() -> String {
    let epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{}", std::process::id(), epoch_ms)
}

fn spawn_heartbeat_task(
    heartbeat: Arc<ServiceHeartbeat>,
    slot_tracker: Arc<SlotTracker>,
    protocol_config: Arc<ProtocolConfig>,
    run_id: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut previous = heartbeat.snapshot();

        loop {
            interval.tick().await;

            let slot = slot_tracker.estimated_current_slot();
            let epoch = protocol_config.get_current_active_epoch(slot).ok();
            let epoch_known = epoch.is_some();
            let epoch_value = epoch.unwrap_or_default();
            let current = heartbeat.snapshot();

            let delta_active = current.active_cycles.saturating_sub(previous.active_cycles);
            let delta_queues_started = current
                .queues_started
                .saturating_sub(previous.queues_started);
            let delta_queues_finished = current
                .queues_finished
                .saturating_sub(previous.queues_finished);
            let delta_items = current
                .items_processed
                .saturating_sub(previous.items_processed);
            let delta_work_reports = current
                .work_reports_sent
                .saturating_sub(previous.work_reports_sent);
            let delta_v2_recoverable = current
                .v2_recoverable_errors
                .saturating_sub(previous.v2_recoverable_errors);
            let delta_tree_tasks = current
                .tree_tasks_spawned
                .saturating_sub(previous.tree_tasks_spawned);

            info!(
                event = "heartbeat",
                run_id = %run_id,
                slot,
                epoch_known,
                epoch = epoch_value,
                delta_active_cycles = delta_active,
                delta_queues_started,
                delta_queues_finished,
                delta_items_processed = delta_items,
                delta_work_reports = delta_work_reports,
                delta_v2_recoverable_errors = delta_v2_recoverable,
                delta_tree_tasks_spawned = delta_tree_tasks,
                cumulative_active_cycles = current.active_cycles,
                cumulative_items_processed = current.items_processed,
                cumulative_work_reports = current.work_reports_sent,
                cumulative_tree_tasks = current.tree_tasks_spawned,
                "Service heartbeat"
            );

            previous = current;
        }
    })
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(
    level = "debug",
    skip(
        config,
        protocol_config,
        rpc_pool,
        shutdown,
        work_report_sender,
        slot_tracker,
        tx_cache,
        ops_cache,
        compressible_tracker,
        pda_tracker,
        mint_tracker,
        run_id
    ),
    fields(forester = %config.payer_keypair.pubkey())
)]
pub async fn run_service<R: Rpc + Indexer>(
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<SolanaRpcPool<R>>,
    mut shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
    slot_tracker: Arc<SlotTracker>,
    tx_cache: Arc<Mutex<ProcessedHashCache>>,
    ops_cache: Arc<Mutex<ProcessedHashCache>>,
    compressible_tracker: Option<Arc<CTokenAccountTracker>>,
    pda_tracker: Option<Arc<crate::compressible::pda::PdaAccountTracker>>,
    mint_tracker: Option<Arc<crate::compressible::mint::MintAccountTracker>>,
    run_id: String,
) -> Result<()> {
    let heartbeat = Arc::new(ServiceHeartbeat::default());
    let heartbeat_handle = spawn_heartbeat_task(
        heartbeat.clone(),
        slot_tracker.clone(),
        protocol_config.clone(),
        run_id.clone(),
    );

    let run_id_for_logs = run_id.clone();
    let result = info_span!(
        "run_service",
        forester = %config.payer_keypair.pubkey()
    )
    .in_scope(|| async move {
            let processor_mode_str = match (
                config.general_config.skip_v1_state_trees
                    && config.general_config.skip_v1_address_trees,
                config.general_config.skip_v2_state_trees
                    && config.general_config.skip_v2_address_trees,
            ) {
                (true, false) => "v2",
                (false, true) => "v1",
                (false, false) => "all",
                _ => "unknown",
            };
            info!(
                event = "forester_starting",
                run_id = %run_id_for_logs,
                processor_mode = processor_mode_str,
                "Starting forester"
            );

            const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
            const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

            let mut retry_count = 0;
            let mut retry_delay = INITIAL_RETRY_DELAY;
            let start_time = Instant::now();

            let trees = {
                let max_attempts = 10;
                let mut attempts = 0;
                let mut delay = Duration::from_secs(2);

                loop {
                    tokio::select! {
                        biased;
                        _ = &mut shutdown => {
                            info!(
                                event = "shutdown_received",
                                run_id = %run_id_for_logs,
                                phase = "tree_fetch",
                                "Received shutdown signal during tree fetch. Stopping."
                            );
                            return Ok(());
                        }
                        result = rpc_pool.get_connection() => {
                            match result {
                                Ok(rpc) => {
                                    tokio::select! {
                                        biased;
                                        _ = &mut shutdown => {
                                            info!(
                                                event = "shutdown_received",
                                                run_id = %run_id_for_logs,
                                                phase = "tree_fetch",
                                                "Received shutdown signal during tree fetch. Stopping."
                                            );
                                            return Ok(());
                                        }
                                        fetch_result = fetch_trees(&*rpc) => {
                                            match fetch_result {
                                                Ok(mut fetched_trees) => {
                                                    let group_authority = match config.general_config.group_authority {
                                                        Some(ga) => Some(ga),
                                                        None => {
                                                            match fetch_protocol_group_authority(&*rpc, run_id_for_logs.as_str()).await {
                                                                Ok(ga) => {
                                                                    info!(
                                                                        event = "group_authority_default_fetched",
                                                                        run_id = %run_id_for_logs,
                                                                        group_authority = %ga,
                                                                        "Using protocol default group authority"
                                                                    );
                                                                    Some(ga)
                                                                }
                                                                Err(e) => {
                                                                    tracing::warn!(
                                                                        event = "group_authority_fetch_failed",
                                                                        run_id = %run_id_for_logs,
                                                                        error = ?e,
                                                                        "Failed to fetch protocol group authority; processing all trees"
                                                                    );
                                                                    None
                                                                }
                                                            }
                                                        }
                                                    };

                                                    if let Some(group_authority) = group_authority {
                                                        let before_count = fetched_trees.len();
                                                        fetched_trees.retain(|tree| tree.owner == group_authority);
                                                        info!(
                                                            event = "trees_filtered_by_group_authority",
                                                            run_id = %run_id_for_logs,
                                                            group_authority = %group_authority,
                                                            trees_before = before_count,
                                                            trees_after = fetched_trees.len(),
                                                            "Filtered trees by group authority"
                                                        );
                                                    }

                                                    if !config.general_config.tree_ids.is_empty() {
                                                        let tree_ids = &config.general_config.tree_ids;
                                                        fetched_trees.retain(|tree| tree_ids.contains(&tree.merkle_tree));
                                                        if fetched_trees.is_empty() {
                                                            error!(
                                                                event = "trees_filter_explicit_ids_empty",
                                                                run_id = %run_id_for_logs,
                                                                requested_tree_count = tree_ids.len(),
                                                                requested_trees = ?tree_ids,
                                                                "None of the specified trees were found"
                                                            );
                                                            return Err(anyhow::anyhow!(
                                                                "None of the specified trees found: {:?}",
                                                                tree_ids
                                                            ));
                                                        }
                                                        info!(
                                                            event = "trees_filter_explicit_ids",
                                                            run_id = %run_id_for_logs,
                                                            tree_count = tree_ids.len(),
                                                            "Processing only explicitly requested trees"
                                                        );
                                                    }
                                                    break fetched_trees;
                                                }
                                                Err(e) => {
                                                    attempts += 1;
                                                    if attempts >= max_attempts {
                                                        return Err(anyhow::anyhow!(
                                                            "Failed to fetch trees after {} attempts: {:?}",
                                                            max_attempts,
                                                            e
                                                        ));
                                                    }
                                                    tracing::warn!(
                                                        event = "fetch_trees_failed_retrying",
                                                        run_id = %run_id_for_logs,
                                                        attempt = attempts,
                                                        max_attempts,
                                                        retry_delay_ms = delay.as_millis() as u64,
                                                        error = ?e,
                                                        "Failed to fetch trees; retrying"
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    attempts += 1;
                                    if attempts >= max_attempts {
                                        return Err(anyhow::anyhow!(
                                            "Failed to get RPC connection for trees after {} attempts: {:?}",
                                            max_attempts,
                                            e
                                        ));
                                    }
                                    tracing::warn!(
                                        event = "rpc_connection_failed_retrying",
                                        run_id = %run_id_for_logs,
                                        attempt = attempts,
                                        max_attempts,
                                        retry_delay_ms = delay.as_millis() as u64,
                                        error = ?e,
                                        "Failed to get RPC connection; retrying"
                                    );
                                }
                            }
                        }
                    }

                    tokio::select! {
                        biased;
                        _ = &mut shutdown => {
                            info!(
                                event = "shutdown_received",
                                run_id = %run_id_for_logs,
                                phase = "tree_fetch_retry_wait",
                                "Received shutdown signal during retry wait. Stopping."
                            );
                            return Ok(());
                        }
                        _ = sleep(delay) => {
                            delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                        }
                    }
                }
            };
            tracing::trace!("Fetched initial trees: {:?}", trees);

            if !config.general_config.tree_ids.is_empty() {
                info!(
                    event = "tree_discovery_limited_to_explicit_ids",
                    run_id = %run_id_for_logs,
                    tree_count = config.general_config.tree_ids.len(),
                    "Processing specific trees; tree discovery will be limited"
                );
            }

            while retry_count < config.retry_config.max_retries {
                debug!("Creating EpochManager (attempt {})", retry_count + 1);

                let address_lookup_tables = {
                    if let Some(lut_address) = config.lookup_table_address {
                        let rpc = rpc_pool.get_connection().await?;
                        match load_lookup_table_async(&*rpc, lut_address).await {
                            Ok(lut) => {
                                info!(
                                    event = "lookup_table_loaded",
                                    run_id = %run_id_for_logs,
                                    lookup_table = %lut_address,
                                    address_count = lut.addresses.len(),
                                    "Loaded lookup table"
                                );
                                Arc::new(vec![lut])
                            }
                            Err(e) => {
                                debug!(
                                    "Lookup table {} not available: {}. Using legacy transactions.",
                                    lut_address, e
                                );
                                Arc::new(Vec::new())
                            }
                        }
                    } else {
                        debug!("No lookup table address configured. Using legacy transactions.");
                        Arc::new(Vec::new())
                    }
                };

                match EpochManager::new(
                    config.clone(),
                    protocol_config.clone(),
                    rpc_pool.clone(),
                    work_report_sender.clone(),
                    trees.clone(),
                    slot_tracker.clone(),
                    tx_cache.clone(),
                    ops_cache.clone(),
                    compressible_tracker.clone(),
                    pda_tracker.clone(),
                    mint_tracker.clone(),
                    address_lookup_tables,
                    heartbeat.clone(),
                    run_id.clone(),
                )
                .await
                {
                    Ok(epoch_manager) => {
                        let epoch_manager = Arc::new(epoch_manager);
                        debug!(
                            "Successfully created EpochManager after {} attempts",
                            retry_count + 1
                        );

                        let run_future = epoch_manager.clone().run();
                        tokio::pin!(run_future);

                        let result = tokio::select! {
                            result = &mut run_future => result,
                            _ = &mut shutdown => {
                                info!(
                                    event = "shutdown_received",
                                    run_id = %run_id_for_logs,
                                    phase = "service_run",
                                    "Received shutdown signal. Stopping the service."
                                );
                                epoch_manager.request_shutdown();
                                run_future.await
                            }
                        };

                        return result;
                    }
                    Err(e) => {
                        tracing::warn!(
                            event = "epoch_manager_create_failed",
                            run_id = %run_id_for_logs,
                            attempt = retry_count + 1,
                            error = ?e,
                            "Failed to create EpochManager"
                        );
                        retry_count += 1;
                        if retry_count < config.retry_config.max_retries {
                            debug!("Retrying in {:?}", retry_delay);
                            sleep(retry_delay).await;
                            retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
                        } else {
                            error!(
                                event = "forester_start_failed_max_retries",
                                run_id = %run_id_for_logs,
                                attempts = config.retry_config.max_retries,
                                elapsed_ms = start_time.elapsed().as_millis() as u64,
                                error = ?e,
                                "Failed to start forester after max retries"
                            );
                            return Err(InitializationError::MaxRetriesExceeded {
                                attempts: config.retry_config.max_retries,
                                error: e.to_string(),
                            }
                            .into());
                        }
                    }
                }
            }

            Err(
                InitializationError::Unexpected("Retry loop exited without returning".to_string())
                    .into(),
            )
        })
        .await;

    heartbeat_handle.abort();
    result
}

/// Async version of load_lookup_table that works with the Rpc trait
async fn load_lookup_table_async<R: Rpc>(
    rpc: &R,
    lookup_table_address: solana_program::pubkey::Pubkey,
) -> anyhow::Result<AddressLookupTableAccount> {
    use light_client::rpc::lut::AddressLookupTable;

    let account = rpc
        .get_account(lookup_table_address)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!("Lookup table account not found: {}", lookup_table_address)
        })?;

    let address_lookup_table = AddressLookupTable::deserialize(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize AddressLookupTable: {:?}", e))?;

    Ok(AddressLookupTableAccount {
        key: lookup_table_address,
        addresses: address_lookup_table.addresses.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use light_client::rpc::RetryConfig;
    use solana_sdk::{pubkey::Pubkey, signature::Keypair};

    use super::{context::should_skip_tree, *};
    use crate::{
        config::{ExternalServicesConfig, GeneralConfig},
        ForesterConfig,
    };

    fn create_test_config_with_skip_flags(
        skip_v1_state: bool,
        skip_v1_address: bool,
        skip_v2_state: bool,
        skip_v2_address: bool,
    ) -> ForesterConfig {
        ForesterConfig {
            external_services: ExternalServicesConfig {
                rpc_url: "http://localhost:8899".to_string(),
                ws_rpc_url: None,
                indexer_url: None,
                prover_url: None,
                prover_append_url: None,
                prover_update_url: None,
                prover_address_append_url: None,
                prover_api_key: None,
                photon_grpc_url: None,
                pushgateway_url: None,
                pagerduty_routing_key: None,
                rpc_rate_limit: None,
                photon_rate_limit: None,
                send_tx_rate_limit: None,
                prover_polling_interval: None,
                prover_max_wait_time: None,
                fallback_rpc_url: None,
                fallback_indexer_url: None,
            },
            retry_config: RetryConfig::default(),
            queue_config: Default::default(),
            indexer_config: Default::default(),
            transaction_config: Default::default(),
            general_config: GeneralConfig {
                enable_metrics: false,
                skip_v1_state_trees: skip_v1_state,
                skip_v1_address_trees: skip_v1_address,
                skip_v2_state_trees: skip_v2_state,
                skip_v2_address_trees: skip_v2_address,
                sleep_after_processing_ms: 50,
                sleep_when_idle_ms: 100,
                ..Default::default()
            },
            rpc_pool_config: Default::default(),
            registry_pubkey: Pubkey::default(),
            payer_keypair: Keypair::new(),
            derivation_pubkey: Pubkey::default(),
            address_tree_data: vec![],
            state_tree_data: vec![],
            compressible_config: None,
            lookup_table_address: None,
            min_queue_items: None,
            enable_v1_multi_nullify: false,
            work_item_batch_size: 50,
        }
    }

    #[test]
    fn test_should_skip_tree_none_skipped() {
        let config = create_test_config_with_skip_flags(false, false, false, false);
        assert!(!should_skip_tree(&config, &TreeType::StateV1));
        assert!(!should_skip_tree(&config, &TreeType::StateV2));
        assert!(!should_skip_tree(&config, &TreeType::AddressV1));
        assert!(!should_skip_tree(&config, &TreeType::AddressV2));
    }

    #[test]
    fn test_should_skip_tree_all_v1_skipped() {
        let config = create_test_config_with_skip_flags(true, true, false, false);
        assert!(should_skip_tree(&config, &TreeType::StateV1));
        assert!(should_skip_tree(&config, &TreeType::AddressV1));
        assert!(!should_skip_tree(&config, &TreeType::StateV2));
        assert!(!should_skip_tree(&config, &TreeType::AddressV2));
    }

    #[test]
    fn test_should_skip_tree_all_v2_skipped() {
        let config = create_test_config_with_skip_flags(false, false, true, true);
        assert!(!should_skip_tree(&config, &TreeType::StateV1));
        assert!(!should_skip_tree(&config, &TreeType::AddressV1));
        assert!(should_skip_tree(&config, &TreeType::StateV2));
        assert!(should_skip_tree(&config, &TreeType::AddressV2));
    }

    #[test]
    fn test_should_skip_tree_only_state_trees() {
        let config = create_test_config_with_skip_flags(true, false, true, false);
        assert!(should_skip_tree(&config, &TreeType::StateV1));
        assert!(should_skip_tree(&config, &TreeType::StateV2));
        assert!(!should_skip_tree(&config, &TreeType::AddressV1));
        assert!(!should_skip_tree(&config, &TreeType::AddressV2));
    }

    #[test]
    fn test_should_skip_tree_only_address_trees() {
        let config = create_test_config_with_skip_flags(false, true, false, true);
        assert!(!should_skip_tree(&config, &TreeType::StateV1));
        assert!(!should_skip_tree(&config, &TreeType::StateV2));
        assert!(should_skip_tree(&config, &TreeType::AddressV1));
        assert!(should_skip_tree(&config, &TreeType::AddressV2));
    }

    #[test]
    fn test_should_skip_tree_mixed_config() {
        let config = create_test_config_with_skip_flags(true, false, false, true);
        assert!(should_skip_tree(&config, &TreeType::StateV1));
        assert!(!should_skip_tree(&config, &TreeType::StateV2));
        assert!(!should_skip_tree(&config, &TreeType::AddressV1));
        assert!(should_skip_tree(&config, &TreeType::AddressV2));
    }

    #[test]
    fn test_general_config_test_address_v2() {
        let config = GeneralConfig::test_address_v2();
        assert!(config.skip_v1_state_trees);
        assert!(config.skip_v1_address_trees);
        assert!(config.skip_v2_state_trees);
        assert!(!config.skip_v2_address_trees);
    }

    #[test]
    fn test_general_config_test_state_v2() {
        let config = GeneralConfig::test_state_v2();
        assert!(config.skip_v1_state_trees);
        assert!(config.skip_v1_address_trees);
        assert!(!config.skip_v2_state_trees);
        assert!(config.skip_v2_address_trees);
    }

    #[test]
    fn test_work_item_is_address_tree() {
        let tree_account = TreeAccounts {
            merkle_tree: Pubkey::new_unique(),
            queue: Pubkey::new_unique(),
            is_rolledover: false,
            tree_type: TreeType::AddressV1,
            owner: Default::default(),
        };
        let work_item = WorkItem {
            tree_account,
            queue_item_data: QueueItemData {
                hash: [0u8; 32],
                index: 0,
                leaf_index: None,
            },
        };
        assert!(work_item.is_address_tree());
        assert!(!work_item.is_state_tree());
    }

    #[test]
    fn test_work_item_is_state_tree() {
        let tree_account = TreeAccounts {
            merkle_tree: Pubkey::new_unique(),
            queue: Pubkey::new_unique(),
            is_rolledover: false,
            tree_type: TreeType::StateV1,
            owner: Default::default(),
        };
        let work_item = WorkItem {
            tree_account,
            queue_item_data: QueueItemData {
                hash: [0u8; 32],
                index: 0,
                leaf_index: None,
            },
        };
        assert!(!work_item.is_address_tree());
        assert!(work_item.is_state_tree());
    }

    #[test]
    fn test_work_report_creation() {
        let report = WorkReport {
            epoch: 42,
            processed_items: 100,
            metrics: ProcessingMetrics {
                append: CircuitMetrics {
                    circuit_inputs_duration: std::time::Duration::from_secs(1),
                    proof_generation_duration: std::time::Duration::from_secs(3),
                    round_trip_duration: std::time::Duration::from_secs(10),
                },
                nullify: CircuitMetrics {
                    circuit_inputs_duration: std::time::Duration::from_secs(1),
                    proof_generation_duration: std::time::Duration::from_secs(2),
                    round_trip_duration: std::time::Duration::from_secs(8),
                },
                address_append: CircuitMetrics {
                    circuit_inputs_duration: std::time::Duration::from_secs(1),
                    proof_generation_duration: std::time::Duration::from_secs(2),
                    round_trip_duration: std::time::Duration::from_secs(9),
                },
                tx_sending_duration: std::time::Duration::ZERO,
            },
        };
        assert_eq!(report.epoch, 42);
        assert_eq!(report.processed_items, 100);
        assert_eq!(report.metrics.total().as_secs(), 10);
        assert_eq!(report.metrics.total_circuit_inputs().as_secs(), 3);
        assert_eq!(report.metrics.total_proof_generation().as_secs(), 7);
        assert_eq!(report.metrics.total_round_trip().as_secs(), 27);
    }

    #[tokio::test]
    async fn watch_shutdown_observed_after_request() {
        let (tx, _initial_rx) = watch::channel(false);
        tx.send(true).expect("send");
        let mut rx = tx.subscribe();
        assert!(*rx.borrow_and_update());
    }
}
