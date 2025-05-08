use std::sync::Arc;

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use light_client::{indexer::Indexer, rpc::RpcConnection, rpc_pool::SolanaRpcPool};
use solana_program::hash::Hash;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokio::sync::Mutex;
use tracing::{debug, warn};

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
pub trait TransactionBuilder {
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

pub struct EpochManagerTransactions<R: RpcConnection, I: Indexer<R>> {
    pub indexer: Arc<Mutex<I>>,
    pub pool: Arc<SolanaRpcPool<R>>,
    pub epoch: u64,
    pub phantom: std::marker::PhantomData<R>,
    pub processed_hash_cache: Arc<Mutex<ProcessedHashCache>>,
}

impl<R: RpcConnection, I: Indexer<R>> EpochManagerTransactions<R, I> {
    pub fn new(
        indexer: Arc<Mutex<I>>,
        pool: Arc<SolanaRpcPool<R>>,
        epoch: u64,
        cache: Arc<Mutex<ProcessedHashCache>>,
    ) -> Self {
        Self {
            indexer,
            pool,
            epoch,
            phantom: std::marker::PhantomData,
            processed_hash_cache: cache,
        }
    }
}

#[async_trait]
impl<R: RpcConnection, I: Indexer<R>> TransactionBuilder for EpochManagerTransactions<R, I> {
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
                    debug!("Skipping already processed hash: {}", hash_str);
                    false
                } else {
                    true
                }
            })
            .collect();

        for item in &work_items {
            let hash_str = bs58::encode(&item.queue_item_data.hash).into_string();
            cache.add(&hash_str);
        }
        drop(cache);

        if work_items.is_empty() {
            debug!("All items in this batch were recently processed, skipping batch");
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
            self.indexer.clone(),
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

        for instruction in all_instructions {
            let (transaction, _) = create_smart_transaction(CreateSmartTransactionConfig {
                payer: payer.insecure_clone(),
                instructions: vec![instruction],
                recent_blockhash: *recent_blockhash,
                compute_unit_price: Some(priority_fee),
                compute_unit_limit: config.compute_unit_limit,
                last_valid_block_hash: last_valid_block_height,
            })
            .await?;
            transactions.push(transaction);
        }
        Ok((transactions, last_valid_block_height))
    }
}
