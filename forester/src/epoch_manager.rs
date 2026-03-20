use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use borsh::BorshSerialize;
use dashmap::DashMap;
use forester_utils::{
    forester_epoch::{get_epoch_phases, Epoch, ForesterSlot, TreeAccounts, TreeForesterSchedule},
    rpc_pool::SolanaRpcPool,
};
use futures::future::join_all;
use light_client::{
    indexer::{Indexer, MerkleProof, NewAddressProofWithContext},
    rpc::{LightClient, LightClientConfig, RetryConfig, Rpc, RpcError},
};
use light_compressed_account::TreeType;
use light_registry::{
    account_compression_cpi::sdk::{
        create_batch_append_instruction, create_batch_nullify_instruction,
        create_batch_update_address_tree_instruction,
    },
    protocol_config::state::{EpochState, ProtocolConfig},
    sdk::{create_finalize_registration_instruction, create_report_work_instruction},
    utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority},
    EpochPda, ForesterEpochPda,
};
use solana_program::{
    instruction::InstructionError, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
};
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
    signature::{Keypair, Signer},
    transaction::TransactionError,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
    time::{sleep, Instant, MissedTickBehavior},
};
use tracing::{debug, error, info, info_span, instrument, trace, warn};

use crate::{
    compressible::{
        traits::{Cancelled, CompressibleTracker, CompressionOutcome, CompressionTaskError},
        CTokenAccountTracker, CTokenCompressor, CompressibleConfig,
    },
    errors::{
        rpc_is_already_processed, ChannelError, ForesterError, InitializationError,
        RegistrationError, WorkReportError,
    },
    logging::{should_emit_rate_limited_warning, ServiceHeartbeat},
    metrics::{
        push_metrics, queue_metric_update, update_epoch_detected, update_epoch_registered,
        update_forester_sol_balance,
    },
    pagerduty::send_pagerduty_alert,
    priority_fee::PriorityFeeConfig,
    processor::{
        tx_cache::ProcessedHashCache,
        v1::{
            config::{BuildTransactionBatchConfig, SendBatchedTransactionsConfig},
            send_transaction::send_batched_transactions,
            tx_builder::EpochManagerTransactions,
        },
        v2::{
            strategy::{AddressTreeStrategy, StateTreeStrategy},
            BatchContext, BatchInstruction, ProcessingResult, ProverConfig, QueueProcessor,
            SharedProofCache,
        },
    },
    queue_helpers::QueueItemData,
    rollover::{
        is_tree_ready_for_rollover, perform_address_merkle_tree_rollover,
        perform_state_merkle_tree_rollover_forester,
    },
    slot_tracker::{slot_duration, wait_until_slot_reached, SlotTracker},
    smart_transaction::{
        send_smart_transaction, ComputeBudgetConfig, ConfirmationConfig,
        SendSmartTransactionConfig, TransactionPolicy,
    },
    transaction_timing::{scheduled_confirmation_deadline, scheduled_v1_batch_timeout},
    tree_data_sync::{fetch_protocol_group_authority, fetch_trees},
    ForesterConfig, ForesterEpochInfo, Result,
};

type StateBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>)>>;
type AddressBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>)>>;
type ProcessorInitLockMap = Arc<DashMap<Pubkey, Arc<Mutex<()>>>>;
type TreeProcessingTask = JoinHandle<Result<()>>;

/// Coordinates re-finalization across parallel `process_queue` tasks when new
/// foresters register mid-epoch. Only one task performs the on-chain
/// `finalize_registration` tx; others wait for it to complete.
#[derive(Debug)]
pub(crate) struct RegistrationTracker {
    cached_registered_weight: AtomicU64,
    refinalize_in_progress: AtomicBool,
    refinalized: tokio::sync::Notify,
}

impl RegistrationTracker {
    fn new(weight: u64) -> Self {
        Self {
            cached_registered_weight: AtomicU64::new(weight),
            refinalize_in_progress: AtomicBool::new(false),
            refinalized: tokio::sync::Notify::new(),
        }
    }

    fn cached_weight(&self) -> u64 {
        self.cached_registered_weight.load(Ordering::Acquire)
    }

    /// Returns `true` if this caller won the race to perform re-finalization.
    fn try_claim_refinalize(&self) -> bool {
        self.refinalize_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Called by the winner after the on-chain tx succeeds.
    fn complete_refinalize(&self, new_weight: u64) {
        self.cached_registered_weight
            .store(new_weight, Ordering::Release);
        self.refinalize_in_progress.store(false, Ordering::Release);
        self.refinalized.notify_waiters();
    }

    /// Called by non-winners to block until re-finalization is done.
    async fn wait_for_refinalize(&self) {
        if !self.refinalize_in_progress.load(Ordering::Acquire) {
            return;
        }
        let fut = self.refinalized.notified();
        if !self.refinalize_in_progress.load(Ordering::Acquire) {
            return;
        }
        fut.await;
    }
}

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
    AddressProof(NewAddressProofWithContext),
    StateProof(MerkleProof),
}

#[derive(Debug)]
pub struct EpochManager<R: Rpc + Indexer> {
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<SolanaRpcPool<R>>,
    authority: Arc<Keypair>,
    work_report_sender: mpsc::Sender<WorkReport>,
    processed_items_per_epoch_count: Arc<Mutex<HashMap<u64, AtomicUsize>>>,
    processing_metrics_per_epoch: Arc<Mutex<HashMap<u64, ProcessingMetrics>>>,
    trees: Arc<Mutex<Vec<TreeAccounts>>>,
    slot_tracker: Arc<SlotTracker>,
    processing_epochs: Arc<DashMap<u64, Arc<AtomicBool>>>,
    tx_cache: Arc<Mutex<ProcessedHashCache>>,
    ops_cache: Arc<Mutex<ProcessedHashCache>>,
    /// Proof caches for pre-warming during idle slots
    proof_caches: Arc<DashMap<Pubkey, Arc<SharedProofCache>>>,
    state_processors: StateBatchProcessorMap<R>,
    address_processors: AddressBatchProcessorMap<R>,
    state_processor_init_locks: ProcessorInitLockMap,
    address_processor_init_locks: ProcessorInitLockMap,
    compressible_tracker: Option<Arc<CTokenAccountTracker>>,
    pda_tracker: Option<Arc<crate::compressible::pda::PdaAccountTracker>>,
    mint_tracker: Option<Arc<crate::compressible::mint::MintAccountTracker>>,
    /// Cached zkp_batch_size per tree to filter queue updates below threshold
    zkp_batch_sizes: Arc<DashMap<Pubkey, u64>>,
    address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
    heartbeat: Arc<ServiceHeartbeat>,
    run_id: Arc<str>,
    /// Per-epoch registration trackers to coordinate re-finalization when new foresters register mid-epoch
    registration_trackers: Arc<DashMap<u64, Arc<RegistrationTracker>>>,
}

impl<R: Rpc + Indexer> Clone for EpochManager<R> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            protocol_config: self.protocol_config.clone(),
            rpc_pool: self.rpc_pool.clone(),
            authority: self.authority.clone(),
            work_report_sender: self.work_report_sender.clone(),
            processed_items_per_epoch_count: self.processed_items_per_epoch_count.clone(),
            processing_metrics_per_epoch: self.processing_metrics_per_epoch.clone(),
            trees: self.trees.clone(),
            slot_tracker: self.slot_tracker.clone(),
            processing_epochs: self.processing_epochs.clone(),
            tx_cache: self.tx_cache.clone(),
            ops_cache: self.ops_cache.clone(),
            proof_caches: self.proof_caches.clone(),
            state_processors: self.state_processors.clone(),
            address_processors: self.address_processors.clone(),
            state_processor_init_locks: self.state_processor_init_locks.clone(),
            address_processor_init_locks: self.address_processor_init_locks.clone(),
            compressible_tracker: self.compressible_tracker.clone(),
            pda_tracker: self.pda_tracker.clone(),
            mint_tracker: self.mint_tracker.clone(),
            zkp_batch_sizes: self.zkp_batch_sizes.clone(),
            address_lookup_tables: self.address_lookup_tables.clone(),
            heartbeat: self.heartbeat.clone(),
            run_id: self.run_id.clone(),
            registration_trackers: self.registration_trackers.clone(),
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
        Ok(Self {
            config,
            protocol_config,
            rpc_pool,
            authority,
            work_report_sender,
            processed_items_per_epoch_count: Arc::new(Mutex::new(HashMap::new())),
            processing_metrics_per_epoch: Arc::new(Mutex::new(HashMap::new())),
            trees: Arc::new(Mutex::new(trees)),
            slot_tracker,
            processing_epochs: Arc::new(DashMap::new()),
            tx_cache,
            ops_cache,
            proof_caches: Arc::new(DashMap::new()),
            state_processors: Arc::new(DashMap::new()),
            address_processors: Arc::new(DashMap::new()),
            state_processor_init_locks: Arc::new(DashMap::new()),
            address_processor_init_locks: Arc::new(DashMap::new()),
            compressible_tracker,
            pda_tracker,
            mint_tracker,
            zkp_batch_sizes: Arc::new(DashMap::new()),
            address_lookup_tables,
            heartbeat,
            run_id: Arc::<str>::from(run_id),
            registration_trackers: Arc::new(DashMap::new()),
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let tx = Arc::new(tx);

        let mut monitor_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            let tx_clone = Arc::clone(&tx);
            async move { self_clone.monitor_epochs(tx_clone).await }
        });

        // Process current and previous epochs
        let current_previous_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            let tx_clone = Arc::clone(&tx);
            async move {
                self_clone
                    .process_current_and_previous_epochs(tx_clone)
                    .await
            }
        });

        let tree_discovery_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move { self_clone.discover_trees_periodically().await }
        });

        let balance_check_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move { self_clone.check_sol_balance_periodically().await }
        });

        let queue_metrics_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move { self_clone.update_queue_metrics_periodically().await }
        });

        let _guard = scopeguard::guard(
            (
                current_previous_handle,
                tree_discovery_handle,
                balance_check_handle,
                queue_metrics_handle,
            ),
            |(h2, h3, h4, h5)| {
                info!(
                    event = "background_tasks_aborting",
                    run_id = %self.run_id,
                    "Aborting EpochManager background tasks"
                );
                h2.abort();
                h3.abort();
                h4.abort();
                h5.abort();
            },
        );

        let result = loop {
            tokio::select! {
                epoch_opt = rx.recv() => {
                    match epoch_opt {
                        Some(epoch) => {
                            debug!(
                                event = "epoch_queued_for_processing",
                                run_id = %self.run_id,
                                epoch,
                                "Received epoch from monitor"
                            );
                            let self_clone = Arc::clone(&self);
                            tokio::spawn(async move {
                                if let Err(e) = self_clone.process_epoch(epoch).await {
                                    error!(
                                        event = "epoch_processing_failed",
                                        run_id = %self_clone.run_id,
                                        epoch,
                                        error = ?e,
                                        "Error processing epoch"
                                    );
                                }
                            });
                        }
                        None => {
                            error!(
                                event = "epoch_monitor_channel_closed",
                                run_id = %self.run_id,
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
                                run_id = %self.run_id,
                                "Epoch monitor exited unexpectedly with Ok(())"
                            );
                        }
                        Ok(Err(e)) => {
                            error!(
                                event = "epoch_monitor_exited_with_error",
                                run_id = %self.run_id,
                                error = ?e,
                                "Epoch monitor exited with error"
                            );
                        }
                        Err(e) => {
                            error!(
                                event = "epoch_monitor_task_failed",
                                run_id = %self.run_id,
                                error = ?e,
                                "Epoch monitor task panicked or was cancelled"
                            );
                        }
                    }
                    if let Some(pagerduty_key) = &self.config.external_services.pagerduty_routing_key {
                        let _ = send_pagerduty_alert(
                            pagerduty_key,
                            &format!("Forester epoch monitor died unexpectedly on {}", self.config.payer_keypair.pubkey()),
                            "critical",
                            "epoch_monitor_dead",
                        ).await;
                    }
                    break Err(anyhow!("Epoch monitor exited unexpectedly - forester cannot function without it"));
                }
            }
        };

        // Abort monitor_handle on exit
        monitor_handle.abort();
        result
    }

    /// Periodically updates queue_length and queue_capacity Prometheus gauges
    /// so Grafana dashboards can show queue trends over time.
    async fn update_queue_metrics_periodically(self: Arc<Self>) -> Result<()> {
        let interval_secs = self.config.general_config.tree_discovery_interval_seconds;
        if interval_secs == 0 {
            return Ok(());
        }
        // Use same interval as tree discovery (default 30s)
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip first tick — let tree discovery populate the tree list first
        interval.tick().await;

        loop {
            interval.tick().await;

            let trees = self.trees.lock().await;
            let trees_snapshot: Vec<_> = trees.clone();
            drop(trees);

            if trees_snapshot.is_empty() {
                continue;
            }

            for tree_type in [
                TreeType::StateV1,
                TreeType::AddressV1,
                TreeType::StateV2,
                TreeType::AddressV2,
            ] {
                if let Err(e) =
                    crate::run_queue_info(self.config.clone(), &trees_snapshot, tree_type).await
                {
                    debug!(
                        event = "queue_metrics_update_failed",
                        run_id = %self.run_id,
                        tree_type = ?tree_type,
                        error = ?e,
                        "Failed to update queue metrics"
                    );
                }
            }
        }
    }

    async fn check_sol_balance_periodically(self: Arc<Self>) -> Result<()> {
        let interval_duration = Duration::from_secs(300);
        let mut interval = tokio::time::interval(interval_duration);

        loop {
            interval.tick().await;
            match self.rpc_pool.get_connection().await {
                Ok(rpc) => match rpc.get_balance(&self.config.payer_keypair.pubkey()).await {
                    Ok(balance) => {
                        let balance_in_sol = balance as f64 / (LAMPORTS_PER_SOL as f64);
                        update_forester_sol_balance(
                            &self.config.payer_keypair.pubkey().to_string(),
                            balance_in_sol,
                        );
                        debug!(
                            event = "forester_balance_updated",
                            run_id = %self.run_id,
                            balance_sol = balance_in_sol,
                            "Current SOL balance updated"
                        );
                    }
                    Err(e) => error!(
                        event = "forester_balance_fetch_failed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Failed to get balance"
                    ),
                },
                Err(e) => error!(
                    event = "forester_balance_rpc_connection_failed",
                    run_id = %self.run_id,
                    error = ?e,
                    "Failed to get RPC connection for balance check"
                ),
            }
        }
    }

    /// Periodically fetches trees from on-chain and adds newly discovered ones.
    async fn discover_trees_periodically(self: Arc<Self>) -> Result<()> {
        let interval_secs = self.config.general_config.tree_discovery_interval_seconds;
        if interval_secs == 0 {
            info!(event = "tree_discovery_disabled", run_id = %self.run_id, "Tree discovery disabled (interval=0)");
            return Ok(());
        }
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the first immediate tick — initial trees are already loaded at startup
        interval.tick().await;

        info!(
            event = "tree_discovery_started",
            run_id = %self.run_id,
            interval_secs,
            "Starting periodic tree discovery"
        );

        let mut group_authority: Option<Pubkey> = self.config.general_config.group_authority;

        loop {
            interval.tick().await;

            let rpc = match self.rpc_pool.get_connection().await {
                Ok(rpc) => rpc,
                Err(e) => {
                    warn!(event = "tree_discovery_rpc_failed", run_id = %self.run_id, error = ?e, "Tree discovery: failed to get RPC connection");
                    continue;
                }
            };

            // Lazily resolve group authority (retry each tick until successful)
            if group_authority.is_none() {
                if let Ok(ga) = fetch_protocol_group_authority(&*rpc, &self.run_id).await {
                    group_authority = Some(ga);
                    // Retroactively filter already-tracked trees that were added
                    // before group_authority was resolved.
                    let mut trees = self.trees.lock().await;
                    let before = trees.len();
                    trees.retain(|t| t.owner == ga);
                    if !self.config.general_config.tree_ids.is_empty() {
                        let tree_ids = &self.config.general_config.tree_ids;
                        trees.retain(|t| tree_ids.contains(&t.merkle_tree));
                    }
                    if trees.len() < before {
                        info!(
                            event = "tree_discovery_retroactive_filter",
                            run_id = %self.run_id,
                            group_authority = %ga,
                            trees_before = before,
                            trees_after = trees.len(),
                            "Filtered existing trees after resolving group authority"
                        );
                    }
                }
            }

            let mut fetched_trees = match fetch_trees(&*rpc).await {
                Ok(trees) => trees,
                Err(e) => {
                    warn!(event = "tree_discovery_fetch_failed", run_id = %self.run_id, error = ?e, "Tree discovery: failed to fetch trees");
                    continue;
                }
            };

            if let Some(ga) = group_authority {
                fetched_trees.retain(|tree| tree.owner == ga);
            }
            if !self.config.general_config.tree_ids.is_empty() {
                let tree_ids = &self.config.general_config.tree_ids;
                fetched_trees.retain(|tree| tree_ids.contains(&tree.merkle_tree));
            }

            let known_trees = self.trees.lock().await;
            let known_pubkeys: std::collections::HashSet<Pubkey> =
                known_trees.iter().map(|t| t.merkle_tree).collect();
            drop(known_trees);

            for tree in fetched_trees {
                if known_pubkeys.contains(&tree.merkle_tree) {
                    continue;
                }
                if should_skip_tree(&self.config, &tree.tree_type) {
                    debug!(
                        event = "tree_discovery_skipped",
                        run_id = %self.run_id,
                        tree = %tree.merkle_tree,
                        tree_type = ?tree.tree_type,
                        "Skipping tree due to fee filter config"
                    );
                    continue;
                }
                info!(
                    event = "tree_discovery_new_tree",
                    run_id = %self.run_id,
                    tree = %tree.merkle_tree,
                    tree_type = ?tree.tree_type,
                    queue = %tree.queue,
                    "Discovered new tree"
                );
                if let Err(e) = self.add_new_tree(tree).await {
                    error!(
                        event = "tree_discovery_add_failed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Failed to add discovered tree"
                    );
                }
            }
        }
    }

    async fn add_new_tree(&self, new_tree: TreeAccounts) -> Result<()> {
        info!(
            event = "new_tree_add_started",
            run_id = %self.run_id,
            tree = %new_tree.merkle_tree,
            tree_type = ?new_tree.tree_type,
            "Adding new tree"
        );
        let mut trees = self.trees.lock().await;
        trees.push(new_tree);
        drop(trees);

        info!(
            event = "new_tree_added",
            run_id = %self.run_id,
            tree = %new_tree.merkle_tree,
            "New tree added to tracked list"
        );

        let (current_slot, current_epoch) = self.get_current_slot_and_epoch().await?;
        let phases = get_epoch_phases(&self.protocol_config, current_epoch);

        // Check if we're currently in the active phase
        if current_slot >= phases.active.start && current_slot < phases.active.end {
            info!(
                event = "new_tree_active_phase_injection",
                run_id = %self.run_id,
                tree = %new_tree.merkle_tree,
                current_slot,
                active_phase_start_slot = phases.active.start,
                active_phase_end_slot = phases.active.end,
                "In active phase; attempting immediate processing for new tree"
            );
            info!(
                event = "new_tree_recover_registration_started",
                run_id = %self.run_id,
                tree = %new_tree.merkle_tree,
                epoch = current_epoch,
                "Recovering registration info for new tree"
            );
            match self
                .recover_registration_info_if_exists(current_epoch)
                .await
            {
                Ok(Some(mut epoch_info)) => {
                    info!(
                        event = "new_tree_recover_registration_succeeded",
                        run_id = %self.run_id,
                        tree = %new_tree.merkle_tree,
                        epoch = current_epoch,
                        "Recovered registration info for current epoch"
                    );
                    let tree_schedule = TreeForesterSchedule::new_with_schedule(
                        &new_tree,
                        current_slot,
                        &epoch_info.forester_epoch_pda,
                        &epoch_info.epoch_pda,
                    )?;
                    epoch_info.trees.push(tree_schedule.clone());

                    let self_clone = Arc::new(self.clone());
                    let tracker = self
                        .registration_trackers
                        .entry(current_epoch)
                        .or_insert_with(|| {
                            Arc::new(RegistrationTracker::new(
                                epoch_info.epoch_pda.registered_weight,
                            ))
                        })
                        .value()
                        .clone();

                    info!(
                        event = "new_tree_processing_task_spawned",
                        run_id = %self.run_id,
                        tree = %new_tree.merkle_tree,
                        epoch = current_epoch,
                        "Spawning task to process new tree in current epoch"
                    );
                    tokio::spawn(async move {
                        let tree_pubkey = tree_schedule.tree_accounts.merkle_tree;
                        if let Err(e) = self_clone
                            .process_queue(
                                &epoch_info.epoch,
                                epoch_info.forester_epoch_pda.clone(),
                                tree_schedule,
                                tracker,
                            )
                            .await
                        {
                            error!(
                                event = "new_tree_process_queue_failed",
                                run_id = %self_clone.run_id,
                                tree = %tree_pubkey,
                                error = ?e,
                                "Error processing queue for new tree"
                            );
                        } else {
                            info!(
                                event = "new_tree_process_queue_succeeded",
                                run_id = %self_clone.run_id,
                                tree = %tree_pubkey,
                                "Successfully processed new tree in current epoch"
                            );
                        }
                    });
                }
                Ok(None) => {
                    debug!(
                        "Not registered for current epoch yet, new tree will be picked up during next registration"
                    );
                }
                Err(e) => {
                    warn!(
                        event = "new_tree_recover_registration_failed",
                        run_id = %self.run_id,
                        tree = %new_tree.merkle_tree,
                        epoch = current_epoch,
                        error = ?e,
                        "Failed to recover registration info for new tree"
                    );
                }
            }

            info!(
                event = "new_tree_injected_into_current_epoch",
                run_id = %self.run_id,
                tree = %new_tree.merkle_tree,
                epoch = current_epoch,
                "Injected new tree into current epoch"
            );
        } else {
            info!(
                event = "new_tree_queued_for_next_registration",
                run_id = %self.run_id,
                tree = %new_tree.merkle_tree,
                current_slot,
                active_phase_start_slot = phases.active.start,
                "Not in active phase; new tree will be picked up in next registration"
            );
        }

        Ok(())
    }

    #[instrument(level = "debug", skip(self, tx))]
    async fn monitor_epochs(&self, tx: Arc<mpsc::Sender<u64>>) -> Result<()> {
        let mut last_epoch: Option<u64> = None;
        let mut consecutive_failures = 0u32;
        const MAX_BACKOFF_SECS: u64 = 60;

        info!(
            event = "epoch_monitor_started",
            run_id = %self.run_id,
            "Starting epoch monitor"
        );

        loop {
            let (slot, current_epoch) = match self.get_current_slot_and_epoch().await {
                Ok(result) => {
                    if consecutive_failures > 0 {
                        info!(
                            event = "epoch_monitor_recovered",
                            run_id = %self.run_id,
                            consecutive_failures, "Epoch monitor recovered after failures"
                        );
                    }
                    consecutive_failures = 0;
                    result
                }
                Err(e) => {
                    consecutive_failures += 1;
                    let backoff_secs = 2u64.pow(consecutive_failures.min(6)).min(MAX_BACKOFF_SECS);
                    let backoff = Duration::from_secs(backoff_secs);

                    if consecutive_failures == 1 {
                        warn!(
                            event = "epoch_monitor_slot_epoch_failed",
                            run_id = %self.run_id,
                            consecutive_failures,
                            error = ?e,
                            backoff_ms = backoff.as_millis() as u64,
                            "Epoch monitor failed to get slot/epoch; retrying"
                        );
                    } else if consecutive_failures.is_multiple_of(10) {
                        error!(
                            event = "epoch_monitor_slot_epoch_failed_repeated",
                            run_id = %self.run_id,
                            consecutive_failures,
                            error = ?e,
                            backoff_ms = backoff.as_millis() as u64,
                            "Epoch monitor still failing repeatedly"
                        );
                    }

                    tokio::time::sleep(backoff).await;
                    continue;
                }
            };

            debug!(
                event = "epoch_monitor_tick",
                run_id = %self.run_id,
                last_epoch = ?last_epoch,
                current_epoch,
                slot,
                "Epoch monitor tick"
            );

            if last_epoch.is_none_or(|last| current_epoch > last) {
                debug!(
                    event = "epoch_monitor_new_epoch_detected",
                    run_id = %self.run_id,
                    epoch = current_epoch,
                    "New epoch detected; sending for processing"
                );
                if let Err(e) = tx.send(current_epoch).await {
                    error!(
                        event = "epoch_monitor_send_current_epoch_failed",
                        run_id = %self.run_id,
                        epoch = current_epoch,
                        error = ?e,
                        "Failed to send current epoch for processing; channel closed"
                    );
                    return Err(anyhow!("Epoch channel closed: {}", e));
                }
                last_epoch = Some(current_epoch);
            }

            // Find the next epoch to process
            let target_epoch = current_epoch + 1;
            if last_epoch.is_none_or(|last| target_epoch > last) {
                let target_phases = get_epoch_phases(&self.protocol_config, target_epoch);

                // If registration hasn't started yet, wait for it
                if slot < target_phases.registration.start {
                    let mut rpc = match self.rpc_pool.get_connection().await {
                        Ok(rpc) => rpc,
                        Err(e) => {
                            warn!(
                                event = "epoch_monitor_wait_rpc_connection_failed",
                                run_id = %self.run_id,
                                target_epoch,
                                error = ?e,
                                "Failed to get RPC connection while waiting for registration slot"
                            );
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    };

                    const REGISTRATION_BUFFER_SLOTS: u64 = 30;
                    let wait_target = target_phases
                        .registration
                        .start
                        .saturating_sub(REGISTRATION_BUFFER_SLOTS);
                    let slots_to_wait = wait_target.saturating_sub(slot);

                    debug!(
                        event = "epoch_monitor_wait_for_registration",
                        run_id = %self.run_id,
                        target_epoch,
                        current_slot = slot,
                        wait_target_slot = wait_target,
                        registration_start_slot = target_phases.registration.start,
                        slots_to_wait,
                        "Waiting for target epoch registration phase"
                    );

                    if let Err(e) =
                        wait_until_slot_reached(&mut *rpc, &self.slot_tracker, wait_target).await
                    {
                        error!(
                            event = "epoch_monitor_wait_for_registration_failed",
                            run_id = %self.run_id,
                            target_epoch,
                            error = ?e,
                            "Error waiting for registration phase"
                        );
                        continue;
                    }
                }

                debug!(
                    event = "epoch_monitor_send_target_epoch",
                    run_id = %self.run_id,
                    target_epoch,
                    "Sending target epoch for processing"
                );
                if let Err(e) = tx.send(target_epoch).await {
                    error!(
                        event = "epoch_monitor_send_target_epoch_failed",
                        run_id = %self.run_id,
                        target_epoch,
                        error = ?e,
                        "Failed to send target epoch for processing; channel closed"
                    );
                    return Err(anyhow!("Epoch channel closed: {}", e));
                }
                last_epoch = Some(target_epoch);
                continue; // Re-check state after processing
            } else {
                // we've already sent the next epoch, wait a bit before checking again
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }

    async fn get_processed_items_count(&self, epoch: u64) -> usize {
        let counts = self.processed_items_per_epoch_count.lock().await;
        counts
            .get(&epoch)
            .map_or(0, |count| count.load(Ordering::Relaxed))
    }

    async fn increment_processed_items_count(&self, epoch: u64, increment_by: usize) {
        let mut counts = self.processed_items_per_epoch_count.lock().await;
        counts
            .entry(epoch)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(increment_by, Ordering::Relaxed);
    }

    async fn get_processing_metrics(&self, epoch: u64) -> ProcessingMetrics {
        let metrics = self.processing_metrics_per_epoch.lock().await;
        metrics.get(&epoch).copied().unwrap_or_default()
    }

    async fn add_processing_metrics(&self, epoch: u64, new_metrics: ProcessingMetrics) {
        let mut metrics = self.processing_metrics_per_epoch.lock().await;
        *metrics.entry(epoch).or_default() += new_metrics;
    }

    async fn recover_registration_info_if_exists(
        &self,
        epoch: u64,
    ) -> std::result::Result<Option<ForesterEpochInfo>, ForesterError> {
        debug!("Recovering registration info for epoch {}", epoch);

        let forester_epoch_pda_pubkey =
            get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0;

        let existing_pda = {
            let rpc = self.rpc_pool.get_connection().await?;
            rpc.get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?
        };

        match existing_pda {
            Some(pda) => self
                .recover_registration_info_internal(epoch, forester_epoch_pda_pubkey, pda)
                .await
                .map(Some)
                .map_err(ForesterError::from),
            None => Ok(None),
        }
    }

    async fn process_current_and_previous_epochs(&self, tx: Arc<mpsc::Sender<u64>>) -> Result<()> {
        let (slot, current_epoch) = self.get_current_slot_and_epoch().await?;
        let current_phases = get_epoch_phases(&self.protocol_config, current_epoch);
        let previous_epoch = current_epoch.saturating_sub(1);

        // Process the previous epoch if still in active or later phase
        if slot > current_phases.registration.start {
            debug!("Processing previous epoch: {}", previous_epoch);
            if let Err(e) = tx.send(previous_epoch).await {
                error!(
                    event = "initial_epoch_send_previous_failed",
                    run_id = %self.run_id,
                    epoch = previous_epoch,
                    error = ?e,
                    "Failed to send previous epoch for processing"
                );
                return Ok(());
            }
        }

        // Always process the current epoch (registration is allowed at any time)
        debug!("Processing current epoch: {}", current_epoch);
        if let Err(e) = tx.send(current_epoch).await {
            error!(
                event = "initial_epoch_send_current_failed",
                run_id = %self.run_id,
                epoch = current_epoch,
                error = ?e,
                "Failed to send current epoch for processing"
            );
            return Ok(()); // Channel closed, exit gracefully
        }

        debug!("Finished processing current and previous epochs");
        Ok(())
    }

    #[instrument(level = "debug", skip(self), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch))]
    async fn process_epoch(&self, epoch: u64) -> Result<()> {
        // Clone the Arc immediately to release the DashMap shard lock.
        // Without .clone(), the RefMut guard would be held across async operations,
        // blocking other epochs from accessing the DashMap if they hash to the same shard.
        let processing_flag = self
            .processing_epochs
            .entry(epoch)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();

        if processing_flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Another task is already processing this epoch
            debug!("Epoch {} is already being processed, skipping", epoch);
            return Ok(());
        }

        // Ensure we reset the processing flag when this scope exits
        // (whether by normal return, early return, or panic).
        let _reset_guard = scopeguard::guard((), |_| {
            processing_flag.store(false, Ordering::SeqCst);
        });

        let phases = get_epoch_phases(&self.protocol_config, epoch);
        update_epoch_detected(epoch);

        // Attempt to recover registration info
        debug!("Recovering registration info for epoch {}", epoch);
        let mut registration_info = match self.recover_registration_info_if_exists(epoch).await {
            Ok(Some(info)) => info,
            Ok(None) => {
                debug!(
                    "No existing registration found for epoch {}, will register fresh",
                    epoch
                );
                match self
                    .register_for_epoch_with_retry(epoch, 100, Duration::from_millis(1000))
                    .await
                {
                    Ok(info) => info,
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => {
                warn!(
                    event = "recover_registration_info_failed",
                    run_id = %self.run_id,
                    epoch,
                    error = ?e,
                    "Failed to recover registration info"
                );
                return Err(e.into());
            }
        };
        debug!("Recovered registration info for epoch {}", epoch);
        update_epoch_registered(epoch);

        // Wait for the active phase
        registration_info = match self.wait_for_active_phase(&registration_info).await? {
            Some(info) => info,
            None => {
                let current_slot = self.slot_tracker.estimated_current_slot();
                debug!(
                    event = "epoch_processing_skipped_finalize_registration_phase_ended",
                    run_id = %self.run_id,
                    epoch,
                    current_slot,
                    active_phase_end_slot = registration_info.epoch.phases.active.end,
                    "Skipping epoch processing because FinalizeRegistration is no longer possible"
                );
                return Ok(());
            }
        };

        // Perform work
        if self.sync_slot().await? < phases.active.end {
            self.perform_active_work(&registration_info).await?;
        }
        // Wait for report work phase
        if self.sync_slot().await? < phases.report_work.start {
            self.wait_for_report_work_phase(&registration_info).await?;
        }

        // Always send metrics report to channel for monitoring/testing
        // This ensures metrics are captured even if we missed the report_work phase
        self.send_work_report(&registration_info).await?;

        // Report work on-chain only if within the report_work phase
        if self.sync_slot().await? < phases.report_work.end {
            self.report_work_onchain(&registration_info).await?;
        } else {
            let current_slot = self.slot_tracker.estimated_current_slot();
            info!(
                event = "skip_onchain_work_report_phase_ended",
                run_id = %self.run_id,
                epoch = registration_info.epoch.epoch,
                current_slot,
                report_work_end_slot = phases.report_work.end,
                "Skipping on-chain work report because report_work phase has ended"
            );
        }

        // TODO: implement
        // self.claim(&registration_info).await?;

        // Clean up per-epoch state now that this epoch is complete.
        // In-flight tasks still hold their own Arc clones, so removal is safe.
        self.registration_trackers.remove(&epoch);
        self.processing_epochs.remove(&epoch);
        self.processed_items_per_epoch_count
            .lock()
            .await
            .remove(&epoch);
        self.processing_metrics_per_epoch
            .lock()
            .await
            .remove(&epoch);

        info!(
            event = "process_epoch_completed",
            run_id = %self.run_id,
            epoch, "Exiting process_epoch"
        );
        Ok(())
    }

    async fn get_current_slot_and_epoch(&self) -> Result<(u64, u64)> {
        let slot = self.slot_tracker.estimated_current_slot();
        Ok((slot, self.protocol_config.get_current_epoch(slot)))
    }

    #[instrument(level = "debug", skip(self), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch
    ))]
    async fn register_for_epoch_with_retry(
        &self,
        epoch: u64,
        max_retries: u32,
        retry_delay: Duration,
    ) -> std::result::Result<ForesterEpochInfo, ForesterError> {
        let rpc = LightClient::new(LightClientConfig {
            url: self.config.external_services.rpc_url.to_string(),
            photon_url: self.config.external_services.indexer_url.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            fetch_active_tree: false,
        })
        .await
        .map_err(ForesterError::Rpc)?;
        let slot = rpc.get_slot().await.map_err(ForesterError::Rpc)?;
        let phases = get_epoch_phases(&self.protocol_config, epoch);

        if slot < phases.registration.start {
            let slots_to_wait = phases.registration.start.saturating_sub(slot);
            info!(
                event = "registration_wait_for_window",
                run_id = %self.run_id,
                epoch,
                current_slot = slot,
                registration_start_slot = phases.registration.start,
                slots_to_wait,
                "Registration window not open yet; waiting"
            );
            let wait_duration = slot_duration() * slots_to_wait as u32;
            sleep(wait_duration).await;
        }

        for attempt in 0..max_retries {
            match self.recover_registration_info_if_exists(epoch).await {
                Ok(Some(registration_info)) => return Ok(registration_info),
                Ok(None) => {}
                Err(e) => return Err(e),
            }

            match self.register_for_epoch(epoch).await {
                Ok(registration_info) => return Ok(registration_info),
                Err(e) => {
                    warn!(
                        event = "registration_attempt_failed",
                        run_id = %self.run_id,
                        epoch,
                        attempt = attempt + 1,
                        max_attempts = max_retries,
                        error = ?e,
                        "Failed to register for epoch; retrying"
                    );
                    if attempt < max_retries - 1 {
                        sleep(retry_delay).await;
                    } else {
                        if let Some(pagerduty_key) =
                            self.config.external_services.pagerduty_routing_key.clone()
                        {
                            if let Err(alert_err) = send_pagerduty_alert(
                                &pagerduty_key,
                                &format!(
                                    "Forester failed to register for epoch {} after {} attempts",
                                    epoch, max_retries
                                ),
                                "critical",
                                &format!("Forester {}", self.config.payer_keypair.pubkey()),
                            )
                            .await
                            {
                                error!(
                                    event = "pagerduty_alert_failed",
                                    run_id = %self.run_id,
                                    epoch,
                                    error = ?alert_err,
                                    "Failed to send PagerDuty alert"
                                );
                            }
                        }
                        return Err(ForesterError::Other(e));
                    }
                }
            }
        }
        Err(RegistrationError::MaxRetriesExceeded {
            epoch,
            attempts: max_retries,
        }
        .into())
    }

    #[instrument(level = "debug", skip(self), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch
    ))]
    async fn register_for_epoch(&self, epoch: u64) -> Result<ForesterEpochInfo> {
        info!(
            event = "registration_attempt_started",
            run_id = %self.run_id,
            epoch, "Registering for epoch"
        );
        let mut rpc = LightClient::new(LightClientConfig {
            url: self.config.external_services.rpc_url.to_string(),
            photon_url: self.config.external_services.indexer_url.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::processed()),
            fetch_active_tree: false,
        })
        .await?;
        let slot = rpc.get_slot().await?;
        let phases = get_epoch_phases(&self.protocol_config, epoch);

        if slot >= phases.registration.start {
            let forester_epoch_pda_pubkey =
                get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0;
            let existing_registration = rpc
                .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?;

            if let Some(existing_pda) = existing_registration {
                info!(
                    event = "registration_already_exists",
                    run_id = %self.run_id,
                    epoch, "Already registered for epoch; recovering registration info"
                );
                let registration_info = self
                    .recover_registration_info_internal(
                        epoch,
                        forester_epoch_pda_pubkey,
                        existing_pda,
                    )
                    .await?;
                return Ok(registration_info);
            }

            let registration_info = {
                debug!("Registering epoch {}", epoch);
                let registered_epoch = match Epoch::register(
                    &mut rpc,
                    &self.protocol_config,
                    &self.config.payer_keypair,
                    &self.config.derivation_pubkey,
                    Some(epoch),
                )
                .await
                .with_context(|| {
                    format!("Failed to execute epoch registration for epoch {}", epoch)
                })? {
                    Some(epoch) => {
                        debug!("Registered epoch: {:?}", epoch);
                        epoch
                    }
                    None => {
                        return Err(RegistrationError::EmptyRegistration.into());
                    }
                };

                let forester_epoch_pda = rpc
                    .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to fetch ForesterEpochPda from RPC for address {}",
                            registered_epoch.forester_epoch_pda
                        )
                    })?
                    .ok_or(RegistrationError::ForesterEpochPdaNotFound {
                        epoch,
                        pda_address: registered_epoch.forester_epoch_pda,
                    })?;

                let epoch_pda_address = get_epoch_pda_address(epoch);
                let epoch_pda = rpc
                    .get_anchor_account::<EpochPda>(&epoch_pda_address)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to fetch EpochPda from RPC for address {}",
                            epoch_pda_address
                        )
                    })?
                    .ok_or(RegistrationError::EpochPdaNotFound {
                        epoch,
                        pda_address: epoch_pda_address,
                    })?;

                ForesterEpochInfo {
                    epoch: registered_epoch,
                    epoch_pda,
                    forester_epoch_pda,
                    trees: Vec::new(),
                }
            };
            debug!("Registered: {:?}", registration_info);
            Ok(registration_info)
        } else {
            warn!(
                event = "registration_too_early",
                run_id = %self.run_id,
                epoch,
                current_slot = slot,
                registration_start_slot = phases.registration.start,
                "Too early to register for epoch"
            );
            Err(RegistrationError::RegistrationPhaseNotStarted {
                epoch,
                current_slot: slot,
                registration_start: phases.registration.start,
            }
            .into())
        }
    }

    async fn recover_registration_info_internal(
        &self,
        epoch: u64,
        forester_epoch_pda_address: Pubkey,
        forester_epoch_pda: ForesterEpochPda,
    ) -> Result<ForesterEpochInfo> {
        let rpc = self.rpc_pool.get_connection().await?;

        let phases = get_epoch_phases(&self.protocol_config, epoch);
        let slot = rpc.get_slot().await?;
        let state = phases.get_current_epoch_state(slot);

        let epoch_pda_address = get_epoch_pda_address(epoch);
        let epoch_pda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await
            .with_context(|| format!("Failed to fetch EpochPda for epoch {}", epoch))?
            .ok_or(RegistrationError::EpochPdaNotFound {
                epoch,
                pda_address: epoch_pda_address,
            })?;

        let epoch_info = Epoch {
            epoch,
            epoch_pda: epoch_pda_address,
            forester_epoch_pda: forester_epoch_pda_address,
            phases,
            state,
            merkle_trees: Vec::new(),
        };

        let forester_epoch_info = ForesterEpochInfo {
            epoch: epoch_info,
            epoch_pda,
            forester_epoch_pda,
            trees: Vec::new(),
        };

        Ok(forester_epoch_info)
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch
    ))]
    async fn wait_for_active_phase(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> std::result::Result<Option<ForesterEpochInfo>, ForesterError> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        let active_phase_start_slot = epoch_info.epoch.phases.active.start;
        let active_phase_end_slot = epoch_info.epoch.phases.active.end;
        let current_slot = self.slot_tracker.estimated_current_slot();

        if current_slot >= active_phase_start_slot {
            info!(
                event = "active_phase_already_started",
                run_id = %self.run_id,
                current_slot,
                active_phase_start_slot,
                active_phase_end_slot,
                slots_left = active_phase_end_slot.saturating_sub(current_slot),
                "Active phase has already started"
            );
        } else {
            let waiting_slots = active_phase_start_slot - current_slot;
            let waiting_secs = waiting_slots / 2;
            info!(
                event = "wait_for_active_phase",
                run_id = %self.run_id,
                current_slot,
                active_phase_start_slot,
                waiting_slots,
                approx_wait_seconds = waiting_secs,
                "Waiting for active phase to start"
            );
        }

        self.prewarm_all_trees_during_wait(epoch_info, active_phase_start_slot)
            .await;

        wait_until_slot_reached(&mut *rpc, &self.slot_tracker, active_phase_start_slot).await?;

        let forester_epoch_pda_pubkey = get_forester_epoch_pda_from_authority(
            &self.config.derivation_pubkey,
            epoch_info.epoch.epoch,
        )
        .0;
        let existing_registration = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?;

        if let Some(registration) = existing_registration {
            if registration.total_epoch_weight.is_none() {
                let current_slot = rpc.get_slot().await?;
                if current_slot > epoch_info.epoch.phases.active.end {
                    info!(
                        event = "skip_finalize_registration_phase_ended",
                        run_id = %self.run_id,
                        epoch = epoch_info.epoch.epoch,
                        current_slot,
                        active_phase_end_slot = epoch_info.epoch.phases.active.end,
                        "Skipping FinalizeRegistration because active phase ended"
                    );
                    return Ok(None);
                }

                // TODO: we can put this ix into every tx of the first batch of the current active phase
                let ix = create_finalize_registration_instruction(
                    &self.config.payer_keypair.pubkey(),
                    &self.config.derivation_pubkey,
                    epoch_info.epoch.epoch,
                );
                let priority_fee = self
                    .resolve_epoch_priority_fee(&*rpc, epoch_info.epoch.epoch)
                    .await?;
                let Some(confirmation_deadline) = scheduled_confirmation_deadline(
                    epoch_info
                        .epoch
                        .phases
                        .active
                        .end
                        .saturating_sub(current_slot),
                ) else {
                    info!(
                        event = "skip_finalize_registration_confirmation_budget_exhausted",
                        run_id = %self.run_id,
                        epoch = epoch_info.epoch.epoch,
                        current_slot,
                        active_phase_end_slot = epoch_info.epoch.phases.active.end,
                        "Skipping FinalizeRegistration because not enough active-phase time remains for confirmation"
                    );
                    return Ok(None);
                };
                let payer = self.config.payer_keypair.pubkey();
                let signers = [&self.config.payer_keypair];
                send_smart_transaction(
                    &mut *rpc,
                    SendSmartTransactionConfig {
                        instructions: vec![ix],
                        payer: &payer,
                        signers: &signers,
                        address_lookup_tables: &self.address_lookup_tables,
                        compute_budget: ComputeBudgetConfig {
                            compute_unit_price: priority_fee,
                            compute_unit_limit: Some(self.config.transaction_config.cu_limit),
                        },
                        confirmation: Some(self.confirmation_config()),
                        confirmation_deadline: Some(confirmation_deadline),
                    },
                )
                .await
                .map_err(RpcError::from)?;
            }
        }

        let mut epoch_info = (*epoch_info).clone();
        epoch_info.forester_epoch_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&epoch_info.epoch.forester_epoch_pda)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch ForesterEpochPda for epoch {} at address {}",
                    epoch_info.epoch.epoch, epoch_info.epoch.forester_epoch_pda
                )
            })?
            .ok_or(RegistrationError::ForesterEpochPdaNotFound {
                epoch: epoch_info.epoch.epoch,
                pda_address: epoch_info.epoch.forester_epoch_pda,
            })?;

        let slot = rpc.get_slot().await?;
        let trees = self.trees.lock().await;
        trace!("Adding schedule for trees: {:?}", *trees);
        epoch_info.add_trees_with_schedule(&trees, slot)?;

        if self.compressible_tracker.is_some() && self.config.compressible_config.is_some() {
            let compression_tree_accounts = TreeAccounts {
                merkle_tree: solana_sdk::pubkey::Pubkey::default(),
                queue: solana_sdk::pubkey::Pubkey::default(),
                tree_type: TreeType::Unknown,
                is_rolledover: false,
                owner: solana_sdk::pubkey::Pubkey::default(),
            };
            let tree_schedule = TreeForesterSchedule::new_with_schedule(
                &compression_tree_accounts,
                slot,
                &epoch_info.forester_epoch_pda,
                &epoch_info.epoch_pda,
            )
            .map_err(anyhow::Error::from)?;
            epoch_info.trees.insert(0, tree_schedule);
            debug!("Added compression tree to epoch {}", epoch_info.epoch.epoch);
        }

        info!(
            event = "active_phase_ready",
            run_id = %self.run_id,
            epoch = epoch_info.epoch.epoch,
            "Finished waiting for active phase"
        );
        Ok(Some(epoch_info))
    }

    // TODO: add receiver for new tree discovered -> spawn new task to process this tree derive schedule etc.
    // TODO: optimize active phase startup time
    #[instrument(
        level = "debug",
        skip(self, epoch_info),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch
    ))]
    async fn perform_active_work(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        self.heartbeat.increment_active_cycle();

        let current_slot = self.slot_tracker.estimated_current_slot();
        let active_phase_end = epoch_info.epoch.phases.active.end;

        if !self.is_in_active_phase(current_slot, epoch_info)? {
            info!(
                event = "active_work_skipped_not_in_phase",
                run_id = %self.run_id,
                current_slot,
                active_phase_end,
                "No longer in active phase. Skipping work."
            );
            return Ok(());
        }

        self.sync_slot().await?;

        let trees_to_process: Vec<_> = epoch_info
            .trees
            .iter()
            .filter(|tree| !should_skip_tree(&self.config, &tree.tree_accounts.tree_type))
            .cloned()
            .collect();

        info!(
            event = "active_work_cycle_started",
            run_id = %self.run_id,
            current_slot,
            active_phase_end,
            tree_count = trees_to_process.len(),
            "Starting active work cycle"
        );

        let self_arc = Arc::new(self.clone());
        let registration_tracker = self
            .registration_trackers
            .entry(epoch_info.epoch.epoch)
            .or_insert_with(|| {
                Arc::new(RegistrationTracker::new(
                    epoch_info.epoch_pda.registered_weight,
                ))
            })
            .value()
            .clone();

        let mut handles: Vec<TreeProcessingTask> = Vec::with_capacity(trees_to_process.len());

        for tree in trees_to_process {
            debug!(
                event = "tree_processing_task_spawned",
                run_id = %self.run_id,
                tree = %tree.tree_accounts.merkle_tree,
                tree_type = ?tree.tree_accounts.tree_type,
                "Spawning tree processing task"
            );
            self.heartbeat.add_tree_tasks_spawned(1);

            let self_clone = self_arc.clone();
            let epoch_clone = epoch_info.epoch.clone();
            let forester_epoch_pda = epoch_info.forester_epoch_pda.clone();
            let tracker = registration_tracker.clone();

            let handle = tokio::spawn(async move {
                self_clone
                    .process_queue(&epoch_clone, forester_epoch_pda, tree, tracker)
                    .await
            });

            handles.push(handle);
        }

        debug!("Waiting for {} tree processing tasks", handles.len());
        let results = join_all(handles).await;
        let mut success_count = 0usize;
        let mut error_count = 0usize;
        let mut panic_count = 0usize;
        for result in results {
            match result {
                Ok(Ok(())) => success_count += 1,
                Ok(Err(e)) => {
                    error_count += 1;
                    error!(
                        event = "tree_processing_task_failed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Error processing queue"
                    );
                }
                Err(e) => {
                    panic_count += 1;
                    error!(
                        event = "tree_processing_task_panicked",
                        run_id = %self.run_id,
                        error = ?e,
                        "Tree processing task panicked"
                    );
                }
            }
        }
        info!(
            event = "active_work_cycle_completed",
            run_id = %self.run_id,
            tree_tasks = success_count + error_count + panic_count,
            succeeded = success_count,
            failed = error_count,
            panicked = panic_count,
            "Active work cycle completed"
        );

        debug!("Waiting for active phase to end");
        let mut rpc = self.rpc_pool.get_connection().await?;
        wait_until_slot_reached(&mut *rpc, &self.slot_tracker, active_phase_end).await?;
        Ok(())
    }

    async fn sync_slot(&self) -> Result<u64> {
        let rpc = self.rpc_pool.get_connection().await?;
        let current_slot = rpc.get_slot().await?;
        self.slot_tracker.update(current_slot);
        Ok(current_slot)
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, forester_epoch_pda, tree_schedule, registration_tracker),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree_schedule.tree_accounts.merkle_tree)
    )]
    pub(crate) async fn process_queue(
        &self,
        epoch_info: &Epoch,
        mut forester_epoch_pda: ForesterEpochPda,
        mut tree_schedule: TreeForesterSchedule,
        registration_tracker: Arc<RegistrationTracker>,
    ) -> Result<()> {
        self.heartbeat.increment_queue_started();
        let mut current_slot = self.slot_tracker.estimated_current_slot();

        let total_slots = tree_schedule.slots.len();
        let eligible_slots = tree_schedule.slots.iter().filter(|s| s.is_some()).count();
        let tree_type = tree_schedule.tree_accounts.tree_type;

        debug!(
            event = "process_queue_started",
            run_id = %self.run_id,
            tree = %tree_schedule.tree_accounts.merkle_tree,
            tree_type = ?tree_type,
            total_slots,
            eligible_slots,
            current_slot,
            active_phase_end = epoch_info.phases.active.end,
            "Processing queue for tree"
        );

        let mut last_weight_check = Instant::now();
        const WEIGHT_CHECK_INTERVAL: Duration = Duration::from_secs(30);

        'outer_slot_loop: while current_slot < epoch_info.phases.active.end {
            let next_slot_to_process = tree_schedule
                .slots
                .iter_mut()
                .enumerate()
                .find_map(|(idx, opt_slot)| opt_slot.as_ref().map(|s| (idx, s.clone())));

            if let Some((slot_idx, light_slot_details)) = next_slot_to_process {
                let result = match tree_type {
                    TreeType::StateV1 | TreeType::AddressV1 | TreeType::Unknown => {
                        self.process_light_slot(
                            epoch_info,
                            &forester_epoch_pda,
                            &tree_schedule.tree_accounts,
                            &light_slot_details,
                        )
                        .await
                    }
                    TreeType::StateV2 | TreeType::AddressV2 => {
                        let consecutive_end = tree_schedule
                            .get_consecutive_eligibility_end(slot_idx)
                            .unwrap_or(light_slot_details.end_solana_slot);
                        self.process_light_slot_v2(
                            epoch_info,
                            &forester_epoch_pda,
                            &tree_schedule.tree_accounts,
                            &light_slot_details,
                            consecutive_end,
                        )
                        .await
                    }
                };

                let mut force_refinalize = false;
                match result {
                    Ok(_) => {
                        trace!(
                            "Successfully processed light slot {:?}",
                            light_slot_details.slot
                        );
                    }
                    Err(e) => {
                        force_refinalize = e.is_forester_not_eligible();
                        if force_refinalize {
                            warn!(
                                event = "light_slot_processing_stale_eligibility",
                                run_id = %self.run_id,
                                tree = %tree_schedule.tree_accounts.merkle_tree,
                                light_slot = light_slot_details.slot,
                                "Detected ForesterNotEligible; forcing immediate re-finalization"
                            );
                        }
                        error!(
                            event = "light_slot_processing_error",
                            run_id = %self.run_id,
                            light_slot = light_slot_details.slot,
                            error = ?e,
                            "Error processing light slot"
                        );
                    }
                }
                tree_schedule.slots[slot_idx] = None;

                // Check if re-finalization is needed: either forced (after
                // ForesterNotEligible) or periodic (every WEIGHT_CHECK_INTERVAL).
                // force=true bypasses the weight-change check to handle the case
                // where cached_weight is correct but schedule was never recomputed.
                if force_refinalize || last_weight_check.elapsed() >= WEIGHT_CHECK_INTERVAL {
                    last_weight_check = Instant::now();
                    if let Err(e) = self
                        .maybe_refinalize(
                            epoch_info,
                            &mut forester_epoch_pda,
                            &mut tree_schedule,
                            &registration_tracker,
                            force_refinalize,
                        )
                        .await
                    {
                        warn!(
                            event = "refinalize_check_failed",
                            run_id = %self.run_id,
                            forced = force_refinalize,
                            error = ?e,
                            "Failed to check/perform re-finalization"
                        );
                    }
                }
            } else {
                debug!(
                    event = "process_queue_no_eligible_slots",
                    run_id = %self.run_id,
                    tree = %tree_schedule.tree_accounts.merkle_tree,
                    "No further eligible slots in schedule"
                );
                break 'outer_slot_loop;
            }

            current_slot = self.slot_tracker.estimated_current_slot();
        }

        self.heartbeat.increment_queue_finished();
        debug!(
            event = "process_queue_finished",
            run_id = %self.run_id,
            tree = %tree_schedule.tree_accounts.merkle_tree,
            "Exiting process_queue"
        );
        Ok(())
    }

    /// Check if `EpochPda.registered_weight` changed on-chain. If so,
    /// one task sends a `finalize_registration` tx while others wait,
    /// then all tasks refresh their `ForesterEpochPda` and recompute schedules.
    ///
    /// When `force` is true (e.g. after a ForesterNotEligible error), skips
    /// the weight-change check and unconditionally refreshes the schedule.
    async fn maybe_refinalize(
        &self,
        epoch_info: &Epoch,
        forester_epoch_pda: &mut ForesterEpochPda,
        tree_schedule: &mut TreeForesterSchedule,
        registration_tracker: &RegistrationTracker,
        force: bool,
    ) -> Result<()> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        let epoch_pda_address = get_epoch_pda_address(epoch_info.epoch);
        let on_chain_epoch_pda: EpochPda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await?
            .ok_or_else(|| anyhow!("EpochPda not found for epoch {}", epoch_info.epoch))?;

        let on_chain_weight = on_chain_epoch_pda.registered_weight;
        let cached_weight = registration_tracker.cached_weight();
        let weight_changed = on_chain_weight != cached_weight;

        if !weight_changed && !force {
            return Ok(());
        }

        if weight_changed {
            info!(
                event = "registered_weight_changed",
                run_id = %self.run_id,
                epoch = epoch_info.epoch,
                old_weight = cached_weight,
                new_weight = on_chain_weight,
                "Detected new forester registration, re-finalizing"
            );

            if registration_tracker.try_claim_refinalize() {
                // This task sends the finalize_registration tx
                let ix = create_finalize_registration_instruction(
                    &self.config.payer_keypair.pubkey(),
                    &self.config.derivation_pubkey,
                    epoch_info.epoch,
                );
                let priority_fee = self
                    .resolve_epoch_priority_fee(&*rpc, epoch_info.epoch)
                    .await?;
                let current_slot = rpc.get_slot().await?;
                let Some(confirmation_deadline) = scheduled_confirmation_deadline(
                    epoch_info.phases.active.end.saturating_sub(current_slot),
                ) else {
                    info!(
                        event = "refinalize_registration_skipped_confirmation_budget_exhausted",
                        run_id = %self.run_id,
                        epoch = epoch_info.epoch,
                        current_slot,
                        active_phase_end_slot = epoch_info.phases.active.end,
                        "Skipping re-finalization because not enough active-phase time remains for confirmation"
                    );
                    registration_tracker.complete_refinalize(cached_weight);
                    return Ok(());
                };
                let payer = self.config.payer_keypair.pubkey();
                let signers = [&self.config.payer_keypair];
                match send_smart_transaction(
                    &mut *rpc,
                    SendSmartTransactionConfig {
                        instructions: vec![ix],
                        payer: &payer,
                        signers: &signers,
                        address_lookup_tables: &self.address_lookup_tables,
                        compute_budget: ComputeBudgetConfig {
                            compute_unit_price: priority_fee,
                            compute_unit_limit: Some(self.config.transaction_config.cu_limit),
                        },
                        confirmation: Some(self.confirmation_config()),
                        confirmation_deadline: Some(confirmation_deadline),
                    },
                )
                .await
                .map_err(RpcError::from)
                {
                    Ok(_) => {
                        // Re-fetch EpochPda after finalize to get authoritative
                        // post-finalize weight (another forester may have registered
                        // between our initial read and the finalize tx).
                        // Fallback to on_chain_weight if re-fetch fails to avoid
                        // deadlocking the RegistrationTracker.
                        let post_finalize_weight =
                            match rpc.get_anchor_account::<EpochPda>(&epoch_pda_address).await {
                                Ok(Some(pda)) => pda.registered_weight,
                                _ => on_chain_weight,
                            };
                        info!(
                            event = "refinalize_registration_success",
                            run_id = %self.run_id,
                            epoch = epoch_info.epoch,
                            new_weight = post_finalize_weight,
                            "Re-finalized registration on-chain"
                        );
                        registration_tracker.complete_refinalize(post_finalize_weight);
                    }
                    Err(e) => {
                        // Release the claim so a future check can retry
                        registration_tracker.complete_refinalize(cached_weight);
                        return Err(e.into());
                    }
                }
            } else {
                // Another task is already re-finalizing; wait for it
                registration_tracker.wait_for_refinalize().await;
            }
        }

        // All tasks: re-fetch both PDAs to get latest on-chain state and
        // recompute schedule (after finalize or forced refresh).
        let refreshed_epoch_pda: EpochPda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await?
            .ok_or_else(|| anyhow!("EpochPda not found for epoch {}", epoch_info.epoch))?;
        let updated_pda: ForesterEpochPda = rpc
            .get_anchor_account::<ForesterEpochPda>(&epoch_info.forester_epoch_pda)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "ForesterEpochPda not found at {} after re-finalization",
                    epoch_info.forester_epoch_pda
                )
            })?;

        let current_slot = self.slot_tracker.estimated_current_slot();
        let new_schedule = TreeForesterSchedule::new_with_schedule(
            &tree_schedule.tree_accounts,
            current_slot,
            &updated_pda,
            &refreshed_epoch_pda,
        )?;

        *forester_epoch_pda = updated_pda;
        *tree_schedule = new_schedule;

        info!(
            event = "schedule_recomputed_after_refinalize",
            run_id = %self.run_id,
            epoch = epoch_info.epoch,
            tree = %tree_schedule.tree_accounts.merkle_tree,
            new_eligible_slots = tree_schedule.slots.iter().filter(|s| s.is_some()).count(),
            "Recomputed schedule after re-finalization"
        );

        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree_accounts, forester_slot_details),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree_accounts.merkle_tree)
    )]
    async fn process_light_slot(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
    ) -> std::result::Result<(), ForesterError> {
        debug!(
            event = "light_slot_processing_started",
            run_id = %self.run_id,
            tree = %tree_accounts.merkle_tree,
            epoch = epoch_info.epoch,
            light_slot = forester_slot_details.slot,
            slot_start = forester_slot_details.start_solana_slot,
            slot_end = forester_slot_details.end_solana_slot,
            "Processing light slot"
        );
        let mut rpc = self.rpc_pool.get_connection().await?;
        wait_until_slot_reached(
            &mut *rpc,
            &self.slot_tracker,
            forester_slot_details.start_solana_slot,
        )
        .await?;
        let mut estimated_slot = self.slot_tracker.estimated_current_slot();

        'inner_processing_loop: loop {
            if estimated_slot >= forester_slot_details.end_solana_slot {
                trace!(
                    "Ending processing for slot {:?} due to time limit.",
                    forester_slot_details.slot
                );
                break 'inner_processing_loop;
            }

            let current_light_slot = (estimated_slot - epoch_info.phases.active.start)
                / epoch_pda.protocol_config.slot_length;
            if current_light_slot != forester_slot_details.slot {
                warn!(
                    event = "light_slot_mismatch",
                    run_id = %self.run_id,
                    tree = %tree_accounts.merkle_tree,
                    expected_light_slot = forester_slot_details.slot,
                    actual_light_slot = current_light_slot,
                    estimated_slot,
                    "Light slot mismatch; exiting processing for this slot"
                );
                break 'inner_processing_loop;
            }

            if !self
                .check_forester_eligibility(
                    epoch_pda,
                    current_light_slot,
                    &tree_accounts.queue,
                    epoch_info.epoch,
                    epoch_info,
                )
                .await?
            {
                break 'inner_processing_loop;
            }

            let processing_start_time = Instant::now();
            let items_processed_this_iteration = match self
                .dispatch_tree_processing(
                    epoch_info,
                    epoch_pda,
                    tree_accounts,
                    forester_slot_details,
                    forester_slot_details.end_solana_slot,
                    estimated_slot,
                )
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    if e.is_forester_not_eligible() {
                        return Err(e);
                    }
                    error!(
                        event = "light_slot_processing_failed",
                        run_id = %self.run_id,
                        tree = %tree_accounts.merkle_tree,
                        light_slot = forester_slot_details.slot,
                        error = ?e,
                        "Failed processing in light slot"
                    );
                    break 'inner_processing_loop;
                }
            };
            if items_processed_this_iteration > 0 {
                debug!(
                    event = "light_slot_items_processed",
                    run_id = %self.run_id,
                    light_slot = forester_slot_details.slot,
                    items = items_processed_this_iteration,
                    "Processed items in light slot"
                );
            }

            self.update_metrics_and_counts(
                epoch_info.epoch,
                items_processed_this_iteration,
                processing_start_time.elapsed(),
            )
            .await;

            if let Err(e) = push_metrics(&self.config.external_services.pushgateway_url).await {
                if should_emit_rate_limited_warning("push_metrics_v1", Duration::from_secs(30)) {
                    warn!(
                        event = "metrics_push_failed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Failed to push metrics"
                    );
                } else {
                    debug!(
                        event = "metrics_push_failed_suppressed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Suppressing repeated metrics push failure"
                    );
                }
            }
            estimated_slot = self.slot_tracker.estimated_current_slot();

            if items_processed_this_iteration == 0 {
                // No items processed. Short sleep before re-checking — the queue
                // may grow above min_queue_items within this light slot.
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            // When items were processed, loop immediately to fetch the next batch.
        }
        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree_accounts, forester_slot_details, consecutive_eligibility_end),
        fields(tree = %tree_accounts.merkle_tree)
    )]
    async fn process_light_slot_v2(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        consecutive_eligibility_end: u64,
    ) -> std::result::Result<(), ForesterError> {
        debug!(
            event = "v2_light_slot_processing_started",
            run_id = %self.run_id,
            tree = %tree_accounts.merkle_tree,
            light_slot = forester_slot_details.slot,
            slot_start = forester_slot_details.start_solana_slot,
            slot_end = forester_slot_details.end_solana_slot,
            consecutive_eligibility_end_slot = consecutive_eligibility_end,
            "Processing V2 light slot"
        );

        let tree_pubkey = tree_accounts.merkle_tree;

        let mut rpc = self.rpc_pool.get_connection().await?;
        wait_until_slot_reached(
            &mut *rpc,
            &self.slot_tracker,
            forester_slot_details.start_solana_slot,
        )
        .await?;

        // Try to send any cached proofs first
        let cached_send_start = Instant::now();
        if let Some(items_sent) = self
            .try_send_cached_proofs(epoch_info, tree_accounts, consecutive_eligibility_end)
            .await?
        {
            if items_sent > 0 {
                let cached_send_duration = cached_send_start.elapsed();
                info!(
                    event = "cached_proofs_sent",
                    run_id = %self.run_id,
                    tree = %tree_pubkey,
                    items = items_sent,
                    duration_ms = cached_send_duration.as_millis() as u64,
                    "Sent items from proof cache"
                );
                self.update_metrics_and_counts(epoch_info.epoch, items_sent, cached_send_duration)
                    .await;
            }
        }

        let mut estimated_slot = self.slot_tracker.estimated_current_slot();

        // Polling interval for checking queue
        const POLL_INTERVAL: Duration = Duration::from_millis(200);

        'inner_processing_loop: loop {
            if estimated_slot >= forester_slot_details.end_solana_slot {
                trace!(
                    "Ending V2 processing for slot {:?}",
                    forester_slot_details.slot
                );
                break 'inner_processing_loop;
            }

            let current_light_slot = (estimated_slot - epoch_info.phases.active.start)
                / epoch_pda.protocol_config.slot_length;
            if current_light_slot != forester_slot_details.slot {
                warn!(
                    event = "v2_light_slot_mismatch",
                    run_id = %self.run_id,
                    tree = %tree_pubkey,
                    expected_light_slot = forester_slot_details.slot,
                    actual_light_slot = current_light_slot,
                    estimated_slot,
                    "V2 slot mismatch; exiting processing"
                );
                break 'inner_processing_loop;
            }

            if !self
                .check_forester_eligibility(
                    epoch_pda,
                    current_light_slot,
                    &tree_accounts.merkle_tree,
                    epoch_info.epoch,
                    epoch_info,
                )
                .await?
            {
                break 'inner_processing_loop;
            }

            // Process directly - the processor fetches queue data from the indexer
            let processing_start_time = Instant::now();
            match self
                .dispatch_tree_processing(
                    epoch_info,
                    epoch_pda,
                    tree_accounts,
                    forester_slot_details,
                    consecutive_eligibility_end,
                    estimated_slot,
                )
                .await
            {
                Ok(count) => {
                    if count > 0 {
                        info!(
                            event = "v2_tree_processed_items",
                            run_id = %self.run_id,
                            tree = %tree_pubkey,
                            items = count,
                            epoch = epoch_info.epoch,
                            "V2 processed items for tree"
                        );
                        self.update_metrics_and_counts(
                            epoch_info.epoch,
                            count,
                            processing_start_time.elapsed(),
                        )
                        .await;
                    } else {
                        // No items to process, wait before polling again
                        tokio::time::sleep(POLL_INTERVAL).await;
                    }
                }
                Err(e) => {
                    if e.is_forester_not_eligible() {
                        return Err(e);
                    }
                    error!(
                        event = "v2_tree_processing_failed",
                        run_id = %self.run_id,
                        tree = %tree_pubkey,
                        error = ?e,
                        "V2 processing failed for tree"
                    );
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            }

            if let Err(e) = push_metrics(&self.config.external_services.pushgateway_url).await {
                if should_emit_rate_limited_warning("push_metrics_v2", Duration::from_secs(30)) {
                    warn!(
                        event = "metrics_push_failed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Failed to push metrics"
                    );
                } else {
                    debug!(
                        event = "metrics_push_failed_suppressed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Suppressing repeated metrics push failure"
                    );
                }
            }
            estimated_slot = self.slot_tracker.estimated_current_slot();
        }

        Ok(())
    }

    async fn check_forester_eligibility(
        &self,
        epoch_pda: &ForesterEpochPda,
        current_light_slot: u64,
        queue_pubkey: &Pubkey,
        current_epoch_num: u64,
        epoch_info: &Epoch,
    ) -> Result<bool> {
        let current_slot = self.slot_tracker.estimated_current_slot();
        let current_phase_state = epoch_info.phases.get_current_epoch_state(current_slot);

        if current_phase_state != EpochState::Active {
            trace!(
                "Skipping processing: not in active phase (current phase: {:?}, slot: {})",
                current_phase_state,
                current_slot
            );
            return Ok(false);
        }

        let total_epoch_weight = epoch_pda.total_epoch_weight.ok_or_else(|| {
            anyhow::anyhow!(
                "Total epoch weight not available in ForesterEpochPda for epoch {}",
                current_epoch_num
            )
        })?;

        let eligible_forester_slot_index = ForesterEpochPda::get_eligible_forester_index(
            current_light_slot,
            queue_pubkey,
            total_epoch_weight,
            current_epoch_num,
        )
        .map_err(|e| {
            error!(
                event = "eligibility_index_calculation_failed",
                run_id = %self.run_id,
                queue = %queue_pubkey,
                epoch = current_epoch_num,
                light_slot = current_light_slot,
                error = ?e,
                "Failed to calculate eligible forester index"
            );
            anyhow::anyhow!("Eligibility calculation failed: {}", e)
        })?;

        if !epoch_pda.is_eligible(eligible_forester_slot_index) {
            warn!(
                event = "forester_not_eligible_for_slot",
                run_id = %self.run_id,
                forester = %self.config.payer_keypair.pubkey(),
                queue = %queue_pubkey,
                light_slot = current_light_slot,
                "Forester is no longer eligible to process this queue in current light slot"
            );
            return Ok(false);
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_tree_processing(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        consecutive_eligibility_end: u64,
        current_solana_slot: u64,
    ) -> std::result::Result<usize, ForesterError> {
        match tree_accounts.tree_type {
            TreeType::Unknown => self
                .dispatch_compression(
                    epoch_info,
                    epoch_pda,
                    forester_slot_details,
                    consecutive_eligibility_end,
                )
                .await
                .map_err(ForesterError::from),
            TreeType::StateV1 | TreeType::AddressV1 => {
                self.process_v1(
                    epoch_info,
                    epoch_pda,
                    tree_accounts,
                    forester_slot_details,
                    current_solana_slot,
                )
                .await
            }
            TreeType::StateV2 | TreeType::AddressV2 => {
                let result = self
                    .process_v2(epoch_info, tree_accounts, consecutive_eligibility_end)
                    .await?;
                // Accumulate processing metrics for this epoch
                self.add_processing_metrics(epoch_info.epoch, result.metrics)
                    .await;
                Ok(result.items_processed)
            }
        }
    }

    async fn dispatch_compression(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        forester_slot_details: &ForesterSlot,
        consecutive_eligibility_end: u64,
    ) -> Result<usize> {
        let current_slot = self.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                "Skipping compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                current_slot, consecutive_eligibility_end
            );
            return Ok(0);
        }

        if current_slot >= forester_slot_details.end_solana_slot {
            debug!(
                "Skipping compression: forester slot ended (current_slot={}, slot_end={})",
                current_slot, forester_slot_details.end_solana_slot
            );
            return Ok(0);
        }

        let current_light_slot = current_slot.saturating_sub(epoch_info.phases.active.start)
            / epoch_pda.protocol_config.slot_length;
        if !self
            .check_forester_eligibility(
                epoch_pda,
                current_light_slot,
                &Pubkey::default(),
                epoch_info.epoch,
                epoch_info,
            )
            .await?
        {
            debug!(
                "Skipping compression: forester not eligible for current light slot {}",
                current_light_slot
            );
            return Ok(0);
        }

        debug!("Dispatching compression for epoch {}", epoch_info.epoch);

        let tracker = self
            .compressible_tracker
            .as_ref()
            .ok_or_else(|| anyhow!("Compressible tracker not initialized"))?;

        let config = self
            .config
            .compressible_config
            .as_ref()
            .ok_or_else(|| anyhow!("Compressible config not set"))?;
        let accounts = tracker.get_ready_to_compress(current_slot);

        if accounts.is_empty() {
            trace!("No compressible accounts ready for compression");
            return Ok(0);
        }

        let num_batches = accounts.len().div_ceil(config.batch_size);
        info!(
            event = "compression_ctoken_started",
            run_id = %self.run_id,
            accounts = accounts.len(),
            batches = num_batches,
            batch_size = config.batch_size,
            "Starting ctoken compression batches"
        );

        let compressor = CTokenCompressor::new(
            self.rpc_pool.clone(),
            tracker.clone(),
            self.config.payer_keypair.insecure_clone(),
            self.transaction_policy(),
        );

        // Derive registered forester PDA once for all batches
        let (registered_forester_pda, _) =
            light_registry::utils::get_forester_epoch_pda_from_authority(
                &self.config.derivation_pubkey,
                epoch_info.epoch,
            );

        // Create parallel compression futures
        use futures::stream::StreamExt;

        // Collect chunks into owned vectors to avoid lifetime issues
        let batches: Vec<(usize, Vec<_>)> = accounts
            .chunks(config.batch_size)
            .enumerate()
            .map(|(idx, chunk)| (idx, chunk.to_vec()))
            .collect();

        let slot_tracker = self.slot_tracker.clone();
        // Shared cancellation flag - when set, all pending futures should skip processing
        let cancelled = Arc::new(AtomicBool::new(false));

        let compression_futures = batches.into_iter().map(|(batch_idx, batch)| {
            let compressor = compressor.clone();
            let slot_tracker = slot_tracker.clone();
            let cancelled = cancelled.clone();
            async move {
                // Check if already cancelled by another future
                if cancelled.load(Ordering::Relaxed) {
                    debug!(
                        "Skipping compression batch {}/{}: cancelled",
                        batch_idx + 1,
                        num_batches
                    );
                    return Err((batch_idx, batch.len(), Cancelled.into()));
                }

                // Check forester is still eligible before processing this batch
                let current_slot = slot_tracker.estimated_current_slot();
                if current_slot >= consecutive_eligibility_end {
                    // Signal cancellation to all other futures
                    cancelled.store(true, Ordering::Relaxed);
                    warn!(
                        event = "compression_ctoken_cancelled_not_eligible",
                        run_id = %self.run_id,
                        current_slot,
                        eligibility_end_slot = consecutive_eligibility_end,
                        "Cancelling compression because forester is no longer eligible"
                    );
                    return Err((
                        batch_idx,
                        batch.len(),
                        anyhow!("Forester no longer eligible"),
                    ));
                }

                debug!(
                    "Processing compression batch {}/{} with {} accounts",
                    batch_idx + 1,
                    num_batches,
                    batch.len()
                );

                match compressor
                    .compress_batch(&batch, registered_forester_pda)
                    .await
                {
                    Ok(sig) => {
                        debug!(
                            "Compression batch {}/{} succeeded: {}",
                            batch_idx + 1,
                            num_batches,
                            sig
                        );
                        Ok((batch_idx, batch.len(), sig))
                    }
                    Err(e) => {
                        error!(
                            event = "compression_ctoken_batch_failed",
                            run_id = %self.run_id,
                            batch = batch_idx + 1,
                            total_batches = num_batches,
                            error = ?e,
                            "Compression batch failed"
                        );
                        Err((batch_idx, batch.len(), e))
                    }
                }
            }
        });

        // Execute batches in parallel with concurrency limit
        let results = futures::stream::iter(compression_futures)
            .buffer_unordered(config.max_concurrent_batches)
            .collect::<Vec<_>>()
            .await;

        // Aggregate results
        let mut total_compressed = 0;
        for result in results {
            match result {
                Ok((batch_idx, count, sig)) => {
                    info!(
                        event = "compression_ctoken_batch_succeeded",
                        run_id = %self.run_id,
                        batch = batch_idx + 1,
                        total_batches = num_batches,
                        accounts = count,
                        signature = %sig,
                        "Compression batch succeeded"
                    );
                    total_compressed += count;
                }
                Err((batch_idx, count, e)) => {
                    error!(
                        event = "compression_ctoken_batch_failed_final",
                        run_id = %self.run_id,
                        batch = batch_idx + 1,
                        total_batches = num_batches,
                        accounts = count,
                        error = ?e,
                        "Compression batch failed"
                    );
                }
            }
        }

        info!(
            event = "compression_ctoken_completed",
            run_id = %self.run_id,
            epoch = epoch_info.epoch,
            compressed_accounts = total_compressed,
            "Completed ctoken compression"
        );

        // Process PDA compression if configured
        let pda_compressed = self
            .dispatch_pda_compression(epoch_info, epoch_pda, consecutive_eligibility_end)
            .await
            .unwrap_or_else(|e| {
                error!(
                    event = "compression_pda_dispatch_failed",
                    run_id = %self.run_id,
                    error = ?e,
                    "PDA compression failed"
                );
                0
            });

        // Process Mint compression
        let mint_compressed = self
            .dispatch_mint_compression(epoch_info, epoch_pda, consecutive_eligibility_end)
            .await
            .unwrap_or_else(|e| {
                error!(
                    event = "compression_mint_dispatch_failed",
                    run_id = %self.run_id,
                    error = ?e,
                    "Mint compression failed"
                );
                0
            });

        let total = total_compressed + pda_compressed + mint_compressed;
        info!(
            event = "compression_all_completed",
            run_id = %self.run_id,
            epoch = epoch_info.epoch,
            ctoken_compressed = total_compressed,
            pda_compressed,
            mint_compressed,
            total_compressed = total,
            "Completed all compression"
        );
        Ok(total)
    }

    async fn dispatch_pda_compression(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        consecutive_eligibility_end: u64,
    ) -> Result<usize> {
        let Some((pda_tracker, config, current_slot)) = self
            .prepare_compression_dispatch(
                self.pda_tracker.as_ref(),
                "PDA",
                epoch_info,
                epoch_pda,
                consecutive_eligibility_end,
            )
            .await?
        else {
            return Ok(0);
        };

        if config.pda_programs.is_empty() {
            return Ok(0);
        }

        let mut total_compressed = 0;

        // Shared cancellation flag across all programs
        let cancelled = Arc::new(AtomicBool::new(false));

        // Process each configured PDA program
        for program_config in &config.pda_programs {
            // Check cancellation at program level
            if cancelled.load(Ordering::Relaxed) {
                break;
            }

            let accounts = pda_tracker
                .get_ready_to_compress_for_program(&program_config.program_id, current_slot);

            if accounts.is_empty() {
                trace!(
                    "No compressible PDA accounts ready for program {}",
                    program_config.program_id
                );
                continue;
            }

            info!(
                event = "compression_pda_program_started",
                run_id = %self.run_id,
                program = %program_config.program_id,
                accounts = accounts.len(),
                "Processing compressible PDA accounts for program"
            );

            let pda_compressor = crate::compressible::pda::PdaCompressor::new(
                self.rpc_pool.clone(),
                pda_tracker.clone(),
                self.config.payer_keypair.insecure_clone(),
                self.transaction_policy(),
            );

            // Fetch and cache config once per program
            let cached_config = match pda_compressor.fetch_program_config(program_config).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!(
                        event = "compression_pda_program_config_failed",
                        run_id = %self.run_id,
                        program = %program_config.program_id,
                        error = ?e,
                        "Failed to fetch config for PDA program"
                    );
                    continue;
                }
            };

            // Check eligibility before processing
            let current_slot = self.slot_tracker.estimated_current_slot();
            if current_slot >= consecutive_eligibility_end {
                cancelled.store(true, Ordering::Relaxed);
                warn!(
                    event = "compression_pda_cancelled_not_eligible",
                    run_id = %self.run_id,
                    current_slot,
                    eligibility_end_slot = consecutive_eligibility_end,
                    "Stopping PDA compression because forester is no longer eligible"
                );
                break;
            }

            // Process all accounts for this program concurrently
            let results = pda_compressor
                .compress_batch_concurrent(
                    &accounts,
                    program_config,
                    &cached_config,
                    config.max_concurrent_batches,
                    cancelled.clone(),
                )
                .await;

            // Process results (tracker cleanup already done by compressor)
            for result in results {
                match result {
                    CompressionOutcome::Compressed {
                        signature: sig,
                        state: account_state,
                    } => {
                        debug!(
                            "Compressed PDA {} for program {}: {}",
                            account_state.pubkey, program_config.program_id, sig
                        );
                        total_compressed += 1;
                    }
                    CompressionOutcome::Failed {
                        state: _account_state,
                        error: CompressionTaskError::Cancelled,
                    } => {}
                    CompressionOutcome::Failed {
                        state: account_state,
                        error: CompressionTaskError::Failed(e),
                    } => {
                        error!(
                            event = "compression_pda_account_failed",
                            run_id = %self.run_id,
                            account = %account_state.pubkey,
                            program = %program_config.program_id,
                            error = ?e,
                            "Failed to compress PDA account"
                        );
                    }
                }
            }
        }

        info!(
            event = "compression_pda_completed",
            run_id = %self.run_id,
            compressed_accounts = total_compressed,
            "Completed PDA compression"
        );
        Ok(total_compressed)
    }

    async fn dispatch_mint_compression(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        consecutive_eligibility_end: u64,
    ) -> Result<usize> {
        let Some((mint_tracker, config, current_slot)) = self
            .prepare_compression_dispatch(
                self.mint_tracker.as_ref(),
                "Mint",
                epoch_info,
                epoch_pda,
                consecutive_eligibility_end,
            )
            .await?
        else {
            return Ok(0);
        };

        let accounts = mint_tracker.get_ready_to_compress(current_slot);

        if accounts.is_empty() {
            trace!("No compressible Mint accounts ready");
            return Ok(0);
        }

        info!(
            event = "compression_mint_started",
            run_id = %self.run_id,
            accounts = accounts.len(),
            max_concurrent = config.max_concurrent_batches,
            "Processing compressible Mint accounts"
        );

        let mint_compressor = crate::compressible::mint::MintCompressor::new(
            self.rpc_pool.clone(),
            mint_tracker.clone(),
            self.config.payer_keypair.insecure_clone(),
            self.transaction_policy(),
        );

        // Shared cancellation flag
        let cancelled = Arc::new(AtomicBool::new(false));

        // Process all mints concurrently
        let results = mint_compressor
            .compress_batch_concurrent(&accounts, config.max_concurrent_batches, cancelled)
            .await;

        // Process results (tracker cleanup already done by compressor)
        let mut total_compressed = 0;
        for result in results {
            match result {
                CompressionOutcome::Compressed {
                    signature: sig,
                    state: mint_state,
                } => {
                    debug!("Compressed Mint {}: {}", mint_state.pubkey, sig);
                    total_compressed += 1;
                }
                CompressionOutcome::Failed {
                    state: _mint_state,
                    error: CompressionTaskError::Cancelled,
                } => {}
                CompressionOutcome::Failed {
                    state: mint_state,
                    error: CompressionTaskError::Failed(e),
                } => {
                    error!(
                        event = "compression_mint_account_failed",
                        run_id = %self.run_id,
                        mint = %mint_state.pubkey,
                        error = ?e,
                        "Failed to compress mint account"
                    );
                }
            }
        }

        info!(
            event = "compression_mint_completed",
            run_id = %self.run_id,
            compressed_accounts = total_compressed,
            "Completed Mint compression"
        );
        Ok(total_compressed)
    }

    async fn prepare_compression_dispatch<'a, T>(
        &'a self,
        tracker: Option<&'a T>,
        label: &'static str,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        consecutive_eligibility_end: u64,
    ) -> Result<Option<(&'a T, &'a CompressibleConfig, u64)>> {
        let Some(tracker) = tracker else {
            return Ok(None);
        };

        let Some(config) = self.config.compressible_config.as_ref() else {
            return Ok(None);
        };

        let current_slot = self.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                "Skipping {} compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                label, current_slot, consecutive_eligibility_end
            );
            return Ok(None);
        }

        let current_light_slot = current_slot.saturating_sub(epoch_info.phases.active.start)
            / epoch_pda.protocol_config.slot_length;
        if !self
            .check_forester_eligibility(
                epoch_pda,
                current_light_slot,
                &Pubkey::default(),
                epoch_info.epoch,
                epoch_info,
            )
            .await?
        {
            debug!(
                "Skipping {} compression: forester not eligible for current light slot {}",
                label, current_light_slot
            );
            return Ok(None);
        }

        Ok(Some((tracker, config, current_slot)))
    }

    async fn process_v1(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        current_solana_slot: u64,
    ) -> std::result::Result<usize, ForesterError> {
        let slots_remaining = forester_slot_details
            .end_solana_slot
            .saturating_sub(current_solana_slot);
        let Some(remaining_time_timeout) = scheduled_v1_batch_timeout(slots_remaining) else {
            debug!(
                event = "v1_tree_skipped_low_slot_budget",
                run_id = %self.run_id,
                tree = %tree_accounts.merkle_tree,
                slots_remaining,
                "Skipping V1 tree: not enough scheduled slot budget left to confirm a transaction"
            );
            return Ok(0);
        };

        let batched_tx_config = SendBatchedTransactionsConfig {
            num_batches: 1,
            build_transaction_batch_config: BuildTransactionBatchConfig {
                batch_size: self.config.transaction_config.legacy_ixs_per_tx as u64,
                compute_unit_price: self.config.transaction_config.priority_fee_microlamports,
                compute_unit_limit: Some(self.config.transaction_config.cu_limit),
                enable_priority_fees: self.config.transaction_config.enable_priority_fees,
                max_concurrent_sends: Some(self.config.transaction_config.max_concurrent_sends),
            },
            queue_config: self.config.queue_config,
            retry_config: RetryConfig {
                timeout: remaining_time_timeout,
                ..self.config.retry_config
            },
            light_slot_length: epoch_pda.protocol_config.slot_length,
            confirmation_poll_interval: Duration::from_millis(
                self.config.transaction_config.confirmation_poll_interval_ms,
            ),
            confirmation_max_attempts: self.config.transaction_config.confirmation_max_attempts
                as usize,
            min_queue_items: if self.config.enable_v1_multi_nullify
                && !self.address_lookup_tables.is_empty()
            {
                self.config.min_queue_items
            } else {
                None
            },
            enable_presort: self.config.enable_v1_multi_nullify
                && !self.address_lookup_tables.is_empty(),
            work_item_batch_size: self.config.work_item_batch_size,
        };

        let alt_snapshot = (*self.address_lookup_tables).clone();
        let transaction_builder = Arc::new(EpochManagerTransactions::new(
            self.rpc_pool.clone(),
            epoch_info.epoch,
            self.tx_cache.clone(),
            alt_snapshot,
            self.config.enable_v1_multi_nullify,
        ));

        let num_sent = send_batched_transactions(
            &self.config.payer_keypair,
            &self.config.derivation_pubkey,
            self.rpc_pool.clone(),
            &batched_tx_config,
            *tree_accounts,
            transaction_builder,
        )
        .await?;

        if num_sent > 0 {
            debug!(
                event = "v1_tree_items_processed",
                run_id = %self.run_id,
                tree = %tree_accounts.merkle_tree,
                items = num_sent,
                "Processed items for V1 tree"
            );
        }

        match self.rollover_if_needed(tree_accounts).await {
            Ok(_) => Ok(num_sent),
            Err(e) => {
                error!(
                    event = "tree_rollover_failed",
                    run_id = %self.run_id,
                    tree = %tree_accounts.merkle_tree,
                    tree_type = ?tree_accounts.tree_type,
                    error = ?e,
                    "Failed to rollover tree"
                );
                Err(e.into())
            }
        }
    }

    fn build_batch_context(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        input_queue_hint: Option<u64>,
        output_queue_hint: Option<u64>,
        eligibility_end: Option<u64>,
        address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
    ) -> BatchContext<R> {
        let default_prover_url = "http://127.0.0.1:3001".to_string();
        let eligibility_end = eligibility_end.unwrap_or(0);
        BatchContext {
            rpc_pool: self.rpc_pool.clone(),
            authority: self.authority.clone(),
            run_id: self.run_id.clone(),
            derivation: self.config.derivation_pubkey,
            epoch: epoch_info.epoch,
            merkle_tree: tree_accounts.merkle_tree,
            output_queue: tree_accounts.queue,
            prover_config: Arc::new(ProverConfig {
                append_url: self
                    .config
                    .external_services
                    .prover_append_url
                    .clone()
                    .unwrap_or_else(|| default_prover_url.clone()),
                update_url: self
                    .config
                    .external_services
                    .prover_update_url
                    .clone()
                    .unwrap_or_else(|| default_prover_url.clone()),
                address_append_url: self
                    .config
                    .external_services
                    .prover_address_append_url
                    .clone()
                    .unwrap_or_else(|| default_prover_url.clone()),
                api_key: self.config.external_services.prover_api_key.clone(),
                polling_interval: self
                    .config
                    .external_services
                    .prover_polling_interval
                    .unwrap_or(Duration::from_secs(1)),
                max_wait_time: self
                    .config
                    .external_services
                    .prover_max_wait_time
                    .unwrap_or(Duration::from_secs(600)),
            }),
            ops_cache: self.ops_cache.clone(),
            epoch_phases: epoch_info.phases.clone(),
            slot_tracker: self.slot_tracker.clone(),
            input_queue_hint,
            output_queue_hint,
            num_proof_workers: self.config.transaction_config.max_concurrent_batches,
            forester_eligibility_end_slot: Arc::new(AtomicU64::new(eligibility_end)),
            address_lookup_tables,
            transaction_policy: self.transaction_policy(),
            max_batches_per_tree: self.config.transaction_config.max_batches_per_tree,
        }
    }

    fn confirmation_config(&self) -> ConfirmationConfig {
        ConfirmationConfig {
            max_attempts: self.config.transaction_config.confirmation_max_attempts,
            poll_interval: Duration::from_millis(
                self.config.transaction_config.confirmation_poll_interval_ms,
            ),
        }
    }

    fn transaction_priority_fee_config(&self) -> PriorityFeeConfig {
        PriorityFeeConfig {
            compute_unit_price: self.config.transaction_config.priority_fee_microlamports,
            enable_priority_fees: self.config.transaction_config.enable_priority_fees,
        }
    }

    fn transaction_policy(&self) -> TransactionPolicy {
        TransactionPolicy {
            priority_fee_config: self.transaction_priority_fee_config(),
            compute_unit_limit: Some(self.config.transaction_config.cu_limit),
            confirmation: Some(self.confirmation_config()),
        }
    }

    async fn resolve_epoch_priority_fee<RpcT: Rpc>(
        &self,
        rpc: &RpcT,
        epoch: u64,
    ) -> Result<Option<u64>> {
        self.transaction_priority_fee_config()
            .resolve(
                rpc,
                vec![
                    self.config.payer_keypair.pubkey(),
                    get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0,
                ],
            )
            .await
    }

    async fn resolve_tree_priority_fee<RpcT: Rpc>(
        &self,
        rpc: &RpcT,
        epoch: u64,
        tree_accounts: &TreeAccounts,
    ) -> Result<Option<u64>> {
        self.transaction_priority_fee_config()
            .resolve(
                rpc,
                vec![
                    self.config.payer_keypair.pubkey(),
                    get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0,
                    tree_accounts.queue,
                    tree_accounts.merkle_tree,
                ],
            )
            .await
    }

    async fn get_or_create_state_processor(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
    ) -> Result<Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>> {
        // Serialize initialization per tree to avoid duplicate expensive processor construction.
        let init_lock = self
            .state_processor_init_locks
            .entry(tree_accounts.merkle_tree)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _init_guard = init_lock.lock().await;

        // First check if we already have a processor for this tree
        // We REUSE processors across epochs to preserve cached state for optimistic processing
        if let Some(entry) = self.state_processors.get(&tree_accounts.merkle_tree) {
            let (stored_epoch, processor_ref) = entry.value();
            let processor_clone = processor_ref.clone();
            let old_epoch = *stored_epoch;
            drop(entry); // Release read lock before any async operation

            if old_epoch != epoch_info.epoch {
                // Update epoch in the map (processor is reused with its cached state)
                debug!(
                    "Reusing StateBatchProcessor for tree {} across epoch transition ({} -> {})",
                    tree_accounts.merkle_tree, old_epoch, epoch_info.epoch
                );
                self.state_processors.insert(
                    tree_accounts.merkle_tree,
                    (epoch_info.epoch, processor_clone.clone()),
                );
                // Update the processor's epoch context and phases
                processor_clone
                    .lock()
                    .await
                    .update_epoch(epoch_info.epoch, epoch_info.phases.clone());
            }
            return Ok(processor_clone);
        }

        // No existing processor - create new one
        let batch_context = self.build_batch_context(
            epoch_info,
            tree_accounts,
            None,
            None,
            None,
            self.address_lookup_tables.clone(),
        );
        let processor = Arc::new(Mutex::new(
            QueueProcessor::new(batch_context, StateTreeStrategy).await?,
        ));

        // Cache the zkp_batch_size for early filtering of queue updates
        let batch_size = processor.lock().await.zkp_batch_size();
        self.zkp_batch_sizes
            .insert(tree_accounts.merkle_tree, batch_size);

        self.state_processors.insert(
            tree_accounts.merkle_tree,
            (epoch_info.epoch, processor.clone()),
        );
        Ok(processor)
    }

    async fn get_or_create_address_processor(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
    ) -> Result<Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>> {
        // Serialize initialization per tree to avoid duplicate expensive processor construction.
        let init_lock = self
            .address_processor_init_locks
            .entry(tree_accounts.merkle_tree)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _init_guard = init_lock.lock().await;

        if let Some(entry) = self.address_processors.get(&tree_accounts.merkle_tree) {
            let (stored_epoch, processor_ref) = entry.value();
            let processor_clone = processor_ref.clone();
            let old_epoch = *stored_epoch;
            drop(entry);

            if old_epoch != epoch_info.epoch {
                debug!(
                    "Reusing AddressBatchProcessor for tree {} across epoch transition ({} -> {})",
                    tree_accounts.merkle_tree, old_epoch, epoch_info.epoch
                );
                self.address_processors.insert(
                    tree_accounts.merkle_tree,
                    (epoch_info.epoch, processor_clone.clone()),
                );
                processor_clone
                    .lock()
                    .await
                    .update_epoch(epoch_info.epoch, epoch_info.phases.clone());
            }
            return Ok(processor_clone);
        }

        // No existing processor - create new one
        let batch_context = self.build_batch_context(
            epoch_info,
            tree_accounts,
            None,
            None,
            None,
            self.address_lookup_tables.clone(),
        );
        let processor = Arc::new(Mutex::new(
            QueueProcessor::new(batch_context, AddressTreeStrategy).await?,
        ));

        // Cache the zkp_batch_size for early filtering of queue updates
        let batch_size = processor.lock().await.zkp_batch_size();
        self.zkp_batch_sizes
            .insert(tree_accounts.merkle_tree, batch_size);

        self.address_processors.insert(
            tree_accounts.merkle_tree,
            (epoch_info.epoch, processor.clone()),
        );
        Ok(processor)
    }

    async fn process_v2(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        consecutive_eligibility_end: u64,
    ) -> std::result::Result<ProcessingResult, ForesterError> {
        match tree_accounts.tree_type {
            TreeType::StateV2 => {
                let processor = self
                    .get_or_create_state_processor(epoch_info, tree_accounts)
                    .await?;

                let cache = self
                    .proof_caches
                    .entry(tree_accounts.merkle_tree)
                    .or_insert_with(|| Arc::new(SharedProofCache::new(tree_accounts.merkle_tree)))
                    .clone();

                {
                    let mut proc = processor.lock().await;
                    proc.update_eligibility(consecutive_eligibility_end);
                    proc.set_proof_cache(cache);
                }

                let mut proc = processor.lock().await;
                match proc.process().await {
                    Ok(res) => Ok(res),
                    Err(error) if matches!(&error, ForesterError::V2(v2_error) if v2_error.is_constraint()) =>
                    {
                        warn!(
                            event = "v2_state_constraint_error",
                            run_id = %self.run_id,
                            tree = %tree_accounts.merkle_tree,
                            error = %error,
                            "State processing hit constraint error. Dropping processor to flush cache."
                        );
                        drop(proc); // Release lock before removing
                        self.state_processors.remove(&tree_accounts.merkle_tree);
                        self.proof_caches.remove(&tree_accounts.merkle_tree);
                        Err(error)
                    }
                    Err(ForesterError::V2(v2_error)) if v2_error.is_hashchain_mismatch() => {
                        let warning_key =
                            format!("v2_state_hashchain_mismatch:{}", tree_accounts.merkle_tree);
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(15)) {
                            warn!(
                                event = "v2_state_hashchain_mismatch",
                                run_id = %self.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %v2_error,
                                "State processing hit hashchain mismatch. Clearing cache and retrying."
                            );
                        }
                        self.heartbeat.increment_v2_recoverable_error();
                        proc.clear_cache().await;
                        Ok(ProcessingResult::default())
                    }
                    Err(e) => {
                        let warning_key =
                            format!("v2_state_process_failed:{}", tree_accounts.merkle_tree);
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(10)) {
                            warn!(
                                event = "v2_state_process_failed_retrying",
                                run_id = %self.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %e,
                                "Failed to process state queue. Will retry next tick without dropping processor."
                            );
                        }
                        self.heartbeat.increment_v2_recoverable_error();
                        Ok(ProcessingResult::default())
                    }
                }
            }
            TreeType::AddressV2 => {
                let processor = self
                    .get_or_create_address_processor(epoch_info, tree_accounts)
                    .await?;

                let cache = self
                    .proof_caches
                    .entry(tree_accounts.merkle_tree)
                    .or_insert_with(|| Arc::new(SharedProofCache::new(tree_accounts.merkle_tree)))
                    .clone();

                {
                    let mut proc = processor.lock().await;
                    proc.update_eligibility(consecutive_eligibility_end);
                    proc.set_proof_cache(cache);
                }

                let mut proc = processor.lock().await;
                match proc.process().await {
                    Ok(res) => Ok(res),
                    Err(error) if matches!(&error, ForesterError::V2(v2_error) if v2_error.is_constraint()) =>
                    {
                        warn!(
                            event = "v2_address_constraint_error",
                            run_id = %self.run_id,
                            tree = %tree_accounts.merkle_tree,
                            error = %error,
                            "Address processing hit constraint error. Dropping processor to flush cache."
                        );
                        drop(proc);
                        self.address_processors.remove(&tree_accounts.merkle_tree);
                        self.proof_caches.remove(&tree_accounts.merkle_tree);
                        Err(error)
                    }
                    Err(ForesterError::V2(v2_error)) if v2_error.is_hashchain_mismatch() => {
                        let warning_key = format!(
                            "v2_address_hashchain_mismatch:{}",
                            tree_accounts.merkle_tree
                        );
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(15)) {
                            warn!(
                                event = "v2_address_hashchain_mismatch",
                                run_id = %self.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %v2_error,
                                "Address processing hit hashchain mismatch. Clearing cache and retrying."
                            );
                        }
                        self.heartbeat.increment_v2_recoverable_error();
                        proc.clear_cache().await;
                        Ok(ProcessingResult::default())
                    }
                    Err(e) => {
                        let warning_key =
                            format!("v2_address_process_failed:{}", tree_accounts.merkle_tree);
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(10)) {
                            warn!(
                                event = "v2_address_process_failed_retrying",
                                run_id = %self.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %e,
                                "Failed to process address queue. Will retry next tick without dropping processor."
                            );
                        }
                        self.heartbeat.increment_v2_recoverable_error();
                        Ok(ProcessingResult::default())
                    }
                }
            }
            _ => {
                warn!(
                    event = "v2_unsupported_tree_type",
                    run_id = %self.run_id,
                    tree_type = ?tree_accounts.tree_type,
                    "Unsupported tree type for V2 processing"
                );
                Ok(ProcessingResult::default())
            }
        }
    }

    async fn update_metrics_and_counts(
        &self,
        epoch_num: u64,
        items_processed: usize,
        duration: Duration,
    ) {
        if items_processed > 0 {
            trace!(
                "{} items processed in this iteration, duration: {:?}",
                items_processed,
                duration
            );
            queue_metric_update(epoch_num, items_processed, duration);
            self.increment_processed_items_count(epoch_num, items_processed)
                .await;
            self.heartbeat.add_items_processed(items_processed);
        }
    }

    async fn prewarm_all_trees_during_wait(
        &self,
        epoch_info: &ForesterEpochInfo,
        deadline_slot: u64,
    ) {
        let current_slot = self.slot_tracker.estimated_current_slot();
        let slots_until_active = deadline_slot.saturating_sub(current_slot);

        let trees = self.trees.lock().await;
        let total_v2_state = trees
            .iter()
            .filter(|t| matches!(t.tree_type, TreeType::StateV2))
            .count();
        let v2_state_trees: Vec<_> = trees
            .iter()
            .filter(|t| {
                matches!(t.tree_type, TreeType::StateV2)
                    && !should_skip_tree(&self.config, &t.tree_type)
            })
            .cloned()
            .collect();
        let skipped_count = total_v2_state - v2_state_trees.len();
        drop(trees);

        if v2_state_trees.is_empty() {
            if skipped_count > 0 {
                info!(
                    event = "prewarm_skipped_all_trees_filtered",
                    run_id = %self.run_id,
                    skipped_trees = skipped_count,
                    "No trees to pre-warm; all StateV2 trees skipped by config"
                );
            }
            return;
        }

        if slots_until_active < 15 {
            info!(
                event = "prewarm_skipped_not_enough_time",
                run_id = %self.run_id,
                slots_until_active,
                min_required_slots = 15,
                "Skipping pre-warming; not enough slots until active phase"
            );
            return;
        }

        let prewarm_futures: Vec<_> = v2_state_trees
            .iter()
            .map(|tree_accounts| {
                let tree_pubkey = tree_accounts.merkle_tree;
                let epoch_info = epoch_info.clone();
                let tree_accounts = *tree_accounts;
                let self_clone = self.clone();

                async move {
                    let cache = self_clone
                        .proof_caches
                        .entry(tree_pubkey)
                        .or_insert_with(|| Arc::new(SharedProofCache::new(tree_pubkey)))
                        .clone();

                    let cache_len = cache.len().await;
                    if cache_len > 0 && !cache.is_warming().await {
                        let mut rpc = match self_clone.rpc_pool.get_connection().await {
                            Ok(r) => r,
                            Err(e) => {
                                warn!(
                                    event = "prewarm_cache_validation_rpc_failed",
                                    run_id = %self_clone.run_id,
                                    tree = %tree_pubkey,
                                    error = ?e,
                                    "Failed to get RPC for cache validation"
                                );
                                return;
                            }
                        };
                        if let Ok(current_root) =
                            self_clone.fetch_current_root(&mut *rpc, &tree_accounts).await
                        {
                            info!(
                                event = "prewarm_skipped_cache_already_warm",
                                run_id = %self_clone.run_id,
                                tree = %tree_pubkey,
                                cached_proofs = cache_len,
                                root_prefix = ?&current_root[..4],
                                "Tree already has cached proofs from previous epoch; skipping pre-warm"
                            );
                            return;
                        }
                    }

                    let processor = match self_clone
                        .get_or_create_state_processor(&epoch_info.epoch, &tree_accounts)
                        .await
                    {
                        Ok(p) => p,
                        Err(e) => {
                            warn!(
                                event = "prewarm_processor_create_failed",
                                run_id = %self_clone.run_id,
                                tree = %tree_pubkey,
                                error = ?e,
                                "Failed to create processor for pre-warming tree"
                            );
                            return;
                        }
                    };

                    const PREWARM_MAX_BATCHES: usize = 4;
                    let mut p = processor.lock().await;
                    match p
                        .prewarm_from_indexer(
                            cache.clone(),
                            light_compressed_account::QueueType::OutputStateV2,
                            PREWARM_MAX_BATCHES,
                        )
                        .await
                    {
                        Ok(result) => {
                            if result.items_processed > 0 {
                                info!(
                                    event = "prewarm_tree_completed",
                                    run_id = %self_clone.run_id,
                                    tree = %tree_pubkey,
                                    items = result.items_processed,
                                    "Pre-warmed items for tree during wait"
                                );
                                self_clone
                                    .add_processing_metrics(epoch_info.epoch.epoch, result.metrics)
                                    .await;
                            }
                        }
                        Err(e) => {
                            debug!(
                                "Pre-warming from indexer failed for tree {}: {:?}",
                                tree_pubkey, e
                            );
                            cache.clear().await;
                        }
                    }
                }
            })
            .collect();

        let timeout_slots = slots_until_active.saturating_sub(5);
        let timeout_duration =
            (slot_duration() * timeout_slots as u32).min(Duration::from_secs(30));

        info!(
            event = "prewarm_started",
            run_id = %self.run_id,
            trees = v2_state_trees.len(),
            skipped_trees = skipped_count,
            timeout_ms = timeout_duration.as_millis() as u64,
            "Starting pre-warming"
        );

        match tokio::time::timeout(timeout_duration, futures::future::join_all(prewarm_futures))
            .await
        {
            Ok(_) => {
                info!(
                    event = "prewarm_completed",
                    run_id = %self.run_id,
                    trees = v2_state_trees.len(),
                    "Completed pre-warming for all trees"
                );
            }
            Err(_) => {
                info!(
                    event = "prewarm_timed_out",
                    run_id = %self.run_id,
                    timeout_ms = timeout_duration.as_millis() as u64,
                    "Pre-warming timed out"
                );
            }
        }
    }

    async fn try_send_cached_proofs(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        consecutive_eligibility_end: u64,
    ) -> Result<Option<usize>> {
        let tree_pubkey = tree_accounts.merkle_tree;

        // Check eligibility window before attempting to send cached proofs
        let current_slot = self.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                event = "cached_proofs_skipped_outside_eligibility",
                run_id = %self.run_id,
                tree = %tree_pubkey,
                current_slot,
                eligibility_end_slot = consecutive_eligibility_end,
                "Skipping cached proof send because eligibility window has ended"
            );
            return Ok(None);
        }

        let Some(confirmation_deadline) = scheduled_confirmation_deadline(
            consecutive_eligibility_end.saturating_sub(current_slot),
        ) else {
            debug!(
                event = "cached_proofs_skipped_confirmation_budget_exhausted",
                run_id = %self.run_id,
                tree = %tree_pubkey,
                current_slot,
                eligibility_end_slot = consecutive_eligibility_end,
                "Skipping cached proofs because not enough eligible slots remain for confirmation"
            );
            return Ok(None);
        };

        let cache = match self.proof_caches.get(&tree_pubkey) {
            Some(c) => c.clone(),
            None => return Ok(None),
        };

        if cache.is_warming().await {
            debug!(
                event = "cached_proofs_skipped_cache_warming",
                run_id = %self.run_id,
                tree = %tree_pubkey,
                "Skipping cached proofs because cache is still warming"
            );
            return Ok(None);
        }

        let mut rpc = self.rpc_pool.get_connection().await?;
        let current_root = match self.fetch_current_root(&mut *rpc, tree_accounts).await {
            Ok(root) => root,
            Err(e) => {
                warn!(
                    event = "cached_proofs_root_fetch_failed",
                    run_id = %self.run_id,
                    tree = %tree_pubkey,
                    error = ?e,
                    "Failed to fetch current root for tree"
                );
                return Ok(None);
            }
        };

        let cached_proofs = match cache.take_if_valid(&current_root).await {
            Some(proofs) => proofs,
            None => {
                debug!(
                    event = "cached_proofs_not_available",
                    run_id = %self.run_id,
                    tree = %tree_pubkey,
                    root_prefix = ?&current_root[..4],
                    "No valid cached proofs for tree"
                );
                return Ok(None);
            }
        };

        if cached_proofs.is_empty() {
            return Ok(Some(0));
        }

        info!(
            event = "cached_proofs_send_started",
            run_id = %self.run_id,
            tree = %tree_pubkey,
            proofs = cached_proofs.len(),
            root_prefix = ?&current_root[..4],
            "Sending cached proofs for tree"
        );

        let items_sent = self
            .send_cached_proofs_as_transactions(
                epoch_info,
                tree_accounts,
                cached_proofs,
                confirmation_deadline,
            )
            .await?;

        Ok(Some(items_sent))
    }

    async fn fetch_current_root(
        &self,
        rpc: &mut impl Rpc,
        tree_accounts: &TreeAccounts,
    ) -> Result<[u8; 32]> {
        use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;

        let mut account = rpc
            .get_account(tree_accounts.merkle_tree)
            .await?
            .ok_or_else(|| anyhow!("Tree account not found: {}", tree_accounts.merkle_tree))?;

        let tree = match tree_accounts.tree_type {
            TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
                &mut account.data,
                &tree_accounts.merkle_tree.into(),
            )?,
            TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
                &mut account.data,
                &tree_accounts.merkle_tree.into(),
            )?,
            _ => return Err(anyhow!("Unsupported tree type for root fetch")),
        };

        let root = tree.root_history.last().copied().unwrap_or([0u8; 32]);
        Ok(root)
    }

    async fn send_cached_proofs_as_transactions(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        cached_proofs: Vec<crate::processor::v2::CachedProof>,
        confirmation_deadline: Instant,
    ) -> Result<usize> {
        let mut total_items = 0;
        let authority = self.config.payer_keypair.pubkey();
        let derivation = self.config.derivation_pubkey;

        const PROOFS_PER_TX: usize = 4;
        for chunk in cached_proofs.chunks(PROOFS_PER_TX) {
            let mut instructions = Vec::new();
            let mut chunk_items = 0;

            for proof in chunk {
                match &proof.instruction {
                    BatchInstruction::Append(data) => {
                        for d in data {
                            let serialized = d
                                .try_to_vec()
                                .with_context(|| "Failed to serialize batch append payload")?;
                            instructions.push(create_batch_append_instruction(
                                authority,
                                derivation,
                                tree_accounts.merkle_tree,
                                tree_accounts.queue,
                                epoch_info.epoch,
                                serialized,
                            ));
                        }
                    }
                    BatchInstruction::Nullify(data) => {
                        for d in data {
                            let serialized = d
                                .try_to_vec()
                                .with_context(|| "Failed to serialize batch nullify payload")?;
                            instructions.push(create_batch_nullify_instruction(
                                authority,
                                derivation,
                                tree_accounts.merkle_tree,
                                epoch_info.epoch,
                                serialized,
                            ));
                        }
                    }
                    BatchInstruction::AddressAppend(data) => {
                        for d in data {
                            let serialized = d.try_to_vec().with_context(|| {
                                "Failed to serialize batch address append payload"
                            })?;
                            instructions.push(create_batch_update_address_tree_instruction(
                                authority,
                                derivation,
                                tree_accounts.merkle_tree,
                                epoch_info.epoch,
                                serialized,
                            ));
                        }
                    }
                }
                chunk_items += proof.items;
            }

            if !instructions.is_empty() {
                let mut rpc = self.rpc_pool.get_connection().await?;
                let priority_fee = self
                    .resolve_tree_priority_fee(&*rpc, epoch_info.epoch, tree_accounts)
                    .await?;
                let instruction_count = instructions.len();
                let payer = self.config.payer_keypair.pubkey();
                let signers = [&self.config.payer_keypair];
                match send_smart_transaction(
                    &mut *rpc,
                    SendSmartTransactionConfig {
                        instructions,
                        payer: &payer,
                        signers: &signers,
                        address_lookup_tables: &self.address_lookup_tables,
                        compute_budget: ComputeBudgetConfig {
                            compute_unit_price: priority_fee,
                            compute_unit_limit: Some(self.config.transaction_config.cu_limit),
                        },
                        confirmation: Some(self.confirmation_config()),
                        confirmation_deadline: Some(confirmation_deadline),
                    },
                )
                .await
                .map_err(RpcError::from)
                {
                    Ok(sig) => {
                        info!(
                            event = "cached_proofs_tx_sent",
                            run_id = %self.run_id,
                            signature = %sig,
                            instruction_count,
                            "Sent cached proofs transaction"
                        );
                        total_items += chunk_items;
                    }
                    Err(e) => {
                        warn!(
                            event = "cached_proofs_tx_send_failed",
                            run_id = %self.run_id,
                            error = ?e,
                            "Failed to send cached proofs transaction"
                        );
                    }
                }
            }
        }

        Ok(total_items)
    }

    async fn rollover_if_needed(&self, tree_account: &TreeAccounts) -> Result<()> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        if is_tree_ready_for_rollover(&mut *rpc, tree_account.merkle_tree, tree_account.tree_type)
            .await?
        {
            info!(
                event = "tree_rollover_started",
                run_id = %self.run_id,
                tree = %tree_account.merkle_tree,
                tree_type = ?tree_account.tree_type,
                "Starting tree rollover"
            );
            self.perform_rollover(tree_account).await?;
        }
        Ok(())
    }

    fn is_in_active_phase(&self, slot: u64, epoch_info: &ForesterEpochInfo) -> Result<bool> {
        let current_epoch = self.protocol_config.get_current_active_epoch(slot)?;
        if current_epoch != epoch_info.epoch.epoch {
            return Ok(false);
        }

        Ok(self
            .protocol_config
            .is_active_phase(slot, epoch_info.epoch.epoch)
            .is_ok())
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch
    ))]
    async fn wait_for_report_work_phase(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        info!(
            event = "wait_for_report_work_phase",
            run_id = %self.run_id,
            epoch = epoch_info.epoch.epoch,
            report_work_start_slot = epoch_info.epoch.phases.report_work.start,
            "Waiting for report work phase"
        );
        let mut rpc = self.rpc_pool.get_connection().await?;
        let report_work_start_slot = epoch_info.epoch.phases.report_work.start;
        wait_until_slot_reached(&mut *rpc, &self.slot_tracker, report_work_start_slot).await?;

        info!(
            event = "report_work_phase_ready",
            run_id = %self.run_id,
            epoch = epoch_info.epoch.epoch,
            "Finished waiting for report work phase"
        );
        Ok(())
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch
    ))]
    async fn send_work_report(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        let report = WorkReport {
            epoch: epoch_info.epoch.epoch,
            processed_items: self.get_processed_items_count(epoch_info.epoch.epoch).await,
            metrics: self.get_processing_metrics(epoch_info.epoch.epoch).await,
        };

        info!(
            event = "work_report_sent_to_channel",
            run_id = %self.run_id,
            epoch = report.epoch,
            items = report.processed_items,
            total_circuit_inputs_ms = report.metrics.total_circuit_inputs().as_millis() as u64,
            total_proof_generation_ms = report.metrics.total_proof_generation().as_millis() as u64,
            total_round_trip_ms = report.metrics.total_round_trip().as_millis() as u64,
            tx_sending_ms = report.metrics.tx_sending_duration.as_millis() as u64,
            "Sending work report to channel"
        );

        self.work_report_sender
            .send(report)
            .await
            .map_err(|e| ChannelError::WorkReportSend {
                epoch: report.epoch,
                error: e.to_string(),
            })?;
        self.heartbeat.increment_work_report_sent();

        Ok(())
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch
    ))]
    async fn report_work_onchain(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        info!(
            event = "work_report_onchain_started",
            run_id = %self.run_id,
            epoch = epoch_info.epoch.epoch,
            "Reporting work on-chain"
        );
        let mut rpc = LightClient::new(LightClientConfig {
            url: self.config.external_services.rpc_url.to_string(),
            photon_url: self.config.external_services.indexer_url.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::processed()),
            fetch_active_tree: false,
        })
        .await?;

        let forester_epoch_pda_pubkey = get_forester_epoch_pda_from_authority(
            &self.config.derivation_pubkey,
            epoch_info.epoch.epoch,
        )
        .0;
        if let Some(forester_epoch_pda) = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?
        {
            if forester_epoch_pda.has_reported_work {
                return Ok(());
            }
        }

        let forester_epoch_pda = &epoch_info.forester_epoch_pda;
        if forester_epoch_pda.has_reported_work {
            return Ok(());
        }

        let ix = create_report_work_instruction(
            &self.config.payer_keypair.pubkey(),
            &self.config.derivation_pubkey,
            epoch_info.epoch.epoch,
        );

        let priority_fee = self
            .resolve_epoch_priority_fee(&rpc, epoch_info.epoch.epoch)
            .await?;
        let payer = self.config.payer_keypair.pubkey();
        let signers = [&self.config.payer_keypair];
        match send_smart_transaction(
            &mut rpc,
            SendSmartTransactionConfig {
                instructions: vec![ix],
                payer: &payer,
                signers: &signers,
                address_lookup_tables: &self.address_lookup_tables,
                compute_budget: ComputeBudgetConfig {
                    compute_unit_price: priority_fee,
                    compute_unit_limit: Some(self.config.transaction_config.cu_limit),
                },
                confirmation: Some(self.confirmation_config()),
                confirmation_deadline: None,
            },
        )
        .await
        .map_err(RpcError::from)
        {
            Ok(_) => {
                info!(
                    event = "work_report_onchain_succeeded",
                    run_id = %self.run_id,
                    epoch = epoch_info.epoch.epoch,
                    "Work reported on-chain"
                );
            }
            Err(e) => {
                if rpc_is_already_processed(&e) {
                    info!(
                        event = "work_report_onchain_already_reported",
                        run_id = %self.run_id,
                        epoch = epoch_info.epoch.epoch,
                        "Work already reported on-chain for epoch"
                    );
                    return Ok(());
                }
                if let RpcError::ClientError(client_error) = &e {
                    if let Some(TransactionError::InstructionError(
                        _,
                        InstructionError::Custom(error_code),
                    )) = client_error.get_transaction_error()
                    {
                        return WorkReportError::from_registry_error(
                            error_code,
                            epoch_info.epoch.epoch,
                        )
                        .map_err(|e| anyhow::Error::from(ForesterError::from(e)));
                    }
                }
                return Err(anyhow::Error::from(WorkReportError::Transaction(Box::new(
                    e,
                ))));
            }
        }

        Ok(())
    }

    async fn perform_rollover(&self, tree_account: &TreeAccounts) -> Result<()> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        let (_, current_epoch) = self.get_current_slot_and_epoch().await?;

        let result = match tree_account.tree_type {
            TreeType::AddressV1 => {
                let new_nullifier_queue_keypair = Keypair::new();
                let new_merkle_tree_keypair = Keypair::new();

                let rollover_signature = perform_address_merkle_tree_rollover(
                    &self.config.payer_keypair,
                    &self.config.derivation_pubkey,
                    &mut *rpc,
                    &new_nullifier_queue_keypair,
                    &new_merkle_tree_keypair,
                    &tree_account.merkle_tree,
                    &tree_account.queue,
                    current_epoch,
                )
                .await?;

                info!(
                    event = "address_tree_rollover_succeeded",
                    run_id = %self.run_id,
                    tree = %tree_account.merkle_tree,
                    signature = %rollover_signature,
                    "Address tree rollover succeeded"
                );
                Ok(())
            }
            TreeType::StateV1 => {
                let new_nullifier_queue_keypair = Keypair::new();
                let new_merkle_tree_keypair = Keypair::new();
                let new_cpi_signature_keypair = Keypair::new();

                let rollover_signature = perform_state_merkle_tree_rollover_forester(
                    &self.config.payer_keypair,
                    &self.config.derivation_pubkey,
                    &mut *rpc,
                    &new_nullifier_queue_keypair,
                    &new_merkle_tree_keypair,
                    &new_cpi_signature_keypair,
                    &tree_account.merkle_tree,
                    &tree_account.queue,
                    &Pubkey::default(),
                    current_epoch,
                )
                .await?;

                info!(
                    event = "state_tree_rollover_succeeded",
                    run_id = %self.run_id,
                    tree = %tree_account.merkle_tree,
                    signature = %rollover_signature,
                    "State tree rollover succeeded"
                );

                Ok(())
            }
            _ => Err(ForesterError::InvalidTreeType(tree_account.tree_type)),
        };

        match result {
            Ok(_) => debug!(
                "{:?} tree rollover completed successfully",
                tree_account.tree_type
            ),
            Err(e) => warn!("{:?} tree rollover failed: {:?}", tree_account.tree_type, e),
        }
        Ok(())
    }
}

fn should_skip_tree(config: &ForesterConfig, tree_type: &TreeType) -> bool {
    match tree_type {
        TreeType::AddressV1 => config.general_config.skip_v1_address_trees,
        TreeType::AddressV2 => config.general_config.skip_v2_address_trees,
        TreeType::StateV1 => config.general_config.skip_v1_state_trees,
        TreeType::StateV2 => config.general_config.skip_v2_state_trees,
        TreeType::Unknown => false, // Never skip compression tree
    }
}

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
            let delta = current.delta_since(&previous);
            previous = current;

            info!(
                event = "service_heartbeat",
                run_id = %run_id,
                slot,
                epoch = epoch_value,
                epoch_known,
                cycle_delta = delta.active_cycles,
                tree_tasks_delta = delta.tree_tasks_spawned,
                queues_started_delta = delta.queues_started,
                queues_finished_delta = delta.queues_finished,
                items_processed_delta = delta.items_processed,
                work_reports_delta = delta.work_reports_sent,
                recoverable_v2_errors_delta = delta.v2_recoverable_errors,
                cycle_total = current.active_cycles,
                items_processed_total = current.items_processed,
                "Forester heartbeat"
            );
        }
    })
}

#[instrument(
    level = "info",
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
#[allow(clippy::too_many_arguments)]
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
                                                                    warn!(
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
                                                    warn!(
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
                                    warn!(
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
            trace!("Fetched initial trees: {:?}", trees);

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
                        let lut = load_lookup_table_async(&*rpc, lut_address).await
                            .map_err(|e| {
                                error!(
                                    event = "lookup_table_load_failed",
                                    run_id = %run_id_for_logs,
                                    lookup_table = %lut_address,
                                    error = %e,
                                    "Failed to load lookup table"
                                );
                                e
                            })?;
                        info!(
                            event = "lookup_table_loaded",
                            run_id = %run_id_for_logs,
                            lookup_table = %lut_address,
                            address_count = lut.addresses.len(),
                            "Loaded lookup table"
                        );
                        Arc::new(vec![lut])
                    } else {
                        debug!("No lookup table address configured. Using v1 state single nullify transactions.");
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

                        let result = tokio::select! {
                            result = epoch_manager.run() => result,
                            _ = shutdown => {
                                info!(
                                    event = "shutdown_received",
                                    run_id = %run_id_for_logs,
                                    phase = "service_run",
                                    "Received shutdown signal. Stopping the service."
                                );
                                Ok(())
                            }
                        };

                        return result;
                    }
                    Err(e) => {
                        warn!(
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
    lookup_table_address: Pubkey,
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

    use super::*;
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
        // Skip V1 state and V2 address
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
}
