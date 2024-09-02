use crate::epoch_manager::{MerkleProofType, WorkItem};
use crate::errors::ForesterError;
use crate::queue_helpers::fetch_queue_item_data;
use crate::rpc_pool::SolanaRpcPool;
use crate::utils::get_current_system_time_ms;
use crate::Result;
use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
    STATE_MERKLE_TREE_CHANGELOG,
};
use forester_utils::forester_epoch::{TreeAccounts, TreeType};
use forester_utils::indexer::Indexer;
use forester_utils::rpc::{RpcConnection, RpcError, SolanaRpcConnection};
use futures::future::join_all;
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use log::info;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{
    hash::Hash,
    signature::{Keypair, Signature, Signer},
};
use std::sync::Arc;
use std::{time::Duration, vec};
use tokio::join;
use tokio::sync::Mutex;
use tokio::time::sleep;

pub trait TransactionBuilder {
    fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        recent_blockhash: &Hash,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> impl std::future::Future<Output = Vec<Transaction>> + Send;
}

/// Setting:
/// 1. We have 1 light slot 15 seconds and a lot of elements in the queue
/// 2. we want to send as many elements from the queue as possible
///
/// Strategy:
/// 1. Execute transaction batches until max number of batches is
/// reached or light slot ended (global timeout).
/// 2. Fetch queue items.
/// 3. Fetch recent blockhash.
/// 4. Iterate over work items in chunks of batch size.
/// 5. Check if we have reached the max number of batches or the global timeout.
/// 6. Spawn new thread to build and send transactions.
/// 7. Await minimum batch time.
/// 8. If work items is empty, await minimum batch time.
///
/// Questions:
/// - How do we make sure that we have send all the transactions?
/// - How can we montinor how many tx have been dropped?
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
    config: SendBatchedTransactionsConfig,
    tree_accounts: TreeAccounts,
    transaction_builder: &T,
    epoch: u64,
) -> Result<usize> {
    let mut rpc = pool.get_connection().await?;
    let mut num_batches = 0;
    let mut num_sent_transactions: usize = 0;
    // 1. Execute batches until max number of batches is reached or light slot
    //    ended (global timeout)
    while num_batches < config.num_batches
        && get_current_system_time_ms() < config.retry_config.global_timeout
    {
        // 2. Fetch queue items.
        let queue_item_data = fetch_queue_item_data(&mut *rpc, &tree_accounts.queue).await?;
        let mut work_items = Vec::new();
        for data in queue_item_data {
            work_items.push(WorkItem {
                tree_account: tree_accounts,
                queue_item_data: data,
            });
        }
        let batch_time = Duration::from_millis(config.batch_time_ms);
        // 3. Fetch recent blockhash.
        // A recent blockhash is valid for 2 mins we only need one per batch. We
        // use a new one per batch in case that we want to retry these same
        // transactions and identical transactions might be dropped.
        let recent_blockhash = (*rpc)
            .get_latest_blockhash()
            .await
            .map_err(RpcError::from)?;
        // 4. Iterate over work items in chunks of batch size.
        for work_items in
            work_items.chunks(config.build_transaction_batch_config.batch_size as usize)
        {
            // 5. Check if we have reached the max number of batches or the global timeout.
            if num_batches > config.num_batches
                || get_current_system_time_ms() >= config.retry_config.global_timeout
            {
                break;
            }
            num_batches += 1;

            // Minimum time to wait for the next batch of transactions.
            // Can be used to avoid rate limits.
            let batch_min_time = tokio::time::sleep(batch_time);
            let start_time = tokio::time::Instant::now();
            let transactions: Vec<Transaction> = transaction_builder
                .build_signed_transaction_batch(
                    payer,
                    &recent_blockhash,
                    work_items,
                    config.build_transaction_batch_config,
                )
                .await;
            num_sent_transactions += transactions.len();

            info!("build transaction time {:?}", start_time.elapsed());
            let start_time_get_connections = tokio::time::Instant::now();
            info!(
                "get get connections txs time {:?}",
                start_time_get_connections.elapsed()
            );
            let url = (*rpc).get_url().to_string();
            // 6. Spawn new thread to build and send transactions.
            // Will time out with retry_config.max_retries or retry_config.global_timeout.
            tokio::spawn(async move {
                let rpc = SolanaRpcConnection::new(url, None);
                let mut results = vec![];
                for transaction in transactions.iter() {
                    let res = send_signed_transaction(transaction, &rpc, config.retry_config);
                    results.push(res);
                }
                let all_results = join_all(results);
                all_results.await;
            });

            info!(
                "get send txs time {:?}",
                start_time_get_connections.elapsed()
            );
            // 7. Await minimum batch time.
            batch_min_time.await;
        }

        // 8. If work items is empty, await minimum batch time.
        // If this is triggered we could switch to subscribing to the queue
        if work_items.is_empty() {
            info!("Work items empty, waiting for next batch epoch {:?}", epoch);
            tokio::time::sleep(batch_time).await;
        }
    }
    Ok(num_sent_transactions)
}

#[derive(Debug, Clone, Copy)]
pub struct SendBatchedTransactionsConfig {
    pub num_batches: u64,
    pub batch_time_ms: u64,
    pub build_transaction_batch_config: BuildTransactionBatchConfig,
    pub retry_config: RetryConfig,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildTransactionBatchConfig {
    pub batch_size: u64,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    pub max_retries: u8,
    pub retry_wait_time_ms: u64,
    pub global_timeout: u128,
}

/// Sends a transaction and retries if not confirmed after retry wait time.
/// Stops retrying at the global timeout (end of light slot).
pub async fn send_signed_transaction(
    transaction: &Transaction,
    connection: &SolanaRpcConnection,
    config: RetryConfig,
) -> Result<Signature> {
    let mut retries = 0;
    let retry_wait_time = Duration::from_millis(config.retry_wait_time_ms);
    let txid = transaction.signatures[0];
    while retries < config.max_retries && get_current_system_time_ms() < config.global_timeout {
        match connection.send_transaction(transaction).await {
            Ok(_signature) => {
                // info!("Transaction sent: {}", signature);
            }
            Err(e) => {
                info!("Error sending transaction: {:?}", e);
                return Err(ForesterError::from(e));
            }
        }
        sleep(retry_wait_time).await;
        // TODO: find a way to get failed transactions and handle errors
        // The current confirm does not expose errors.
        // For example:
        // let response = connection
        //     .client
        //     .get_transaction_with_config(
        //         &txid,
        //         RpcTransactionConfig {
        //             encoding: None,
        //             commitment: Some(CommitmentConfig::confirmed()),
        //             max_supported_transaction_version: None,
        //         },
        //     )
        //     .unwrap(); // confirm_transaction(txid).await?;
        // info!("Retrying transaction: {}", txid);
        // if response.transaction.meta.unwrap().status.is_ok() {
        //     return Ok(txid);
        // }
        let response = connection.confirm_transaction(txid).await?;
        if response {
            return Ok(txid);
        }
        retries += 1;
    }
    Ok(txid)
}

pub struct EpochManagerTransactions<R: RpcConnection, I: Indexer<R>> {
    pub indexer: Arc<Mutex<I>>,
    pub epoch: u64,
    pub phantom: std::marker::PhantomData<R>,
}

impl<R: RpcConnection, I: Indexer<R>> TransactionBuilder for EpochManagerTransactions<R, I> {
    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        recent_blockhash: &Hash,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Vec<Transaction> {
        let mut transactions = vec![];
        let (_, all_instructions) = fetch_proofs_and_create_instructions(
            payer.pubkey(),
            payer.pubkey(),
            self.indexer.clone(),
            self.epoch,
            work_items,
        )
        .await
        .unwrap();
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
        transactions
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
