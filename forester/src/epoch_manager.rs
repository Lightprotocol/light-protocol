use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context};
use dashmap::DashMap;
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
    protocol_config::state::{EpochState, ProtocolConfig},
    sdk::{create_finalize_registration_instruction, create_report_work_instruction},
    utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority},
    EpochPda, ForesterEpochPda,
};
use solana_program::{
    instruction::InstructionError, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
};
use solana_sdk::{
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
    errors::{
        ChannelError, ForesterError, InitializationError, RegistrationError, WorkReportError,
    },
    grpc::{QueueEventRouter, QueueUpdateMessage},
    metrics::{push_metrics, queue_metric_update, update_forester_sol_balance},
    pagerduty::send_pagerduty_alert,
    processor::{
        tx_cache::ProcessedHashCache,
        v1::{
            config::{BuildTransactionBatchConfig, SendBatchedTransactionsConfig},
            send_transaction::send_batched_transactions,
            tx_builder::EpochManagerTransactions,
        },
        v2::{process_batched_operations, BatchContext},
    },
    queue_helpers::QueueItemData,
    rollover::{
        is_tree_ready_for_rollover, perform_address_merkle_tree_rollover,
        perform_state_merkle_tree_rollover_forester,
    },
    slot_tracker::{slot_duration, wait_until_slot_reached, SlotTracker},
    tree_data_sync::fetch_trees,
    tree_finder::TreeFinder,
    ForesterConfig, ForesterEpochInfo, Result,
};

#[derive(Copy, Clone, Debug)]
pub struct WorkReport {
    pub epoch: u64,
    pub processed_items: usize,
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
    work_report_sender: mpsc::Sender<WorkReport>,
    processed_items_per_epoch_count: Arc<Mutex<HashMap<u64, AtomicUsize>>>,
    trees: Arc<Mutex<Vec<TreeAccounts>>>,
    slot_tracker: Arc<SlotTracker>,
    processing_epochs: Arc<DashMap<u64, Arc<AtomicBool>>>,
    new_tree_sender: broadcast::Sender<TreeAccounts>,
    tx_cache: Arc<Mutex<ProcessedHashCache>>,
    ops_cache: Arc<Mutex<ProcessedHashCache>>,
    coordinator: Option<Arc<QueueEventRouter>>,
}

impl<R: Rpc> Clone for EpochManager<R> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            protocol_config: self.protocol_config.clone(),
            rpc_pool: self.rpc_pool.clone(),
            work_report_sender: self.work_report_sender.clone(),
            processed_items_per_epoch_count: self.processed_items_per_epoch_count.clone(),
            trees: self.trees.clone(),
            slot_tracker: self.slot_tracker.clone(),
            processing_epochs: self.processing_epochs.clone(),
            new_tree_sender: self.new_tree_sender.clone(),
            tx_cache: self.tx_cache.clone(),
            ops_cache: self.ops_cache.clone(),
            coordinator: self.coordinator.clone(),
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
    ) -> Result<Self> {
        let coordinator = if let Some(url) = &config.external_services.photon_grpc_url {
            match QueueEventRouter::new(url.clone()).await {
                Ok(coord) => {
                    let coord_arc = Arc::new(coord);

                    tokio::spawn({
                        let coord_clone = Arc::clone(&coord_arc);
                        async move {
                            if let Err(e) = coord_clone.run_dispatcher().await {
                                error!("dispatcher error: {:?}", e);
                            }
                        }
                    });

                    Some(coord_arc)
                }
                Err(e) => {
                    warn!("{:?}. V2 trees will use polling fallback.", e);
                    None
                }
            }
        } else {
            info!("photon_grpc_url not configured, V2 trees will use polling mode");
            None
        };

        Ok(Self {
            config,
            protocol_config,
            rpc_pool,
            work_report_sender,
            processed_items_per_epoch_count: Arc::new(Mutex::new(HashMap::new())),
            trees: Arc::new(Mutex::new(trees)),
            slot_tracker,
            processing_epochs: Arc::new(DashMap::new()),
            new_tree_sender,
            tx_cache,
            ops_cache,
            coordinator,
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

        while let Some(epoch) = rx.recv().await {
            debug!("Received new epoch: {}", epoch);
            let self_clone = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = self_clone.process_epoch(epoch).await {
                    error!("Error processing epoch {}: {:?}", epoch, e);
                }
            });
        }

        monitor_handle.await??;
        current_previous_handle.await??;
        new_tree_handle.await??;
        balance_check_handle.await??;

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
                    self.add_new_tree(new_tree).await?;
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
                    tx.send(current_epoch).await?;
                    last_epoch = Some(current_epoch);
                }
            }

            let next_epoch = current_epoch + 1;
            if last_epoch.is_none_or(|last| next_epoch > last) {
                let next_phases = get_epoch_phases(&self.protocol_config, next_epoch);

                // If the next epoch's registration phase has started, send it immediately
                if slot >= next_phases.registration.start && slot < next_phases.registration.end {
                    debug!(
                        "Next epoch {} registration phase already started, sending for processing",
                        next_epoch
                    );
                    tx.send(next_epoch).await?;
                    last_epoch = Some(next_epoch);
                    continue; // Check for further epochs immediately
                }

                // Otherwise, wait for the next epoch's registration phase to start
                let mut rpc = self.rpc_pool.get_connection().await?;
                let slots_to_wait = next_phases.registration.start.saturating_sub(slot);
                debug!(
                "Waiting for epoch {} registration phase to start. Current slot: {}, Registration phase start slot: {}, Slots to wait: {}",
                next_epoch, slot, next_phases.registration.start, slots_to_wait
            );

                if let Err(e) = wait_until_slot_reached(
                    &mut *rpc,
                    &self.slot_tracker,
                    next_phases.registration.start,
                )
                .await
                {
                    error!("Error waiting for next registration phase: {:?}", e);
                    continue;
                }

                debug!(
                    "Next epoch {} registration phase started, sending for processing",
                    next_epoch
                );
                if let Err(e) = tx.send(next_epoch).await {
                    error!(
                        "Failed to send next epoch {} for processing: {:?}",
                        next_epoch, e
                    );
                    continue;
                }
                last_epoch = Some(next_epoch);
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
            tx.send(previous_epoch).await?;
        }

        // Only process current epoch if we can still register or are already registered
        // If registration has ended and we haven't registered, skip it to avoid errors
        if slot < current_phases.registration.end {
            debug!(
                "Processing current epoch: {} (registration still open)",
                current_epoch
            );
            tx.send(current_epoch).await?;
        } else {
            // Check if we're already registered for this epoch
            let forester_epoch_pda_pubkey = get_forester_epoch_pda_from_authority(
                &self.config.derivation_pubkey,
                current_epoch,
            )
            .0;
            let rpc = self.rpc_pool.get_connection().await?;
            if let Ok(Some(_)) = rpc
                .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await
            {
                debug!(
                    "Processing current epoch: {} (already registered)",
                    current_epoch
                );
                tx.send(current_epoch).await?;
            } else {
                warn!(
                    "Skipping current epoch {} - registration ended at slot {} (current slot: {})",
                    current_epoch, current_phases.registration.end, slot
                );
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

        // Report work
        if self.sync_slot().await? < phases.report_work.end {
            self.report_work(&registration_info).await?;
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

        let (_, v2_trees): (Vec<_>, Vec<_>) = epoch_info
            .trees
            .iter()
            .filter(|tree| !should_skip_tree(&self.config, &tree.tree_accounts.tree_type))
            .partition(|tree| {
                matches!(
                    tree.tree_accounts.tree_type,
                    TreeType::StateV1 | TreeType::AddressV1
                )
            });

        let coordinator = self.coordinator.clone();

        if let Some(ref coord) = coordinator {
            if coord.is_healthy() {
                info!("Using WorkCoordinator for {} V2 trees", v2_trees.len());
            } else {
                info!(
                    "WorkCoordinator exists but not yet healthy. V2 trees will use polling fallback until connection establishes."
                );
            }
        } else if !v2_trees.is_empty() {
            info!("No WorkCoordinator available. V2 trees will use polling mode.");
        }

        let self_arc = Arc::new(self.clone());
        let epoch_info_arc = Arc::new(epoch_info.clone());
        let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

        for tree in epoch_info.trees.iter() {
            if should_skip_tree(&self.config, &tree.tree_accounts.tree_type) {
                continue;
            }

            let queue_update_rx = if matches!(
                tree.tree_accounts.tree_type,
                TreeType::StateV2 | TreeType::AddressV2
            ) {
                if let Some(ref coord) = coordinator {
                    Some(coord.register_tree(tree.tree_accounts.merkle_tree).await)
                } else {
                    None
                }
            } else {
                None
            };

            let has_channel = queue_update_rx.is_some();
            info!(
                "Creating thread for tree {} (type: {:?}, event: {})",
                tree.tree_accounts.merkle_tree, tree.tree_accounts.tree_type, has_channel
            );

            let self_clone = self_arc.clone();
            let epoch_info_clone = epoch_info_arc.clone();
            let tree = tree.clone();
            let coordinator_clone = coordinator.clone();

            let handle = tokio::spawn(async move {
                self_clone
                    .process_queue_v2(
                        &epoch_info_clone.epoch,
                        &epoch_info_clone.forester_epoch_pda,
                        tree.clone(),
                        queue_update_rx,
                        coordinator_clone.clone(),
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
        'outer_slot_loop: while current_slot < epoch_info.phases.active.end {
            let next_slot_to_process = tree_schedule
                .slots
                .iter_mut()
                .enumerate()
                .find_map(|(idx, opt_slot)| opt_slot.as_ref().map(|s| (idx, s.clone())));

            if let Some((slot_idx, light_slot_details)) = next_slot_to_process {
                match self
                    .process_light_slot(
                        epoch_info,
                        epoch_pda,
                        &tree_schedule.tree_accounts,
                        &light_slot_details,
                    )
                    .await
                {
                    Ok(_) => {
                        trace!(
                            "Successfully processed light slot {:?}",
                            light_slot_details.slot
                        );
                    }
                    Err(e) => {
                        error!(
                            "Error processing light slot {:?}: {:?}. Skipping this slot.",
                            light_slot_details.slot, e
                        );
                    }
                }
                tree_schedule.slots[slot_idx] = None; // Mark as attempted/processed
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
        skip(self, epoch_info, epoch_pda, tree_schedule, queue_update_rx, coordinator),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree_schedule.tree_accounts.merkle_tree)
    )]
    pub async fn process_queue_v2(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        mut tree_schedule: TreeForesterSchedule,
        mut queue_update_rx: Option<mpsc::Receiver<QueueUpdateMessage>>,
        coordinator: Option<Arc<QueueEventRouter>>,
    ) -> Result<()> {
        let mut current_slot = self.slot_tracker.estimated_current_slot();

        let total_slots = tree_schedule.slots.len();
        let eligible_slots = tree_schedule.slots.iter().filter(|s| s.is_some()).count();
        let tree_type = tree_schedule.tree_accounts.tree_type;

        info!(
            "process_queue_v2 tree={}, total_slots={}, eligible_slots={}, current_slot={}, active_phase_end={}",
            tree_schedule.tree_accounts.merkle_tree,
            total_slots,
            eligible_slots,
            current_slot,
            epoch_info.phases.active.end
        );

        let use_events = queue_update_rx.is_some();

        'outer_slot_loop: while current_slot < epoch_info.phases.active.end {
            let next_slot_to_process = tree_schedule
                .slots
                .iter_mut()
                .enumerate()
                .find_map(|(idx, opt_slot)| opt_slot.as_ref().map(|s| (idx, s.clone())));

            if let Some((slot_idx, light_slot_details)) = next_slot_to_process {
                let result = match tree_type {
                    TreeType::StateV1 | TreeType::AddressV1 => {
                        self.process_light_slot(
                            epoch_info,
                            epoch_pda,
                            &tree_schedule.tree_accounts,
                            &light_slot_details,
                        )
                        .await
                    }
                    TreeType::StateV2 | TreeType::AddressV2 => {
                        if use_events && queue_update_rx.is_some() {
                            self.process_light_slot_v2_event(
                                epoch_info,
                                epoch_pda,
                                &tree_schedule.tree_accounts,
                                &light_slot_details,
                                queue_update_rx.as_mut().unwrap(),
                                coordinator.clone(),
                            )
                            .await
                        } else {
                            self.process_light_slot_v2_fallback(
                                epoch_info,
                                epoch_pda,
                                &tree_schedule.tree_accounts,
                                &light_slot_details,
                            )
                            .await
                        }
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
            "Exiting process_queue_v2 for tree {}",
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
            "Processing slot {} ({}-{})",
            forester_slot_details.slot,
            forester_slot_details.start_solana_slot,
            forester_slot_details.end_solana_slot
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
                    estimated_slot,
                    None,
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
        skip(self, epoch_info, epoch_pda, tree_accounts, forester_slot_details, queue_update_rx, coordinator),
        fields(tree = %tree_accounts.merkle_tree)
    )]
    async fn process_light_slot_v2_event(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        queue_update_rx: &mut mpsc::Receiver<QueueUpdateMessage>,
        coordinator: Option<Arc<QueueEventRouter>>,
    ) -> Result<()> {
        info!(
            "Processing V2 light slot {} ({}-{})",
            forester_slot_details.slot,
            forester_slot_details.start_solana_slot,
            forester_slot_details.end_solana_slot
        );

        let mut rpc = self.rpc_pool.get_connection().await?;
        wait_until_slot_reached(
            &mut *rpc,
            &self.slot_tracker,
            forester_slot_details.start_solana_slot,
        )
        .await?;

        let tree_pubkey = tree_accounts.merkle_tree;
        let mut estimated_slot = self.slot_tracker.estimated_current_slot();

        let mut fallback_timer = tokio::time::interval(Duration::from_secs(5));
        fallback_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        'inner_processing_loop: loop {
            if estimated_slot >= forester_slot_details.end_solana_slot {
                trace!(
                    "Ending V2 event processing for slot {:?}",
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

            tokio::select! {
                Some(update) = queue_update_rx.recv() => {
                    if update.queue_size > 0 {
                        info!(
                            "V2 Queue update received for tree {}: {} items (type: {:?})",
                            tree_pubkey, update.queue_size, update.queue_type
                        );

                        let processing_start_time = Instant::now();
                        match self.dispatch_tree_processing(
                            epoch_info,
                            epoch_pda,
                            tree_accounts,
                            forester_slot_details,
                            estimated_slot,
                            Some(&update), // Pass gRPC queue update hint
                        ).await {
                            Ok(count) => {
                                if count > 0 {
                                    info!("V2 event processed {} items", count);
                                    self.update_metrics_and_counts(
                                        epoch_info.epoch,
                                        count,
                                        processing_start_time.elapsed(),
                                    ).await;
                                }
                            }
                            Err(e) => {
                                error!("V2 event processing failed: {:?}", e);
                            }
                        }
                    } else {
                        trace!("V2 received empty queue update for tree {}", tree_pubkey);
                    }
                }

                _ = fallback_timer.tick() => {
                    let is_healthy = coordinator
                        .as_ref()
                        .map(|c| c.is_healthy())
                        .unwrap_or(false);

                    if !is_healthy {
                        warn!("V2 gRPC connection unhealthy, running fallback check for tree {}", tree_pubkey);
                        let processing_start_time = Instant::now();
                        match self.dispatch_tree_processing(
                            epoch_info,
                            epoch_pda,
                            tree_accounts,
                            forester_slot_details,
                            estimated_slot,
                            None, // No queue update hint in fallback path
                        ).await {
                            Ok(count) if count > 0 => {
                                info!("V2 fallback found {} items", count);
                                self.update_metrics_and_counts(
                                    epoch_info.epoch,
                                    count,
                                    processing_start_time.elapsed(),
                                ).await;
                            }
                            Ok(_) => trace!("V2 fallback check: no work"),
                            Err(e) => error!("V2 fallback check failed: {:?}", e),
                        }
                    } else {
                        trace!("V2 fallback check skipped (gRPC healthy)");
                    }
                }
            }

            push_metrics(&self.config.external_services.pushgateway_url).await?;
            estimated_slot = self.slot_tracker.estimated_current_slot();
        }

        Ok(())
    }

    /// V2 polling fallback (when gRPC unavailable)
    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree_accounts, forester_slot_details),
        fields(tree = %tree_accounts.merkle_tree)
    )]
    async fn process_light_slot_v2_fallback(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
    ) -> Result<()> {
        info!(
            "Processing V2 light slot {} fallback ({}-{})",
            forester_slot_details.slot,
            forester_slot_details.start_solana_slot,
            forester_slot_details.end_solana_slot
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
                break 'inner_processing_loop;
            }

            let current_light_slot = (estimated_slot - epoch_info.phases.active.start)
                / epoch_pda.protocol_config.slot_length;
            if current_light_slot != forester_slot_details.slot {
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
                    estimated_slot,
                    None, // No queue update hint for regular processing
                )
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    error!("Failed V2 polling fallback: {:?}", e);
                    break 'inner_processing_loop;
                }
            };

            if items_processed_this_iteration > 0 {
                info!(
                    "V2 polling fallback processed {} items",
                    items_processed_this_iteration
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
                1_000
            } else {
                5_000
            };

            tokio::time::sleep(Duration::from_millis(sleep_duration_ms)).await;
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

    async fn dispatch_tree_processing(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        current_solana_slot: u64,
        queue_update: Option<&QueueUpdateMessage>,
    ) -> Result<usize> {
        match tree_accounts.tree_type {
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
                self.process_v2(epoch_info, tree_accounts, queue_update)
                    .await
            }
        }
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

    async fn process_v2(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        queue_update: Option<&QueueUpdateMessage>,
    ) -> Result<usize> {
        let default_prover_url = "http://127.0.0.1:3001".to_string();

        let (input_queue_hint, output_queue_hint) = if let Some(update) = queue_update {
            match update.queue_type {
                light_compressed_account::QueueType::InputStateV2 => {
                    (Some(update.queue_size), None)
                }
                light_compressed_account::QueueType::OutputStateV2 => {
                    (None, Some(update.queue_size))
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        let batch_context = BatchContext {
            rpc_pool: self.rpc_pool.clone(),
            authority: self.config.payer_keypair.insecure_clone(),
            derivation: self.config.derivation_pubkey,
            epoch: epoch_info.epoch,
            merkle_tree: tree_accounts.merkle_tree,
            output_queue: tree_accounts.queue,
            prover_append_url: self
                .config
                .external_services
                .prover_append_url
                .clone()
                .unwrap_or_else(|| default_prover_url.clone()),
            prover_update_url: self
                .config
                .external_services
                .prover_update_url
                .clone()
                .unwrap_or_else(|| default_prover_url.clone()),
            prover_address_append_url: self
                .config
                .external_services
                .prover_address_append_url
                .clone()
                .unwrap_or_else(|| default_prover_url.clone()),
            prover_api_key: self.config.external_services.prover_api_key.clone(),
            prover_polling_interval: Duration::from_secs(1),
            prover_max_wait_time: Duration::from_secs(600),
            ops_cache: self.ops_cache.clone(),
            epoch_phases: epoch_info.phases.clone(),
            slot_tracker: self.slot_tracker.clone(),
            input_queue_hint,
            output_queue_hint,
        };

        process_batched_operations(batch_context, tree_accounts.tree_type)
            .await
            .map_err(|e| anyhow!("Failed to process V2 operations: {}", e))
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
    async fn report_work(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        info!("Reporting work");
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
                info!("Work reported");
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

        let report = WorkReport {
            epoch: epoch_info.epoch.epoch,
            processed_items: self.get_processed_items_count(epoch_info.epoch.epoch).await,
        };

        self.work_report_sender
            .send(report)
            .await
            .map_err(|e| ChannelError::WorkReportSend {
                epoch: report.epoch,
                error: e.to_string(),
            })?;

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

    #[allow(dead_code)]
    async fn claim(&self, _forester_epoch_info: ForesterEpochInfo) {
        todo!()
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
    shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
    slot_tracker: Arc<SlotTracker>,
    tx_cache: Arc<Mutex<ProcessedHashCache>>,
    ops_cache: Arc<Mutex<ProcessedHashCache>>,
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
                let rpc = rpc_pool.get_connection().await?;
                let mut fetched_trees = fetch_trees(&*rpc).await?;
                if let Some(tree_id) = config.general_config.tree_id {
                    fetched_trees.retain(|tree| tree.merkle_tree == tree_id);
                    if fetched_trees.is_empty() {
                        error!("Specified tree {} not found", tree_id);
                        return Err(anyhow::anyhow!("Specified tree {} not found", tree_id));
                    }
                    info!("Processing only tree: {}", tree_id);
                }
                fetched_trees
            };
            trace!("Fetched initial trees: {:?}", trees);

            let (new_tree_sender, _) = broadcast::channel(100);

            // Only run tree finder if not filtering by specific tree
            let _tree_finder_handle = if config.general_config.tree_id.is_none() {
                let mut tree_finder = TreeFinder::new(
                    rpc_pool.clone(),
                    trees.clone(),
                    new_tree_sender.clone(),
                    Duration::from_secs(config.general_config.tree_discovery_interval_seconds),
                );

                Some(tokio::spawn(async move {
                    if let Err(e) = tree_finder.run().await {
                        error!("Tree finder error: {:?}", e);
                    }
                }))
            } else {
                info!("Tree discovery disabled when processing single tree");
                None
            };

            while retry_count < config.retry_config.max_retries {
                debug!("Creating EpochManager (attempt {})", retry_count + 1);
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
                )
                .await
                {
                    Ok(epoch_manager) => {
                        let epoch_manager = Arc::new(epoch_manager);
                        debug!(
                            "Successfully created EpochManager after {} attempts",
                            retry_count + 1
                        );

                        return tokio::select! {
                            result = epoch_manager.run() => result,
                            _ = shutdown => {
                                info!("Received shutdown signal. Stopping the service.");
                                Ok(())
                            }
                        };
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
                tree_id: None,
                sleep_after_processing_ms: 50,
                sleep_when_idle_ms: 100,
            },
            rpc_pool_config: Default::default(),
            registry_pubkey: Pubkey::default(),
            payer_keypair: Keypair::new(),
            derivation_pubkey: Pubkey::default(),
            address_tree_data: vec![],
            state_tree_data: vec![],
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
        };

        assert_eq!(report.epoch, 42);
        assert_eq!(report.processed_items, 100);
    }
}
