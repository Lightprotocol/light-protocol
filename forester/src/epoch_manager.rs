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
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{mem, thread};
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct QueueData {
    pub hash: [u8; 32],
    pub index: usize,
}

pub async fn fetch_queue_data<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    queue_pubkey: &Pubkey,
) -> Result<Vec<QueueData>, ForesterError> {
    debug!("Fetching queue data for {:?}", queue_pubkey);

    let mut rpc = rpc.lock().await;
    let mut account = rpc.get_account(*queue_pubkey).await?.unwrap();
    let nullifier_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };
    let mut queue_data_list = Vec::new();

    for i in 0..nullifier_queue.capacity {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                queue_data_list.push(QueueData {
                    hash: bucket.value_bytes(),
                    index: i,
                });
            }
        }
    }
    Ok(queue_data_list)
}

pub async fn get_queue_length<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    queue_pubkey: &Pubkey,
) -> usize {
    let queue = fetch_queue_data(rpc, queue_pubkey).await.unwrap();
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

struct EpochManager<R: RpcConnection, I: Indexer<R>> {
    config: Arc<ForesterConfig>,
    protocol_config: Arc<ProtocolConfig>,
    rpc_pool: Arc<RpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    work_report_sender: mpsc::Sender<WorkReport>,
    processed_items_count: Mutex<HashMap<u64, AtomicUsize>>,
}

#[derive(Debug)]
struct WorkItem {
    tree_account: TreeAccounts,
    queue_data: QueueData,
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
    ) -> Result<Self, ForesterError> {
        Ok(Self {
            config,
            protocol_config,
            rpc_pool,
            indexer,
            work_report_sender,
            processed_items_count: Mutex::new(HashMap::new()),
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<(), ForesterError> {
        let (tx, mut rx) = mpsc::channel(100);

        let monitor_handle = {
            let self_clone: Arc<EpochManager<R, I>> = Arc::clone(&self);
            tokio::spawn(async move { self_clone.monitor_epochs(tx).await })
        };

        while let Some(epoch) = rx.recv().await {
            let self_clone: Arc<EpochManager<R, I>> = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = self_clone.process_epoch(epoch).await {
                    error!("Error processing epoch {}: {:?}", epoch, e);
                }
            });
        }

        monitor_handle.await??;
        Ok(())
    }

    async fn monitor_epochs(&self, tx: mpsc::Sender<u64>) -> Result<(), ForesterError> {
        let mut last_epoch: Option<u64> = None;
        debug!("Starting epoch monitor");
        let phases = get_epoch_phases(&self.protocol_config, 0);
        debug!("Phases: {:?}", phases);

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
        let counts = self.processed_items_count.lock().await;
        counts
            .get(&epoch)
            .map_or(0, |count| count.load(Ordering::Relaxed))
    }

    async fn increment_processed_items_count(&self, epoch: u64) {
        let mut counts = self.processed_items_count.lock().await;
        counts
            .entry(epoch)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    async fn process_epoch(&self, epoch: u64) -> Result<(), ForesterError> {
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

    async fn get_current_slot_and_epoch(&self) -> Result<(u64, u64), ForesterError> {
        let rpc = self.rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        let slot = rpc_guard.get_slot().await?;
        Ok((slot, self.protocol_config.get_current_epoch(slot)))
    }

    async fn register_for_epoch(&self, epoch: u64) -> Result<ForesterEpochInfo, ForesterError> {
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
    ) -> Result<ForesterEpochInfo, ForesterError> {
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
    ) -> Result<ForesterEpochInfo, ForesterError> {
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

        let trees = fetch_trees(&self.config.external_services.rpc_url).await;
        debug!(
            "Fetched trees for epoch {}: {:?}",
            epoch_info.epoch.epoch, trees
        );
        epoch_info.add_trees_with_schedule(trees, slot);

        Ok(epoch_info)
    }

    async fn perform_active_work(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> Result<(), ForesterError> {
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
        if self.is_in_active_phase(slot, epoch_info).await? {
            self.process_queues(epoch_info).await?;
        } else {
            debug!("Not in active phase, skipping initial queue processing");
            return Ok(());
        }

        let mut last_processed_slot = slot;

        while self.is_in_active_phase(slot, epoch_info).await? {
            tokio::select! {
                Some(update) = update_rx.recv() => {
                    if update.slot > last_processed_slot {
                        if self.is_in_active_phase(update.slot, epoch_info).await? {
                            self.process_queue(epoch_info, update.pubkey).await?;
                            last_processed_slot = update.slot;
                        }
                        else {
                             info!("Active phase has ended, stopping queue processing");
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

    async fn is_in_active_phase(
        &self,
        slot: u64,
        epoch_info: &ForesterEpochInfo,
    ) -> Result<bool, ForesterError> {
        let current_epoch = self.protocol_config.get_current_active_epoch(slot)?;
        if current_epoch != epoch_info.epoch.epoch {
            return Ok(false);
        }

        Ok(self
            .protocol_config
            .is_active_phase(slot, epoch_info.epoch.epoch)
            .is_ok())
    }

    async fn process_queues(&self, epoch_info: &ForesterEpochInfo) -> Result<(), ForesterError> {
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
    ) -> Result<(), ForesterError> {
        let rpc = self.rpc_pool.get_connection().await;
        let current_slot = rpc.lock().await.get_slot().await?;
        if !self.is_in_active_phase(current_slot, epoch_info).await? {
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

        match self.process_work_items(epoch_info, &work_items).await {
            Ok(results) => {
                for (idx, res) in results.iter().enumerate() {
                    debug!("Transaction {}: {:?}", idx, res);
                }
            }
            Err(e) => {
                error!("Error processing work items: {:?}", e);
            }
        }

        Ok(())
    }

    async fn fetch_work_items(
        &self,
        rpc: Arc<Mutex<R>>,
        trees: &[TreeForesterSchedule],
    ) -> Result<Vec<WorkItem>, ForesterError> {
        let mut work_items = Vec::new();

        for tree in trees {
            let queue_data = fetch_queue_data(rpc.clone(), &tree.tree_accounts.queue).await?;
            for data in queue_data {
                work_items.push(WorkItem {
                    tree_account: tree.tree_accounts,
                    queue_data: data,
                });
            }
        }

        Ok(work_items)
    }

    async fn process_work_items(
        &self,
        epoch_info: &ForesterEpochInfo,
        work_items: &[WorkItem],
    ) -> Result<Vec<Signature>, ForesterError> {
        let mut results = Vec::new();

        for indexer_chunk in work_items.chunks(self.config.indexer_batch_size) {
            debug!("Processing indexer chunk of size: {}", indexer_chunk.len());
            let rpc = self.rpc_pool.get_connection().await;
            let current_slot = rpc.lock().await.get_slot().await?;
            if !self.is_in_active_phase(current_slot, epoch_info).await? {
                debug!("Not in active phase, skipping process_work_items");
                return Err(ForesterError::Custom("Not in active phase".to_string()));
            }

            const MAX_RETRIES: u32 = 3;
            const RETRY_DELAY: Duration = Duration::from_millis(200);

            let mut retry_count = 0;
            let mut proofs = Vec::new();
            let mut all_instructions = Vec::new();

            while retry_count < MAX_RETRIES {
                match self
                    .fetch_proofs_and_create_instructions(epoch_info, indexer_chunk)
                    .await
                {
                    Ok((fetched_proofs, fetched_instructions)) => {
                        proofs = fetched_proofs;
                        all_instructions = fetched_instructions;
                        break;
                    }
                    Err(e) => {
                        if retry_count == MAX_RETRIES - 1 {
                            error!(
                                "Failed to fetch proofs after {} attempts: {:?}",
                                MAX_RETRIES, e
                            );
                            return Err(e);
                        }
                        warn!(
                            "Error fetching proofs (attempt {}): {:?}. Retrying in {:?}...",
                            retry_count + 1,
                            e,
                            RETRY_DELAY
                        );
                        sleep(RETRY_DELAY).await;
                        retry_count += 1;
                    }
                }
            }

            for (batch_index, (transaction_chunk, proof_chunk)) in all_instructions
                .chunks(self.config.transaction_batch_size)
                .zip(proofs.chunks(self.config.transaction_batch_size))
                .enumerate()
            {
                let work_item = &indexer_chunk[batch_index * self.config.transaction_batch_size];

                if !self
                    .check_eligibility(epoch_info, &work_item.tree_account)
                    .await?
                {
                    debug!("Forester not eligible for this slot, skipping batch");
                    continue;
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
                            work_item.queue_data.hash, signature
                        );
                        results.push(signature);
                        self.increment_processed_items_count(epoch_info.epoch.epoch)
                            .await;
                    }
                    Err(e) => {
                        error!("Error processing transaction batch: {:?}", e);
                        // Continue processing other batches
                    }
                }
            }
        }

        Ok(results)
    }

    async fn check_eligibility(
        &self,
        registration_info: &ForesterEpochInfo,
        tree_account: &TreeAccounts,
    ) -> Result<bool, ForesterError> {
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

        Ok(tree_schedule.is_eligible(light_slot))
    }

    async fn process_transaction_batch(
        &self,
        epoch_info: &ForesterEpochInfo,
        instructions: &[Instruction],
        proofs: &[Proof],
        work_items: &[WorkItem],
    ) -> Result<Signature, ForesterError> {
        let rpc = self.rpc_pool.get_connection().await;
        let current_slot = rpc.lock().await.get_slot().await?;
        if !self.is_in_active_phase(current_slot, epoch_info).await? {
            debug!("Not in active phase, skipping queue processing");
            return Err(ForesterError::Custom("Not in active phase".to_string()));
        }
        let mut rpc_guard = rpc.lock().await;
        let recent_blockhash = rpc_guard.get_latest_blockhash().await?;
        drop(rpc_guard); // Release the lock before the potentially long-running operation

        let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.cu_limit,
        )];
        ixs.extend_from_slice(instructions);
        let mut transaction =
            Transaction::new_with_payer(&ixs, Some(&self.config.payer_keypair.pubkey()));
        transaction.sign(&[&self.config.payer_keypair], recent_blockhash);

        let mut rpc_guard = rpc.lock().await;
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

    async fn wait_for_report_work_phase(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> Result<(), ForesterError> {
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

    async fn report_work(&self, epoch_info: &ForesterEpochInfo) -> Result<(), ForesterError> {
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
    ) -> Result<(Vec<Proof>, Vec<Instruction>), ForesterError> {
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
                .map(|item| item.queue_data.hash)
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
                        value: item.queue_data.index as u16,
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
                .map(|item| bs58::encode(&item.queue_data.hash).into_string())
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
                        leaves_queue_indices: vec![item.queue_data.index as u16],
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

    async fn perform_rollover(&self, tree_account: &TreeAccounts) -> Result<(), ForesterError> {
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
) -> Result<(), ForesterError> {
    const MAX_RETRIES: u32 = 5;
    const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
    const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

    let mut retry_count = 0;
    let mut retry_delay = INITIAL_RETRY_DELAY;
    let start_time = Instant::now();

    while retry_count < MAX_RETRIES {
        match EpochManager::new(
            config.clone(),
            protocol_config.clone(),
            rpc_pool.clone(),
            indexer.clone(),
            work_report_sender.clone(),
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
                if retry_count < MAX_RETRIES {
                    info!("Retrying in {:?}", retry_delay);
                    sleep(retry_delay).await;
                    retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
                } else {
                    error!(
                        "Failed to start forester after {} attempts over {:?}",
                        MAX_RETRIES,
                        start_time.elapsed()
                    );
                    return Err(ForesterError::Custom(format!(
                        "Failed to start forester after {} attempts: {:?}",
                        MAX_RETRIES, e
                    )));
                }
            }
        }
    }

    Err(ForesterError::Custom(
        "Unexpected error: Retry loop exited without returning".to_string(),
    ))
}
