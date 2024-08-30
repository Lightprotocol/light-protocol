use crate::errors::ForesterError;
use crate::queue_helpers::QueueItemData;
use crate::rollover::{
    is_tree_ready_for_rollover, rollover_address_merkle_tree, rollover_state_merkle_tree,
};
use crate::rpc_pool::SolanaRpcPool;
use crate::send_transaction::{
    send_batched_transactions, BuildTransactionBatchConfig, EpochManagerTransactions, RetryConfig,
    SendBatchedTransactionsConfig,
};
use crate::slot_tracker::{wait_until_slot_reached, SlotTracker};
use crate::tree_data_sync::fetch_trees;
use crate::utils::get_current_system_time_ms;
use crate::Result;
use crate::{ForesterConfig, ForesterEpochInfo};

use crate::metrics::{process_queued_metrics, push_metrics, queue_metric_update};
use light_registry::protocol_config::state::ProtocolConfig;
use light_registry::sdk::{
    create_finalize_registration_instruction, create_report_work_instruction,
};
use light_registry::utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority};
use light_registry::{EpochPda, ForesterEpochPda};
use light_test_utils::forester_epoch::{
    get_epoch_phases, Epoch, TreeAccounts, TreeForesterSchedule, TreeType,
};
use light_test_utils::indexer::{Indexer, MerkleProof, NewAddressProofWithContext};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, Instant};
use tracing::{debug, error, info, info_span, instrument, warn};

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
    AddressProof(NewAddressProofWithContext),
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
    trees: Vec<TreeAccounts>,
    slot_tracker: Arc<SlotTracker>,
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
        }
    }
}

impl<R: RpcConnection, I: Indexer<R>> EpochManager<R, I> {
    pub async fn new(
        config: Arc<ForesterConfig>,
        protocol_config: Arc<ProtocolConfig>,
        rpc_pool: Arc<SolanaRpcPool<R>>,
        indexer: Arc<Mutex<I>>,
        work_report_sender: mpsc::Sender<WorkReport>,
        trees: Vec<TreeAccounts>,
        slot_tracker: Arc<SlotTracker>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            protocol_config,
            rpc_pool,
            indexer,
            work_report_sender,
            processed_items_per_epoch_count: Arc::new(Mutex::new(HashMap::new())),
            trees,
            slot_tracker,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        let monitor_handle = tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move { self_clone.monitor_epochs(tx).await }
        });

        while let Some(epoch) = rx.recv().await {
            let self_clone = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = self_clone.process_epoch(epoch).await {
                    error!("Error processing epoch {}: {:?}", epoch, e);
                }
            });
        }

        monitor_handle.await??;
        Ok(())
    }

    async fn monitor_epochs(&self, tx: mpsc::Sender<u64>) -> Result<()> {
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
            info!(
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

    #[instrument(level = "debug", skip(self), fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch
    ))]
    async fn process_epoch(&self, epoch: u64) -> Result<()> {
        info!("Entering process_epoch");

        // Registration
        let mut registration_info = self.register_for_epoch(epoch).await?;

        // Wait for active phase
        registration_info = self.wait_for_active_phase(&registration_info).await?;

        // Perform work
        self.perform_active_work(&registration_info).await?;

        // Wait for report work phase
        self.wait_for_report_work_phase(&registration_info).await?;

        // Report work
        self.report_work(&registration_info).await?;

        // TODO: implement
        // self.claim(&registration_info).await?;

        info!("Exiting process_epoch");
        Ok(())
    }

    async fn get_current_slot_and_epoch(&self) -> Result<(u64, u64)> {
        let slot = self.slot_tracker.estimated_current_slot();
        Ok((slot, self.protocol_config.get_current_epoch(slot)))
    }

    async fn register_for_epoch(&self, epoch: u64) -> Result<ForesterEpochInfo> {
        info!("Registering for epoch: {}", epoch);
        let mut rpc = self.rpc_pool.get_connection().await?;
        let slot = rpc.get_slot().await?;
        let phases = get_epoch_phases(&self.protocol_config, epoch);

        if slot < phases.registration.end {
            let forester_epoch_pda_pubkey =
                get_forester_epoch_pda_from_authority(&self.config.payer_keypair.pubkey(), epoch).0;
            let existing_registration = rpc
                .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?;

            if let Some(existing_pda) = existing_registration {
                info!(
                    "Already registered for epoch {}. Recovering registration info.",
                    epoch
                );
                let registration_info = self
                    .recover_registration_info(epoch, forester_epoch_pda_pubkey, existing_pda)
                    .await?;
                return Ok(registration_info);
            }

            let registration_info = {
                debug!("Registering epoch {}", epoch);
                let registered_epoch = match Epoch::register(
                    &mut *rpc,
                    &self.protocol_config,
                    &self.config.payer_keypair,
                )
                .await
                {
                    Ok(Some(epoch)) => {
                        info!("Registered epoch: {:?}", epoch);
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
                        info!("ForesterEpochPda: {:?}", pda);
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

    async fn recover_registration_info(
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
            &self.config.payer_keypair.pubkey(),
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
        epoch_info.add_trees_with_schedule(&self.trees, slot);
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
        info!("Entering perform_active_work");

        let current_slot = self.slot_tracker.estimated_current_slot();
        let active_phase_end = epoch_info.epoch.phases.active.end;

        debug!(
            current_slot,
            active_phase_end, "Checking if in active phase"
        );

        if self.is_in_active_phase(current_slot, epoch_info)? {
            debug!("Forester in active phase, processing initial queues");
        } else {
            debug!("Forester not in active phase, skipping initial queue processing");
            return Ok(());
        }

        // Sync estimated slot before creating threads.
        // Threads rely on the estimated slot.
        {
            let mut rpc = self.rpc_pool.get_connection().await?;
            let current_slot = rpc.get_slot().await?;
            self.slot_tracker.update(current_slot);
        }
        for tree in epoch_info.trees.iter() {
            info!("Creating thread for queue {}", tree.tree_accounts.queue);
            // TODO: inefficient try to only clone necessary fields
            let epoch_info_clone = epoch_info.clone();
            let self_clone = self.clone();
            let tree = tree.clone();
            // TODO: consider passing global shutdown signal (might be overkill since we have timeouts)
            tokio::spawn(async move {
                if let Err(e) = self_clone
                    .process_queue(
                        epoch_info_clone.epoch, // TODO: only clone the necessary fields
                        epoch_info_clone.forester_epoch_pda.clone(),
                        tree,
                    )
                    .await
                {
                    error!("Error processing queue: {:?}", e);
                }
            });
        }

        info!("Threads created. Waiting for active phase to end");

        let mut rpc = self.rpc_pool.get_connection().await?;
        wait_until_slot_reached(&mut *rpc, &self.slot_tracker, active_phase_end).await?;
        let estimated_slot = self.slot_tracker.estimated_current_slot();
        debug!(
            "Estimated current slot: {}, active phase end: {}",
            estimated_slot, active_phase_end
        );

        // TODO: move (Jorrit low prio)
        // Should be called every multiple times per epoch for every tree. It is
        // tricky because we need to fetch both the Merkle tree and the queue
        // (by default we just fetch the queue account).
        info!("Checking for rollover eligibility");
        for tree in &epoch_info.trees {
            let mut rpc = self.rpc_pool.get_connection().await?;
            if is_tree_ready_for_rollover(
                &mut *rpc,
                tree.tree_accounts.merkle_tree,
                tree.tree_accounts.tree_type,
            )
            .await?
            {
                self.perform_rollover(&tree.tree_accounts).await?;
            }
        }

        info!("Completed active work");
        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree),
        fields(forester = %self.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree.tree_accounts.merkle_tree)
    )]
    pub async fn process_queue(
        &self,
        epoch_info: Epoch,
        epoch_pda: ForesterEpochPda,
        mut tree: TreeForesterSchedule,
    ) -> Result<()> {
        info!("enter process_queue");
        info!("Tree schedule slots: {:?}", tree.slots);
        // TODO: sync at some point
        let mut estimated_slot = self.slot_tracker.estimated_current_slot();

        while estimated_slot < epoch_info.phases.active.end {
            info!("Processing queue");
            // search for next eligible slot
            let index_and_forester_slot = tree
                .slots
                .iter()
                .enumerate()
                .find(|(_, slot)| slot.is_some());

            info!("Result: {:?}", index_and_forester_slot);
            if let Some((index, forester_slot)) = index_and_forester_slot {
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

                // TODO: measure accuracy
                // Optional replace with shutdown signal for all child processes
                let solana_slot_len = 500;
                let global_timeout = get_current_system_time_ms()
                    + epoch_pda.protocol_config.slot_length as u128 * solana_slot_len;
                let config = SendBatchedTransactionsConfig {
                    num_batches: 10,
                    batch_time_ms: 1000, // TODO: make batch size configurable and or dynamic based on queue usage
                    build_transaction_batch_config: BuildTransactionBatchConfig {
                        batch_size: 50, // TODO: make batch size configurable and or dynamic based on queue usage
                        compute_unit_price: None, // Make dynamic based on queue usage
                        compute_unit_limit: None,
                    },
                    retry_config: RetryConfig {
                        max_retries: 10,          // TODO: make configurable
                        retry_wait_time_ms: 1000, // TODO: make configurable
                        global_timeout,
                    },
                };

                let transaction_builder = EpochManagerTransactions {
                    indexer: self.indexer.clone(), // TODO: remove clone
                    epoch: epoch_info.epoch,
                    phantom: std::marker::PhantomData::<R>,
                };

                let start_time = Instant::now();
                let num_tx_sent = send_batched_transactions(
                    &self.config.payer_keypair,
                    self.rpc_pool.clone(),
                    config, // TODO: define config in epoch manager
                    tree.tree_accounts,
                    &transaction_builder,
                    epoch_pda.epoch,
                )
                .await?;

                // Prometheus metrics
                let chunk_duration = start_time.elapsed();
                queue_metric_update(epoch_info.epoch, num_tx_sent, chunk_duration).await;

                // TODO: consider do we really need WorkReport
                self.increment_processed_items_count(epoch_info.epoch, num_tx_sent)
                    .await;
            } else {
                // The forester is not eligible for any more slots in the current epoch
                break;
            }

            process_queued_metrics().await;
            if let Err(e) = push_metrics(&self.config.external_services.pushgateway_url).await {
                error!("Failed to push metrics: {:?}", e);
            }

            estimated_slot = self.slot_tracker.estimated_current_slot();
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
        let mut rpc = self.rpc_pool.get_connection().await?;

        let ix = create_report_work_instruction(
            &self.config.payer_keypair.pubkey(),
            epoch_info.epoch.epoch,
        );
        rpc.create_and_send_transaction(
            &[ix],
            &self.config.payer_keypair.pubkey(),
            &[&self.config.payer_keypair],
        )
        .await?;

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
        let result = match tree_account.tree_type {
            TreeType::Address => {
                rollover_address_merkle_tree(
                    self.config.clone(),
                    &mut *rpc,
                    self.indexer.clone(),
                    tree_account,
                )
                .await
            }
            TreeType::State => {
                rollover_state_merkle_tree(
                    self.config.clone(),
                    &mut *rpc,
                    self.indexer.clone(),
                    tree_account,
                )
                .await
            }
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
                fetch_trees(&*rpc).await
            };

            while retry_count < config.max_retries {
                debug!("Creating EpochManager (attempt {})", retry_count + 1);
                match EpochManager::new(
                    config.clone(),
                    protocol_config.clone(),
                    rpc_pool.clone(),
                    indexer.clone(),
                    work_report_sender.clone(),
                    trees.clone(),
                    slot_tracker.clone(),
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
                        if retry_count < config.max_retries {
                            debug!("Retrying in {:?}", retry_delay);
                            sleep(retry_delay).await;
                            retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
                        } else {
                            error!(
                                "Failed to start forester after {} attempts over {:?}",
                                config.max_retries,
                                start_time.elapsed()
                            );
                            return Err(ForesterError::Custom(format!(
                                "Failed to start forester after {} attempts: {:?}",
                                config.max_retries, e
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
