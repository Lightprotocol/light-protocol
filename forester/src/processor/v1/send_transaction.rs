use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
    vec,
};

use forester_utils::{forester_epoch::TreeAccounts, rpc_pool::SolanaRpcPool};
use futures::StreamExt;
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_registry::utils::get_forester_epoch_pda_from_authority;
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

const WORK_ITEM_BATCH_SIZE: usize = 100;
use crate::{
    epoch_manager::WorkItem,
    errors::ForesterError,
    metrics::increment_transactions_failed,
    priority_fee::PriorityFeeConfig,
    processor::v1::{config::SendBatchedTransactionsConfig, tx_builder::TransactionBuilder},
    queue_helpers::fetch_queue_item_data,
    smart_transaction::{ConfirmationConfig, PreparedTransaction, SmartTransactionError},
    Result,
};

struct PreparedBatchData {
    work_items: Vec<WorkItem>,
    recent_blockhash: Hash,
    last_valid_block_height: u64,
    priority_fee: Option<u64>,
    timeout_deadline: Instant,
}

#[derive(Clone)]
struct ChunkSendContext<R: Rpc> {
    pool: Arc<SolanaRpcPool<R>>,
    max_concurrent_sends: usize,
    timeout_deadline: Instant,
    cancel_signal: Arc<AtomicBool>,
    num_sent_transactions: Arc<AtomicUsize>,
    confirmation: ConfirmationConfig,
}

#[allow(clippy::large_enum_variant)]
enum TransactionSendResult {
    Success(Signature),
    SendFailure(ForesterError, Option<Signature>),
    ExecutionFailure(ForesterError, Option<Signature>),
    ConfirmationUnknown(ForesterError, Option<Signature>),
    DeadlineExceeded(Signature),
    Cancelled,
    Timeout,
}

/// Setting:
/// 1. We have 1 light slot (n solana slots), and elements in queue
/// 2. we want to send as many elements from the queue as possible
pub async fn send_batched_transactions<T: TransactionBuilder + Send + Sync + 'static, R: Rpc>(
    payer: &Keypair,
    derivation: &Pubkey,
    pool: Arc<SolanaRpcPool<R>>,
    config: &SendBatchedTransactionsConfig,
    tree_accounts: TreeAccounts,
    transaction_builder: Arc<T>,
) -> std::result::Result<usize, ForesterError> {
    let function_start_time = Instant::now();

    let num_sent_transactions = Arc::new(AtomicUsize::new(0));
    let operation_cancel_signal = Arc::new(AtomicBool::new(false));

    let data = match prepare_batch_prerequisites(
        &payer.pubkey(),
        derivation,
        &pool,
        config,
        tree_accounts,
        &*transaction_builder,
        function_start_time,
    )
    .await
    .map_err(ForesterError::from)?
    {
        Some(data) => data,
        None => {
            trace!(tree.id = %tree_accounts.merkle_tree, queue.id = %tree_accounts.queue, "Preparation returned no data, 0 transactions sent.");
            return Ok(0);
        }
    };

    let max_concurrent_sends = config
        .build_transaction_batch_config
        .max_concurrent_sends
        .unwrap_or(1)
        .max(1);
    let effective_max_concurrent_sends =
        compute_effective_max_concurrent_sends(config, max_concurrent_sends, data.work_items.len());

    info!(
        tree = %tree_accounts.merkle_tree,
        "Starting transaction sending loop. work_items={}, work_batch_size={}, timeout={:?}, max_concurrent_sends={} (requested={})",
        data.work_items.len(),
        WORK_ITEM_BATCH_SIZE,
        config.retry_config.timeout,
        effective_max_concurrent_sends,
        max_concurrent_sends
    );

    let tree_id_str = tree_accounts.merkle_tree.to_string();
    let mut recent_blockhash = data.recent_blockhash;
    let mut last_valid_block_height = data.last_valid_block_height;

    const BLOCKHASH_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
    let mut last_blockhash_refresh = Instant::now();

    for work_chunk in data.work_items.chunks(WORK_ITEM_BATCH_SIZE) {
        if operation_cancel_signal.load(Ordering::SeqCst) {
            trace!(tree = %tree_accounts.merkle_tree, "Global cancellation signal received, stopping batch processing.");
            break;
        }
        if Instant::now() >= data.timeout_deadline {
            trace!(tree = %tree_accounts.merkle_tree, "Reached global timeout deadline before processing next chunk, stopping.");
            break;
        }

        if last_blockhash_refresh.elapsed() > BLOCKHASH_REFRESH_INTERVAL {
            match fetch_latest_blockhash(&pool, &tree_id_str).await {
                Ok((new_hash, new_height)) => {
                    recent_blockhash = new_hash;
                    last_valid_block_height = new_height;
                    last_blockhash_refresh = Instant::now();
                    debug!(tree = %tree_accounts.merkle_tree, "Refreshed blockhash");
                }
                Err(e) => {
                    warn!(tree = %tree_accounts.merkle_tree, "Failed to refresh blockhash: {:?}", e);
                }
            }
        }

        trace!(tree = %tree_accounts.merkle_tree, "Processing chunk of size {}", work_chunk.len());
        let build_start_time = Instant::now();

        let (transactions_to_send, chunk_last_valid_block_height) = match transaction_builder
            .build_signed_transaction_batch(
                payer,
                derivation,
                &recent_blockhash,
                last_valid_block_height,
                data.priority_fee,
                work_chunk,
                config.build_transaction_batch_config,
            )
            .await
        {
            Ok(res) => res,
            Err(e) => {
                error!(tree = %tree_accounts.merkle_tree, "Failed to build transaction batch: {:?}", e);
                continue;
            }
        };
        trace!(tree = %tree_accounts.merkle_tree, "Built {} transactions in {:?}", transactions_to_send.len(), build_start_time.elapsed());

        if Instant::now() >= data.timeout_deadline {
            trace!(tree = %tree_accounts.merkle_tree, "Reached global timeout deadline after building transactions, stopping.");
            break;
        }

        if transactions_to_send.is_empty() {
            trace!(tree = %tree_accounts.merkle_tree, "Built batch resulted in 0 transactions, skipping send for this chunk.");
            continue;
        }

        let send_context = ChunkSendContext {
            pool: Arc::clone(&pool),
            max_concurrent_sends: effective_max_concurrent_sends,
            timeout_deadline: data.timeout_deadline,
            cancel_signal: Arc::clone(&operation_cancel_signal),
            num_sent_transactions: Arc::clone(&num_sent_transactions),
            confirmation: ConfirmationConfig {
                max_attempts: config.confirmation_max_attempts as u32,
                poll_interval: config.confirmation_poll_interval,
            },
        };

        if let Err(e) = execute_transaction_chunk_sending(
            transactions_to_send,
            chunk_last_valid_block_height,
            &send_context,
        )
        .await
        {
            if e.is_forester_not_eligible() {
                warn!(
                    tree = %tree_accounts.merkle_tree,
                    "Detected ForesterNotEligible while sending V1 transactions; stopping batch loop for re-schedule"
                );
                return Err(ForesterError::NotEligible);
            }
            warn!(
                tree = %tree_accounts.merkle_tree,
                error = ?e,
                "Chunk send finished with recoverable errors"
            );
        }
    }

    let total_sent_successfully = num_sent_transactions.load(Ordering::SeqCst);
    trace!(tree = %tree_accounts.merkle_tree, "Transaction sending loop finished. Total transactions sent successfully: {}", total_sent_successfully);

    Ok(total_sent_successfully)
}

async fn prepare_batch_prerequisites<R: Rpc, T: TransactionBuilder>(
    payer_pubkey: &Pubkey,
    derivation: &Pubkey,
    pool: &Arc<SolanaRpcPool<R>>,
    config: &SendBatchedTransactionsConfig,
    tree_accounts: TreeAccounts,
    transaction_builder: &T,
    start_time: Instant,
) -> Result<Option<PreparedBatchData>> {
    let tree_id_str = tree_accounts.merkle_tree.to_string();

    let queue_fetch_start_index = match tree_accounts.tree_type {
        TreeType::StateV1 => config.queue_config.state_queue_start_index,
        TreeType::AddressV1 => config.queue_config.address_queue_start_index,
        _ => {
            error!(
                tree = %tree_id_str,
                "prepare_batch_prerequisites called with unsupported tree type: {:?}",
                tree_accounts.tree_type
            );
            return Err(ForesterError::InvalidTreeType(tree_accounts.tree_type).into());
        }
    };

    let queue_item_data = {
        let mut rpc = pool.get_connection().await.map_err(|e| {
            error!(tree = %tree_id_str, "Failed to get RPC for queue data: {:?}", e);
            ForesterError::RpcPool(e)
        })?;
        fetch_queue_item_data(&mut *rpc, &tree_accounts.queue, queue_fetch_start_index)
            .await
            .map_err(|e| {
                warn!(tree = %tree_id_str, "Failed to fetch queue item data: {:?}", e);
                ForesterError::General {
                    error: format!("Fetch queue data failed for {}: {}", tree_id_str, e),
                }
            })?
            .items
    };

    if queue_item_data.is_empty() {
        trace!(tree = %tree_id_str, "Queue is empty, no transactions to send.");
        return Ok(None); // Return None to indicate no work
    }

    let (priority_fee, recent_blockhash, last_valid_block_height) = {
        let rpc = pool.get_connection().await.map_err(|e| {
            error!(
                tree = %tree_id_str,
                "Failed to get RPC for priority fee: {:?}",
                e
            );
            ForesterError::RpcPool(e)
        })?;
        let forester_epoch_pda_pubkey =
            get_forester_epoch_pda_from_authority(derivation, transaction_builder.epoch()).0;
        let account_keys = vec![
            *payer_pubkey,
            forester_epoch_pda_pubkey,
            tree_accounts.queue,
            tree_accounts.merkle_tree,
        ];
        let priority_fee = PriorityFeeConfig {
            compute_unit_price: config.build_transaction_batch_config.compute_unit_price,
            enable_priority_fees: config.build_transaction_batch_config.enable_priority_fees,
        }
        .resolve(&*rpc, account_keys)
        .await?;

        let (recent_blockhash, last_valid_block_height) =
            fetch_latest_blockhash(pool, &tree_id_str).await?;

        (priority_fee, recent_blockhash, last_valid_block_height)
    };

    let work_items: Vec<WorkItem> = queue_item_data
        .into_iter()
        .map(|data| WorkItem {
            tree_account: tree_accounts,
            queue_item_data: data,
        })
        .collect();

    let timeout_deadline = start_time + config.retry_config.timeout;

    Ok(Some(PreparedBatchData {
        work_items,
        recent_blockhash,
        last_valid_block_height,
        priority_fee,
        timeout_deadline,
    }))
}

async fn fetch_latest_blockhash<R: Rpc>(
    pool: &Arc<SolanaRpcPool<R>>,
    tree_id_str: &str,
) -> std::result::Result<(Hash, u64), ForesterError> {
    let mut rpc = pool.get_connection().await.map_err(|e| {
        error!(
            tree = %tree_id_str,
            "Failed to get RPC for blockhash fetch: {:?}",
            e
        );
        ForesterError::RpcPool(e)
    })?;
    rpc.get_latest_blockhash().await.map_err(|e| {
        error!(tree = %tree_id_str, "Failed to get latest blockhash: {:?}", e);
        ForesterError::Rpc(e)
    })
}

fn compute_effective_max_concurrent_sends(
    config: &SendBatchedTransactionsConfig,
    configured_max: usize,
    work_item_count: usize,
) -> usize {
    let mut effective = configured_max.max(1);

    // Near slot boundary, reduce fan-out so stale-eligibility failures cannot drain fees quickly.
    if config.retry_config.timeout <= Duration::from_secs(5) {
        effective = effective.min(4);
    } else if config.retry_config.timeout <= Duration::from_secs(15) {
        effective = effective.min(8);
    }

    // Large backlog increases blast radius when eligibility changed mid-epoch.
    if work_item_count >= 2_000 {
        effective = effective.min(8);
    } else if work_item_count >= 500 {
        effective = effective.min(12);
    }

    effective.max(1)
}

async fn execute_transaction_chunk_sending<R: Rpc>(
    transactions: Vec<Transaction>,
    last_valid_block_height: u64,
    context: &ChunkSendContext<R>,
) -> std::result::Result<(), ForesterError> {
    if transactions.is_empty() {
        trace!("No transactions in this chunk to send.");
        return Ok(());
    }

    let pool = Arc::clone(&context.pool);
    let cancel_signal = Arc::clone(&context.cancel_signal);
    let num_sent_transactions = Arc::clone(&context.num_sent_transactions);
    let timeout_deadline = context.timeout_deadline;
    let max_concurrent_sends = context.max_concurrent_sends;
    let confirmation = context.confirmation;
    let transaction_send_futures = transactions.into_iter().map(|tx| {
        let pool_clone = Arc::clone(&pool);
        let cancel_signal_clone = Arc::clone(&cancel_signal);
        let num_sent_transactions_clone = Arc::clone(&num_sent_transactions);

        async move {
            if cancel_signal_clone.load(Ordering::SeqCst) || Instant::now() >= timeout_deadline {
                return TransactionSendResult::Cancelled; // Or Timeout
            }

            let tx_signature = tx.signatures.first().copied().unwrap_or_default();
            let tx_signature_str = tx_signature.to_string();

            match pool_clone.get_connection().await {
                Ok(mut rpc) => {
                    if Instant::now() >= timeout_deadline {
                        warn!(tx.signature = %tx_signature_str, "Timeout after getting RPC, before sending tx");
                        return TransactionSendResult::Timeout;
                    }

                    let send_time = Instant::now();
                    let prepared_transaction =
                        PreparedTransaction::legacy(tx, last_valid_block_height);
                    match prepared_transaction
                        .send(&mut *rpc, Some(confirmation), Some(timeout_deadline))
                        .await
                    {
                        Ok(signature) => {
                            if !cancel_signal_clone.load(Ordering::SeqCst) {
                                num_sent_transactions_clone.fetch_add(1, Ordering::SeqCst);
                                trace!(
                                    tx.signature = %signature,
                                    elapsed = ?send_time.elapsed(),
                                    "Transaction sent and confirmed successfully"
                                );
                                TransactionSendResult::Success(signature)
                            } else {
                                trace!(tx.signature = %signature, "Transaction confirmed but run was cancelled post-send");
                                TransactionSendResult::Cancelled
                            }
                        }
                        Err(e) => match e {
                            SmartTransactionError::ConfirmationDeadlineExceeded { signature } => {
                                TransactionSendResult::DeadlineExceeded(signature)
                            }
                            other => {
                                let is_execution_failure = other.has_transaction_error();
                                let is_confirmation_unknown = other.is_confirmation_unknown();
                                let error = ForesterError::from(other);
                                if is_execution_failure {
                                    TransactionSendResult::ExecutionFailure(error, Some(tx_signature))
                                } else if is_confirmation_unknown {
                                    TransactionSendResult::ConfirmationUnknown(
                                        error,
                                        Some(tx_signature),
                                    )
                                } else {
                                    TransactionSendResult::SendFailure(error, Some(tx_signature))
                                }
                            }
                        },
                    }
                }
                Err(e) => {
                    error!(tx.signature_attempt = %tx_signature_str, error = ?e, "Failed to get RPC connection for sending transaction");
                    TransactionSendResult::SendFailure(ForesterError::from(e), Some(tx_signature))
                }
            }
        }
    });

    trace!(
        "Executing batch of sends with concurrency limit {}",
        max_concurrent_sends
    );
    let exec_start = Instant::now();
    let result = futures::stream::iter(transaction_send_futures)
        .buffer_unordered(max_concurrent_sends) // buffer_unordered for concurrency
        .collect::<Vec<TransactionSendResult>>()
        .await;
    let mut saw_not_eligible = false;
    for res in result {
        match res {
            TransactionSendResult::Success(sig) => {
                trace!(tx.signature = %sig, "Transaction confirmed sent");
            }
            TransactionSendResult::SendFailure(err, sig_opt) => {
                increment_transactions_failed("send_failed", 1);
                if err.is_forester_not_eligible() {
                    saw_not_eligible = true;
                    cancel_signal.store(true, Ordering::SeqCst);
                }
                if let Some(sig) = sig_opt {
                    error!(tx.signature = %sig, error = ?err, "Transaction failed to send");
                } else {
                    error!(error = ?err, "Transaction failed to send, no signature available");
                }
            }
            TransactionSendResult::ExecutionFailure(err, sig_opt) => {
                increment_transactions_failed("execution_failed", 1);
                if err.is_forester_not_eligible() {
                    saw_not_eligible = true;
                    cancel_signal.store(true, Ordering::SeqCst);
                }
                if let Some(sig) = sig_opt {
                    error!(
                        tx.signature = %sig,
                        error = ?err,
                        "Transaction failed after send while waiting for confirmation"
                    );
                } else {
                    error!(
                        error = ?err,
                        "Transaction failed after send while waiting for confirmation"
                    );
                }
            }
            TransactionSendResult::ConfirmationUnknown(err, sig_opt) => {
                increment_transactions_failed("confirmation_timeout", 1);
                if let Some(sig) = sig_opt {
                    warn!(
                        tx.signature = %sig,
                        error = ?err,
                        "Transaction confirmation remained unknown after send"
                    );
                } else {
                    warn!(
                        error = ?err,
                        "Transaction confirmation remained unknown after send"
                    );
                }
            }
            TransactionSendResult::DeadlineExceeded(sig) => {
                increment_transactions_failed("deadline_exceeded", 1);
                warn!(
                    tx.signature = %sig,
                    "Transaction missed the scheduled confirmation deadline"
                );
            }
            TransactionSendResult::Cancelled => {
                trace!("Transaction send cancelled due to global signal or timeout");
            }
            TransactionSendResult::Timeout => {
                warn!("Transaction send timed out due to global timeout");
            }
        }
    }
    trace!("Finished executing batch in {:?}", exec_start.elapsed());
    if saw_not_eligible {
        return Err(ForesterError::NotEligible);
    }
    Ok(())
}
