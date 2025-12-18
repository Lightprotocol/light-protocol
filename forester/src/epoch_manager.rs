use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context};
use borsh::BorshSerialize;
use dashmap::{mapref::entry::Entry, DashMap};
use forester_utils::{
    forester_epoch::{get_epoch_phases, Epoch, ForesterSlot, TreeAccounts, TreeForesterSchedule},
    rpc_pool::SolanaRpcPool,
};
use futures::future::join_all;
use light_client::{
    indexer::{MerkleProof, NewAddressProofWithContext},
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
use solana_pubkey::pubkey;
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
    signature::{Keypair, Signer},
    transaction::TransactionError,
};
use tokio::{
    sync::{broadcast, broadcast::error::RecvError, mpsc, oneshot, Mutex},
    task::JoinHandle,
    time::{sleep, Instant},
};
use tracing::{debug, error, info, info_span, instrument, trace, warn};

use crate::{
    compressible::{CompressibleAccountTracker, Compressor},
    errors::{
        ChannelError, ForesterError, InitializationError, RegistrationError, WorkReportError,
    },
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
    tree_data_sync::fetch_trees,
    ForesterConfig, ForesterEpochInfo, Result,
};

pub const LOOKUP_TABLE_ADDRESS: Pubkey = pubkey!("3zjBapEbu8usyEmtCMhEL2i2xPLoNkq6A4kquwwZKUiE");

fn is_v2_error(err: &anyhow::Error, predicate: impl FnOnce(&V2Error) -> bool) -> bool {
    err.downcast_ref::<V2Error>().is_some_and(predicate)
}

type StateBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>)>>;
type AddressBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>)>>;

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
pub struct EpochManager<R: Rpc> {
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
    compressible_tracker: Option<Arc<CompressibleAccountTracker>>,
    /// Cached zkp_batch_size per tree to filter queue updates below threshold
    zkp_batch_sizes: Arc<DashMap<Pubkey, u64>>,
    address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
}

impl<R: Rpc> Clone for EpochManager<R> {
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
            compressible_tracker: self.compressible_tracker.clone(),
            zkp_batch_sizes: self.zkp_batch_sizes.clone(),
            address_lookup_tables: self.address_lookup_tables.clone(),
        }
    }
}

impl<R: Rpc> EpochManager<R> {
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
        compressible_tracker: Option<Arc<CompressibleAccountTracker>>,
        address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
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
            compressible_tracker,
            zkp_batch_sizes: Arc::new(DashMap::new()),
            address_lookup_tables,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let tx = Arc::new(tx);

        let monitor_handle = tokio::spawn({
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
                monitor_handle,
                current_previous_handle,
                new_tree_handle,
                balance_check_handle,
            ),
            |(h1, h2, h3, h4)| {
                info!("Aborting EpochManager background tasks");
                h1.abort();
                h2.abort();
                h3.abort();
                h4.abort();
            },
        );

        while let Some(epoch) = rx.recv().await {
            debug!("Received new epoch: {}", epoch);

            let self_clone = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = self_clone.process_epoch(epoch).await {
                    error!("Error processing epoch {}: {:?}", epoch, e);
                }
            });
        }

        Ok(())
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
                        debug!("Current SOL balance: {} SOL", balance_in_sol);
                    }
                    Err(e) => error!("Failed to get balance: {:?}", e),
                },
                Err(e) => error!("Failed to get RPC connection for balance check: {:?}", e),
            }
        }
    }

    async fn handle_new_trees(self: Arc<Self>) -> Result<()> {
        let mut receiver = self.new_tree_sender.subscribe();
        loop {
            match receiver.recv().await {
                Ok(new_tree) => {
                    info!("Received new tree: {:?}", new_tree);
                    if let Err(e) = self.add_new_tree(new_tree).await {
                        error!("Failed to add new tree: {:?}", e);
                        // Continue processing other trees instead of crashing
                    }
                }
                Err(e) => match e {
                    RecvError::Lagged(lag) => {
                        warn!("Lagged in receiving new trees: {:?}", lag);
                    }
                    RecvError::Closed => {
                        info!("New tree receiver closed");
                        break;
                    }
                },
            }
        }
        Ok(())
    }

    async fn add_new_tree(&self, new_tree: TreeAccounts) -> Result<()> {
        info!("Adding new tree: {:?}", new_tree);
        let mut trees = self.trees.lock().await;
        trees.push(new_tree);
        drop(trees);

        info!("New tree added to the list of trees");

        let (current_slot, current_epoch) = self.get_current_slot_and_epoch().await?;
        let phases = get_epoch_phases(&self.protocol_config, current_epoch);

        // Check if we're currently in the active phase
        if current_slot >= phases.active.start && current_slot < phases.active.end {
            info!("Currently in active phase. Attempting to process the new tree immediately.");
            info!("Recovering registration info...");
            match self.recover_registration_info(current_epoch).await {
                Ok(mut epoch_info) => {
                    info!("Recovered registration info for current epoch");
                    let tree_schedule = TreeForesterSchedule::new_with_schedule(
                        &new_tree,
                        current_slot,
                        &epoch_info.forester_epoch_pda,
                        &epoch_info.epoch_pda,
                    )?;
                    epoch_info.trees.push(tree_schedule.clone());

                    let self_clone = Arc::new(self.clone());

                    info!("Spawning task to process new tree in current epoch");
                    tokio::spawn(async move {
                        if let Err(e) = self_clone
                            .process_queue(
                                &epoch_info.epoch,
                                &epoch_info.forester_epoch_pda,
                                tree_schedule,
                            )
                            .await
                        {
                            error!("Error processing queue for new tree: {:?}", e);
                        } else {
                            info!("Successfully processed new tree in current epoch");
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
                        warn!("Failed to recover registration info for new tree: {:?}", e);
                    }
                }
            }

            info!(
                "Injected new tree into current epoch {}: {:?}",
                current_epoch, new_tree
            );
        } else {
            info!(
                "Not in active phase (current slot: {}, active start: {}). Tree will be picked up in next registration.",
                current_slot, phases.active.start
            );
        }

        Ok(())
    }

    #[instrument(level = "debug", skip(self, tx))]
    async fn monitor_epochs(&self, tx: Arc<mpsc::Sender<u64>>) -> Result<()> {
        let mut last_epoch: Option<u64> = None;
        debug!("Starting epoch monitor");

        loop {
            let (slot, current_epoch) = self.get_current_slot_and_epoch().await?;
            debug!(
                "last_epoch: {:?}, current_epoch: {:?}, slot: {:?}",
                last_epoch, current_epoch, slot
            );

            if last_epoch.is_none_or(|last| current_epoch > last) {
                debug!("New epoch detected: {}", current_epoch);
                let phases = get_epoch_phases(&self.protocol_config, current_epoch);
                if slot < phases.registration.end {
                    debug!("Sending current epoch {} for processing", current_epoch);
                    if let Err(e) = tx.send(current_epoch).await {
                        error!(
                            "Failed to send current epoch {} for processing: {:?}",
                            current_epoch, e
                        );
                        return Ok(());
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
                                warn!("Failed to get RPC connection for slot waiting: {:?}", e);
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
                            "Waiting for epoch {} registration phase. Current slot: {}, Wait target: {} (registration starts at {}), Slots to wait: {}",
                            target_epoch, slot, wait_target, target_phases.registration.start, slots_to_wait
                        );

                        if let Err(e) =
                            wait_until_slot_reached(&mut *rpc, &self.slot_tracker, wait_target)
                                .await
                        {
                            error!("Error waiting for registration phase: {:?}", e);
                            break;
                        }

                        let current_slot = self.slot_tracker.estimated_current_slot();
                        if current_slot >= target_phases.registration.end {
                            debug!(
                                "Epoch {} registration ended while waiting (current slot {} >= end {}), trying next epoch",
                                target_epoch, current_slot, target_phases.registration.end
                            );
                            target_epoch += 1;
                            continue;
                        }

                        debug!(
                            "Epoch {} registration phase ready, sending for processing (current slot: {}, registration end: {})",
                            target_epoch, current_slot, target_phases.registration.end
                        );
                        if let Err(e) = tx.send(target_epoch).await {
                            error!(
                                "Failed to send epoch {} for processing: {:?}",
                                target_epoch, e
                            );
                            break;
                        }
                        last_epoch = Some(target_epoch);
                        break;
                    }

                    // If we're within the registration window, send it
                    if slot < target_phases.registration.end {
                        debug!(
                            "Epoch {} registration phase is open (slot {} < end {}), sending for processing",
                            target_epoch, slot, target_phases.registration.end
                        );
                        if let Err(e) = tx.send(target_epoch).await {
                            error!(
                                "Failed to send epoch {} for processing: {:?}",
                                target_epoch, e
                            );
                            break;
                        }
                        last_epoch = Some(target_epoch);
                        break;
                    }

                    // Registration already ended, try next epoch
                    debug!(
                        "Epoch {} registration already ended (slot {} >= end {}), checking next epoch",
                        target_epoch, slot, target_phases.registration.end
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
        let rpc = self.rpc_pool.get_connection().await?;
        let existing_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?;

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
                error!("Failed to send previous epoch for processing: {:?}", e);
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
                error!("Failed to send current epoch for processing: {:?}", e);
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
                            error!("Failed to send current epoch for processing: {:?}", e);
                            return Ok(()); // Channel closed, exit gracefully
                        }
                    } else {
                        warn!(
                            "Skipping current epoch {} - registration ended at slot {} (current slot: {})",
                            current_epoch, current_phases.registration.end, slot
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to get RPC connection to check registration, skipping: {:?}",
                        e
                    );
                }
            }
        }

        debug!("Finished processing current and previous epochs");
        Ok(())
    }

    #[instrument(level = "debug", skip(self), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch
    ))]
    async fn process_epoch(&self, epoch: u64) -> Result<()> {
        info!("Entering process_epoch");

        let processing_flag = self
            .processing_epochs
            .entry(epoch)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)));

        if processing_flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Another task is already processing this epoch
            debug!("Epoch {} is already being processed, skipping", epoch);
            return Ok(());
        }
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
                    warn!("Failed to recover registration info: {:?}", e);
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
                            "Too late to register for epoch {} (registration ended at slot {}, current slot: {}). Next available epoch: {}. Registration opens at slot {} ({} slots to wait).",
                            failed_epoch, registration_end, current_slot, next_epoch, next_phases.registration.start, slots_to_wait
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
            info!(
                "Skipping on-chain work report for epoch {} (report_work phase ended)",
                registration_info.epoch.epoch
            );
        }

        // TODO: implement
        // self.claim(&registration_info).await?;

        // Ensure we reset the processing flag when we're done
        let _reset_guard = scopeguard::guard((), |_| {
            processing_flag.store(false, Ordering::SeqCst);
        });

        info!("Exiting process_epoch");
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
            api_key: self.config.external_services.photon_api_key.clone(),
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

        for attempt in 0..max_retries {
            match self.register_for_epoch(epoch).await {
                Ok(registration_info) => return Ok(registration_info),
                Err(e) => {
                    warn!(
                        "Failed to register for epoch {} (attempt {}): {:?}",
                        epoch,
                        attempt + 1,
                        e
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
                                error!("Failed to send PagerDuty alert: {:?}", alert_err);
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
        info!("Registering for epoch: {}", epoch);
        let mut rpc = LightClient::new(LightClientConfig {
            url: self.config.external_services.rpc_url.to_string(),
            photon_url: self.config.external_services.indexer_url.clone(),
            api_key: self.config.external_services.photon_api_key.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::processed()),
            fetch_active_tree: false,
        })
        .await?;
        let slot = rpc.get_slot().await?;
        let phases = get_epoch_phases(&self.protocol_config, epoch);

        if slot < phases.registration.end {
            let forester_epoch_pda_pubkey =
                get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0;
            let existing_registration = rpc
                .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?;

            if let Some(existing_pda) = existing_registration {
                info!(
                    "Already registered for epoch {}. Recovering registration info.",
                    epoch
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
                "Too late to register for epoch {}. Current slot: {}, Registration end: {}",
                epoch, slot, phases.registration.end
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
                "Active phase has already started. Current slot: {}. Active phase start slot: {}. Slots left: {}.",
                current_slot, active_phase_start_slot, active_phase_end_slot.saturating_sub(current_slot)
            );
        } else {
            let waiting_slots = active_phase_start_slot - current_slot;
            let waiting_secs = waiting_slots / 2;
            info!("Waiting for active phase to start. Current slot: {}. Active phase start slot: {}. Waiting time: ~ {} seconds",
                current_slot,
                active_phase_start_slot,
                waiting_secs);
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
                        "Skipping FinalizeRegistration for epoch {}: active phase ended (current slot: {}, end: {})",
                        epoch_info.epoch.epoch, current_slot, epoch_info.epoch.phases.active.end
                    );
                    return Err(anyhow::anyhow!(
                        "Epoch {} active phase has ended, cannot finalize registration",
                        epoch_info.epoch.epoch
                    ));
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

        info!("Finished waiting for active phase");
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
        info!("Performing active work");

        let current_slot = self.slot_tracker.estimated_current_slot();
        let active_phase_end = epoch_info.epoch.phases.active.end;

        if !self.is_in_active_phase(current_slot, epoch_info)? {
            info!("No longer in active phase. Skipping work.");
            return Ok(());
        }

        self.sync_slot().await?;

        let trees_to_process: Vec<_> = epoch_info
            .trees
            .iter()
            .filter(|tree| !should_skip_tree(&self.config, &tree.tree_accounts.tree_type))
            .cloned()
            .collect();

        let self_arc = Arc::new(self.clone());
        let epoch_info_arc = Arc::new(epoch_info.clone());

        let mut handles: Vec<JoinHandle<Result<()>>> = Vec::with_capacity(trees_to_process.len());

        for tree in trees_to_process {
            info!(
                "Creating thread for tree {} (type: {:?})",
                tree.tree_accounts.merkle_tree, tree.tree_accounts.tree_type
            );

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

        for result in join_all(handles).await {
            match result {
                Ok(Ok(())) => {
                    debug!("Queue processed successfully");
                }
                Ok(Err(e)) => error!("Error processing queue: {:?}", e),
                Err(e) => error!("Task panicked: {:?}", e),
            }
        }

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
        let mut current_slot = self.slot_tracker.estimated_current_slot();

        let total_slots = tree_schedule.slots.len();
        let eligible_slots = tree_schedule.slots.iter().filter(|s| s.is_some()).count();
        let tree_type = tree_schedule.tree_accounts.tree_type;

        info!(
            "process_queue tree={}, total_slots={}, eligible_slots={}, current_slot={}, active_phase_end={}",
            tree_schedule.tree_accounts.merkle_tree,
            total_slots,
            eligible_slots,
            current_slot,
            epoch_info.phases.active.end
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
                            "Error processing light slot {:?}: {:?}",
                            light_slot_details.slot, e
                        );
                    }
                }
                tree_schedule.slots[slot_idx] = None;
            } else {
                info!(
                    "No further eligible slots in schedule for tree {}",
                    tree_schedule.tree_accounts.merkle_tree
                );
                break 'outer_slot_loop;
            }

            current_slot = self.slot_tracker.estimated_current_slot();
        }

        info!(
            "Exiting process_queue for tree {}",
            tree_schedule.tree_accounts.merkle_tree
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
        info!(
            "Processing slot {} ({}-{}) epoch {}",
            forester_slot_details.slot,
            forester_slot_details.start_solana_slot,
            forester_slot_details.end_solana_slot,
            epoch_info.epoch
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
                warn!("Light slot mismatch. Exiting processing for this slot.");
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
                        "Failed processing in slot {:?}: {:?}",
                        forester_slot_details.slot, e
                    );
                    break 'inner_processing_loop;
                }
            };
            if items_processed_this_iteration > 0 {
                debug!(
                    "Processed {} items in slot {:?}",
                    items_processed_this_iteration, forester_slot_details.slot
                );
            }

            self.update_metrics_and_counts(
                epoch_info.epoch,
                items_processed_this_iteration,
                processing_start_time.elapsed(),
            )
            .await;

            push_metrics(&self.config.external_services.pushgateway_url).await?;
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
        info!(
            "Processing V2 light slot {} ({}-{}, consecutive_end={})",
            forester_slot_details.slot,
            forester_slot_details.start_solana_slot,
            forester_slot_details.end_solana_slot,
            consecutive_eligibility_end
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
        if let Some(items_sent) = self
            .try_send_cached_proofs(epoch_info, tree_accounts, consecutive_eligibility_end)
            .await?
        {
            if items_sent > 0 {
                info!(
                    "Sent {} items from cache for tree {}",
                    items_sent, tree_pubkey
                );
                self.update_metrics_and_counts(
                    epoch_info.epoch,
                    items_sent,
                    Duration::from_millis(1),
                )
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
                warn!("V2 slot mismatch. Exiting processing.");
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
                        info!("V2 processed {} items for tree {}", count, tree_pubkey);
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
                    error!("V2 processing failed for tree {}: {:?}", tree_pubkey, e);
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            }

            push_metrics(&self.config.external_services.pushgateway_url).await?;
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
            error!("Failed to calculate eligible forester index: {:?}", e);
            anyhow::anyhow!("Eligibility calculation failed: {}", e)
        })?;

        if !epoch_pda.is_eligible(eligible_forester_slot_index) {
            warn!(
                "Forester {} is no longer eligible to process tree {} in light slot {}.",
                self.config.payer_keypair.pubkey(),
                queue_pubkey,
                current_light_slot
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
            "Processing {} compressible accounts in {} batches (batch_size={})",
            accounts.len(),
            num_batches,
            config.batch_size
        );

        let compressor = Compressor::new(
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
                        "Cancelling compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                        current_slot,
                        consecutive_eligibility_end
                    );
                    return Err((batch_idx, batch.len(), anyhow!("Forester no longer eligible")));
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
                            "Compression batch {}/{} failed: {:?}",
                            batch_idx + 1,
                            num_batches,
                            e
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
                        "Successfully compressed {} accounts in batch {}/{}: {}",
                        count,
                        batch_idx + 1,
                        num_batches,
                        sig
                    );
                    total_compressed += count;
                }
                Err((batch_idx, count, e)) => {
                    error!(
                        "Compression batch {}/{} ({} accounts) failed: {:?}",
                        batch_idx + 1,
                        num_batches,
                        count,
                        e
                    );
                }
            }
        }

        info!(
            "Completed compression for epoch {}: compressed {} accounts",
            epoch_info.epoch, total_compressed
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
                "processed {} items v1 tree {}",
                num_sent, tree_accounts.merkle_tree
            );
        }

        match self.rollover_if_needed(tree_accounts).await {
            Ok(_) => Ok(num_sent),
            Err(e) => {
                error!("Failed to rollover tree: {:?}", e);
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
        }
    }

    async fn get_or_create_state_processor(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
    ) -> Result<Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>> {
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

        // Insert the new processor (or get existing if another task beat us to it)
        match self.state_processors.entry(tree_accounts.merkle_tree) {
            Entry::Occupied(occupied) => {
                // Another task already inserted - use theirs (they may have cached state)
                Ok(occupied.get().1.clone())
            }
            Entry::Vacant(vacant) => {
                vacant.insert((epoch_info.epoch, processor.clone()));
                Ok(processor)
            }
        }
    }

    async fn get_or_create_address_processor(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
    ) -> Result<Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>> {
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

        // Insert the new processor (or get existing if another task beat us to it)
        match self.address_processors.entry(tree_accounts.merkle_tree) {
            Entry::Occupied(occupied) => Ok(occupied.get().1.clone()),
            Entry::Vacant(vacant) => {
                vacant.insert((epoch_info.epoch, processor.clone()));
                Ok(processor)
            }
        }
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
                                "State processing hit constraint error for tree {}: {}. Dropping processor to flush cache.",
                                tree_accounts.merkle_tree,
                                e
                            );
                            drop(proc); // Release lock before removing
                            self.state_processors.remove(&tree_accounts.merkle_tree);
                            self.proof_caches.remove(&tree_accounts.merkle_tree);
                            Err(e)
                        } else if is_v2_error(&e, V2Error::is_hashchain_mismatch) {
                            warn!(
                                "State processing hit hashchain mismatch for tree {}: {}. Clearing cache and retrying.",
                                tree_accounts.merkle_tree,
                                e
                            );
                            proc.clear_cache().await;
                            Ok(ProcessingResult::default())
                        } else {
                            warn!(
                                "Failed to process state queue for tree {}: {}. Will retry next tick without dropping processor.",
                                tree_accounts.merkle_tree,
                                e
                            );
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
                                "Address processing hit constraint error for tree {}: {}. Dropping processor to flush cache.",
                                tree_accounts.merkle_tree,
                                e
                            );
                            self.address_processors.remove(&tree_accounts.merkle_tree);
                            self.proof_caches.remove(&tree_accounts.merkle_tree);
                            Err(e)
                        } else if is_v2_error(&e, V2Error::is_hashchain_mismatch) {
                            warn!(
                                "Address processing hit hashchain mismatch for tree {}: {}. Clearing cache and retrying.",
                                tree_accounts.merkle_tree,
                                e
                            );
                            proc.clear_cache().await;
                            Ok(ProcessingResult::default())
                        } else {
                            warn!(
                                "Failed to process address queue for tree {}: {}. Will retry next tick without dropping processor.",
                                tree_accounts.merkle_tree,
                                e
                            );
                            Ok(ProcessingResult::default())
                        }
                    }
                }
            }
            _ => {
                warn!(
                    "Unsupported tree type for V2 processing: {:?}",
                    tree_accounts.tree_type
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
            queue_metric_update(epoch_num, items_processed, duration).await;
            self.increment_processed_items_count(epoch_num, items_processed)
                .await;
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
        let v2_state_trees: Vec<_> = trees
            .iter()
            .filter(|t| matches!(t.tree_type, TreeType::StateV2))
            .cloned()
            .collect();
        drop(trees);

        if v2_state_trees.is_empty() {
            return;
        }

        if slots_until_active < 15 {
            info!(
                "Skipping pre-warming: only {} slots until active phase, not enough time",
                slots_until_active
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
                                warn!("Failed to get RPC for cache validation: {:?}", e);
                                return;
                            }
                        };
                        if let Ok(current_root) =
                            self_clone.fetch_current_root(&mut *rpc, &tree_accounts).await
                        {
                            info!(
                                "Tree {} has {} cached proofs from previous epoch (root: {:?}), skipping pre-warm",
                                tree_pubkey, cache_len, &current_root[..4]
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
                                "Failed to create processor for pre-warming tree {}: {:?}",
                                tree_pubkey, e
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
                                    "Pre-warmed {} items for tree {} during wait (metrics: {:?})",
                                    result.items_processed, tree_pubkey, result.metrics
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
        let timeout_duration = Duration::from_millis((timeout_slots * 400).min(30_000));

        info!(
            "Starting pre-warming for {} trees with {}ms timeout",
            v2_state_trees.len(),
            timeout_duration.as_millis()
        );

        match tokio::time::timeout(timeout_duration, futures::future::join_all(prewarm_futures))
            .await
        {
            Ok(_) => {
                info!("Completed pre-warming for all trees");
            }
            Err(_) => {
                info!("Pre-warming timed out after {:?}", timeout_duration);
            }
        }
    }

    async fn try_send_cached_proofs(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        _consecutive_eligibility_end: u64,
    ) -> Result<Option<usize>> {
        let tree_pubkey = tree_accounts.merkle_tree;

        let cache = match self.proof_caches.get(&tree_pubkey) {
            Some(c) => c.clone(),
            None => return Ok(None),
        };

        if cache.is_warming().await {
            debug!("Cache still warming for tree {}, skipping", tree_pubkey);
            return Ok(None);
        }

        let mut rpc = self.rpc_pool.get_connection().await?;
        let current_root = match self.fetch_current_root(&mut *rpc, tree_accounts).await {
            Ok(root) => root,
            Err(e) => {
                warn!(
                    "Failed to fetch current root for tree {}: {:?}",
                    tree_pubkey, e
                );
                return Ok(None);
            }
        };

        let cached_proofs = match cache.take_if_valid(&current_root).await {
            Some(proofs) => proofs,
            None => {
                debug!(
                    "No valid cached proofs for tree {} (root: {:?})",
                    tree_pubkey,
                    &current_root[..4]
                );
                return Ok(None);
            }
        };

        if cached_proofs.is_empty() {
            return Ok(Some(0));
        }

        info!(
            "Sending {} cached proofs for tree {} (root: {:?})",
            cached_proofs.len(),
            tree_pubkey,
            &current_root[..4]
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
                total_items += proof.items;
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
                            "Sent cached proofs tx: {} ({} instructions)",
                            sig,
                            instructions.len()
                        );
                    }
                    Err(e) => {
                        warn!("Failed to send cached proofs tx: {:?}", e);
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
            info!("Starting {} rollover.", tree_account.merkle_tree);
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
        info!("Waiting for report work phase");
        let mut rpc = self.rpc_pool.get_connection().await?;
        let report_work_start_slot = epoch_info.epoch.phases.report_work.start;
        wait_until_slot_reached(&mut *rpc, &self.slot_tracker, report_work_start_slot).await?;

        info!("Finished waiting for report work phase");
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
            "Sending work report: epoch={} items={} metrics={:?}",
            report.epoch, report.processed_items, report.metrics
        );

        self.work_report_sender
            .send(report)
            .await
            .map_err(|e| ChannelError::WorkReportSend {
                epoch: report.epoch,
                error: e.to_string(),
            })?;

        Ok(())
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch
    ))]
    async fn report_work_onchain(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        info!("Reporting work on-chain");
        let mut rpc = LightClient::new(LightClientConfig {
            url: self.config.external_services.rpc_url.to_string(),
            photon_url: self.config.external_services.indexer_url.clone(),
            api_key: self.config.external_services.photon_api_key.clone(),
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
                info!("Work reported on-chain");
            }
            Err(e) => {
                if e.to_string().contains("already been processed") {
                    info!("Work already reported for epoch {}", epoch_info.epoch.epoch);
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

                info!("Address rollover signature: {:?}", rollover_signature);
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

                info!("State rollover signature: {:?}", rollover_signature);

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

#[instrument(
    level = "info",
    skip(config, protocol_config, rpc_pool, shutdown, work_report_sender, slot_tracker),
    fields(forester = %config.payer_keypair.pubkey())
)]
#[allow(clippy::too_many_arguments)]
pub async fn run_service<R: Rpc>(
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<SolanaRpcPool<R>>,
    mut shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
    slot_tracker: Arc<SlotTracker>,
    tx_cache: Arc<Mutex<ProcessedHashCache>>,
    ops_cache: Arc<Mutex<ProcessedHashCache>>,
    compressible_tracker: Option<Arc<CompressibleAccountTracker>>,
) -> Result<()> {
    info_span!("run_service", forester = %config.payer_keypair.pubkey())
        .in_scope(|| async {
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
            info!("Starting forester in {} mode", processor_mode_str);

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
                            info!("Received shutdown signal during tree fetch. Stopping.");
                            return Ok(());
                        }
                        result = rpc_pool.get_connection() => {
                            match result {
                                Ok(rpc) => {
                                    tokio::select! {
                                        biased;
                                        _ = &mut shutdown => {
                                            info!("Received shutdown signal during tree fetch. Stopping.");
                                            return Ok(());
                                        }
                                        fetch_result = fetch_trees(&*rpc) => {
                                            match fetch_result {
                                                Ok(mut fetched_trees) => {
                                                    if !config.general_config.tree_ids.is_empty() {
                                                        let tree_ids = &config.general_config.tree_ids;
                                                        fetched_trees.retain(|tree| tree_ids.contains(&tree.merkle_tree));
                                                        if fetched_trees.is_empty() {
                                                            error!("None of the specified trees found: {:?}", tree_ids);
                                                            return Err(anyhow::anyhow!(
                                                                "None of the specified trees found: {:?}",
                                                                tree_ids
                                                            ));
                                                        }
                                                        info!("Processing only trees: {:?}", tree_ids);
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
                                                        "Failed to fetch trees (attempt {}/{}), retrying in {:?}: {:?}",
                                                        attempts, max_attempts, delay, e
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
                                        "Failed to get RPC connection (attempt {}/{}), retrying in {:?}: {:?}",
                                        attempts, max_attempts, delay, e
                                    );
                                }
                            }
                        }
                    }

                    tokio::select! {
                        biased;
                        _ = &mut shutdown => {
                            info!("Received shutdown signal during retry wait. Stopping.");
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
                info!("Processing specific trees, tree discovery will be limited");
            }

            while retry_count < config.retry_config.max_retries {
                debug!("Creating EpochManager (attempt {})", retry_count + 1);

                let address_lookup_tables = {
                    let rpc = rpc_pool.get_connection().await?;
                    match load_lookup_table_async(&*rpc, LOOKUP_TABLE_ADDRESS).await
                    {
                        Ok(lut) => {
                            info!(
                                "Loaded lookup table {} with {} addresses",
                                LOOKUP_TABLE_ADDRESS,
                                lut.addresses.len()
                            );
                            Arc::new(vec![lut])
                        }
                        Err(e) => {
                            warn!(
                                "Failed to load lookup table {}: {:?}. Falling back to legacy transactions.",
                                LOOKUP_TABLE_ADDRESS, e
                            );
                            Arc::new(Vec::new())
                        }
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
                    address_lookup_tables,
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
                                info!("Received shutdown signal. Stopping the service.");
                                Ok(())
                            }
                        };

                        return result;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create EpochManager (attempt {}): {:?}",
                            retry_count + 1,
                            e
                        );
                        retry_count += 1;
                        if retry_count < config.retry_config.max_retries {
                            debug!("Retrying in {:?}", retry_delay);
                            sleep(retry_delay).await;
                            retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
                        } else {
                            error!(
                                "Failed to start forester after {} attempts over {:?}",
                                config.retry_config.max_retries,
                                start_time.elapsed()
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
        .await
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
                photon_api_key: None,
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
                slot_update_interval_seconds: 10,
                tree_discovery_interval_seconds: 1,
                enable_metrics: false,
                skip_v1_state_trees: skip_v1_state,
                skip_v1_address_trees: skip_v1_address,
                skip_v2_state_trees: skip_v2_state,
                skip_v2_address_trees: skip_v2_address,
                tree_ids: vec![],
                sleep_after_processing_ms: 50,
                sleep_when_idle_ms: 100,
                queue_polling_mode: crate::cli::QueuePollingMode::Indexer,
            },
            rpc_pool_config: Default::default(),
            registry_pubkey: Pubkey::default(),
            payer_keypair: Keypair::new(),
            derivation_pubkey: Pubkey::default(),
            address_tree_data: vec![],
            state_tree_data: vec![],
            compressible_config: None,
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
