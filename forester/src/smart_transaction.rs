// adapted from https://github.com/helius-labs/helius-rust-sdk/blob/dev/src/optimized_transaction.rs
// optimized for forester client
use std::time::{Duration, Instant};

use light_client::{rpc::RpcConnection, rpc_pool::SolanaConnectionManager};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    transaction::Transaction,
};
use solana_transaction_status::TransactionConfirmationStatus;
use tokio::time::sleep;
use tracing::log::info;

pub struct CreateSmartTransactionConfig {
    pub payer: Keypair,
    pub recent_blockhash: Hash,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
    pub instructions: Vec<Instruction>,
    pub last_valid_slot: u64,
}

/// Poll a transaction to check whether it has been confirmed
///
/// * `txt-sig` - The transaction signature to check
///
/// # Returns
/// The confirmed transaction signature or an error if the confirmation times out
pub async fn poll_transaction_confirmation<'a, R: RpcConnection>(
    connection: &mut bb8::PooledConnection<'a, SolanaConnectionManager<R>>,
    txt_sig: Signature,
    abort_timeout: Duration,
) -> Result<Signature, light_client::rpc::RpcError> {
    let timeout: Duration = Duration::from_secs(12);
    let interval: Duration = Duration::from_secs(6);
    let start: Instant = Instant::now();

    info!(
        "Starting transaction confirmation polling: sig={}, timeout={:?}, abort_timeout={:?}",
        txt_sig, timeout, abort_timeout
    );

    loop {
        let elapsed = start.elapsed();
        if elapsed >= timeout || elapsed >= abort_timeout {
            info!(
                "Transaction confirmation timed out: elapsed={:?}, timeout={:?}, abort_timeout={:?}",
                elapsed,
                timeout,
                abort_timeout
            );
            return Err(light_client::rpc::RpcError::CustomError(format!(
                "Transaction {}'s confirmation timed out",
                txt_sig
            )));
        }

        info!(
            "Checking transaction status: sig={}, elapsed={:?}/{:?}",
            txt_sig, elapsed, timeout
        );

        let status: Vec<Option<solana_transaction_status::TransactionStatus>> =
            connection.get_signature_statuses(&[txt_sig]).await?;

        match status[0].clone() {
            Some(status) => {
                info!(
                    "Got transaction status: sig={}, error={:?}, confirmation_status={:?}",
                    txt_sig, status.err, status.confirmation_status
                );
                if status.err.is_none()
                    && (status.confirmation_status
                        == Some(TransactionConfirmationStatus::Confirmed)
                        || status.confirmation_status
                            == Some(TransactionConfirmationStatus::Finalized))
                {
                    return Ok(txt_sig);
                }
                if status.err.is_some() {
                    return Err(light_client::rpc::RpcError::CustomError(format!(
                        "Transaction {}'s confirmation failed",
                        txt_sig
                    )));
                }
            }
            None => {
                info!(
                    "No status found, waiting: sig={}, interval={:?}",
                    txt_sig, interval
                );
                sleep(interval).await;
            }
        }
    }
}

// Sends a transaction and handles its confirmation. Retries until timeout or last_valid_block_height is reached.
pub async fn send_and_confirm_transaction<'a, R: RpcConnection>(
    connection: &mut bb8::PooledConnection<'a, SolanaConnectionManager<R>>,
    transaction: &Transaction,
    send_transaction_config: RpcSendTransactionConfig,
    last_valid_slot: u64,
    timeout: Duration,
) -> Result<Signature, light_client::rpc::RpcError> {
    let start_time: Instant = Instant::now();

    // Check current slot before attempting send
    let current_slot = connection.get_slot().await?;
    if current_slot > last_valid_slot {
        info!(
            "Transaction blockhash already expired: current_slot={}, last_valid_slot={}",
            current_slot, last_valid_slot
        );
        return Err(light_client::rpc::RpcError::CustomError(
            "Blockhash expired before transaction send".to_string(),
        ));
    }

    info!(
        "Starting send_and_confirm_transaction with timeout={:?}, last_valid_slot={}",
        timeout, last_valid_slot
    );
    while Instant::now().duration_since(start_time) < timeout
        && connection.get_slot().await? <= last_valid_slot
    {
        let elapsed = Instant::now().duration_since(start_time);
        let current_slot = connection.get_slot().await?;
        info!(
            "Transaction confirmation attempt: elapsed={:?}/{:?}, current_slot={}/{}",
            elapsed, timeout, current_slot, last_valid_slot
        );
        let result = connection.send_transaction_with_config(transaction, send_transaction_config);

        match result.await {
            Ok(signature) => {
                info!(
                    "Transaction sent successfully, signature={}, polling for confirmation",
                    signature
                );
                match poll_transaction_confirmation(connection, signature, timeout).await {
                    Ok(sig) => {
                        info!("Transaction confirmed successfully: {}", sig);
                        return Ok(sig);
                    }
                    Err(e) => {
                        info!("Transaction confirmation polling failed: {:?}", e);
                        continue;
                    }
                }
            }
            Err(e) => {
                info!("Failed to send transaction: {:?}", e);
                continue;
            }
        }
    }

    let final_elapsed = Instant::now().duration_since(start_time);
    let final_slot = connection.get_slot().await?;
    info!(
        "Transaction failed to confirm. Final state: elapsed={:?}/{:?}, slot={}/{}",
        final_elapsed, timeout, final_slot, last_valid_slot
    );

    Err(light_client::rpc::RpcError::CustomError(
        "Transaction failed to confirm within timeout.".to_string(),
    ))
}

/// Creates an optimized transaction based on the provided configuration
///
/// # Arguments
/// * `config` - The configuration for the smart transaction, which includes the transaction's instructions, signers, and lookup tables, depending on
///     whether it's a legacy or versioned smart transaction. The transaction's send configuration can also be changed, if provided
///
/// # Returns
/// An optimized `Transaction` and the `last_valid_block_height`
pub async fn create_smart_transaction(
    config: CreateSmartTransactionConfig,
) -> Result<(Transaction, u64), light_client::rpc::RpcError> {
    let payer_pubkey: Pubkey = config.payer.pubkey();
    let mut final_instructions: Vec<Instruction> = if let Some(price) = config.compute_unit_price {
        vec![ComputeBudgetInstruction::set_compute_unit_price(price)]
    } else {
        vec![]
    };
    if let Some(limit) = config.compute_unit_limit {
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    final_instructions.extend(config.instructions);

    let mut tx = Transaction::new_with_payer(&final_instructions, Some(&payer_pubkey));
    tx.sign(&[&config.payer], config.recent_blockhash);

    Ok((tx, config.last_valid_slot))
}
