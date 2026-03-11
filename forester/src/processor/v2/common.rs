use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use forester_utils::{forester_epoch::EpochPhases, rpc_pool::SolanaRpcPool};
pub use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
use light_client::rpc::Rpc;
use light_registry::{
    protocol_config::state::EpochState, utils::get_forester_epoch_pda_from_authority,
};
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount, instruction::Instruction, pubkey::Pubkey,
    signature::Keypair, signer::Signer,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::{errors::V2Error, proof_worker::ProofJob};
use crate::{
    errors::ForesterError,
    metrics::increment_transactions_failed,
    processor::tx_cache::ProcessedHashCache,
    slot_tracker::SlotTracker,
    smart_transaction::{
        send_transaction_with_policy, SendTransactionWithPolicyConfig, SmartTransactionError,
        TransactionPolicy,
    },
    transaction_timing::scheduled_confirmation_deadline,
};

#[derive(Debug)]
pub struct WorkerPool {
    pub job_tx: async_channel::Sender<ProofJob>,
}

pub fn clamp_to_u16(value: u64, name: &str) -> u16 {
    match value.try_into() {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!(
                "{} {} exceeds u16::MAX, clamping to {}",
                name,
                value,
                u16::MAX
            );
            u16::MAX
        }
    }
}

#[inline]
pub fn batch_range(zkp_batch_size: u64, total_len: usize, start: usize) -> std::ops::Range<usize> {
    let end = (start + zkp_batch_size as usize).min(total_len);
    start..end
}

pub fn get_leaves_hashchain(
    leaves_hash_chains: &[[u8; 32]],
    batch_idx: usize,
) -> crate::Result<[u8; 32]> {
    leaves_hash_chains.get(batch_idx).copied().ok_or_else(|| {
        anyhow::anyhow!(
            "Missing leaves_hash_chain for batch {} (available: {})",
            batch_idx,
            leaves_hash_chains.len()
        )
    })
}

#[derive(Debug, Clone)]
pub struct ProverConfig {
    pub append_url: String,
    pub update_url: String,
    pub address_append_url: String,
    pub api_key: Option<String>,
    pub polling_interval: Duration,
    pub max_wait_time: Duration,
}

#[derive(Debug)]
pub struct BatchContext<R: Rpc> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub authority: Arc<Keypair>,
    pub run_id: Arc<str>,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub prover_config: Arc<ProverConfig>,
    pub ops_cache: Arc<Mutex<ProcessedHashCache>>,
    pub epoch_phases: EpochPhases,
    pub slot_tracker: Arc<SlotTracker>,
    pub input_queue_hint: Option<u64>,
    pub output_queue_hint: Option<u64>,
    pub num_proof_workers: usize,
    pub forester_eligibility_end_slot: Arc<AtomicU64>,
    pub address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
    pub transaction_policy: TransactionPolicy,
    /// Maximum batches to process per tree per iteration
    pub max_batches_per_tree: usize,
}

impl<R: Rpc> Clone for BatchContext<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: self.rpc_pool.clone(),
            authority: self.authority.clone(),
            run_id: self.run_id.clone(),
            derivation: self.derivation,
            epoch: self.epoch,
            merkle_tree: self.merkle_tree,
            output_queue: self.output_queue,
            prover_config: self.prover_config.clone(),
            ops_cache: self.ops_cache.clone(),
            epoch_phases: self.epoch_phases.clone(),
            slot_tracker: self.slot_tracker.clone(),
            input_queue_hint: self.input_queue_hint,
            output_queue_hint: self.output_queue_hint,
            num_proof_workers: self.num_proof_workers,
            forester_eligibility_end_slot: self.forester_eligibility_end_slot.clone(),
            address_lookup_tables: self.address_lookup_tables.clone(),
            transaction_policy: self.transaction_policy,
            max_batches_per_tree: self.max_batches_per_tree,
        }
    }
}

pub(crate) async fn send_transaction_batch<R: Rpc>(
    context: &BatchContext<R>,
    instructions: Vec<Instruction>,
) -> std::result::Result<String, ForesterError> {
    let current_slot = context.slot_tracker.estimated_current_slot();
    let current_phase_state = context.epoch_phases.get_current_epoch_state(current_slot);

    if current_phase_state != EpochState::Active {
        debug!(
            "Skipping transaction send: not in active phase (current phase: {:?}, slot: {})",
            current_phase_state, current_slot
        );
        return Err(ForesterError::NotInActivePhase);
    }

    let forester_end = context
        .forester_eligibility_end_slot
        .load(Ordering::Acquire);
    let eligibility_end_slot = if forester_end > 0 {
        forester_end
    } else {
        context.epoch_phases.active.end
    };
    let slots_remaining = eligibility_end_slot.saturating_sub(current_slot);
    let Some(confirmation_deadline) = scheduled_confirmation_deadline(slots_remaining) else {
        debug!(
            "Skipping transaction send: only {} slots remaining until eligibility ends",
            slots_remaining
        );
        return Err(ForesterError::NotInActivePhase);
    };

    info!(
        "Sending transaction with {} instructions for tree: {}...",
        instructions.len(),
        context.merkle_tree
    );
    let mut rpc = context.rpc_pool.get_connection().await?;
    let forester_epoch_pda_pubkey =
        get_forester_epoch_pda_from_authority(&context.derivation, context.epoch).0;
    let payer = context.authority.pubkey();
    let signers = [context.authority.as_ref()];
    let address_lookup_tables = context.address_lookup_tables.as_ref();

    if !address_lookup_tables.is_empty() {
        debug!(
            "Using versioned transaction with {} lookup tables",
            address_lookup_tables.len()
        );
    }

    let signature = send_transaction_with_policy(
        &mut *rpc,
        SendTransactionWithPolicyConfig {
            instructions,
            payer: &payer,
            signers: &signers,
            address_lookup_tables,
            priority_fee_accounts: vec![
                context.authority.pubkey(),
                forester_epoch_pda_pubkey,
                context.output_queue,
                context.merkle_tree,
            ],
            policy: context.transaction_policy,
            confirmation_deadline: Some(confirmation_deadline),
        },
    )
    .await
    .map_err(|error| map_send_error(context.merkle_tree, error))?;

    info!(
        "Transaction confirmed successfully: {} for tree: {}",
        signature, context.merkle_tree
    );
    Ok(signature.to_string())
}

fn map_send_error(tree: Pubkey, error: SmartTransactionError) -> ForesterError {
    if let Some(transaction_error) = error.transaction_error() {
        increment_transactions_failed("execution_failed", 1);
        error!(
            tree = %tree,
            error = ?transaction_error,
            "V2 transaction execution failed"
        );
        V2Error::from_transaction_error(tree, &transaction_error).into()
    } else if error.is_confirmation_deadline_exceeded() {
        increment_transactions_failed("deadline_exceeded", 1);
        warn!(
            tree = %tree,
            error = ?error,
            "V2 transaction missed the scheduled confirmation deadline"
        );
        ForesterError::from(error)
    } else if error.is_confirmation_unknown() {
        increment_transactions_failed("confirmation_timeout", 1);
        warn!(
            tree = %tree,
            error = ?error,
            "V2 transaction confirmation remained unknown after send"
        );
        ForesterError::from(error)
    } else {
        increment_transactions_failed("send_failed", 1);
        error!(
            tree = %tree,
            error = ?error,
            "V2 transaction send failed"
        );
        ForesterError::from(error)
    }
}
