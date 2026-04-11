use std::{sync::Arc, time::Duration};

use light_client::{
    indexer::Indexer,
    rpc::Rpc,
};
use light_compressed_account::TreeType;
use light_registry::utils::get_forester_epoch_pda_from_authority;
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
    signature::{Keypair, Signer},
};
use tokio::sync::Mutex;

use forester_utils::{
    forester_epoch::{Epoch, TreeAccounts},
    rpc_pool::SolanaRpcPool,
};
use light_registry::protocol_config::state::ProtocolConfig;
use std::sync::atomic::AtomicU64;

use crate::{
    logging::ServiceHeartbeat,
    priority_fee::PriorityFeeConfig,
    processor::{
        tx_cache::ProcessedHashCache,
        v2::{BatchContext, ProverConfig},
    },
    slot_tracker::SlotTracker,
    smart_transaction::{ConfirmationConfig, TransactionPolicy},
    ForesterConfig, Result,
};

/// Shared immutable infrastructure, cheap to clone (all Arc).
///
/// This is the "context" that nearly every method needs: config, RPC access,
/// slot tracking, and transaction helpers. Extracted from EpochManager to
/// make dependencies explicit and enable isolated testing.
#[derive(Debug)]
pub(crate) struct ForesterContext<R: Rpc + Indexer> {
    pub config: Arc<ForesterConfig>,
    pub protocol_config: Arc<ProtocolConfig>,
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub authority: Arc<Keypair>,
    pub slot_tracker: Arc<SlotTracker>,
    pub address_lookup_tables: Arc<Vec<AddressLookupTableAccount>>,
    pub heartbeat: Arc<ServiceHeartbeat>,
    pub run_id: Arc<str>,
}

impl<R: Rpc + Indexer> Clone for ForesterContext<R> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            protocol_config: self.protocol_config.clone(),
            rpc_pool: self.rpc_pool.clone(),
            authority: self.authority.clone(),
            slot_tracker: self.slot_tracker.clone(),
            address_lookup_tables: self.address_lookup_tables.clone(),
            heartbeat: self.heartbeat.clone(),
            run_id: self.run_id.clone(),
        }
    }
}

impl<R: Rpc + Indexer> ForesterContext<R> {
    pub fn confirmation_config(&self) -> ConfirmationConfig {
        ConfirmationConfig {
            max_attempts: self.config.transaction_config.confirmation_max_attempts,
            poll_interval: Duration::from_millis(
                self.config.transaction_config.confirmation_poll_interval_ms,
            ),
        }
    }

    pub fn transaction_priority_fee_config(&self) -> PriorityFeeConfig {
        PriorityFeeConfig {
            compute_unit_price: self.config.transaction_config.priority_fee_microlamports,
            enable_priority_fees: self.config.transaction_config.enable_priority_fees,
        }
    }

    pub fn transaction_policy(&self) -> TransactionPolicy {
        TransactionPolicy {
            priority_fee_config: self.transaction_priority_fee_config(),
            compute_unit_limit: Some(self.config.transaction_config.cu_limit),
            confirmation: Some(self.confirmation_config()),
        }
    }

    pub async fn resolve_epoch_priority_fee<RpcT: Rpc>(
        &self,
        rpc: &RpcT,
        epoch: u64,
    ) -> Result<Option<u64>> {
        self.transaction_priority_fee_config()
            .resolve(
                rpc,
                vec![
                    self.config.payer_keypair.pubkey(),
                    get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0,
                ],
            )
            .await
    }

    pub async fn resolve_tree_priority_fee<RpcT: Rpc>(
        &self,
        rpc: &RpcT,
        epoch: u64,
        tree_accounts: &TreeAccounts,
    ) -> Result<Option<u64>> {
        self.transaction_priority_fee_config()
            .resolve(
                rpc,
                vec![
                    self.config.payer_keypair.pubkey(),
                    get_forester_epoch_pda_from_authority(&self.config.derivation_pubkey, epoch).0,
                    tree_accounts.queue,
                    tree_accounts.merkle_tree,
                ],
            )
            .await
    }

    pub async fn sync_slot(&self) -> Result<u64> {
        let rpc = self.rpc_pool.get_connection().await?;
        let current_slot = rpc.get_slot().await?;
        self.slot_tracker.update(current_slot);
        Ok(current_slot)
    }

    pub async fn get_current_slot_and_epoch(&self) -> Result<(u64, u64)> {
        let slot = self.slot_tracker.estimated_current_slot();
        let epoch = self.protocol_config.get_current_active_epoch(slot)?;
        Ok((slot, epoch))
    }

    pub fn build_batch_context(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        input_queue_hint: Option<u64>,
        output_queue_hint: Option<u64>,
        eligibility_end: Option<u64>,
        ops_cache: Arc<Mutex<ProcessedHashCache>>,
    ) -> BatchContext<R> {
        let default_prover_url = "http://127.0.0.1:3001".to_string();
        let eligibility_end = eligibility_end.unwrap_or(0);
        BatchContext {
            rpc_pool: self.rpc_pool.clone(),
            authority: self.authority.clone(),
            run_id: self.run_id.clone(),
            derivation: self.config.derivation_pubkey,
            epoch: epoch_info.epoch,
            merkle_tree: tree_accounts.merkle_tree,
            output_queue: tree_accounts.queue,
            prover_config: Arc::new(ProverConfig {
                append_url: self
                    .config
                    .external_services
                    .prover_append_url
                    .clone()
                    .unwrap_or_else(|| default_prover_url.clone()),
                update_url: self
                    .config
                    .external_services
                    .prover_update_url
                    .clone()
                    .unwrap_or_else(|| default_prover_url.clone()),
                address_append_url: self
                    .config
                    .external_services
                    .prover_address_append_url
                    .clone()
                    .unwrap_or_else(|| default_prover_url.clone()),
                api_key: self.config.external_services.prover_api_key.clone(),
                polling_interval: self
                    .config
                    .external_services
                    .prover_polling_interval
                    .unwrap_or(Duration::from_secs(1)),
                max_wait_time: self
                    .config
                    .external_services
                    .prover_max_wait_time
                    .unwrap_or(Duration::from_secs(600)),
            }),
            ops_cache,
            epoch_phases: epoch_info.phases.clone(),
            slot_tracker: self.slot_tracker.clone(),
            input_queue_hint,
            output_queue_hint,
            num_proof_workers: self.config.transaction_config.max_concurrent_batches,
            forester_eligibility_end_slot: Arc::new(AtomicU64::new(eligibility_end)),
            address_lookup_tables: self.address_lookup_tables.clone(),
            transaction_policy: self.transaction_policy(),
            max_batches_per_tree: self.config.transaction_config.max_batches_per_tree,
        }
    }
}

pub(crate) fn should_skip_tree(config: &ForesterConfig, tree_type: &TreeType) -> bool {
    match tree_type {
        TreeType::AddressV1 => config.general_config.skip_v1_address_trees,
        TreeType::AddressV2 => config.general_config.skip_v2_address_trees,
        TreeType::StateV1 => config.general_config.skip_v1_state_trees,
        TreeType::StateV2 => config.general_config.skip_v2_state_trees,
        TreeType::Unknown => false, // Never skip compression tree
    }
}
