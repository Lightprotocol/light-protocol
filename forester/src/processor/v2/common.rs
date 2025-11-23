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
use light_registry::protocol_config::state::EpochState;
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount, instruction::Instruction, pubkey::Pubkey,
    signature::Keypair, signer::Signer,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::{errors::V2Error, proof_worker::ProofJob};
use crate::{
    errors::ForesterError, processor::tx_cache::ProcessedHashCache, slot_tracker::SlotTracker,
    Result,
};

const SLOTS_STOP_THRESHOLD: u64 = 3;

#[derive(Debug, Clone)]
pub struct QueueWork {
    pub queue_type: light_compressed_account::QueueType,
    pub queue_size: u64,
}

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
}

impl<R: Rpc> Clone for BatchContext<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: self.rpc_pool.clone(),
            authority: self.authority.clone(),
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
        }
    }
}

pub(crate) async fn send_transaction_batch<R: Rpc>(
    context: &BatchContext<R>,
    instructions: Vec<Instruction>,
) -> Result<String> {
    let current_slot = context.slot_tracker.estimated_current_slot();
    let current_phase_state = context.epoch_phases.get_current_epoch_state(current_slot);

    if current_phase_state != EpochState::Active {
        debug!(
            "Skipping transaction send: not in active phase (current phase: {:?}, slot: {})",
            current_phase_state, current_slot
        );
        return Err(ForesterError::NotInActivePhase.into());
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
    if slots_remaining < SLOTS_STOP_THRESHOLD {
        debug!(
            "Skipping transaction send: only {} slots remaining until eligibility ends",
            slots_remaining
        );
        return Err(ForesterError::NotInActivePhase.into());
    }

    info!(
        "Sending transaction with {} instructions for tree: {}...",
        instructions.len(),
        context.merkle_tree
    );
    let mut rpc = context.rpc_pool.get_connection().await?;

    let signature = if !context.address_lookup_tables.is_empty() {
        debug!(
            "Using versioned transaction with {} lookup tables",
            context.address_lookup_tables.len()
        );
        rpc.create_and_send_versioned_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[context.authority.as_ref()],
            &context.address_lookup_tables,
        )
        .await?
    } else {
        rpc.create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[context.authority.as_ref()],
        )
        .await?
    };

    debug!("Waiting for transaction confirmation: {}", signature);

    const MAX_CONFIRMATION_ATTEMPTS: u32 = 30;
    const CONFIRMATION_POLL_INTERVAL: Duration = Duration::from_millis(500);

    for attempt in 0..MAX_CONFIRMATION_ATTEMPTS {
        let statuses = rpc.get_signature_statuses(&[signature]).await?;

        if let Some(Some(status)) = statuses.first() {
            if let Some(err) = &status.err {
                error!(
                    "transaction {} failed for tree {}: {:?}",
                    signature, context.merkle_tree, err
                );
                return Err(V2Error::from_transaction_error(context.merkle_tree, err).into());
            }

            // Transaction succeeded - check confirmation status
            // confirmations == None means finalized, Some(n) means n confirmations
            let is_confirmed = status.confirmations.is_none() || status.confirmations >= Some(1);
            if is_confirmed {
                info!(
                    "Transaction confirmed successfully: {} for tree: {} (slot: {}, confirmations: {:?})",
                    signature, context.merkle_tree, status.slot, status.confirmations
                );
                return Ok(signature.to_string());
            }

            debug!(
                "Transaction {} pending confirmation (attempt {}/{}, confirmations: {:?})",
                signature,
                attempt + 1,
                MAX_CONFIRMATION_ATTEMPTS,
                status.confirmations
            );
        } else {
            debug!(
                "Transaction {} not yet visible (attempt {}/{})",
                signature,
                attempt + 1,
                MAX_CONFIRMATION_ATTEMPTS
            );
        }

        tokio::time::sleep(CONFIRMATION_POLL_INTERVAL).await;
    }

    warn!(
        "Transaction {} timed out waiting for confirmation for tree {}",
        signature, context.merkle_tree
    );
    Err(anyhow::anyhow!(
        "Transaction {} timed out waiting for confirmation for tree {}",
        signature,
        context.merkle_tree
    ))
}
