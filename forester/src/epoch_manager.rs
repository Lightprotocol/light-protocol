use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use dashmap::DashMap;
use forester_utils::{
    forester_epoch::{get_epoch_phases, Epoch, TreeAccounts, TreeForesterSchedule, TreeType},
    indexer::{Indexer, MerkleProof, NewAddressProofWithContext},
};
use futures::future::join_all;
use light_client::{
    rpc::{RetryConfig, RpcConnection, RpcError, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_registry::{
    errors::RegistryError,
    protocol_config::state::ProtocolConfig,
    sdk::{create_finalize_registration_instruction, create_report_work_instruction},
    utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority},
    EpochPda, ForesterEpochPda,
};
use solana_program::{instruction::InstructionError, pubkey::Pubkey};
use solana_sdk::{signature::Signer, transaction::TransactionError};
use tokio::{
    sync::{broadcast, broadcast::error::RecvError, mpsc, oneshot, Mutex},
    task::JoinHandle,
    time::{sleep, Instant},
};
use tracing::{debug, error, info, info_span, instrument, warn};

use crate::{
    batched_address_ops::process_batched_address_operations,
    batched_ops::process_batched_operations,
    errors::ForesterError,
    metrics::{push_metrics, queue_metric_update, update_forester_sol_balance},
    pagerduty::send_pagerduty_alert,
    queue_helpers::QueueItemData,
    rollover::{
        is_tree_ready_for_rollover, rollover_address_merkle_tree, rollover_state_merkle_tree,
    },
    send_transaction::{
        send_batched_transactions, BuildTransactionBatchConfig, EpochManagerTransactions,
        SendBatchedTransactionsConfig,
    },
    slot_tracker::{slot_duration, wait_until_slot_reached, SlotTracker},
    tree_data_sync::fetch_trees,
    tree_finder::TreeFinder,
    ForesterConfig, ForesterEpochInfo, Result,
};

#[derive(Clone, Debug)]
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
        self.tree_account.tree_type == TreeType::Address
    }
    pub fn is_state_tree(&self) -> bool {
        self.tree_account.tree_type == TreeType::State
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum MerkleProofType {
    AddressProof(NewAddressProofWithContext<16>),
    StateProof(MerkleProof),
}

#[derive(Debug)]
pub struct EpochManager<R: RpcConnection, I: Indexer<R>> {
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    work_report_sender: mpsc::Sender<WorkReport>,
    processed_items_per_epoch_count: Arc<Mutex<HashMap<u64, AtomicUsize>>>,
    trees: Arc<Mutex<Vec<TreeAccounts>>>,
    slot_tracker: Arc<SlotTracker>,
    processing_epochs: Arc<DashMap<u64, Arc<AtomicBool>>>,
    new_tree_sender: broadcast::Sender<TreeAccounts>,
}

impl<R: RpcConnection, I: Indexer<R>> Clone for EpochManager<R, I> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            protocol_config: self.protocol_config.clone(),
            rpc_pool: self.rpc_pool.clone(),
            indexer: self.indexer.clone(),
            work_report_sender: self.work_report_sender.clone(),
            processed_items_per_epoch_count: self.processed_items_per_epoch_count.clone(),
            trees: self.trees.clone(),
            slot_tracker: self.slot_tracker.clone(),
            processing_epochs: self.processing_epochs.clone(),
            new_tree_sender: self.new_tree_sender.clone(),
        }
    }
}

impl<R: RpcConnection, I: Indexer<R>> EpochManager<R, I> {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        config: Arc<ForesterConfig>,
        protocol_config: Arc<ProtocolConfig>,
        rpc_pool: Arc<SolanaRpcPool<R>>,
        indexer: Arc<Mutex<I>>,
        work_report_sender: mpsc::Sender<WorkReport>,
        trees: Vec<TreeAccounts>,
        slot_tracker: Arc<SlotTracker>,
        new_tree_sender: broadcast::Sender<TreeAccounts>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            protocol_config,
            rpc_pool,
            indexer,
            work_report_sender,
            processed_items_per_epoch_count: Arc::new(Mutex::new(HashMap::new())),
            trees: Arc::new(Mutex::new(trees)),
            slot_tracker,
            processing_epochs: Arc::new(DashMap::new()),
            new_tree_sender,
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
        let interval = Duration::from_secs(60);
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;
            let mut rpc = self.rpc_pool.get_connection().await?;
            let balance = rpc.get_balance(&self.config.payer_keypair.pubkey()).await?;
            let balance_in_sol = balance as f64 / 1e9;
            update_forester_sol_balance(
                &self.config.payer_keypair.pubkey().to_string(),
                balance_in_sol,
            );
            info!("Current SOL balance: {} SOL", balance_in_sol);
            tokio::task::yield_now().await;
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
            if let Ok(mut epoch_info) = self.recover_registration_info(current_epoch).await {
                info!("Recovered registration info for current epoch");
                let tree_schedule = TreeForesterSchedule::new_with_schedule(
                    &new_tree,
                    current_slot,
                    &epoch_info.forester_epoch_pda,
                    &epoch_info.epoch_pda,
                );
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

                info!(
                    "Injected new tree into current epoch {}: {:?}",
                    current_epoch, new_tree
                );
            } else {
                warn!("Failed to retrieve current epoch info for processing new tree");
            }
        } else {
            info!("Not in active phase. New tree will be processed in the next active phase");
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
            if last_epoch.map_or(true, |last| current_epoch > last) {
                debug!("New epoch detected: {}", current_epoch);
                let phases = get_epoch_phases(&self.protocol_config, current_epoch);
                if slot < phases.registration.end {
                    tx.send(current_epoch).await.map_err(|e| {
                        ForesterError::Custom(format!("Failed to send new epoch: {}", e))
                    })?;
                    last_epoch = Some(current_epoch);
                }
            }

            let next_epoch = current_epoch + 1;
            let next_phases = get_epoch_phases(&self.protocol_config, next_epoch);
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
        let mut rpc = self.rpc_pool.get_connection().await?;
        let existing_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?;

        match existing_pda {
            Some(pda) => {
                self.recover_registration_info_internal(epoch, forester_epoch_pda_pubkey, pda)
                    .await
            }
            None => Err(ForesterError::ForesterEpochPdaNotFound),
        }
    }

    #[instrument(level = "debug", skip(self))]
    async fn process_current_and_previous_epochs(&self, tx: Arc<mpsc::Sender<u64>>) -> Result<()> {
        let (slot, current_epoch) = self.get_current_slot_and_epoch().await?;
        let current_phases = get_epoch_phases(&self.protocol_config, current_epoch);
        let previous_epoch = current_epoch.saturating_sub(1);

        // Process previous epoch if still in active or later phase
        if slot > current_phases.registration.start {
            debug!("Processing previous epoch: {}", previous_epoch);
            tx.send(previous_epoch).await.map_err(|e| {
                ForesterError::Custom(format!("Failed to send previous epoch: {}", e))
            })?;
        }

        // Process current epoch
        debug!("Processing current epoch: {}", current_epoch);
        tx.send(current_epoch)
            .await
            .map_err(|e| ForesterError::Custom(format!("Failed to send current epoch: {}", e)))?;

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
                warn!("Failed to recover registration info: {:?}", e);
                // If recovery fails, attempt to register
                self.register_for_epoch_with_retry(epoch, 20, Duration::from_millis(1000))
                    .await?
            }
        };
        debug!("Recovered registration info for epoch {}", epoch);

        // Wait for active phase
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
    ) -> Result<ForesterEpochInfo> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        let slot = rpc.get_slot().await?;
        let phases = get_epoch_phases(&self.protocol_config, epoch);

        // Check if it's already too late to register
        if slot >= phases.registration.end {
            return Err(ForesterError::Custom(format!(
                "Too late to register for epoch {}. Current slot: {}, Registration end: {}",
                epoch, slot, phases.registration.end
            )));
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
                        if let Err(alert_err) = send_pagerduty_alert(
                            &self
                                .config
                                .external_services
                                .pagerduty_routing_key
                                .clone()
                                .unwrap(),
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
                        return Err(e);
                    }
                }
            }
        }
        Err(ForesterError::Custom(format!(
            "Failed to register for epoch {} after {} attempts",
            epoch, max_retries
        )))
    }

    #[instrument(level = "debug", skip(self), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch
    ))]
    async fn register_for_epoch(&self, epoch: u64) -> Result<ForesterEpochInfo> {
        info!("Registering for epoch: {}", epoch);
        let mut rpc =
            SolanaRpcConnection::new(self.config.external_services.rpc_url.as_str(), None);
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
                {
                    Ok(Some(epoch)) => {
                        debug!("Registered epoch: {:?}", epoch);
                        epoch
                    }
                    Ok(None) => {
                        return Err(ForesterError::Custom(
                            "Epoch::register returned None".into(),
                        ))
                    }
                    Err(e) => {
                        return Err(ForesterError::Custom(format!(
                            "Epoch::register failed: {:?}",
                            e
                        )))
                    }
                };

                let forester_epoch_pda = match rpc
                    .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
                    .await
                {
                    Ok(Some(pda)) => {
                        debug!("ForesterEpochPda: {:?}", pda);
                        pda
                    }
                    Ok(None) => {
                        return Err(ForesterError::Custom(
                            "Failed to get ForesterEpochPda: returned None".into(),
                        ))
                    }
                    Err(e) => {
                        return Err(ForesterError::Custom(format!(
                            "Failed to get ForesterEpochPda: {:?}",
                            e
                        )))
                    }
                };

                let epoch_pda_address = get_epoch_pda_address(epoch);
                let epoch_pda = match rpc
                    .get_anchor_account::<EpochPda>(&epoch_pda_address)
                    .await?
                {
                    Some(pda) => pda,
                    None => {
                        return Err(ForesterError::Custom(
                            "Failed to get EpochPda: returned None".into(),
                        ))
                    }
                };

                ForesterEpochInfo {
                    epoch: registered_epoch,
                    epoch_pda,
                    forester_epoch_pda,
                    trees: Vec::new(),
                }
            };
            debug!("Registration for epoch completed");
            debug!("Registration Info: {:?}", registration_info);
            Ok(registration_info)
        } else {
            warn!(
                "Too late to register for epoch {}. Current slot: {}, Registration end: {}",
                epoch, slot, phases.registration.end
            );
            Err(ForesterError::Custom(
                "Too late to register for epoch".into(),
            ))
        }
    }

    async fn recover_registration_info_internal(
        &self,
        epoch: u64,
        forester_epoch_pda_address: Pubkey,
        forester_epoch_pda: ForesterEpochPda,
    ) -> Result<ForesterEpochInfo> {
        let mut rpc = self.rpc_pool.get_connection().await?;

        let phases = get_epoch_phases(&self.protocol_config, epoch);
        let slot = rpc.get_slot().await?;
        let state = phases.get_current_epoch_state(slot);

        let epoch_pda_address = get_epoch_pda_address(epoch);
        let epoch_pda = match rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await?
        {
            Some(pda) => pda,
            None => {
                return Err(ForesterError::Custom(
                    "Failed to get EpochPda: returned None".into(),
                ))
            }
        };

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
        info!("Waiting for active phase");
        let mut rpc = self.rpc_pool.get_connection().await?;

        let active_phase_start_slot = epoch_info.epoch.phases.active.start;
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
            .await?
            .ok_or_else(|| ForesterError::Custom("Failed to get ForesterEpochPda".to_string()))?;

        let slot = rpc.get_slot().await?;
        let trees = self.trees.lock().await;
        info!("Adding schedule for trees: {:?}", *trees);
        epoch_info.add_trees_with_schedule(&trees, slot);
        info!("Finished waiting for active phase");
        Ok(epoch_info)
    }

    // TODO: add receiver for new tree discoverd -> spawn new task to process this tree derive schedule etc.
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

        let self_arc = Arc::new(self.clone());
        let epoch_info_arc = Arc::new(epoch_info.clone());

        let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

        info!(
            "Creating threads for tree processing. Trees: {:?}",
            epoch_info.trees
        );
        for tree in epoch_info.trees.iter() {
            info!(
                "Creating thread for tree {}",
                tree.tree_accounts.merkle_tree
            );
            let self_clone = self_arc.clone();
            let epoch_info_clone = epoch_info_arc.clone();
            let tree = tree.clone();
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

        debug!("Threads created. Waiting for active phase to end");

        // Wait for all tasks to complete
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

        info!("Completed active work");
        Ok(())
    }

    // Sync estimated slot before creating threads.
    // Threads rely on the estimated slot.
    async fn sync_slot(&self) -> Result<u64> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        let current_slot = rpc.get_slot().await?;
        self.slot_tracker.update(current_slot);
        Ok(current_slot)
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree.tree_accounts.merkle_tree)
    )]
    pub async fn process_queue(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        mut tree: TreeForesterSchedule,
    ) -> Result<()> {
        info!("enter process_queue");
        info!("Tree schedule slots: {:?}", tree.slots);
        // TODO: sync at some point
        let mut estimated_slot = self.slot_tracker.estimated_current_slot();

        debug!(
            "Estimated slot: {}, epoch end: {}",
            estimated_slot, epoch_info.phases.active.end
        );
        while estimated_slot < epoch_info.phases.active.end {
            debug!("Searching for next eligible slot");
            // search for next eligible slot
            let index_and_forester_slot = tree
                .slots
                .iter()
                .enumerate()
                .find(|(_, slot)| slot.is_some());

            if let Some((index, forester_slot)) = index_and_forester_slot {
                info!(
                    "Found eligible slot, index: {}, tree: {}",
                    index,
                    tree.tree_accounts.merkle_tree.to_string()
                );
                let forester_slot = forester_slot.as_ref().unwrap().clone();
                tree.slots.remove(index);

                let mut rpc = self.rpc_pool.get_connection().await?;
                // Wait until next eligible light slot is reached (until the start solana slot is reached)
                wait_until_slot_reached(
                    &mut *rpc,
                    &self.slot_tracker,
                    forester_slot.start_solana_slot,
                )
                .await?;

                let light_slot_timeout = {
                    let slot_length_u32 = u32::try_from(epoch_pda.protocol_config.slot_length)
                        .map_err(|_| ForesterError::Custom("Slot length overflow".into()))?;

                    slot_duration()
                        .checked_mul(slot_length_u32)
                        .ok_or_else(|| {
                            ForesterError::Custom("Timeout calculation overflow".into())
                        })?
                };

                if tree.tree_accounts.tree_type == TreeType::BatchedState {
                    let start_time = Instant::now();
                    info!("Processing batched state operations");

                    let rpc_pool = self.rpc_pool.clone();
                    let indexer = self.indexer.clone();
                    let payer = self.config.payer_keypair.insecure_clone();
                    let derivation = self.config.derivation_pubkey;
                    let merkle_tree = tree.tree_accounts.merkle_tree;
                    let queue = tree.tree_accounts.queue;

                    // TODO: measure & spawn child task for processing batched state operations
                    let processed_count = process_batched_operations(
                        rpc_pool,
                        indexer,
                        payer,
                        derivation,
                        epoch_info.epoch,
                        merkle_tree,
                        queue,
                    )
                    .await?;
                    info!("Processed {} batched state operations", processed_count);
                    queue_metric_update(epoch_info.epoch, 1, start_time.elapsed()).await;
                    self.increment_processed_items_count(epoch_info.epoch, processed_count)
                        .await;
                } else if tree.tree_accounts.tree_type == TreeType::BatchedAddress {
                    let start_time = Instant::now();
                    println!("Processing batched address operations");

                    let rpc_pool = self.rpc_pool.clone();
                    let indexer = self.indexer.clone();
                    let payer = self.config.payer_keypair.insecure_clone();
                    let derivation = self.config.derivation_pubkey;
                    let merkle_tree = tree.tree_accounts.merkle_tree;
                    let queue = tree.tree_accounts.queue;

                    // TODO: measure & spawn child task for processing batched state operations
                    let processed_count = process_batched_address_operations(
                        rpc_pool,
                        indexer,
                        payer,
                        derivation,
                        epoch_info.epoch,
                        merkle_tree,
                        queue,
                    )
                    .await?;

                    info!("Processed {} batched address operations", processed_count);
                    queue_metric_update(epoch_info.epoch, 1, start_time.elapsed()).await;
                    self.increment_processed_items_count(epoch_info.epoch, processed_count)
                        .await;
                } else {
                    // TODO: measure accuracy
                    // Optional replace with shutdown signal for all child processes
                    let batched_tx_config = SendBatchedTransactionsConfig {
                        num_batches: 10,
                        build_transaction_batch_config: BuildTransactionBatchConfig {
                            batch_size: 50, // TODO: make batch size configurable and or dynamic based on queue usage
                            compute_unit_price: None, // Make dynamic based on queue usage
                            compute_unit_limit: Some(1_000_000),
                        },
                        queue_config: self.config.queue_config,
                        retry_config: RetryConfig {
                            timeout: light_slot_timeout,
                            ..self.config.retry_config
                        },

                        light_slot_length: epoch_pda.protocol_config.slot_length,
                    };

                    let transaction_builder = EpochManagerTransactions {
                        indexer: self.indexer.clone(), // TODO: remove clone
                        epoch: epoch_info.epoch,
                        phantom: std::marker::PhantomData::<R>,
                    };

                    debug!("Sending transactions...");
                    let start_time = Instant::now();
                    let batch_tx_future = send_batched_transactions(
                        &self.config.payer_keypair,
                        &self.config.derivation_pubkey,
                        self.rpc_pool.clone(),
                        &batched_tx_config, // TODO: define config in epoch manager
                        tree.tree_accounts,
                        &transaction_builder,
                    );

                    // Check whether the tree is ready for rollover once per slot.
                    let future = self.rollover_if_needed(&tree.tree_accounts);

                    // Wait for both operations to complete
                    let (num_tx_sent, rollover_result) = tokio::join!(batch_tx_future, future);
                    rollover_result?;

                    match num_tx_sent {
                        Ok(num_tx_sent) => {
                            debug!("Transactions sent successfully");
                            let chunk_duration = start_time.elapsed();
                            queue_metric_update(epoch_info.epoch, num_tx_sent, chunk_duration)
                                .await;
                            self.increment_processed_items_count(epoch_info.epoch, num_tx_sent)
                                .await;
                        }
                        Err(e) => {
                            error!("Failed to send transactions: {:?}", e);
                        }
                    }
                }
            } else {
                debug!("No eligible slot found");
                // The forester is not eligible for any more slots in the current epoch
                break;
            }

            push_metrics(&self.config.external_services.pushgateway_url).await?;

            // Yield to allow other tasks to run
            tokio::task::yield_now().await;

            estimated_slot = self.slot_tracker.estimated_current_slot();

            debug!(
                "Estimated slot: {}, epoch end: {}",
                estimated_slot, epoch_info.phases.active.end
            );
        }
        Ok(())
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
        let mut rpc =
            SolanaRpcConnection::new(self.config.external_services.rpc_url.as_str(), None);

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
                info!("Work reported successfully");
            }
            Err(e) => {
                if let RpcError::ClientError(client_error) = &e {
                    if let Some(TransactionError::InstructionError(
                        _,
                        InstructionError::Custom(error_code),
                    )) = client_error.get_transaction_error()
                    {
                        let reported_work_code = RegistryError::ForesterAlreadyReportedWork as u32;
                        let not_in_report_work_phase_code =
                            RegistryError::NotInReportWorkPhase as u32;

                        if error_code == reported_work_code {
                            info!("Work already reported for this epoch. Skipping.");
                            return Ok(());
                        } else if error_code == not_in_report_work_phase_code {
                            warn!("Not in report work phase. Skipping report.");
                            return Ok(());
                        } else {
                            // Log other registry errors but still return an Err
                            warn!("Registry error encountered: {:?}", client_error);
                        }
                    }
                }
                return Err(ForesterError::from(e));
            }
        }

        let report = WorkReport {
            epoch: epoch_info.epoch.epoch,
            processed_items: self.get_processed_items_count(epoch_info.epoch.epoch).await,
        };

        self.work_report_sender
            .send(report)
            .await
            .map_err(|e| ForesterError::Custom(format!("Failed to send work report: {}", e)))?;

        info!("Work reported");
        Ok(())
    }

    async fn perform_rollover(&self, tree_account: &TreeAccounts) -> Result<()> {
        let mut rpc = self.rpc_pool.get_connection().await?;
        let (_, current_epoch) = self.get_current_slot_and_epoch().await?;

        let result = match tree_account.tree_type {
            TreeType::Address => {
                rollover_address_merkle_tree(
                    self.config.clone(),
                    &mut *rpc,
                    self.indexer.clone(),
                    tree_account,
                    current_epoch,
                )
                .await
            }
            TreeType::State => {
                rollover_state_merkle_tree(
                    self.config.clone(),
                    &mut *rpc,
                    self.indexer.clone(),
                    tree_account,
                    current_epoch,
                )
                .await
            }
            _ => panic!(
                "perform rollover: Invalid tree type {:?}",
                tree_account.tree_type
            ),
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

#[instrument(
    level = "info",
    skip(config, protocol_config, rpc_pool, indexer, shutdown, work_report_sender, slot_tracker),
    fields(forester = %config.payer_keypair.pubkey())
)]
pub async fn run_service<R: RpcConnection, I: Indexer<R>>(
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
    slot_tracker: Arc<SlotTracker>,
) -> Result<()> {
    info_span!("run_service", forester = %config.payer_keypair.pubkey())
        .in_scope(|| async {
            const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
            const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

            let mut retry_count = 0;
            let mut retry_delay = INITIAL_RETRY_DELAY;
            let start_time = Instant::now();

            let trees = {
                let rpc = rpc_pool.get_connection().await?;
                fetch_trees(&*rpc).await?
            };
            info!("Fetched initial trees: {:?}", trees);

            let (new_tree_sender, _) = broadcast::channel(100);

            let mut tree_finder = TreeFinder::new(
                rpc_pool.clone(),
                trees.clone(),
                new_tree_sender.clone(),
                Duration::from_secs(config.general_config.tree_discovery_interval_seconds),
            );

            let _tree_finder_handle = tokio::spawn(async move {
                if let Err(e) = tree_finder.run().await {
                    error!("Tree finder error: {:?}", e);
                }
            });

            while retry_count < config.retry_config.max_retries {
                debug!("Creating EpochManager (attempt {})", retry_count + 1);
                match EpochManager::new(
                    config.clone(),
                    protocol_config.clone(),
                    rpc_pool.clone(),
                    indexer.clone(),
                    work_report_sender.clone(),
                    trees.clone(),
                    slot_tracker.clone(),
                    new_tree_sender.clone(),
                )
                .await
                {
                    Ok(epoch_manager) => {
                        let epoch_manager: Arc<EpochManager<R, I>> = Arc::new(epoch_manager);
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
                            return Err(ForesterError::Custom(format!(
                                "Failed to start forester after {} attempts: {:?}",
                                config.retry_config.max_retries, e
                            )));
                        }
                    }
                }
            }

            Err(ForesterError::Custom(
                "Unexpected error: Retry loop exited without returning".to_string(),
            ))
        })
        .await
}
