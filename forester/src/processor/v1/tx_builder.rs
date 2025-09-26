use std::{sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::rpc::Rpc;
use solana_program::hash::Hash;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokio::sync::Mutex;
use tracing::{trace, warn};

use crate::{
    epoch_manager::WorkItem,
    processor::{
        tx_cache::ProcessedHashCache,
        v1::{config::BuildTransactionBatchConfig, helpers::fetch_proofs_and_create_instructions},
    },
    smart_transaction::{create_smart_transaction, CreateSmartTransactionConfig},
    Result,
};

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait TransactionBuilder: Send + Sync {
    fn epoch(&self) -> u64;
    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        derivation: &Pubkey,
        recent_blockhash: &Hash,
        last_valid_block_height: u64,
        priority_fee: u64,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<(Vec<Transaction>, u64)>;
}

pub struct EpochManagerTransactions<R: Rpc> {
    pub pool: Arc<SolanaRpcPool<R>>,
    pub epoch: u64,
    pub phantom: std::marker::PhantomData<R>,
    pub processed_hash_cache: Arc<Mutex<ProcessedHashCache>>,
}

impl<R: Rpc> EpochManagerTransactions<R> {
    pub fn new(
        pool: Arc<SolanaRpcPool<R>>,
        epoch: u64,
        cache: Arc<Mutex<ProcessedHashCache>>,
    ) -> Self {
        Self {
            pool,
            epoch,
            phantom: std::marker::PhantomData,
            processed_hash_cache: cache,
        }
    }
}

#[async_trait]
impl<R: Rpc> TransactionBuilder for EpochManagerTransactions<R> {
    fn epoch(&self) -> u64 {
        self.epoch
    }

    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        derivation: &Pubkey,
        recent_blockhash: &Hash,
        last_valid_block_height: u64,
        priority_fee: u64,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<(Vec<Transaction>, u64)> {
        let mut cache = self.processed_hash_cache.lock().await;

        let work_items: Vec<&WorkItem> = work_items
            .iter()
            .filter(|item| {
                let hash_str = bs58::encode(&item.queue_item_data.hash).into_string();
                if cache.contains(&hash_str) {
                    trace!("Skipping already processed hash: {}", hash_str);
                    false
                } else {
                    true
                }
            })
            .collect();

        // Add items with short timeout (30 seconds) for processing
        for item in &work_items {
            let hash_str = bs58::encode(&item.queue_item_data.hash).into_string();
            cache.add_with_timeout(&hash_str, Duration::from_secs(15));
            trace!("Added {} to cache with 15s timeout", hash_str);
        }

        let work_item_hashes: Vec<String> = work_items
            .iter()
            .map(|item| bs58::encode(&item.queue_item_data.hash).into_string())
            .collect();

        drop(cache);

        if work_items.is_empty() {
            trace!("All items in this batch were recently processed, skipping batch");
            return Ok((vec![], last_valid_block_height));
        }

        let work_items = work_items
            .iter()
            .map(|&item| item.clone())
            .collect::<Vec<_>>();

        let mut transactions = vec![];
        let all_instructions = match fetch_proofs_and_create_instructions(
            payer.pubkey(),
            *derivation,
            self.pool.clone(),
            self.epoch,
            work_items.as_slice(),
        )
        .await
        {
            Ok((_, instructions)) => instructions,
            Err(e) => {
                // Check if it's a "Record Not Found" error
                return if e.to_string().contains("Record Not Found") {
                    warn!("Record not found in indexer, skipping batch: {}", e);
                    // Return empty transactions but don't propagate the error
                    Ok((vec![], last_valid_block_height))
                } else {
                    // For any other error, propagate it
                    Err(e)
                };
            }
        };

        let batch_size = config.batch_size.max(1) as usize;

        for instruction_chunk in all_instructions.chunks(batch_size) {
            let (transaction, _) = create_smart_transaction(CreateSmartTransactionConfig {
                payer: payer.insecure_clone(),
                instructions: instruction_chunk.to_vec(),
                recent_blockhash: *recent_blockhash,
                compute_unit_price: Some(priority_fee),
                compute_unit_limit: config.compute_unit_limit,
                last_valid_block_hash: last_valid_block_height,
            })
            .await?;
            transactions.push(transaction);
        }

        if !transactions.is_empty() {
            let mut cache = self.processed_hash_cache.lock().await;
            for hash_str in work_item_hashes {
                cache.extend_timeout(&hash_str, Duration::from_secs(30));
                trace!(
                    "Extended cache timeout for {} to 30s after successful transaction creation",
                    hash_str
                );
            }
        }

        Ok((transactions, last_valid_block_height))
    }
}
