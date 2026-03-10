// adapted from https://github.com/helius-labs/helius-rust-sdk/blob/dev/src/optimized_transaction.rs
// optimized for forester client
use std::time::{Duration, Instant};

use forester_utils::rpc_pool::SolanaConnectionManager;
use light_client::rpc::Rpc;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ComputeBudgetConfig {
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfirmationConfig {
    pub max_attempts: u32,
    pub poll_interval: Duration,
}

pub struct CreateSmartTransactionConfig {
    pub payer: Keypair,
    pub recent_blockhash: Hash,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
    pub instructions: Vec<Instruction>,
    pub last_valid_block_height: u64,
}

pub struct SendSmartTransactionConfig<'a> {
    pub instructions: Vec<Instruction>,
    pub payer: &'a Pubkey,
    pub signers: &'a [&'a Keypair],
    pub address_lookup_tables: &'a [AddressLookupTableAccount],
    pub compute_budget: ComputeBudgetConfig,
    pub confirmation: Option<ConfirmationConfig>,
}

fn with_compute_budget_instructions(
    mut instructions: Vec<Instruction>,
    compute_budget: ComputeBudgetConfig,
) -> Vec<Instruction> {
    let mut final_instructions = Vec::with_capacity(
        instructions.len()
            + usize::from(compute_budget.compute_unit_price.is_some())
            + usize::from(compute_budget.compute_unit_limit.is_some()),
    );

    if let Some(price) = compute_budget.compute_unit_price {
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
    }
    if let Some(limit) = compute_budget.compute_unit_limit {
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    final_instructions.append(&mut instructions);

    final_instructions
}

/// Poll a transaction to check whether it has been confirmed
///
/// * `txt-sig` - The transaction signature to check
///
/// # Returns
/// The confirmed transaction signature or an error if the confirmation times out
pub async fn poll_transaction_confirmation<R: Rpc>(
    connection: &mut bb8::PooledConnection<'_, SolanaConnectionManager<R>>,
    txt_sig: Signature,
    abort_timeout: Duration,
) -> Result<Signature, light_client::rpc::RpcError> {
    // 12 second total timeout before exiting
    let timeout: Duration = Duration::from_secs(12);
    // 6 second retry interval
    let interval: Duration = Duration::from_secs(6);
    let start: Instant = Instant::now();

    loop {
        if start.elapsed() >= timeout || start.elapsed() >= abort_timeout {
            return Err(light_client::rpc::RpcError::CustomError(format!(
                "Transaction {}'s confirmation timed out",
                txt_sig
            )));
        }

        let status: Vec<Option<solana_transaction_status::TransactionStatus>> =
            (**connection).get_signature_statuses(&[txt_sig]).await?;

        match status[0].clone() {
            Some(status) => {
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
                tokio::task::yield_now().await;
                sleep(interval).await;
            }
        }
    }
}

// Sends a transaction and handles its confirmation. Retries until timeout or last_valid_block_height is reached.
pub async fn send_and_confirm_transaction<R: Rpc>(
    connection: &mut bb8::PooledConnection<'_, SolanaConnectionManager<R>>,
    transaction: &Transaction,
    send_transaction_config: RpcSendTransactionConfig,
    last_valid_block_height: u64,
    timeout: Duration,
) -> Result<Signature, light_client::rpc::RpcError> {
    let start_time: Instant = Instant::now();

    while Instant::now().duration_since(start_time) < timeout
        && (**connection).get_slot().await? <= last_valid_block_height
    {
        let result =
            (**connection).send_transaction_with_config(transaction, send_transaction_config);

        match result.await {
            Ok(signature) => {
                // Poll for transaction confirmation
                match poll_transaction_confirmation(connection, signature, timeout).await {
                    Ok(sig) => return Ok(sig),
                    // Retry on polling failure
                    Err(_) => continue,
                }
            }
            // Retry on send failure
            Err(_) => continue,
        }
    }

    Err(light_client::rpc::RpcError::CustomError(
        "Transaction failed to confirm within timeout.".to_string(),
    ))
}

/// Creates an optimized transaction based on the provided configuration
///
/// # Arguments
/// * `config` - The configuration for the smart transaction, which includes the transaction's instructions, signers, and lookup tables, depending on
///   whether it's a legacy or versioned smart transaction. The transaction's send configuration can also be changed, if provided
///
/// # Returns
/// An optimized `Transaction` and the `last_valid_block_height`
pub async fn create_smart_transaction(
    config: CreateSmartTransactionConfig,
) -> Result<(Transaction, u64), light_client::rpc::RpcError> {
    let payer_pubkey: Pubkey = config.payer.pubkey();
    let final_instructions = with_compute_budget_instructions(
        config.instructions,
        ComputeBudgetConfig {
            compute_unit_price: config.compute_unit_price,
            compute_unit_limit: config.compute_unit_limit,
        },
    );

    let mut tx = Transaction::new_with_payer(&final_instructions, Some(&payer_pubkey));
    tx.sign(&[&config.payer], config.recent_blockhash);

    Ok((tx, config.last_valid_block_height))
}

pub async fn send_smart_transaction<R: Rpc>(
    rpc: &mut R,
    config: SendSmartTransactionConfig<'_>,
) -> Result<Signature, light_client::rpc::RpcError> {
    let final_instructions =
        with_compute_budget_instructions(config.instructions, config.compute_budget);

    if config.address_lookup_tables.is_empty() {
        if let Some(confirmation) = config.confirmation {
            send_and_poll_confirmation(rpc, &final_instructions, config.payer, config.signers, confirmation).await
        } else {
            rpc.create_and_send_transaction(&final_instructions, config.payer, config.signers)
                .await
        }
    } else {
        rpc.create_and_send_versioned_transaction(
            &final_instructions,
            config.payer,
            config.signers,
            config.address_lookup_tables,
        )
        .await
    }
}

/// Send a legacy transaction and poll for confirmation with configurable parameters.
async fn send_and_poll_confirmation<R: Rpc>(
    rpc: &mut R,
    instructions: &[Instruction],
    payer: &Pubkey,
    signers: &[&Keypair],
    confirmation: ConfirmationConfig,
) -> Result<Signature, light_client::rpc::RpcError> {
    let (blockhash, _) = rpc.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(instructions, Some(payer));
    transaction
        .try_sign(&signers.iter().copied().collect::<Vec<_>>(), blockhash)
        .map_err(|e| light_client::rpc::RpcError::SigningError(e.to_string()))?;

    let signature = rpc
        .send_transaction_with_config(
            &transaction,
            RpcSendTransactionConfig {
                skip_preflight: true,
                max_retries: Some(0),
                ..Default::default()
            },
        )
        .await?;

    for _ in 0..confirmation.max_attempts {
        sleep(confirmation.poll_interval).await;

        let statuses = rpc.get_signature_statuses(&[signature]).await?;
        if let Some(Some(status)) = statuses.first() {
            if let Some(err) = &status.err {
                return Err(light_client::rpc::RpcError::TransactionError(err.clone()));
            }
            if matches!(
                status.confirmation_status,
                Some(TransactionConfirmationStatus::Confirmed | TransactionConfirmationStatus::Finalized)
            ) {
                return Ok(signature);
            }
        }
    }

    Err(light_client::rpc::RpcError::CustomError(format!(
        "Transaction {} confirmation timed out after {} attempts",
        signature, confirmation.max_attempts
    )))
}
