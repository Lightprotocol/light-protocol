use crate::config::QueueConfig;
use crate::epoch_manager::{MerkleProofType, WorkItem};
use crate::errors::ForesterError;
use crate::queue_helpers::fetch_queue_item_data;
use crate::Result;
use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG, ADDRESS_QUEUE_VALUES,
    STATE_MERKLE_TREE_CHANGELOG, STATE_NULLIFIER_QUEUE_VALUES,
};
use async_trait::async_trait;
use forester_utils::forester_epoch::{TreeAccounts, TreeType};
use forester_utils::indexer::Indexer;
use futures::future::join_all;
use light_client::rpc::{RetryConfig, RpcConnection};
use light_client::rpc_pool::SolanaRpcPool;
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{
    hash::Hash,
    signature::{Keypair, Signer},
};
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use tokio::join;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tracing::{debug, warn};

#[async_trait]
pub trait TransactionBuilder {
    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        recent_blockhash: &Hash,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<Vec<Transaction>>;
}

// We're assuming that:
// 1. Helius slot latency is ~ 3 slots.
// See also: https://p.us5.datadoghq.com/sb/339e0590-c5d4-11ed-9c7b-da7ad0900005-231a672007c47d70f38e8fa321bc8407?fromUser=false&refresh_mode=sliding&tpl_var_leader_name%5B0%5D=%2A&from_ts=1725348612900&to_ts=1725953412900&live=true
// 2. Latency between forester server and helius is ~ 1 slot.
// 3. Slot duration is 500ms.
const LATENCY: Duration = Duration::from_millis(4 * 500);

/// Setting:
/// 1. We have 1 light slot 15 seconds and a lot of elements in the queue
/// 2. we want to send as many elements from the queue as possible
///
/// Strategy:
/// 1. Execute transaction batches until max number of batches is
/// reached or light slot ended (global timeout).
/// 2. Fetch queue items.
/// 3. If work items is empty, await minimum batch time.
/// 4. Fetch recent blockhash.
/// 5. Iterate over work items in chunks of batch size.
/// 6. Check if we reached the end of the light slot.
/// 7. Asynchronously send all transactions in the batch
/// 8. Await minimum batch time.
/// 9. Check if we reached max number of batches.
///
/// Questions:
/// - How do we make sure that we have send all the transactions?
/// - How can we monitor how many txs have been dropped?
///
/// TODO:
/// - return number of sent transactions
/// - test timeout for any action of this function or subfunctions, timeout is
///   end of slot
/// - consider dynamic batch size based on the number of transactions in the
///   queue
pub async fn send_batched_transactions<T: TransactionBuilder, R: RpcConnection>(
    payer: &Keypair,
    pool: Arc<SolanaRpcPool<R>>,
    config: &SendBatchedTransactionsConfig,
    tree_accounts: TreeAccounts,
    transaction_builder: &T,
) -> Result<usize> {
    let start_time = Instant::now();

    let mut rpc = pool.get_connection().await?;
    let mut num_batches = 0;
    let mut num_sent_transactions: usize = 0;
    // 1. Execute batches until max number of batches is reached or light slot
    //    ended (light_slot_duration)
    while num_batches < config.num_batches && start_time.elapsed() < config.retry_config.timeout {
        debug!("Sending batch: {}", num_batches);
        // 2. Fetch queue items.
        let queue_length = if tree_accounts.tree_type == TreeType::State {
            STATE_NULLIFIER_QUEUE_VALUES
        } else {
            ADDRESS_QUEUE_VALUES
        };
        let start_index = if tree_accounts.tree_type == TreeType::State {
            config.queue_config.state_queue_start_index
        } else {
            config.queue_config.address_queue_start_index
        };
        let length = if tree_accounts.tree_type == TreeType::State {
            config.queue_config.state_queue_length
        } else {
            config.queue_config.address_queue_length
        };
        let queue_item_data = fetch_queue_item_data(
            &mut *rpc,
            &tree_accounts.queue,
            start_index,
            length,
            queue_length,
        )
        .await?;
        let work_items: Vec<WorkItem> = queue_item_data
            .into_iter()
            .map(|data| WorkItem {
                tree_account: tree_accounts,
                queue_item_data: data,
            })
            .collect();

        // 3. If work items is empty, await minimum batch time.
        // If this is triggered we could switch to subscribing to the queue
        if work_items.is_empty() {
            debug!("No work items found, waiting for next batch");
            sleep(config.retry_config.retry_delay).await;
            continue;
        }

        // 4. Fetch recent blockhash.
        // A recent blockhash is valid for 2 mins we only need one per batch. We
        // use a new one per batch in case that we want to retry these same
        // transactions and identical transactions might be dropped.
        let recent_blockhash = rpc.get_latest_blockhash().await?;
        // 5. Iterate over work items in chunks of batch size.
        for work_items in
            work_items.chunks(config.build_transaction_batch_config.batch_size as usize)
        {
            // 6. Check if we reached the end of the light slot.
            let remaining_time = match config
                .retry_config
                .timeout
                .checked_sub(start_time.elapsed())
            {
                Some(time) => time,
                None => {
                    debug!("Reached end of light slot");
                    break;
                }
            };

            if remaining_time < LATENCY {
                debug!("Reached end of light slot");
                break;
            }

            // Minimum time to wait for the next batch of transactions.
            // Can be used to avoid rate limits.
            let transaction_build_time_start = Instant::now();
            let transactions: Vec<Transaction> = transaction_builder
                .build_signed_transaction_batch(
                    payer,
                    &recent_blockhash,
                    work_items,
                    config.build_transaction_batch_config,
                )
                .await?;
            debug!(
                "build transaction time {:?}",
                transaction_build_time_start.elapsed()
            );

            let batch_start = Instant::now();
            let remaining_time = config
                .retry_config
                .timeout
                .saturating_sub(start_time.elapsed());

            if remaining_time < LATENCY {
                debug!("Reached end of light slot");
                break;
            }

            // Asynchronously send all transactions in the batch
            let pool_clone = Arc::clone(&pool);
            let send_futures = transactions.into_iter().map(move |tx| {
                let pool_clone = Arc::clone(&pool_clone);
                tokio::spawn(async move {
                    match pool_clone.get_connection().await {
                        Ok(rpc) => rpc.send_transaction(&tx).await,
                        Err(e) => Err(light_client::rpc::RpcError::CustomError(format!(
                            "Failed to get RPC connection: {}",
                            e
                        ))),
                    }
                })
            });

            let results = join_all(send_futures).await;

            // Process results
            for result in results {
                match result {
                    Ok(Ok(_)) => num_sent_transactions += 1,
                    Ok(Err(e)) => warn!("Transaction failed: {:?}", e),
                    Err(e) => warn!("Task failed: {:?}", e),
                }
            }

            num_batches += 1;
            let batch_duration = batch_start.elapsed();
            debug!("Batch duration: {:?}", batch_duration);

            // 8. Await minimum batch time.
            if start_time.elapsed() + config.retry_config.retry_delay < config.retry_config.timeout
            {
                sleep(config.retry_config.retry_delay).await;
            } else {
                break;
            }

            // 9. Check if we reached max number of batches.
            if num_batches >= config.num_batches {
                debug!("Reached max number of batches");
                break;
            }
        }
    }

    debug!("Sent {} transactions", num_sent_transactions);
    Ok(num_sent_transactions)
}

#[derive(Debug, Clone, Copy)]
pub struct SendBatchedTransactionsConfig {
    pub num_batches: u64,
    pub build_transaction_batch_config: BuildTransactionBatchConfig,
    pub queue_config: QueueConfig,
    pub retry_config: RetryConfig,
    pub light_slot_length: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildTransactionBatchConfig {
    pub batch_size: u64,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
}

pub struct EpochManagerTransactions<R: RpcConnection, I: Indexer<R>> {
    pub indexer: Arc<Mutex<I>>,
    pub epoch: u64,
    pub phantom: std::marker::PhantomData<R>,
}

#[async_trait]
impl<R: RpcConnection, I: Indexer<R>> TransactionBuilder for EpochManagerTransactions<R, I> {
    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        recent_blockhash: &Hash,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<Vec<Transaction>> {
        let mut transactions = vec![];
        let (_, all_instructions) = fetch_proofs_and_create_instructions(
            payer.pubkey(),
            payer.pubkey(),
            self.indexer.clone(),
            self.epoch,
            work_items,
        )
        .await?;
        for instruction in all_instructions {
            let transaction = build_signed_transaction(
                payer,
                recent_blockhash,
                config.compute_unit_price,
                config.compute_unit_limit,
                instruction,
            )
            .await;
            transactions.push(transaction);
        }
        Ok(transactions)
    }
}

async fn build_signed_transaction(
    payer: &Keypair,
    recent_blockhash: &Hash,
    compute_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
    instruction: Instruction,
) -> Transaction {
    let mut instructions: Vec<Instruction> = if let Some(price) = compute_unit_price {
        vec![ComputeBudgetInstruction::set_compute_unit_price(price)]
    } else {
        vec![]
    };
    if let Some(limit) = compute_unit_limit {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    instructions.push(instruction);

    let mut transaction =
        Transaction::new_with_payer(instructions.as_slice(), Some(&payer.pubkey()));
    transaction.sign(&[payer], *recent_blockhash);
    transaction
}

/// Work items should be of only one type and tree
pub async fn fetch_proofs_and_create_instructions<R: RpcConnection, I: Indexer<R>>(
    authority: Pubkey,
    derivation: Pubkey,
    indexer: Arc<Mutex<I>>,
    epoch: u64,
    work_items: &[WorkItem],
) -> Result<(Vec<MerkleProofType>, Vec<Instruction>)> {
    let mut proofs = Vec::new();
    let mut instructions = vec![];

    let (address_items, state_items): (Vec<_>, Vec<_>) = work_items
        .iter()
        .partition(|item| matches!(item.tree_account.tree_type, TreeType::Address));

    // Prepare data for batch fetching
    let address_data = if !address_items.is_empty() {
        let merkle_tree = address_items
            .first()
            .ok_or_else(|| ForesterError::Custom("No address items found".to_string()))?
            .tree_account
            .merkle_tree
            .to_bytes();
        let addresses: Vec<[u8; 32]> = address_items
            .iter()
            .map(|item| item.queue_item_data.hash)
            .collect();
        Some((merkle_tree, addresses))
    } else {
        None
    };

    let state_data = if !state_items.is_empty() {
        let states: Vec<String> = state_items
            .iter()
            .map(|item| bs58::encode(&item.queue_item_data.hash).into_string())
            .collect();
        Some(states)
    } else {
        None
    };

    // Fetch all proofs in parallel
    let (address_proofs, state_proofs) = {
        let indexer = indexer.lock().await;

        let address_future = async {
            if let Some((merkle_tree, addresses)) = address_data {
                indexer
                    .get_multiple_new_address_proofs(merkle_tree, addresses)
                    .await
            } else {
                Ok(vec![])
            }
        };

        let state_future = async {
            if let Some(states) = state_data {
                indexer.get_multiple_compressed_account_proofs(states).await
            } else {
                Ok(vec![])
            }
        };

        join!(address_future, state_future)
    };

    let address_proofs = address_proofs?;
    let state_proofs = state_proofs?;

    // Process address proofs and create instructions
    for (item, proof) in address_items.iter().zip(address_proofs.into_iter()) {
        proofs.push(MerkleProofType::AddressProof(proof.clone()));
        let instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority,
                address_merkle_tree: item.tree_account.merkle_tree,
                address_queue: item.tree_account.queue,
                value: item.queue_item_data.index as u16,
                low_address_index: proof.low_address_index,
                low_address_value: proof.low_address_value,
                low_address_next_index: proof.low_address_next_index,
                low_address_next_value: proof.low_address_next_value,
                low_address_proof: proof.low_address_proof,
                changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
                indexed_changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG)
                    as u16,
                is_metadata_forester: false,
            },
            epoch,
        );
        instructions.push(instruction);
    }

    // Process state proofs and create instructions
    for (item, proof) in state_items.iter().zip(state_proofs.into_iter()) {
        proofs.push(MerkleProofType::StateProof(proof.clone()));
        let instruction = create_nullify_instruction(
            CreateNullifyInstructionInputs {
                nullifier_queue: item.tree_account.queue,
                merkle_tree: item.tree_account.merkle_tree,
                change_log_indices: vec![proof.root_seq % STATE_MERKLE_TREE_CHANGELOG],
                leaves_queue_indices: vec![item.queue_item_data.index as u16],
                indices: vec![proof.leaf_index],
                proofs: vec![proof.proof.clone()],
                authority,
                derivation,
                is_metadata_forester: false,
            },
            epoch,
        );
        instructions.push(instruction);
    }

    Ok((proofs, instructions))
}
