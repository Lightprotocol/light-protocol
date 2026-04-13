use std::sync::Arc;

use dashmap::DashMap;
use forester_utils::forester_epoch::{Epoch, TreeAccounts};
use light_client::{indexer::Indexer, rpc::Rpc};
use solana_program::pubkey::Pubkey;
use tokio::sync::Mutex;
use tracing::debug;

use super::context::ForesterContext;
use crate::{
    processor::v2::{
        strategy::{AddressTreeStrategy, StateTreeStrategy},
        QueueProcessor, SharedProofCache,
    },
    Result,
};

pub(crate) type StateBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>)>>;
pub(crate) type AddressBatchProcessorMap<R> =
    Arc<DashMap<Pubkey, (u64, Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>)>>;
type ProcessorInitLockMap = Arc<DashMap<Pubkey, Arc<Mutex<()>>>>;

/// Cached V2 processors and proof caches, keyed by tree pubkey.
/// Processors are reused across epochs to preserve cached state.
#[derive(Debug)]
pub(crate) struct ProcessorPool<R: Rpc + Indexer> {
    state_processors: StateBatchProcessorMap<R>,
    address_processors: AddressBatchProcessorMap<R>,
    state_init_locks: ProcessorInitLockMap,
    address_init_locks: ProcessorInitLockMap,
    pub proof_caches: Arc<DashMap<Pubkey, Arc<SharedProofCache>>>,
    pub zkp_batch_sizes: Arc<DashMap<Pubkey, u64>>,
}

impl<R: Rpc + Indexer> Clone for ProcessorPool<R> {
    fn clone(&self) -> Self {
        Self {
            state_processors: self.state_processors.clone(),
            address_processors: self.address_processors.clone(),
            state_init_locks: self.state_init_locks.clone(),
            address_init_locks: self.address_init_locks.clone(),
            proof_caches: self.proof_caches.clone(),
            zkp_batch_sizes: self.zkp_batch_sizes.clone(),
        }
    }
}

impl<R: Rpc + Indexer> ProcessorPool<R> {
    pub fn new() -> Self {
        Self {
            state_processors: Arc::new(DashMap::new()),
            address_processors: Arc::new(DashMap::new()),
            state_init_locks: Arc::new(DashMap::new()),
            address_init_locks: Arc::new(DashMap::new()),
            proof_caches: Arc::new(DashMap::new()),
            zkp_batch_sizes: Arc::new(DashMap::new()),
        }
    }

    pub async fn get_or_create_state_processor(
        &self,
        ctx: &ForesterContext<R>,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        ops_cache: Arc<Mutex<crate::processor::tx_cache::ProcessedHashCache>>,
    ) -> Result<Arc<Mutex<QueueProcessor<R, StateTreeStrategy>>>> {
        let init_lock = self
            .state_init_locks
            .entry(tree_accounts.merkle_tree)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _init_guard = init_lock.lock().await;

        if let Some(entry) = self.state_processors.get(&tree_accounts.merkle_tree) {
            let (stored_epoch, processor_ref) = entry.value();
            let processor_clone = processor_ref.clone();
            let old_epoch = *stored_epoch;
            drop(entry);

            if old_epoch != epoch_info.epoch {
                debug!(
                    "Reusing StateBatchProcessor for tree {} across epoch transition ({} -> {})",
                    tree_accounts.merkle_tree, old_epoch, epoch_info.epoch
                );
                self.state_processors.insert(
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

        let batch_context =
            ctx.build_batch_context(epoch_info, tree_accounts, None, None, None, ops_cache);
        let processor = Arc::new(Mutex::new(
            QueueProcessor::new(batch_context, StateTreeStrategy).await?,
        ));

        let batch_size = processor.lock().await.zkp_batch_size();
        self.zkp_batch_sizes
            .insert(tree_accounts.merkle_tree, batch_size);

        self.state_processors.insert(
            tree_accounts.merkle_tree,
            (epoch_info.epoch, processor.clone()),
        );
        Ok(processor)
    }

    pub async fn get_or_create_address_processor(
        &self,
        ctx: &ForesterContext<R>,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        ops_cache: Arc<Mutex<crate::processor::tx_cache::ProcessedHashCache>>,
    ) -> Result<Arc<Mutex<QueueProcessor<R, AddressTreeStrategy>>>> {
        let init_lock = self
            .address_init_locks
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

        let batch_context =
            ctx.build_batch_context(epoch_info, tree_accounts, None, None, None, ops_cache);
        let processor = Arc::new(Mutex::new(
            QueueProcessor::new(batch_context, AddressTreeStrategy).await?,
        ));

        let batch_size = processor.lock().await.zkp_batch_size();
        self.zkp_batch_sizes
            .insert(tree_accounts.merkle_tree, batch_size);

        self.address_processors.insert(
            tree_accounts.merkle_tree,
            (epoch_info.epoch, processor.clone()),
        );
        Ok(processor)
    }

    pub fn remove_state_processor(&self, tree: &Pubkey) {
        self.state_processors.remove(tree);
    }

    pub fn remove_address_processor(&self, tree: &Pubkey) {
        self.address_processors.remove(tree);
    }

    pub fn remove_proof_cache(&self, tree: &Pubkey) {
        self.proof_caches.remove(tree);
    }

    pub fn get_or_create_proof_cache(&self, tree: Pubkey) -> Arc<SharedProofCache> {
        self.proof_caches
            .entry(tree)
            .or_insert_with(|| Arc::new(SharedProofCache::new(tree)))
            .clone()
    }

    pub fn get_proof_cache(&self, tree: &Pubkey) -> Option<Arc<SharedProofCache>> {
        self.proof_caches.get(tree).map(|c| c.clone())
    }
}
