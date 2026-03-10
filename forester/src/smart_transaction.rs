// adapted from https://github.com/helius-labs/helius-rust-sdk/blob/dev/src/optimized_transaction.rs
// optimized for forester client
use std::{
    collections::HashSet,
    future::Future,
    time::{Duration, Instant},
};

use forester_utils::rpc_pool::SolanaConnectionManager;
use light_client::rpc::{Rpc, RpcError};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    transaction::{Transaction, VersionedTransaction},
};
use solana_transaction_status::TransactionConfirmationStatus;
use tokio::time::sleep;

use crate::priority_fee::PriorityFeeConfig;

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

#[derive(Debug, Clone, Copy, Default)]
pub struct TransactionPolicy {
    pub priority_fee_config: PriorityFeeConfig,
    pub compute_unit_limit: Option<u32>,
    pub confirmation: Option<ConfirmationConfig>,
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

pub struct SendTransactionWithPolicyConfig<'a> {
    pub instructions: Vec<Instruction>,
    pub payer: &'a Pubkey,
    pub signers: &'a [&'a Keypair],
    pub address_lookup_tables: &'a [AddressLookupTableAccount],
    pub priority_fee_accounts: Vec<Pubkey>,
    pub policy: TransactionPolicy,
}

pub fn collect_priority_fee_accounts(payer: Pubkey, instructions: &[Instruction]) -> Vec<Pubkey> {
    let mut seen = HashSet::with_capacity(1 + instructions.len() * 4);
    let mut account_keys = Vec::with_capacity(1 + instructions.len() * 4);

    if seen.insert(payer) {
        account_keys.push(payer);
    }

    for instruction in instructions {
        for account_meta in &instruction.accounts {
            if seen.insert(account_meta.pubkey) {
                account_keys.push(account_meta.pubkey);
            }
        }
    }

    account_keys
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
) -> Result<Signature, RpcError> {
    // 12 second total timeout before exiting
    let timeout: Duration = Duration::from_secs(12);
    // 6 second retry interval
    let interval: Duration = Duration::from_secs(6);
    let start: Instant = Instant::now();

    loop {
        if start.elapsed() >= timeout || start.elapsed() >= abort_timeout {
            return Err(RpcError::CustomError(format!(
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
                    return Err(RpcError::CustomError(format!(
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
) -> Result<Signature, RpcError> {
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

    Err(RpcError::CustomError(
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
) -> Result<(Transaction, u64), RpcError> {
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

pub async fn send_transaction_with_policy<R: Rpc>(
    rpc: &mut R,
    config: SendTransactionWithPolicyConfig<'_>,
) -> Result<Signature, RpcError> {
    let compute_unit_price = config
        .policy
        .priority_fee_config
        .resolve(&*rpc, config.priority_fee_accounts)
        .await
        .map_err(|error| {
            RpcError::CustomError(format!("Failed to resolve priority fee: {error}"))
        })?;

    send_smart_transaction(
        rpc,
        SendSmartTransactionConfig {
            instructions: config.instructions,
            payer: config.payer,
            signers: config.signers,
            address_lookup_tables: config.address_lookup_tables,
            compute_budget: ComputeBudgetConfig {
                compute_unit_price,
                compute_unit_limit: config.policy.compute_unit_limit,
            },
            confirmation: config.policy.confirmation,
        },
    )
    .await
}

struct PreparedTransaction {
    transaction: PreparedTransactionKind,
    last_valid_block_height: u64,
}

enum PreparedTransactionKind {
    Legacy(Transaction),
    Versioned(VersionedTransaction),
}

impl PreparedTransaction {
    fn signature(&self) -> Option<Signature> {
        match &self.transaction {
            PreparedTransactionKind::Legacy(transaction) => transaction.signatures.first().copied(),
            PreparedTransactionKind::Versioned(transaction) => {
                transaction.signatures.first().copied()
            }
        }
    }

    fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    async fn process<R: Rpc>(&self, rpc: &mut R) -> Result<Signature, RpcError> {
        match &self.transaction {
            PreparedTransactionKind::Legacy(transaction) => {
                rpc.process_transaction(transaction.clone()).await
            }
            PreparedTransactionKind::Versioned(transaction) => {
                rpc.process_versioned_transaction(transaction.clone()).await
            }
        }
    }

    async fn send_with_confirmation_config<R: Rpc>(&self, rpc: &R) -> Result<Signature, RpcError> {
        let config = confirmation_send_transaction_config();
        match &self.transaction {
            PreparedTransactionKind::Legacy(transaction) => {
                rpc.send_transaction_with_config(transaction, config).await
            }
            PreparedTransactionKind::Versioned(transaction) => {
                rpc.send_versioned_transaction_with_config(transaction, config)
                    .await
            }
        }
    }
}

pub async fn send_smart_transaction<R: Rpc>(
    rpc: &mut R,
    config: SendSmartTransactionConfig<'_>,
) -> Result<Signature, RpcError> {
    let SendSmartTransactionConfig {
        instructions,
        payer,
        signers,
        address_lookup_tables,
        compute_budget,
        confirmation,
    } = config;
    let prepared = prepare_transaction(
        rpc,
        instructions,
        payer,
        signers,
        address_lookup_tables,
        compute_budget,
    )
    .await?;

    send_prepared_transaction(rpc, &prepared, confirmation).await
}

async fn prepare_transaction<R: Rpc>(
    rpc: &mut R,
    instructions: Vec<Instruction>,
    payer: &Pubkey,
    signers: &[&Keypair],
    address_lookup_tables: &[AddressLookupTableAccount],
    compute_budget: ComputeBudgetConfig,
) -> Result<PreparedTransaction, RpcError> {
    let final_instructions = with_compute_budget_instructions(instructions, compute_budget);
    let (blockhash, last_valid_block_height) = rpc.get_latest_blockhash().await?;

    if address_lookup_tables.is_empty() {
        let mut transaction = Transaction::new_with_payer(&final_instructions, Some(payer));
        transaction
            .try_sign(signers, blockhash)
            .map_err(|e| RpcError::SigningError(e.to_string()))?;
        Ok(PreparedTransaction {
            transaction: PreparedTransactionKind::Legacy(transaction),
            last_valid_block_height,
        })
    } else {
        let message =
            v0::Message::try_compile(payer, &final_instructions, address_lookup_tables, blockhash)
                .map_err(|e| {
                    RpcError::CustomError(format!("Failed to compile v0 message: {}", e))
                })?;
        let transaction = VersionedTransaction::try_new(VersionedMessage::V0(message), signers)
            .map_err(|e| RpcError::SigningError(e.to_string()))?;
        Ok(PreparedTransaction {
            transaction: PreparedTransactionKind::Versioned(transaction),
            last_valid_block_height,
        })
    }
}

async fn send_prepared_transaction<R: Rpc>(
    rpc: &mut R,
    transaction: &PreparedTransaction,
    confirmation: Option<ConfirmationConfig>,
) -> Result<Signature, RpcError> {
    match confirmation {
        Some(confirmation) => {
            let signature = transaction.signature().ok_or_else(|| {
                RpcError::CustomError("Prepared transaction missing signature".into())
            })?;
            let rpc = &*rpc;
            resend_until_confirmed(
                rpc,
                signature,
                transaction.last_valid_block_height(),
                confirmation,
                || transaction.send_with_confirmation_config(rpc),
            )
            .await
        }
        None => transaction.process(rpc).await,
    }
}

fn confirmation_send_transaction_config() -> RpcSendTransactionConfig {
    RpcSendTransactionConfig {
        skip_preflight: true,
        // Keep RPC-level retries disabled and drive resend timing from
        // confirmation.max_attempts/poll_interval in resend_until_confirmed.
        max_retries: Some(0),
        ..Default::default()
    }
}

async fn resend_until_confirmed<R, F, Fut>(
    rpc: &R,
    signature: Signature,
    last_valid_block_height: u64,
    confirmation: ConfirmationConfig,
    mut send_transaction: F,
) -> Result<Signature, RpcError>
where
    R: Rpc,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Signature, RpcError>>,
{
    // Mirror send_and_confirm_transaction's resend-until-timeout behavior, but use
    // confirmation.max_attempts and confirmation.poll_interval for caller-controlled timing.
    let mut last_send_error = None;

    for attempt in 0..confirmation.max_attempts {
        if signature_is_confirmed(rpc, signature).await? {
            return Ok(signature);
        }

        if blockhash_has_expired(rpc, last_valid_block_height).await? {
            return Err(blockhash_expired_error(signature, last_valid_block_height));
        }

        match send_transaction().await {
            Ok(_) => last_send_error = None,
            Err(error) if rpc.should_retry(&error) => last_send_error = Some(error),
            Err(error) => return Err(error),
        }

        if signature_is_confirmed(rpc, signature).await? {
            return Ok(signature);
        }

        if attempt + 1 < confirmation.max_attempts {
            sleep(confirmation.poll_interval).await;
        }
    }

    if signature_is_confirmed(rpc, signature).await? {
        return Ok(signature);
    }

    if blockhash_has_expired(rpc, last_valid_block_height).await? {
        return Err(blockhash_expired_error(signature, last_valid_block_height));
    }

    if let Some(error) = last_send_error {
        return Err(error);
    }

    Err(RpcError::CustomError(format!(
        "Transaction {} confirmation timed out after {} attempts",
        signature, confirmation.max_attempts
    )))
}

async fn signature_is_confirmed<R: Rpc>(rpc: &R, signature: Signature) -> Result<bool, RpcError> {
    let statuses = rpc.get_signature_statuses(&[signature]).await?;
    if let Some(Some(status)) = statuses.first() {
        if let Some(err) = &status.err {
            return Err(RpcError::TransactionError(err.clone()));
        }

        return Ok(matches!(
            status.confirmation_status,
            Some(
                TransactionConfirmationStatus::Confirmed | TransactionConfirmationStatus::Finalized
            )
        ));
    }

    Ok(false)
}

async fn blockhash_has_expired<R: Rpc>(
    rpc: &R,
    last_valid_block_height: u64,
) -> Result<bool, RpcError> {
    Ok(rpc.get_block_height().await? > last_valid_block_height)
}

fn blockhash_expired_error(signature: Signature, last_valid_block_height: u64) -> RpcError {
    RpcError::CustomError(format!(
        "Transaction {} blockhash expired at block height {} before confirmation",
        signature, last_valid_block_height
    ))
}
