use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
    vec,
};

use anchor_lang::error::ERROR_CODE_OFFSET;
use forester_utils::{forester_epoch::TreeAccounts, rpc_pool::SolanaRpcPool};
use futures::StreamExt;
use light_client::rpc::{Rpc, RpcError};
use light_compressed_account::TreeType;
use light_registry::{errors::RegistryError, utils::get_forester_epoch_pda_from_authority};
use reqwest::Url;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_commitment_config::CommitmentLevel;
use solana_sdk::{
    hash::Hash,
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, TransactionError},
};
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

const WORK_ITEM_BATCH_SIZE: usize = 100;
const FORESTER_NOT_ELIGIBLE_CODE: u32 =
    ERROR_CODE_OFFSET + RegistryError::ForesterNotEligible as u32;

use crate::{
    epoch_manager::WorkItem,
    errors::ForesterError,
    metrics::increment_transactions_failed,
    processor::v1::{
        config::SendBatchedTransactionsConfig, helpers::request_priority_fee_estimate,
        tx_builder::TransactionBuilder,
    },
    queue_helpers::fetch_queue_item_data,
    Result,
};

struct PreparedBatchData {
    work_items: Vec<WorkItem>,
    recent_blockhash: Hash,
    last_valid_block_height: u64,
    priority_fee: u64,
    timeout_deadline: Instant,
}

#[allow(clippy::large_enum_variant)]
enum TransactionSendResult {
    Success(Signature),
    Failure(ForesterError, Option<Signature>),
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
) -> Result<usize> {
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
    .await?
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

    // Blockhash expires after ~150 blocks (~60s). Refresh every 30s to stay safe.
    const BLOCKHASH_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
    let mut recent_blockhash = data.recent_blockhash;
    let mut last_valid_block_height = data.last_valid_block_height;
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

        // Refresh blockhash if it's getting stale
        if last_blockhash_refresh.elapsed() > BLOCKHASH_REFRESH_INTERVAL {
            match pool.get_connection().await {
                Ok(mut rpc) => match rpc.get_latest_blockhash().await {
                    Ok((new_hash, new_height)) => {
                        recent_blockhash = new_hash;
                        last_valid_block_height = new_height;
                        last_blockhash_refresh = Instant::now();
                        debug!(tree = %tree_accounts.merkle_tree, "Refreshed blockhash");
                    }
                    Err(e) => {
                        warn!(tree = %tree_accounts.merkle_tree, "Failed to refresh blockhash: {:?}", e);
                    }
                },
                Err(e) => {
                    warn!(tree = %tree_accounts.merkle_tree, "Failed to get RPC for blockhash refresh: {:?}", e);
                }
            }
        }

        trace!(tree = %tree_accounts.merkle_tree, "Processing chunk of size {}", work_chunk.len());
        let build_start_time = Instant::now();

        let (transactions_to_send, _) = match transaction_builder
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

        if let Err(e) = execute_transaction_chunk_sending(
            transactions_to_send,
            Arc::clone(&pool),
            effective_max_concurrent_sends,
            data.timeout_deadline,
            Arc::clone(&operation_cancel_signal),
            Arc::clone(&num_sent_transactions),
            config.confirmation_poll_interval,
            config.confirmation_max_attempts,
        )
        .await
        {
            if is_forester_not_eligible_error(&e) {
                warn!(
                    tree = %tree_accounts.merkle_tree,
                    "Detected ForesterNotEligible while sending V1 transactions; stopping batch loop for re-schedule"
                );
                return Err(ForesterError::NotEligible.into());
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

    let (recent_blockhash, last_valid_block_height) = {
        let mut rpc = pool.get_connection().await.map_err(|e| {
            error!(tree = %tree_id_str, "Failed to get RPC for blockhash: {:?}", e);
            ForesterError::RpcPool(e)
        })?;
        let r_blockhash = rpc.get_latest_blockhash().await.map_err(|e| {
            error!(tree = %tree_id_str, "Failed to get latest blockhash: {:?}", e);
            ForesterError::Rpc(e)
        })?;
        (r_blockhash.0, r_blockhash.1)
    };

    let priority_fee = if config.build_transaction_batch_config.enable_priority_fees {
        let rpc_for_fee = pool.get_connection().await.map_err(|e| {
            error!(tree = %tree_id_str, "Failed to get RPC for priority fee: {:?}", e);
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
        let rpc_url_str = rpc_for_fee.get_url();
        let url = Url::parse(&rpc_url_str).map_err(|e| {
            error!(tree = %tree_id_str, "Failed to parse RPC URL for priority fee: {}, error: {:?}", rpc_url_str, e);
            ForesterError::General { error: format!("Invalid RPC URL: {}", rpc_url_str) }
        })?;
        request_priority_fee_estimate(&url, account_keys).await?
    } else {
        10_000
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

fn rpc_custom_error_code(error: &RpcError) -> Option<u32> {
    match error {
        RpcError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(code),
        )) => Some(*code),
        RpcError::ClientError(client_error) => {
            client_error
                .get_transaction_error()
                .and_then(|transaction_error| match transaction_error {
                    TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                        Some(code)
                    }
                    _ => None,
                })
        }
        _ => None,
    }
}

fn is_forester_not_eligible_error(error: &ForesterError) -> bool {
    match error {
        ForesterError::NotEligible => true,
        ForesterError::Rpc(rpc_error) => {
            rpc_custom_error_code(rpc_error) == Some(FORESTER_NOT_ELIGIBLE_CODE)
        }
        _ => false,
    }
}

async fn wait_for_transaction_execution<R: Rpc>(
    rpc: &R,
    signature: Signature,
    timeout_deadline: Instant,
    poll_interval: Duration,
    max_polls: usize,
) -> std::result::Result<(), ForesterError> {
    for _ in 0..max_polls {
        if Instant::now() >= timeout_deadline {
            return Err(ForesterError::General {
                error: format!(
                    "Timed out waiting for execution status for tx {}",
                    signature
                ),
            });
        }

        let statuses = rpc
            .get_signature_statuses(&[signature])
            .await
            .map_err(ForesterError::from)?;

        if let Some(Some(status)) = statuses.first() {
            if let Some(execution_error) = status.err.clone() {
                let rpc_error = RpcError::TransactionError(execution_error);
                if rpc_custom_error_code(&rpc_error) == Some(FORESTER_NOT_ELIGIBLE_CODE) {
                    return Err(ForesterError::NotEligible);
                }
                return Err(ForesterError::Rpc(rpc_error));
            }
            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }

    Err(ForesterError::General {
        error: format!(
            "Execution status not available after {} polls for tx {}",
            max_polls, signature
        ),
    })
}

async fn execute_transaction_chunk_sending<R: Rpc>(
    transactions: Vec<Transaction>,
    pool: Arc<SolanaRpcPool<R>>,
    max_concurrent_sends: usize,
    timeout_deadline: Instant,
    cancel_signal: Arc<AtomicBool>,
    num_sent_transactions: Arc<AtomicUsize>,
    confirmation_poll_interval: Duration,
    confirmation_max_attempts: usize,
) -> std::result::Result<(), ForesterError> {
    if transactions.is_empty() {
        trace!("No transactions in this chunk to send.");
        return Ok(());
    }

    let transaction_send_futures = transactions.into_iter().map(|tx| {
        let pool_clone = Arc::clone(&pool);
        let rpc_send_config = RpcSendTransactionConfig {
            skip_preflight: true,
            max_retries: Some(0),
            preflight_commitment: Some(CommitmentLevel::Confirmed),
            ..Default::default()
        };
        let cancel_signal_clone = Arc::clone(&cancel_signal);
        let num_sent_transactions_clone = Arc::clone(&num_sent_transactions);

        async move {
            if cancel_signal_clone.load(Ordering::SeqCst) || Instant::now() >= timeout_deadline {
                return TransactionSendResult::Cancelled; // Or Timeout
            }

            let tx_signature = tx.signatures.first().copied().unwrap_or_default();
            let tx_signature_str = tx_signature.to_string();

            match pool_clone.get_connection().await {
                Ok(rpc) => {
                    if Instant::now() >= timeout_deadline {
                        warn!(tx.signature = %tx_signature_str, "Timeout after getting RPC, before sending tx");
                        return TransactionSendResult::Timeout;
                    }

                    let send_time = Instant::now();
                    match rpc.send_transaction_with_config(&tx, rpc_send_config).await {
                        Ok(signature) => {
                            match wait_for_transaction_execution(
                                &*rpc,
                                signature,
                                timeout_deadline,
                                confirmation_poll_interval,
                                confirmation_max_attempts,
                            )
                            .await
                            {
                                Ok(()) => {
                                    if !cancel_signal_clone.load(Ordering::SeqCst) {
                                        num_sent_transactions_clone.fetch_add(1, Ordering::SeqCst);
                                        trace!(
                                            tx.signature = %signature,
                                            elapsed = ?send_time.elapsed(),
                                            "Transaction sent and executed successfully"
                                        );
                                        TransactionSendResult::Success(signature)
                                    } else {
                                        trace!(tx.signature = %signature, "Transaction executed but run was cancelled post-send");
                                        TransactionSendResult::Cancelled
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        tx.signature = %signature,
                                        error = ?e,
                                        "Transaction execution failed after send"
                                    );
                                    TransactionSendResult::Failure(e, Some(signature))
                                }
                            }
                        }
                        Err(e) => {
                            warn!(tx.signature = %tx_signature_str, error = ?e, "Transaction send/process failed");
                            TransactionSendResult::Failure(ForesterError::from(e), Some(tx_signature))
                        }
                    }
                }
                Err(e) => {
                    error!(tx.signature_attempt = %tx_signature_str, error = ?e, "Failed to get RPC connection for sending transaction");
                    TransactionSendResult::Failure(ForesterError::from(e), Some(tx_signature))
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
            TransactionSendResult::Failure(err, sig_opt) => {
                increment_transactions_failed("send_failed", 1);
                if is_forester_not_eligible_error(&err) {
                    saw_not_eligible = true;
                    cancel_signal.store(true, Ordering::SeqCst);
                }
                if let Some(sig) = sig_opt {
                    error!(tx.signature = %sig, error = ?err, "Transaction failed to send");
                } else {
                    error!(error = ?err, "Transaction failed to send, no signature available");
                }
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
