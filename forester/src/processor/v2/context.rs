use std::sync::Arc;

use forester_utils::{forester_epoch::EpochPhases, rpc_pool::SolanaRpcPool};
use light_client::rpc::Rpc;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use tokio::sync::Mutex;

use super::types::BatchProcessorConfig;
use crate::{processor::tx_cache::ProcessedHashCache, slot_tracker::SlotTracker};

#[derive(Debug)]
pub struct BatchContext<R: Rpc> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub authority: Keypair,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub config: BatchProcessorConfig,
    pub ops_cache: Arc<Mutex<ProcessedHashCache>>,
    pub epoch_phases: EpochPhases,
    pub slot_tracker: Arc<SlotTracker>,
}

impl<R: Rpc> BatchContext<R> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        authority: Keypair,
        derivation: Pubkey,
        epoch: u64,
        merkle_tree: Pubkey,
        output_queue: Pubkey,
        config: BatchProcessorConfig,
        ops_cache: Arc<Mutex<ProcessedHashCache>>,
        epoch_phases: EpochPhases,
        slot_tracker: Arc<SlotTracker>,
    ) -> Self {
        Self {
            rpc_pool,
            authority,
            derivation,
            epoch,
            merkle_tree,
            output_queue,
            config,
            ops_cache,
            epoch_phases,
            slot_tracker,
        }
    }

    /// Create a new BatchContext with individual prover parameters (for backward compatibility)
    #[allow(clippy::too_many_arguments)]
    pub fn from_params(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        authority: Keypair,
        derivation: Pubkey,
        epoch: u64,
        merkle_tree: Pubkey,
        output_queue: Pubkey,
        prover_append_url: String,
        prover_update_url: String,
        prover_address_append_url: String,
        prover_api_key: Option<String>,
        prover_polling_interval: std::time::Duration,
        prover_max_wait_time: std::time::Duration,
        ops_cache: Arc<Mutex<ProcessedHashCache>>,
        epoch_phases: EpochPhases,
        slot_tracker: Arc<SlotTracker>,
    ) -> Self {
        let config = BatchProcessorConfig {
            prover_append_url,
            prover_update_url,
            prover_address_append_url,
            prover_api_key,
            prover_polling_interval,
            prover_max_wait_time,
        };

        Self::new(
            rpc_pool,
            authority,
            derivation,
            epoch,
            merkle_tree,
            output_queue,
            config,
            ops_cache,
            epoch_phases,
            slot_tracker,
        )
    }
}
