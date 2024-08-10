use crate::errors::ForesterError;
use crate::rollover::{
    is_tree_ready_for_rollover, rollover_address_merkle_tree, rollover_state_merkle_tree,
};
use crate::tree_data_sync::fetch_trees;
use crate::{ForesterConfig, ForesterEpochInfo, RpcPool};
use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
    STATE_MERKLE_TREE_CHANGELOG,
};
use account_compression::QueueAccount;
use futures::future::join_all;
use futures::StreamExt;
use light_hash_set::HashSet;
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use light_registry::protocol_config::state::ProtocolConfig;
use light_registry::sdk::{
    create_finalize_registration_instruction, create_report_work_instruction,
};
use light_registry::ForesterEpochPda;
use light_test_utils::forester_epoch::{
    get_epoch_phases, Epoch, TreeAccounts, TreeForesterSchedule, TreeType,
};
use light_test_utils::indexer::{Indexer, MerkleProof, NewAddressProofWithContext};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, error, info, warn};
use rand::Rng;
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::iter::Zip;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{mem, thread};
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot, Mutex, Semaphore};
use tokio::time::sleep;

type Result<T> = std::result::Result<T, ForesterError>;

#[derive(Debug, Clone)]
pub struct QueueItemData {
    pub hash: [u8; 32],
    pub index: usize,
}

pub async fn fetch_queue_item_data<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    queue_pubkey: &Pubkey,
) -> Result<Vec<QueueItemData>> {
    debug!("Fetching queue data for {:?}", queue_pubkey);

    let mut rpc = rpc.lock().await;
    let mut account = rpc.get_account(*queue_pubkey).await?.unwrap();
    let nullifier_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };
    let mut queue_item_data_list = Vec::new();

    for i in 0..nullifier_queue.capacity {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                queue_item_data_list.push(QueueItemData {
                    hash: bucket.value_bytes(),
                    index: i,
                });
            }
        }
    }

    Ok(queue_item_data_list)
}

pub async fn get_queue_length<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    queue_pubkey: &Pubkey,
) -> usize {
    let queue = fetch_queue_item_data(rpc, queue_pubkey).await.unwrap();
    queue.len()
}

#[derive(Debug)]
struct QueueUpdate {
    pubkey: Pubkey,
    slot: u64,
}

#[derive(Clone, Debug)]
pub struct WorkReport {
    pub epoch: u64,
    pub processed_items: usize,
}

#[derive(Clone, Debug)]
struct EpochManager<R: RpcConnection, I: Indexer<R>> {
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<RpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    work_report_sender: mpsc::Sender<WorkReport>,
    processed_items_per_epoch_count: Arc<Mutex<HashMap<u64, AtomicUsize>>>,
    trees: Vec<TreeAccounts>,
}

#[derive(Debug, Clone)]
struct WorkItem {
    tree_account: TreeAccounts,
    queue_item_data: QueueItemData,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum Proof {
    AddressProof(NewAddressProofWithContext),
    StateProof(MerkleProof),
}

impl<R: RpcConnection, I: Indexer<R>> EpochManager<R, I> {
    pub async fn new(
        config: Arc<ForesterConfig>,
        protocol_config: Arc<ProtocolConfig>,
        rpc_pool: Arc<RpcPool<R>>,
        indexer: Arc<Mutex<I>>,
        work_report_sender: mpsc::Sender<WorkReport>,
        trees: Vec<TreeAccounts>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            protocol_config,
            rpc_pool,
            indexer,
            work_report_sender,
            processed_items_per_epoch_count: Arc::new(Mutex::new(HashMap::new())),
            trees,
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
            if last_epoch.is_none() || current_epoch > last_epoch.unwrap() {
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
            let rpc = self.rpc_pool.get_connection().await;
            let mut slot = rpc.lock().await.get_slot().await?;
            let slots_to_wait = next_phases.registration.start.saturating_sub(slot);
            info!(
                "Waiting for epoch {} registration phase to start. Current slot: {}, Registration phase start slot: {}, Slots to wait: {}",
                next_epoch, slot, next_phases.registration.start, slots_to_wait
            );
            let sleep_duration = Duration::from_millis(400 * slots_to_wait);
            debug!("Sleeping for {} ms", sleep_duration.as_millis());
            tokio::time::sleep(sleep_duration).await;
            while slot < next_phases.registration.start {
                tokio::time::sleep(Duration::from_millis(400)).await;
                slot = rpc.lock().await.get_slot().await?;
                debug!(
                    "Current slot: {}, Registration phase start slot: {}",
                    slot, next_phases.registration.start
                );
            }
        }
    }

    async fn get_processed_items_count(&self, epoch: u64) -> usize {
        let counts = self.processed_items_per_epoch_count.lock().await;
        counts
            .get(&epoch)
            .map_or(0, |count| count.load(Ordering::Relaxed))
    }

    async fn increment_processed_items_count(&self, epoch: u64) {
        let mut counts = self.processed_items_per_epoch_count.lock().await;
        counts
            .entry(epoch)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    async fn process_epoch(&self, epoch: u64) -> Result<()> {
        debug!("Processing epoch: {}", epoch);

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

        debug!("Completed processing epoch: {}", epoch);
        Ok(())
    }

    async fn get_current_slot_and_epoch(&self) -> Result<(u64, u64)> {
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        let slot = rpc_guard.get_slot().await?;
        Ok((slot, self.protocol_config.get_current_epoch(slot)))
    }

    async fn register_for_epoch(&self, epoch: u64) -> Result<ForesterEpochInfo> {
        debug!("Registering for epoch: {}", epoch);
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;

        let slot = rpc_guard.get_slot().await?;
        let phases = get_epoch_phases(&self.protocol_config, epoch);

        if slot < phases.registration.end {
            // TODO: check if we're already registered
            /*
            let (forester_epoch_pda_pubkey, _) = Pubkey::find_program_address(
                &[
                    b"forester_epoch",
                    &epoch.to_le_bytes(),
                    &self.config.payer_keypair.pubkey().to_bytes(),
                ],
                &light_registry::id(),
            );

            let existing_registration = rpc_guard
                .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?;

            if let Some(existing_pda) = existing_registration {
                info!("Already registered for epoch {}. Recovering registration info.", epoch);
                let registration_info = self.recover_registration_info(epoch, existing_pda).await?;
                return Ok(registration_info);
            }
             */

            let registration_info = {
                debug!("Registering epoch {}", epoch);
                let registered_epoch = match Epoch::register(
                    &mut *rpc_guard,
                    &self.protocol_config,
                    &self.config.payer_keypair,
                )
                .await
                {
                    Ok(Some(epoch)) => epoch,
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

                let forester_epoch_pda = match rpc_guard
                    .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
                    .await
                {
                    Ok(Some(pda)) => pda,
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

                ForesterEpochInfo {
                    epoch: registered_epoch,
                    epoch_pda: forester_epoch_pda,
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

    // TODO: implement
    #[allow(dead_code)]
    async fn recover_registration_info(
        &self,
        _epoch: u64,
        _existing_pda: ForesterEpochPda,
    ) -> Result<ForesterEpochInfo> {
        unimplemented!()
        // let rpc = self.rpc_pool.get_connection().await;
        //
        // let registration_info = ForesterEpochInfo {
        //     epoch: ...,
        //     epoch_pda: existing_pda,
        //     trees: ...,
        // };
        // Ok(registration_info)
    }

    async fn wait_for_active_phase(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> Result<ForesterEpochInfo> {
        debug!(
            "Waiting for active phase of epoch: {}",
            epoch_info.epoch.epoch
        );
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        let mut slot = rpc_guard.get_slot().await?;
        let active_phase_start_slot = epoch_info.epoch.phases.active.start;

        if slot < active_phase_start_slot {
            let sleep_ms = 400 * (active_phase_start_slot - slot);
            debug!("Sleeping for {} ms", sleep_ms);
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            slot = rpc_guard.get_slot().await?;
        }

        while slot < active_phase_start_slot {
            tokio::time::sleep(Duration::from_millis(400)).await;
            slot = rpc_guard.get_slot().await?;
            debug!(
                "Current slot: {}, Active phase start slot: {}",
                slot, active_phase_start_slot
            );
        }

        // TODO: we can put this ix into every tx of the first batch of the current active phase
        let ix = create_finalize_registration_instruction(
            &self.config.payer_keypair.pubkey(),
            epoch_info.epoch.epoch,
        );
        rpc_guard
            .create_and_send_transaction(
                &[ix],
                &self.config.payer_keypair.pubkey(),
                &[&self.config.payer_keypair],
            )
            .await?;

        let mut epoch_info = (*epoch_info).clone();
        epoch_info.epoch_pda = rpc_guard
            .get_anchor_account::<ForesterEpochPda>(&epoch_info.epoch.forester_epoch_pda)
            .await?
            .ok_or_else(|| ForesterError::Custom("Failed to get ForesterEpochPda".to_string()))?;

        epoch_info.add_trees_with_schedule(&self.trees, slot);
        Ok(epoch_info)
    }

    async fn perform_active_work(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        debug!("Performing work for epoch: {}", epoch_info.epoch.epoch);

        let rpc = self.rpc_pool.get_connection().await;
        let mut slot = rpc.lock().await.get_slot().await?;

        debug!("Initial slot: {}", slot);

        let queue_pubkeys: std::collections::HashSet<Pubkey> = epoch_info
            .trees
            .iter()
            .map(|tree| tree.tree_accounts.queue)
            .collect();

        // Create a channel for receiving queue updates
        let (update_tx, mut update_rx) = mpsc::channel(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        // Spawn PubsubClient task
        let ws_url = self.config.external_services.ws_rpc_url.clone();

        thread::spawn(move || {
            let rt = Builder::new_current_thread().enable_all().build().unwrap();

            rt.block_on(async {
                let pubsub_client = PubsubClient::new(&ws_url).await.unwrap();

                // Subscribe to all accounts
                let (mut subscription, _) = pubsub_client
                    .program_subscribe(
                        &account_compression::id(),
                        Some(RpcProgramAccountsConfig {
                            filters: None,
                            account_config: RpcAccountInfoConfig {
                                encoding: Some(UiAccountEncoding::Base64),
                                commitment: Some(CommitmentConfig::confirmed()),
                                data_slice: None,
                                min_context_slot: None,
                            },
                            with_context: Some(true),
                        }),
                    )
                    .await
                    .unwrap();

                loop {
                    tokio::select! {
                        Some(update) = subscription.next() => {
                            if let Ok(pubkey) = Pubkey::from_str(&update.value.pubkey) {
                                if queue_pubkeys.contains(&pubkey) && update_tx.send(QueueUpdate {
                                        pubkey,
                                        slot: update.context.slot,
                                    }).await.is_err() {
                                        break;
                                }

                            }
                        }
                        _ = shutdown_rx.recv() => {
                            break;
                        }
                    }
                }
            });
        });

        // Perform initial fetch and processing
        if self.is_in_active_phase(slot, epoch_info)? {
            self.process_queues(epoch_info).await?;
        } else {
            debug!("Not in active phase, skipping initial queue processing");
            return Ok(());
        }

        let mut last_processed_slot = slot;

        while self.is_in_active_phase(slot, epoch_info)? {
            tokio::select! {
                Some(update) = update_rx.recv() => {
                    if update.slot > last_processed_slot {
                        if self.is_in_active_phase(update.slot, epoch_info)? {
                            self.process_queue(epoch_info, update.pubkey).await?;
                            last_processed_slot = update.slot;
                        }
                        else {
                            debug!("Active phase has ended, stopping queue processing");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(400)) => {
                    slot = rpc.lock().await.get_slot().await?;
                    debug!("Updated slot: {}", slot);
                }
            }
        }

        // Cleanup
        let _ = shutdown_tx.send(()).await;

        for tree in &epoch_info.trees {
            if is_tree_ready_for_rollover(
                rpc.clone(),
                tree.tree_accounts.merkle_tree,
                tree.tree_accounts.tree_type,
            )
            .await?
            {
                self.perform_rollover(&tree.tree_accounts).await?;
            }
        }

        debug!(
            "Completed active work for epoch: {}",
            epoch_info.epoch.epoch
        );
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

    async fn process_queues(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        for tree in &epoch_info.trees {
            self.process_queue(epoch_info, tree.tree_accounts.queue)
                .await?;
        }
        Ok(())
    }

    async fn process_queue(
        &self,
        epoch_info: &ForesterEpochInfo,
        queue_pubkey: Pubkey,
    ) -> Result<()> {
        let rpc = self.rpc_pool.get_connection().await;
        let current_slot = rpc.lock().await.get_slot().await?;
        if !self.is_in_active_phase(current_slot, epoch_info)? {
            debug!("Not in active phase, skipping queue processing");
            return Ok(());
        }
        let tree = epoch_info
            .trees
            .iter()
            .find(|t| t.tree_accounts.queue == queue_pubkey)
            .ok_or_else(|| ForesterError::Custom("Tree not found for queue".to_string()))?;

        let work_items = self.fetch_work_items(rpc.clone(), &[tree.clone()]).await?;
        if work_items.is_empty() {
            return Ok(());
        }

        debug!(
            "Processing {} work items for queue {:?}",
            work_items.len(),
            tree.tree_accounts.queue
        );

        let semaphore = Arc::new(Semaphore::new(self.config.indexer_max_concurrent_batches));
        let (tx, mut rx) = mpsc::channel(self.config.indexer_max_concurrent_batches);

        for chunk in work_items.chunks(self.config.indexer_batch_size) {
            let semaphore_clone = semaphore.clone();
            let tx_clone = tx.clone();
            let epoch_info_clone = epoch_info.clone();
            let self_clone = self.clone();
            let chunk = chunk.to_vec();

            tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                let start_time = Instant::now();
                let result = self_clone
                    .process_work_items(&epoch_info_clone, &chunk)
                    .await;
                let duration = start_time.elapsed();
                tx_clone.send((result, duration)).await.unwrap();
            });
        }

        drop(tx);

        let mut completed_chunks = 0;
        let total_chunks = (work_items.len() + self.config.indexer_batch_size - 1)
            / self.config.indexer_batch_size;
        let mut total_transactions = 0;
        let mut total_duration = Duration::new(0, 0);

        while let Some((result, duration)) = rx.recv().await {
            completed_chunks += 1;
            match result {
                Ok(signatures) => {
                    let num_transactions = signatures.len();
                    total_transactions += num_transactions;
                    total_duration += duration;
                    let chunk_tps = num_transactions as f64 / duration.as_secs_f64();
                    let avg_tps = total_transactions as f64 / total_duration.as_secs_f64();

                    for (idx, signature) in signatures.iter().enumerate() {
                        debug!(
                            "Transaction {} in chunk {} processed: {:?}",
                            idx, completed_chunks, signature
                        );
                    }
                    debug!(
                        "Chunk {} TPS: {:.2}, Average TPS: {:.2}",
                        completed_chunks, chunk_tps, avg_tps
                    );
                }
                Err(e) => {
                    error!("Error processing work item chunk: {:?}", e);
                }
            }
            debug!("Completed {}/{} chunks", completed_chunks, total_chunks);
        }

        if total_duration.as_secs_f64() > 0.0 {
            let overall_avg_tps = total_transactions as f64 / total_duration.as_secs_f64();
            debug!("Overall average TPS: {:.2}", overall_avg_tps);
        }

        Ok(())
    }

    async fn fetch_work_items(
        &self,
        rpc: Arc<Mutex<R>>,
        trees: &[TreeForesterSchedule],
    ) -> Result<Vec<WorkItem>> {
        let mut work_items = Vec::new();

        for tree in trees {
            let queue_item_data =
                fetch_queue_item_data(rpc.clone(), &tree.tree_accounts.queue).await?;
            for data in queue_item_data {
                work_items.push(WorkItem {
                    tree_account: tree.tree_accounts,
                    queue_item_data: data,
                });
            }
        }

        Ok(work_items)
    }

    async fn process_work_items(
        &self,
        epoch_info: &ForesterEpochInfo,
        work_items: &[WorkItem],
    ) -> Result<Vec<Signature>> {
        let mut results = Vec::new();
        let semaphore = Arc::new(Semaphore::new(
            self.config.transaction_max_concurrent_batches,
        ));

        let total_start_time = Instant::now();
        let mut total_transactions = 0;
        let mut total_processing_time = Duration::new(0, 0);

        for (chunk_index, indexer_chunk) in work_items
            .chunks(self.config.transaction_batch_size)
            .enumerate()
        {
            let chunk_start_time = Instant::now();
            debug!(
                "Processing chunk {} of size: {}",
                chunk_index,
                indexer_chunk.len()
            );
            let rpc = self.rpc_pool.get_connection().await;
            let current_slot = rpc.lock().await.get_slot().await?;
            if !self.is_in_active_phase(current_slot, epoch_info)? {
                debug!("Not in active phase, skipping process_work_items");
                return Err(ForesterError::Custom("Not in active phase".to_string()));
            }

            let (proofs, all_instructions) = self
                .fetch_proofs_and_create_instructions(epoch_info, indexer_chunk)
                .await?;

            let (tx, mut rx) = mpsc::channel(self.config.transaction_max_concurrent_batches);

            let batch_futures: Vec<_> = Zip::enumerate(
                all_instructions
                    .chunks(self.config.transaction_batch_size)
                    .zip(proofs.chunks(self.config.transaction_batch_size)),
            )
            .map(|(_, (transaction_chunk, proof_chunk))| {
                let epoch_info = epoch_info.clone();
                let self_clone = self.clone();
                let transaction_chunk = transaction_chunk.to_vec();
                let proof_chunk = proof_chunk.to_vec();
                let indexer_chunk = indexer_chunk.to_vec();
                let semaphore_clone = semaphore.clone();
                let tx_clone = tx.clone();

                tokio::spawn(async move {
                    let _permit = semaphore_clone.acquire().await.unwrap();
                    let start_time = Instant::now();

                    let result = self_clone
                        .process_transaction_batch_with_retry(
                            &epoch_info,
                            &transaction_chunk,
                            &proof_chunk,
                            &indexer_chunk,
                        )
                        .await;

                    let duration = start_time.elapsed();
                    tx_clone.send((result.clone(), duration)).await.unwrap();

                    result
                })
            })
            .collect();

            drop(tx);

            let mut chunk_transactions = 0;
            let mut chunk_processing_time = Duration::new(0, 0);

            while let Some((result, duration)) = rx.recv().await {
                match result {
                    Ok(signature) => {
                        results.push(signature);
                        chunk_transactions += 1;
                        chunk_processing_time += duration;
                        let batch_tps = 1.0 / duration.as_secs_f64();
                        debug!("Batch processed successfully. TPS: {:.2}", batch_tps);
                    }
                    Err(e) => {
                        error!("Error processing batch: {:?}", e);
                    }
                }
            }

            join_all(batch_futures).await;

            total_transactions += chunk_transactions;
            total_processing_time += chunk_processing_time;

            let chunk_duration = chunk_start_time.elapsed();
            let chunk_tps = chunk_transactions as f64 / chunk_duration.as_secs_f64();
            let chunk_processing_tps =
                chunk_transactions as f64 / chunk_processing_time.as_secs_f64();
            let total_tps = total_transactions as f64 / total_start_time.elapsed().as_secs_f64();
            let total_processing_tps =
                total_transactions as f64 / total_processing_time.as_secs_f64();

            info!(
                "Chunk {} completed: {} transactions in {:.2?}",
                chunk_index, chunk_transactions, chunk_duration
            );
            info!(
                "Chunk {} TPS: {:.2} (overall: {:.2}), Processing TPS: {:.2} (overall: {:.2})",
                chunk_index, chunk_tps, total_tps, chunk_processing_tps, total_processing_tps
            );
        }

        let total_duration = total_start_time.elapsed();
        let overall_tps = total_transactions as f64 / total_duration.as_secs_f64();
        let overall_processing_tps =
            total_transactions as f64 / total_processing_time.as_secs_f64();

        info!(
            "Overall: {} transactions in {:.2?}",
            total_transactions, total_duration
        );
        info!(
            "Overall TPS: {:.2}, Processing TPS: {:.2}",
            overall_tps, overall_processing_tps
        );

        let results = results.into_iter().flatten().collect();
        Ok(results)
    }

    async fn check_eligibility(
        &self,
        registration_info: &ForesterEpochInfo,
        tree_account: &TreeAccounts,
    ) -> Result<bool> {
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        let current_slot = rpc_guard.get_slot().await?;
        let forester_epoch_pda = rpc_guard
            .get_anchor_account::<ForesterEpochPda>(&registration_info.epoch.forester_epoch_pda)
            .await?
            .ok_or_else(|| {
                ForesterError::Custom("Forester epoch PDA fetching error".to_string())
            })?;
        drop(rpc_guard);

        let light_slot = forester_epoch_pda
            .get_current_light_slot(current_slot)
            .map_err(|e| {
                ForesterError::Custom(format!("Failed to get current light slot: {}", e))
            })?;

        let tree_schedule = registration_info
            .trees
            .iter()
            .find(|ts| ts.tree_accounts == *tree_account)
            .ok_or_else(|| {
                ForesterError::Custom("No tree schedule found for the current tree".to_string())
            })?;

        debug!("tree_schedule: {:?}", tree_schedule);
        debug!(
            "Checking eligibility for tree {:?} at light slot {} / solana slot {}",
            tree_account.merkle_tree, light_slot, current_slot
        );
        debug!(
            "tree_schedule.slots[{}] = {:?}",
            light_slot, tree_schedule.slots[light_slot as usize]
        );
        Ok(tree_schedule.is_eligible(light_slot))
    }

    async fn process_transaction_batch_with_retry(
        &self,
        epoch_info: &ForesterEpochInfo,
        transaction_chunk: &[Instruction],
        proof_chunk: &[Proof],
        indexer_chunk: &[WorkItem],
    ) -> Result<Option<Signature>> {
        let first_work_item = indexer_chunk.first();
        if first_work_item.is_none() {
            return Err(ForesterError::Custom("Empty indexer chunk".to_string()));
        }
        let work_item = first_work_item.unwrap();
        debug!(
            "Processing work item {:?} with {} instructions",
            work_item.queue_item_data.hash,
            transaction_chunk.len()
        );
        const BASE_RETRY_DELAY: Duration = Duration::from_millis(100);

        let mut retries = 0;
        loop {
            if !self
                .check_eligibility(epoch_info, &work_item.tree_account)
                .await?
            {
                debug!("Forester not eligible for this slot, skipping batch");
                return Ok(None);
            }

            match self
                .process_transaction_batch(
                    epoch_info,
                    transaction_chunk,
                    proof_chunk,
                    indexer_chunk,
                )
                .await
            {
                Ok(signature) => {
                    debug!(
                        "Work item {:?} processed successfully. Signature: {:?}",
                        work_item.queue_item_data.hash, signature
                    );
                    self.increment_processed_items_count(epoch_info.epoch.epoch)
                        .await;
                    return Ok(Some(signature));
                }
                Err(e) => {
                    if retries >= self.config.max_retries {
                        error!(
                            "Max retries reached for work item {:?}. Error: {:?}",
                            work_item.queue_item_data.hash, e
                        );
                        return Err(e);
                    }
                    let delay = BASE_RETRY_DELAY * 2u32.pow(retries as u32);
                    let jitter = rand::thread_rng().gen_range(0..=50);
                    sleep(delay + Duration::from_millis(jitter)).await;
                    retries += 1;
                    warn!(
                        "Retrying work item {:?}. Attempt {}/{}",
                        work_item.queue_item_data.hash, retries, self.config.max_retries
                    );
                }
            }
        }
    }

    async fn process_transaction_batch(
        &self,
        epoch_info: &ForesterEpochInfo,
        instructions: &[Instruction],
        proofs: &[Proof],
        work_items: &[WorkItem],
    ) -> Result<Signature> {
        debug!(
            "Processing transaction batch with {} instructions",
            instructions.len()
        );
        let rpc = self.rpc_pool.get_connection().await;
        let current_slot = rpc.lock().await.get_slot().await?;
        if !self.is_in_active_phase(current_slot, epoch_info)? {
            debug!("Not in active phase, skipping queue processing");
            return Err(ForesterError::Custom("Not in active phase".to_string()));
        }
        let mut rpc_guard = rpc.lock().await;
        let recent_blockhash = rpc_guard.get_latest_blockhash().await?;
        drop(rpc_guard);

        let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.cu_limit,
        )];
        ixs.extend_from_slice(instructions);
        let mut transaction =
            Transaction::new_with_payer(&ixs, Some(&self.config.payer_keypair.pubkey()));
        transaction.sign(&[&self.config.payer_keypair], recent_blockhash);

        let mut rpc_guard = rpc.lock().await;

        // TODO: replace it with send, do not wait for confirmation and wait for confirmation on another thread
        // we need to introduce retry on timeout when confirmation is not received
        let signature = rpc_guard.process_transaction(transaction).await?;
        drop(rpc_guard);

        self.update_indexer(work_items, proofs).await;

        Ok(signature)
    }

    async fn update_indexer(&self, work_items: &[WorkItem], proofs: &[Proof]) {
        let mut indexer = self.indexer.lock().await;
        for (work_item, proof) in work_items.iter().zip(proofs.iter()) {
            match proof {
                Proof::AddressProof(address_proof) => {
                    indexer.address_tree_updated(work_item.tree_account.merkle_tree, address_proof);
                }
                Proof::StateProof(state_proof) => {
                    indexer
                        .account_nullified(work_item.tree_account.merkle_tree, &state_proof.hash);
                }
            }
        }
    }

    async fn wait_for_report_work_phase(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;

        let report_work_start_slot = epoch_info.epoch.phases.report_work.start;
        let mut slot = rpc_guard.get_slot().await?;
        debug!(
            "Current slot: {}, Report work start slot: {}",
            slot, report_work_start_slot
        );

        if slot < report_work_start_slot {
            let sleep_ms = 400 * (report_work_start_slot - slot);
            debug!("Sleeping for {} ms", sleep_ms);
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            slot = rpc_guard.get_slot().await?;
        }

        while slot < report_work_start_slot {
            tokio::time::sleep(Duration::from_millis(400)).await;
            slot = rpc_guard.get_slot().await?;
            debug!(
                "Current slot: {}, Report work start slot: {}",
                slot, report_work_start_slot
            );
        }

        Ok(())
    }

    async fn report_work(&self, epoch_info: &ForesterEpochInfo) -> Result<()> {
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;

        let ix = create_report_work_instruction(
            &self.config.payer_keypair.pubkey(),
            epoch_info.epoch.epoch,
        );
        rpc_guard
            .create_and_send_transaction(
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

        Ok(())
    }

    async fn fetch_proofs_and_create_instructions(
        &self,
        registration_info: &ForesterEpochInfo,
        work_items: &[WorkItem],
    ) -> Result<(Vec<Proof>, Vec<Instruction>)> {
        let mut proofs = Vec::new();
        let mut instructions = vec![];

        let (address_items, state_items): (Vec<_>, Vec<_>) = work_items
            .iter()
            .partition(|item| matches!(item.tree_account.tree_type, TreeType::Address));

        // Fetch address proofs in batch
        if !address_items.is_empty() {
            let merkle_tree = address_items[0].tree_account.merkle_tree.to_bytes();
            let addresses: Vec<[u8; 32]> = address_items
                .iter()
                .map(|item| item.queue_item_data.hash)
                .collect();
            let indexer = self.indexer.lock().await;
            let address_proofs = indexer
                .get_multiple_new_address_proofs(merkle_tree, addresses)
                .await?;
            drop(indexer);
            for (item, proof) in address_items.iter().zip(address_proofs.into_iter()) {
                proofs.push(Proof::AddressProof(proof.clone()));
                let instruction = create_update_address_merkle_tree_instruction(
                    UpdateAddressMerkleTreeInstructionInputs {
                        authority: self.config.payer_keypair.pubkey(),
                        address_merkle_tree: item.tree_account.merkle_tree,
                        address_queue: item.tree_account.queue,
                        value: item.queue_item_data.index as u16,
                        low_address_index: proof.low_address_index,
                        low_address_value: proof.low_address_value,
                        low_address_next_index: proof.low_address_next_index,
                        low_address_next_value: proof.low_address_next_value,
                        low_address_proof: proof.low_address_proof,
                        changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
                        indexed_changelog_index: (proof.root_seq
                            % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG)
                            as u16,
                        is_metadata_forester: false,
                    },
                    registration_info.epoch.epoch,
                );
                instructions.push(instruction);
            }
        }

        // Fetch state proofs in batch
        if !state_items.is_empty() {
            let states: Vec<String> = state_items
                .iter()
                .map(|item| bs58::encode(&item.queue_item_data.hash).into_string())
                .collect();
            let indexer = self.indexer.lock().await;
            let state_proofs = indexer
                .get_multiple_compressed_account_proofs(states)
                .await?;
            drop(indexer);
            for (item, proof) in state_items.iter().zip(state_proofs.into_iter()) {
                proofs.push(Proof::StateProof(proof.clone()));
                let instruction = create_nullify_instruction(
                    CreateNullifyInstructionInputs {
                        nullifier_queue: item.tree_account.queue,
                        merkle_tree: item.tree_account.merkle_tree,
                        change_log_indices: vec![proof.root_seq % STATE_MERKLE_TREE_CHANGELOG],
                        leaves_queue_indices: vec![item.queue_item_data.index as u16],
                        indices: vec![proof.leaf_index],
                        proofs: vec![proof.proof.clone()],
                        authority: self.config.payer_keypair.pubkey(),
                        derivation: self.config.payer_keypair.pubkey(),
                        is_metadata_forester: false,
                    },
                    registration_info.epoch.epoch,
                );
                instructions.push(instruction);
            }
        }

        Ok((proofs, instructions))
    }

    async fn perform_rollover(&self, tree_account: &TreeAccounts) -> Result<()> {
        let result = match tree_account.tree_type {
            TreeType::Address => {
                rollover_address_merkle_tree(
                    self.config.clone(),
                    self.rpc_pool.clone(),
                    self.indexer.clone(),
                    tree_account,
                )
                .await
            }
            TreeType::State => {
                rollover_state_merkle_tree(
                    self.config.clone(),
                    self.rpc_pool.clone(),
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
    rpc_pool: Arc<RpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
) -> Result<()> {
    const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
    const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

    let mut retry_count = 0;
    let mut retry_delay = INITIAL_RETRY_DELAY;
    let start_time = Instant::now();

    let trees = {
        let rpc = rpc_pool.get_connection().await;
        let rpc = rpc.lock().await;
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
                    info!("Retrying in {:?}", retry_delay);
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
}
