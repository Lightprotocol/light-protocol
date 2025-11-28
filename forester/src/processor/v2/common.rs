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
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::{
    errors::ForesterError, processor::tx_cache::ProcessedHashCache, slot_tracker::SlotTracker,
    Result,
};

const SLOTS_STOP_THRESHOLD: u64 = 1;

// ============================================================================
// Common Infrastructure
// ============================================================================
// Shared utilities for state and address batch processing.
// Extracted to eliminate code duplication and improve maintainability.

/// Queue work specification for batch processing.
///
/// Used by both state and address processors to specify what needs processing.
#[derive(Debug, Clone)]
pub struct QueueWork {
    /// Type of queue (output/input for state, address for address trees)
    pub queue_type: light_compressed_account::QueueType,
    /// Number of items in the queue
    pub queue_size: u64,
}

/// Worker pool managing persistent proof generation workers.
///
/// Both state and address processors use this pool to distribute
/// proof generation jobs across multiple workers for parallel processing.
#[derive(Debug)]
pub struct WorkerPool {
    /// Channel for sending proof jobs to workers
    pub job_tx: async_channel::Sender<super::state::proof_worker::ProofJob>,
}

/// Clamps a u64 value to u16, warning if clamping occurs.
///
/// Used when converting batch sizes and fetch lengths for the indexer API,
/// which requires u16 parameters.
///
/// # Arguments
/// * `value` - The u64 value to clamp
/// * `name` - Name of the parameter (for logging)
///
/// # Returns
/// The clamped u16 value (u16::MAX if value exceeds u16::MAX)
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

/// Computes the slice range for a batch given total length and start index.
///
/// Ensures the range doesn't exceed the available data by capping at `total_len`.
///
/// # Arguments
/// * `zkp_batch_size` - Size of each ZKP batch
/// * `total_len` - Total length of available data
/// * `start` - Starting index for this batch
///
/// # Returns
/// A range from `start` to `min(start + zkp_batch_size, total_len)`
#[inline]
pub fn batch_range(zkp_batch_size: u64, total_len: usize, start: usize) -> std::ops::Range<usize> {
    let end = (start + zkp_batch_size as usize).min(total_len);
    start..end
}

/// Gets the pre-computed leaves hashchain for a specific batch.
///
/// Each batch has a pre-computed hash chain of its leaves, which is used
/// in the ZK circuit to prove batch integrity.
///
/// # Arguments
/// * `leaves_hash_chains` - Array of hash chains, one per batch
/// * `batch_idx` - Index of the batch to retrieve
///
/// # Returns
/// The 32-byte hash chain for the specified batch
///
/// # Errors
/// Returns an error if `batch_idx` is out of bounds
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

/// Creates a worker pool with persistent proof workers.
///
/// Spawns the specified number of worker tasks that continuously
/// process proof generation jobs from the returned channel.
///
/// # Arguments
/// * `num_workers` - Number of worker tasks to spawn
/// * `prover_config` - Configuration for connecting to the prover service
///
/// # Returns
/// A WorkerPool with a channel for sending proof jobs
pub fn create_worker_pool(num_workers: usize, prover_config: ProverConfig) -> WorkerPool {
    use super::state::proof_worker::spawn_proof_workers;
    let job_tx = spawn_proof_workers(num_workers, prover_config);
    WorkerPool { job_tx }
}

/// Ensures worker pool is initialized, creating it if necessary.
///
/// Lazily initializes the worker pool on first call. Subsequent calls
/// are no-ops if the pool is already initialized.
///
/// # Arguments
/// * `pool` - Mutable reference to the optional worker pool
/// * `num_workers` - Number of workers to spawn (minimum 1)
/// * `prover_config` - Configuration for the prover service
/// * `tree_name` - Name for logging (e.g., "StateSupervisor tree ABC...")
pub fn ensure_worker_pool(
    pool: &mut Option<WorkerPool>,
    num_workers: usize,
    prover_config: &ProverConfig,
    tree_name: &str,
) {
    if pool.is_none() {
        let actual_workers = num_workers.max(1);
        *pool = Some(create_worker_pool(actual_workers, prover_config.clone()));
        tracing::info!(
            "Spawned {} persistent proof workers for {}",
            actual_workers,
            tree_name
        );
    }
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
    pub prover_config: ProverConfig,
    pub ops_cache: Arc<Mutex<ProcessedHashCache>>,
    pub epoch_phases: EpochPhases,
    pub slot_tracker: Arc<SlotTracker>,
    pub input_queue_hint: Option<u64>,
    pub output_queue_hint: Option<u64>,
    pub num_proof_workers: usize,
    pub forester_eligibility_end_slot: Arc<AtomicU64>,
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
            "!! Skipping transaction send: not in active phase (current phase: {:?}, slot: {})",
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
    let signature = rpc
        .create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[context.authority.as_ref()],
        )
        .await?;

    // Ensure transaction is confirmed before returning
    debug!("Waiting for transaction confirmation: {}", signature);
    let confirmed = rpc.confirm_transaction(signature).await?;
    if !confirmed {
        return Err(anyhow::anyhow!(
            "Transaction {} failed to confirm for tree {}",
            signature,
            context.merkle_tree
        ));
    }

    info!(
        "Transaction confirmed successfully: {} for tree: {}",
        signature, context.merkle_tree
    );

    Ok(signature.to_string())
}
