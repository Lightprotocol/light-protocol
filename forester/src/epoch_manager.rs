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
    sync::{broadcast, broadcast::error::RecvError, mpsc, oneshot, Mutex},
    task::JoinHandle,
    time::{sleep, Instant, MissedTickBehavior},
};
use tracing::{debug, error, info, info_span, instrument, trace, warn};

use crate::{
    compressible::{traits::CompressibleTracker, CTokenAccountTracker, CTokenCompressor},
    errors::{
        ChannelError, ForesterError, InitializationError, RegistrationError, WorkReportError,
    },
    logging::{should_emit_rate_limited_warning, ServiceHeartbeat},
    metrics::{push_metrics, queue_metric_update, update_forester_sol_balance},
    pagerduty::send_pagerduty_alert,
    processor::{
        tx_cache::ProcessedHashCache,
        v1::{
            config::{BuildTransactionBatchConfig, SendBatchedTransactionsConfig},
            send_transaction::send_batched_transactions,
            tx_builder::EpochManagerTransactions,
        },
        v2::{
            errors::V2Error,
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
    tree_data_sync::{fetch_protocol_group_authority, fetch_trees},
    ForesterConfig, ForesterEpochInfo, Result,
};

fn is_v2_error(err: &anyhow::Error, predicate: impl FnOnce(&V2Error) -> bool) -> bool {
    err.downcast_ref::<V2Error>().is_some_and(predicate)
}

type StateBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>)>>;
type AddressBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>)>>;
type ProcessorInitLockMap = Arc<DashMap<Pubkey, Arc<Mutex<()>>>>;

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
    new_tree_sender: broadcast::Sender<TreeAccounts>,
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
            new_tree_sender: self.new_tree_sender.clone(),
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
        new_tree_sender: broadcast::Sender<TreeAccounts>,
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
            new_tree_sender,
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

        let new_tree_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move { self_clone.handle_new_trees().await }
        });

        let balance_check_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move { self_clone.check_sol_balance_periodically().await }
        });

        let _guard = scopeguard::guard(
            (
                current_previous_handle,
                new_tree_handle,
                balance_check_handle,
            ),
            |(h2, h3, h4)| {
                info!(
                    event = "background_tasks_aborting",
                    run_id = %self.run_id,
                    "Aborting EpochManager background tasks"
                );
                h2.abort();
                h3.abort();
                h4.abort();
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
                                    if let Some(ForesterError::Registration(
                                        RegistrationError::FinalizeRegistrationPhaseEnded {
                                            epoch,
                                            current_slot,
                                            active_phase_end_slot,
                                        },
                                    )) = e.downcast_ref::<ForesterError>()
                                    {
                                        debug!(
                                            event = "epoch_processing_skipped_finalize_registration_phase_ended",
                                            run_id = %self_clone.run_id,
                                            epoch = *epoch,
                                            current_slot = *current_slot,
                                            active_phase_end_slot = *active_phase_end_slot,
                                            "Skipping epoch processing because FinalizeRegistration is no longer possible"
                                        );
                                    } else {
                                        error!(
                                            event = "epoch_processing_failed",
                                            run_id = %self_clone.run_id,
                                            epoch,
                                            error = ?e,
                                            "Error processing epoch"
                                        );
                                    }
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

    async fn handle_new_trees(self: Arc<Self>) -> Result<()> {
        let mut receiver = self.new_tree_sender.subscribe();
        loop {
            match receiver.recv().await {
                Ok(new_tree) => {
                    info!(
                        event = "new_tree_received",
                        run_id = %self.run_id,
                        tree = %new_tree.merkle_tree,
                        tree_type = ?new_tree.tree_type,
                        "Received new tree"
                    );
                    if let Err(e) = self.add_new_tree(new_tree).await {
                        error!(
                            event = "new_tree_add_failed",
                            run_id = %self.run_id,
                            error = ?e,
                            "Failed to add new tree"
                        );
                        // Continue processing other trees instead of crashing
                    }
                }
                Err(e) => match e {
                    RecvError::Lagged(lag) => {
                        warn!(
                            event = "new_tree_receiver_lagged",
                            run_id = %self.run_id,
                            lag, "Lagged while receiving new trees"
                        );
                    }
                    RecvError::Closed => {
                        info!(
                            event = "new_tree_receiver_closed",
                            run_id = %self.run_id,
                            "New tree receiver closed"
                        );
                        break;
                    }
                },
            }
        }
        Ok(())
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
            match self.recover_registration_info(current_epoch).await {
                Ok(mut epoch_info) => {
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
                                &epoch_info.forester_epoch_pda,
                                tree_schedule,
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
                Err(e) => {
                    // If not registered yet, just log debug (it's expected on first run)
                    if matches!(
                        e.downcast_ref::<RegistrationError>(),
                        Some(RegistrationError::ForesterEpochPdaNotFound { .. })
                    ) {
                        debug!("Not registered for current epoch yet, new tree will be picked up during next registration");
                    } else {
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
                    "New epoch detected"
                );
                let phases = get_epoch_phases(&self.protocol_config, current_epoch);
                if slot < phases.registration.end {
                    debug!(
                        event = "epoch_monitor_send_current_epoch",
                        run_id = %self.run_id,
                        epoch = current_epoch,
                        "Sending current epoch for processing"
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
            }

            // Find the next epoch we can register for (scan forward if needed)
            let mut target_epoch = current_epoch + 1;
            if last_epoch.is_none_or(|last| target_epoch > last) {
                // Scan forward to find an epoch whose registration is still open
                // This handles the case where we missed multiple epochs
                loop {
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
                                break;
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
                            wait_until_slot_reached(&mut *rpc, &self.slot_tracker, wait_target)
                                .await
                        {
                            error!(
                                event = "epoch_monitor_wait_for_registration_failed",
                                run_id = %self.run_id,
                                target_epoch,
                                error = ?e,
                                "Error waiting for registration phase"
                            );
                            break;
                        }

                        let current_slot = self.slot_tracker.estimated_current_slot();
                        if current_slot >= target_phases.registration.end {
                            debug!(
                                event = "epoch_monitor_registration_ended_while_waiting",
                                run_id = %self.run_id,
                                target_epoch,
                                current_slot,
                                registration_end_slot = target_phases.registration.end,
                                "Target epoch registration ended while waiting; trying next epoch"
                            );
                            target_epoch += 1;
                            continue;
                        }

                        debug!(
                            event = "epoch_monitor_send_target_epoch_after_wait",
                            run_id = %self.run_id,
                            target_epoch,
                            current_slot,
                            registration_end_slot = target_phases.registration.end,
                            "Target epoch registration phase ready; sending for processing"
                        );
                        if let Err(e) = tx.send(target_epoch).await {
                            error!(
                                event = "epoch_monitor_send_target_epoch_failed",
                                run_id = %self.run_id,
                                target_epoch,
                                error = ?e,
                                "Failed to send target epoch for processing"
                            );
                            break;
                        }
                        last_epoch = Some(target_epoch);
                        break;
                    }

                    // If we're within the registration window, send it
                    if slot < target_phases.registration.end {
                        debug!(
                            event = "epoch_monitor_send_target_epoch_window_open",
                            run_id = %self.run_id,
                            target_epoch,
                            slot,
                            registration_end_slot = target_phases.registration.end,
                            "Target epoch registration window is open; sending for processing"
                        );
                        if let Err(e) = tx.send(target_epoch).await {
                            error!(
                                event = "epoch_monitor_send_target_epoch_failed",
                                run_id = %self.run_id,
                                target_epoch,
                                error = ?e,
                                "Failed to send target epoch for processing"
                            );
                            break;
                        }
                        last_epoch = Some(target_epoch);
                        break;
                    }

                    // Registration already ended, try next epoch
                    debug!(
                        event = "epoch_monitor_target_epoch_registration_closed",
                        run_id = %self.run_id,
                        target_epoch,
                        slot,
                        registration_end_slot = target_phases.registration.end,
                        "Target epoch registration already ended; checking next epoch"
                    );
                    target_epoch += 1;
                }
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

    async fn recover_registration_info(&self, epoch: u64) -> Result<ForesterEpochInfo> {
        debug!("Recovering registration info for epoch {}", epoch);

        let forester_epoch_pda_pubkey =
            get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0;

        let existing_pda = {
            let rpc = self.rpc_pool.get_connection().await?;
            rpc.get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?
        };

        existing_pda
            .map(|pda| async move {
                self.recover_registration_info_internal(epoch, forester_epoch_pda_pubkey, pda)
                    .await
            })
            .ok_or(RegistrationError::ForesterEpochPdaNotFound {
                epoch,
                pda_address: forester_epoch_pda_pubkey,
            })?
            .await
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

        // Only process current epoch if we can still register or are already registered
        // If registration has ended and we haven't registered, skip it to avoid errors
        if slot < current_phases.registration.end {
            debug!(
                "Processing current epoch: {} (registration still open)",
                current_epoch
            );
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
        } else {
            // Check if we're already registered for this epoch
            let forester_epoch_pda_pubkey = get_forester_epoch_pda_from_authority(
                &self.config.derivation_pubkey,
                current_epoch,
            )
            .0;
            match self.rpc_pool.get_connection().await {
                Ok(rpc) => {
                    if let Ok(Some(_)) = rpc
                        .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                        .await
                    {
                        debug!(
                            "Processing current epoch: {} (already registered)",
                            current_epoch
                        );
                        if let Err(e) = tx.send(current_epoch).await {
                            error!(
                                event = "initial_epoch_send_current_registered_failed",
                                run_id = %self.run_id,
                                epoch = current_epoch,
                                error = ?e,
                                "Failed to send current epoch for processing"
                            );
                            return Ok(()); // Channel closed, exit gracefully
                        }
                    } else {
                        info!(
                            event = "skip_current_epoch_registration_closed",
                            run_id = %self.run_id,
                            epoch = current_epoch,
                            registration_end_slot = current_phases.registration.end,
                            current_slot = slot,
                            "Skipping current epoch because registration has ended"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        event = "registration_check_rpc_failed",
                        run_id = %self.run_id,
                        error = ?e,
                        "Failed to get RPC connection to check registration, skipping"
                    );
                }
            }
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

        // Attempt to recover registration info
        debug!("Recovering registration info for epoch {}", epoch);
        let mut registration_info = match self.recover_registration_info(epoch).await {
            Ok(info) => info,
            Err(e) => {
                // Check if it's the expected "not found" error
                if matches!(
                    e.downcast_ref::<RegistrationError>(),
                    Some(RegistrationError::ForesterEpochPdaNotFound { .. })
                ) {
                    debug!(
                        "No existing registration found for epoch {}, will register fresh",
                        epoch
                    );
                } else {
                    warn!(
                        event = "recover_registration_info_failed",
                        run_id = %self.run_id,
                        epoch,
                        error = ?e,
                        "Failed to recover registration info"
                    );
                }
                // Attempt to register
                match self
                    .register_for_epoch_with_retry(epoch, 100, Duration::from_millis(1000))
                    .await
                {
                    Ok(info) => info,
                    Err(ForesterError::Registration(
                        RegistrationError::RegistrationPhaseEnded {
                            epoch: failed_epoch,
                            current_slot,
                            registration_end,
                        },
                    )) => {
                        let next_epoch = failed_epoch + 1;
                        let next_phases = get_epoch_phases(&self.protocol_config, next_epoch);
                        let slots_to_wait =
                            next_phases.registration.start.saturating_sub(current_slot);

                        info!(
                            event = "registration_window_missed",
                            run_id = %self.run_id,
                            failed_epoch,
                            registration_end_slot = registration_end,
                            current_slot,
                            next_epoch,
                            next_registration_start_slot = next_phases.registration.start,
                            slots_to_wait,
                            "Too late to register for requested epoch; next epoch will be used"
                        );
                        return Ok(());
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        };
        debug!("Recovered registration info for epoch {}", epoch);

        // Wait for the active phase
        registration_info = self.wait_for_active_phase(&registration_info).await?;

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

        // Check if it's already too late to register
        if slot >= phases.registration.end {
            return Err(RegistrationError::RegistrationPhaseEnded {
                epoch,
                current_slot: slot,
                registration_end: phases.registration.end,
            }
            .into());
        }

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
            match self.register_for_epoch(epoch).await {
                Ok(registration_info) => return Ok(registration_info),
                Err(e) => {
                    if let Some(RegistrationError::RegistrationPhaseEnded {
                        epoch: ended_epoch,
                        current_slot,
                        registration_end,
                    }) = e.downcast_ref::<RegistrationError>()
                    {
                        warn!(
                            event = "registration_attempt_non_retryable",
                            run_id = %self.run_id,
                            epoch,
                            attempt = attempt + 1,
                            max_attempts = max_retries,
                            error = ?e,
                            "Registration phase ended; stopping retries for this epoch"
                        );
                        return Err(ForesterError::Registration(
                            RegistrationError::RegistrationPhaseEnded {
                                epoch: *ended_epoch,
                                current_slot: *current_slot,
                                registration_end: *registration_end,
                            },
                        ));
                    }

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

        if slot >= phases.registration.start && slot < phases.registration.end {
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
        } else if slot < phases.registration.start {
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
        } else {
            warn!(
                event = "registration_too_late",
                run_id = %self.run_id,
                epoch,
                current_slot = slot,
                registration_end_slot = phases.registration.end,
                "Too late to register for epoch"
            );
            Err(RegistrationError::RegistrationPhaseEnded {
                epoch,
                current_slot: slot,
                registration_end: phases.registration.end,
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
    ) -> Result<ForesterEpochInfo> {
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
                    return Err(RegistrationError::FinalizeRegistrationPhaseEnded {
                        epoch: epoch_info.epoch.epoch,
                        current_slot,
                        active_phase_end_slot: epoch_info.epoch.phases.active.end,
                    }
                    .into());
                }

                // TODO: we can put this ix into every tx of the first batch of the current active phase
                let ix = create_finalize_registration_instruction(
                    &self.config.payer_keypair.pubkey(),
                    &self.config.derivation_pubkey,
                    epoch_info.epoch.epoch,
                );
                rpc.create_and_send_transaction(
                    &[ix],
                    &self.config.payer_keypair.pubkey(),
                    &[&self.config.payer_keypair],
                )
                .await?;
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
            )?;
            epoch_info.trees.insert(0, tree_schedule);
            debug!("Added compression tree to epoch {}", epoch_info.epoch.epoch);
        }

        info!(
            event = "active_phase_ready",
            run_id = %self.run_id,
            epoch = epoch_info.epoch.epoch,
            "Finished waiting for active phase"
        );
        Ok(epoch_info)
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
        let epoch_info_arc = Arc::new(epoch_info.clone());

        let mut handles: Vec<JoinHandle<Result<()>>> = Vec::with_capacity(trees_to_process.len());

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
            let epoch_info_clone = epoch_info_arc.clone();

            let handle = tokio::spawn(async move {
                self_clone
                    .process_queue(
                        &epoch_info_clone.epoch,
                        &epoch_info_clone.forester_epoch_pda,
                        tree,
                    )
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
        skip(self, epoch_info, epoch_pda, tree_schedule),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree_schedule.tree_accounts.merkle_tree)
    )]
    pub async fn process_queue(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        mut tree_schedule: TreeForesterSchedule,
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
                            epoch_pda,
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
                            epoch_pda,
                            &tree_schedule.tree_accounts,
                            &light_slot_details,
                            consecutive_end,
                        )
                        .await
                    }
                };

                match result {
                    Ok(_) => {
                        trace!(
                            "Successfully processed light slot {:?}",
                            light_slot_details.slot
                        );
                    }
                    Err(e) => {
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
    ) -> Result<()> {
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

            let sleep_duration_ms = if items_processed_this_iteration > 0 {
                self.config.general_config.sleep_after_processing_ms
            } else {
                self.config.general_config.sleep_when_idle_ms
            };

            tokio::time::sleep(Duration::from_millis(sleep_duration_ms)).await;
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
    ) -> Result<()> {
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
    ) -> Result<usize> {
        match tree_accounts.tree_type {
            TreeType::Unknown => {
                self.dispatch_compression(
                    epoch_info,
                    forester_slot_details,
                    consecutive_eligibility_end,
                )
                .await
            }
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
                    return Err((batch_idx, batch.len(), anyhow!("Cancelled")));
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
            .dispatch_pda_compression(consecutive_eligibility_end)
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
            .dispatch_mint_compression(consecutive_eligibility_end)
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

    async fn dispatch_pda_compression(&self, consecutive_eligibility_end: u64) -> Result<usize> {
        let pda_tracker = match &self.pda_tracker {
            Some(tracker) => tracker,
            None => return Ok(0),
        };

        let config = match &self.config.compressible_config {
            Some(cfg) => cfg,
            None => return Ok(0),
        };

        if config.pda_programs.is_empty() {
            return Ok(0);
        }

        let current_slot = self.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                "Skipping PDA compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                current_slot, consecutive_eligibility_end
            );
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
                    Ok((sig, account_state)) => {
                        debug!(
                            "Compressed PDA {} for program {}: {}",
                            account_state.pubkey, program_config.program_id, sig
                        );
                        total_compressed += 1;
                    }
                    Err((account_state, e)) => {
                        if e.to_string() != "Cancelled" {
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
        }

        info!(
            event = "compression_pda_completed",
            run_id = %self.run_id,
            compressed_accounts = total_compressed,
            "Completed PDA compression"
        );
        Ok(total_compressed)
    }

    async fn dispatch_mint_compression(&self, consecutive_eligibility_end: u64) -> Result<usize> {
        let mint_tracker = match &self.mint_tracker {
            Some(tracker) => tracker,
            None => return Ok(0),
        };

        let config = match &self.config.compressible_config {
            Some(cfg) => cfg,
            None => return Ok(0),
        };

        let current_slot = self.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                "Skipping Mint compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                current_slot, consecutive_eligibility_end
            );
            return Ok(0);
        }

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
                Ok((sig, mint_state)) => {
                    debug!("Compressed Mint {}: {}", mint_state.pubkey, sig);
                    total_compressed += 1;
                }
                Err((mint_state, e)) => {
                    if e.to_string() != "Cancelled" {
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
        }

        info!(
            event = "compression_mint_completed",
            run_id = %self.run_id,
            compressed_accounts = total_compressed,
            "Completed Mint compression"
        );
        Ok(total_compressed)
    }

    async fn process_v1(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        current_solana_slot: u64,
    ) -> Result<usize> {
        let transaction_timeout_buffer = Duration::from_secs(2);
        let remaining_time_timeout = calculate_remaining_time_or_default(
            current_solana_slot,
            forester_slot_details.end_solana_slot,
            transaction_timeout_buffer,
        );

        let batched_tx_config = SendBatchedTransactionsConfig {
            num_batches: 1,
            build_transaction_batch_config: BuildTransactionBatchConfig {
                batch_size: self.config.transaction_config.legacy_ixs_per_tx as u64,
                compute_unit_price: Some(10_000), // is dynamic, sets max
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
        };

        let transaction_builder = Arc::new(EpochManagerTransactions::new(
            self.rpc_pool.clone(),
            epoch_info.epoch,
            self.tx_cache.clone(),
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
                Err(e)
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
            address_lookup_tables: self.address_lookup_tables.clone(),
            confirmation_max_attempts: self.config.transaction_config.confirmation_max_attempts,
            confirmation_poll_interval: Duration::from_millis(
                self.config.transaction_config.confirmation_poll_interval_ms,
            ),
            max_batches_per_tree: self.config.transaction_config.max_batches_per_tree,
        }
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
        let batch_context = self.build_batch_context(epoch_info, tree_accounts, None, None, None);
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
        let batch_context = self.build_batch_context(epoch_info, tree_accounts, None, None, None);
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
    ) -> Result<ProcessingResult> {
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
                    Err(e) => {
                        if is_v2_error(&e, V2Error::is_constraint) {
                            warn!(
                                event = "v2_state_constraint_error",
                                run_id = %self.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %e,
                                "State processing hit constraint error. Dropping processor to flush cache."
                            );
                            drop(proc); // Release lock before removing
                            self.state_processors.remove(&tree_accounts.merkle_tree);
                            self.proof_caches.remove(&tree_accounts.merkle_tree);
                            Err(e)
                        } else if is_v2_error(&e, V2Error::is_hashchain_mismatch) {
                            let warning_key = format!(
                                "v2_state_hashchain_mismatch:{}",
                                tree_accounts.merkle_tree
                            );
                            if should_emit_rate_limited_warning(
                                warning_key,
                                Duration::from_secs(15),
                            ) {
                                warn!(
                                    event = "v2_state_hashchain_mismatch",
                                    run_id = %self.run_id,
                                    tree = %tree_accounts.merkle_tree,
                                    error = %e,
                                    "State processing hit hashchain mismatch. Clearing cache and retrying."
                                );
                            }
                            self.heartbeat.increment_v2_recoverable_error();
                            proc.clear_cache().await;
                            Ok(ProcessingResult::default())
                        } else {
                            let warning_key =
                                format!("v2_state_process_failed:{}", tree_accounts.merkle_tree);
                            if should_emit_rate_limited_warning(
                                warning_key,
                                Duration::from_secs(10),
                            ) {
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
                    Err(e) => {
                        if is_v2_error(&e, V2Error::is_constraint) {
                            warn!(
                                event = "v2_address_constraint_error",
                                run_id = %self.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %e,
                                "Address processing hit constraint error. Dropping processor to flush cache."
                            );
                            drop(proc);
                            self.address_processors.remove(&tree_accounts.merkle_tree);
                            self.proof_caches.remove(&tree_accounts.merkle_tree);
                            Err(e)
                        } else if is_v2_error(&e, V2Error::is_hashchain_mismatch) {
                            let warning_key = format!(
                                "v2_address_hashchain_mismatch:{}",
                                tree_accounts.merkle_tree
                            );
                            if should_emit_rate_limited_warning(
                                warning_key,
                                Duration::from_secs(15),
                            ) {
                                warn!(
                                    event = "v2_address_hashchain_mismatch",
                                    run_id = %self.run_id,
                                    tree = %tree_accounts.merkle_tree,
                                    error = %e,
                                    "Address processing hit hashchain mismatch. Clearing cache and retrying."
                                );
                            }
                            self.heartbeat.increment_v2_recoverable_error();
                            proc.clear_cache().await;
                            Ok(ProcessingResult::default())
                        } else {
                            let warning_key =
                                format!("v2_address_process_failed:{}", tree_accounts.merkle_tree);
                            if should_emit_rate_limited_warning(
                                warning_key,
                                Duration::from_secs(10),
                            ) {
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
            .send_cached_proofs_as_transactions(epoch_info, tree_accounts, cached_proofs)
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
                match rpc
                    .create_and_send_transaction(
                        &instructions,
                        &authority,
                        &[&self.config.payer_keypair],
                    )
                    .await
                {
                    Ok(sig) => {
                        info!(
                            event = "cached_proofs_tx_sent",
                            run_id = %self.run_id,
                            signature = %sig,
                            instruction_count = instructions.len(),
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

        match rpc
            .create_and_send_transaction(
                &[ix],
                &self.config.payer_keypair.pubkey(),
                &[&self.config.payer_keypair],
            )
            .await
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
                if e.to_string().contains("already been processed") {
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

fn calculate_remaining_time_or_default(
    current_slot: u64,
    end_slot: u64,
    buffer_duration: Duration,
) -> Duration {
    if current_slot >= end_slot {
        return Duration::ZERO;
    }
    let slots_remaining = end_slot - current_slot;
    let base_remaining_duration = slot_duration()
        .checked_mul(slots_remaining as u32)
        .unwrap_or_default();
    base_remaining_duration
        .checked_sub(buffer_duration)
        .unwrap_or(Duration::ZERO)
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

            let (new_tree_sender, _) = broadcast::channel(100);

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
                    new_tree_sender.clone(),
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
