//! V2 tree processing, proof cache prewarming, and cached proof sending.

use std::time::Duration;

use anyhow::{anyhow, Context};
use borsh::BorshSerialize;
use forester_utils::forester_epoch::{Epoch, TreeAccounts};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::TreeType;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
    create_batch_update_address_tree_instruction,
};
use solana_sdk::signature::Signer;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use super::{context::should_skip_tree, EpochManager};
use crate::{
    errors::ForesterError,
    logging::should_emit_rate_limited_warning,
    processor::v2::{BatchInstruction, ProcessingResult},
    slot_tracker::slot_duration,
    smart_transaction::{send_smart_transaction, ComputeBudgetConfig, SendSmartTransactionConfig},
    transaction_timing::scheduled_confirmation_deadline,
    ForesterEpochInfo,
};

impl<R: Rpc + Indexer> EpochManager<R> {
    pub(crate) async fn process_v2(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        consecutive_eligibility_end: u64,
    ) -> std::result::Result<ProcessingResult, ForesterError> {
        match tree_accounts.tree_type {
            TreeType::StateV2 => {
                let processor = self
                    .processor_pool
                    .get_or_create_state_processor(
                        &self.ctx,
                        epoch_info,
                        tree_accounts,
                        self.ops_cache.clone(),
                    )
                    .await?;

                let cache = self
                    .processor_pool
                    .get_or_create_proof_cache(tree_accounts.merkle_tree);

                {
                    let mut proc = processor.lock().await;
                    proc.update_eligibility(consecutive_eligibility_end);
                    proc.set_proof_cache(cache);
                }

                let mut proc = processor.lock().await;
                match proc.process().await {
                    Ok(res) => Ok(res),
                    Err(error) if matches!(&error, ForesterError::V2(v2_error) if v2_error.is_constraint()) =>
                    {
                        warn!(
                            event = "v2_state_constraint_error",
                            run_id = %self.ctx.run_id,
                            tree = %tree_accounts.merkle_tree,
                            error = %error,
                            "State processing hit constraint error. Dropping processor to flush cache."
                        );
                        drop(proc);
                        self.processor_pool
                            .remove_state_processor(&tree_accounts.merkle_tree);
                        self.processor_pool
                            .remove_proof_cache(&tree_accounts.merkle_tree);
                        Err(error)
                    }
                    Err(ForesterError::V2(v2_error)) if v2_error.is_hashchain_mismatch() => {
                        let warning_key =
                            format!("v2_state_hashchain_mismatch:{}", tree_accounts.merkle_tree);
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(15)) {
                            warn!(
                                event = "v2_state_hashchain_mismatch",
                                run_id = %self.ctx.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %v2_error,
                                "State processing hit hashchain mismatch. Clearing cache and retrying."
                            );
                        }
                        self.ctx.heartbeat.increment_v2_recoverable_error();
                        proc.clear_cache().await;
                        Ok(ProcessingResult::default())
                    }
                    Err(e) => {
                        let warning_key =
                            format!("v2_state_process_failed:{}", tree_accounts.merkle_tree);
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(10)) {
                            warn!(
                                event = "v2_state_process_failed_retrying",
                                run_id = %self.ctx.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %e,
                                "Failed to process state queue. Will retry next tick without dropping processor."
                            );
                        }
                        self.ctx.heartbeat.increment_v2_recoverable_error();
                        Ok(ProcessingResult::default())
                    }
                }
            }
            TreeType::AddressV2 => {
                let processor = self
                    .processor_pool
                    .get_or_create_address_processor(
                        &self.ctx,
                        epoch_info,
                        tree_accounts,
                        self.ops_cache.clone(),
                    )
                    .await?;

                let cache = self
                    .processor_pool
                    .get_or_create_proof_cache(tree_accounts.merkle_tree);

                {
                    let mut proc = processor.lock().await;
                    proc.update_eligibility(consecutive_eligibility_end);
                    proc.set_proof_cache(cache);
                }

                let mut proc = processor.lock().await;
                match proc.process().await {
                    Ok(res) => Ok(res),
                    Err(error) if matches!(&error, ForesterError::V2(v2_error) if v2_error.is_constraint()) =>
                    {
                        warn!(
                            event = "v2_address_constraint_error",
                            run_id = %self.ctx.run_id,
                            tree = %tree_accounts.merkle_tree,
                            error = %error,
                            "Address processing hit constraint error. Dropping processor to flush cache."
                        );
                        drop(proc);
                        self.processor_pool
                            .remove_address_processor(&tree_accounts.merkle_tree);
                        self.processor_pool
                            .remove_proof_cache(&tree_accounts.merkle_tree);
                        Err(error)
                    }
                    Err(ForesterError::V2(v2_error)) if v2_error.is_hashchain_mismatch() => {
                        let warning_key = format!(
                            "v2_address_hashchain_mismatch:{}",
                            tree_accounts.merkle_tree
                        );
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(15)) {
                            warn!(
                                event = "v2_address_hashchain_mismatch",
                                run_id = %self.ctx.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %v2_error,
                                "Address processing hit hashchain mismatch. Clearing cache and retrying."
                            );
                        }
                        self.ctx.heartbeat.increment_v2_recoverable_error();
                        proc.clear_cache().await;
                        Ok(ProcessingResult::default())
                    }
                    Err(e) => {
                        let warning_key =
                            format!("v2_address_process_failed:{}", tree_accounts.merkle_tree);
                        if should_emit_rate_limited_warning(warning_key, Duration::from_secs(10)) {
                            warn!(
                                event = "v2_address_process_failed_retrying",
                                run_id = %self.ctx.run_id,
                                tree = %tree_accounts.merkle_tree,
                                error = %e,
                                "Failed to process address queue. Will retry next tick without dropping processor."
                            );
                        }
                        self.ctx.heartbeat.increment_v2_recoverable_error();
                        Ok(ProcessingResult::default())
                    }
                }
            }
            _ => {
                warn!(
                    event = "v2_unsupported_tree_type",
                    run_id = %self.ctx.run_id,
                    tree_type = ?tree_accounts.tree_type,
                    "Unsupported tree type for V2 processing"
                );
                Ok(ProcessingResult::default())
            }
        }
    }

    pub(crate) async fn prewarm_all_trees_during_wait(
        &self,
        epoch_info: &ForesterEpochInfo,
        deadline_slot: u64,
    ) {
        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        let slots_until_active = deadline_slot.saturating_sub(current_slot);

        let trees = self.trees.lock().await;
        let total_v2_state = trees
            .iter()
            .filter(|t| matches!(t.tree_type, TreeType::StateV2))
            .count();
        let v2_state_trees: Vec<_> = trees
            .iter()
            .filter(|t| {
                matches!(t.tree_type, TreeType::StateV2)
                    && !should_skip_tree(&self.ctx.config, &t.tree_type)
            })
            .cloned()
            .collect();
        let skipped_count = total_v2_state - v2_state_trees.len();
        drop(trees);

        if v2_state_trees.is_empty() {
            if skipped_count > 0 {
                info!(
                    event = "prewarm_skipped_all_trees_filtered",
                    run_id = %self.ctx.run_id,
                    skipped_trees = skipped_count,
                    "No trees to pre-warm; all StateV2 trees skipped by config"
                );
            }
            return;
        }

        if slots_until_active < 15 {
            info!(
                event = "prewarm_skipped_not_enough_time",
                run_id = %self.ctx.run_id,
                slots_until_active,
                min_required_slots = 15,
                "Skipping pre-warming; not enough slots until active phase"
            );
            return;
        }

        let prewarm_futures: Vec<_> = v2_state_trees
            .iter()
            .map(|tree_accounts| {
                let tree_pubkey = tree_accounts.merkle_tree;
                let epoch_info = epoch_info.clone();
                let tree_accounts = *tree_accounts;
                let self_ref = self.clone();

                async move {
                    let cache = self_ref
                        .processor_pool
                        .get_or_create_proof_cache(tree_pubkey);

                    let cache_len = cache.len().await;
                    if cache_len > 0 && !cache.is_warming().await {
                        let mut rpc = match self_ref.ctx.rpc_pool.get_connection().await {
                            Ok(r) => r,
                            Err(e) => {
                                warn!(
                                    event = "prewarm_cache_validation_rpc_failed",
                                    run_id = %self_ref.ctx.run_id,
                                    tree = %tree_pubkey,
                                    error = ?e,
                                    "Failed to get RPC for cache validation"
                                );
                                return;
                            }
                        };
                        if let Ok(current_root) =
                            self_ref.fetch_current_root(&mut *rpc, &tree_accounts).await
                        {
                            info!(
                                event = "prewarm_skipped_cache_already_warm",
                                run_id = %self_ref.ctx.run_id,
                                tree = %tree_pubkey,
                                cached_proofs = cache_len,
                                root_prefix = ?&current_root[..4],
                                "Tree already has cached proofs from previous epoch; skipping pre-warm"
                            );
                            return;
                        }
                    }

                    let processor = match self_ref
                        .processor_pool
                        .get_or_create_state_processor(
                            &self_ref.ctx,
                            &epoch_info.epoch,
                            &tree_accounts,
                            self_ref.ops_cache.clone(),
                        )
                        .await
                    {
                        Ok(p) => p,
                        Err(e) => {
                            warn!(
                                event = "prewarm_processor_create_failed",
                                run_id = %self_ref.ctx.run_id,
                                tree = %tree_pubkey,
                                error = ?e,
                                "Failed to create processor for pre-warming tree"
                            );
                            return;
                        }
                    };

                    const PREWARM_MAX_BATCHES: usize = 4;
                    let mut p = processor.lock().await;
                    match p
                        .prewarm_from_indexer(
                            cache.clone(),
                            light_compressed_account::QueueType::OutputStateV2,
                            PREWARM_MAX_BATCHES,
                        )
                        .await
                    {
                        Ok(result) => {
                            if result.items_processed > 0 {
                                info!(
                                    event = "prewarm_tree_completed",
                                    run_id = %self_ref.ctx.run_id,
                                    tree = %tree_pubkey,
                                    items = result.items_processed,
                                    "Pre-warmed items for tree during wait"
                                );
                                self_ref
                                    .epoch_tracker
                                    .add_processing_metrics(epoch_info.epoch.epoch, result.metrics)
                                    .await;
                            }
                        }
                        Err(e) => {
                            debug!(
                                "Pre-warming from indexer failed for tree {}: {:?}",
                                tree_pubkey, e
                            );
                            cache.clear().await;
                        }
                    }
                }
            })
            .collect();

        let timeout_slots = slots_until_active.saturating_sub(5);
        let timeout_duration =
            (slot_duration() * timeout_slots as u32).min(Duration::from_secs(30));

        info!(
            event = "prewarm_started",
            run_id = %self.ctx.run_id,
            trees = v2_state_trees.len(),
            skipped_trees = skipped_count,
            timeout_ms = timeout_duration.as_millis() as u64,
            "Starting pre-warming"
        );

        match tokio::time::timeout(timeout_duration, futures::future::join_all(prewarm_futures))
            .await
        {
            Ok(_) => {
                info!(
                    event = "prewarm_completed",
                    run_id = %self.ctx.run_id,
                    trees = v2_state_trees.len(),
                    "Completed pre-warming for all trees"
                );
            }
            Err(_) => {
                info!(
                    event = "prewarm_timed_out",
                    run_id = %self.ctx.run_id,
                    timeout_ms = timeout_duration.as_millis() as u64,
                    "Pre-warming timed out"
                );
            }
        }
    }

    pub(crate) async fn try_send_cached_proofs(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        consecutive_eligibility_end: u64,
    ) -> crate::Result<Option<usize>> {
        let tree_pubkey = tree_accounts.merkle_tree;

        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                event = "cached_proofs_skipped_outside_eligibility",
                run_id = %self.ctx.run_id,
                tree = %tree_pubkey,
                current_slot,
                eligibility_end_slot = consecutive_eligibility_end,
                "Skipping cached proof send because eligibility window has ended"
            );
            return Ok(None);
        }

        let Some(confirmation_deadline) = scheduled_confirmation_deadline(
            consecutive_eligibility_end.saturating_sub(current_slot),
        ) else {
            debug!(
                event = "cached_proofs_skipped_confirmation_budget_exhausted",
                run_id = %self.ctx.run_id,
                tree = %tree_pubkey,
                current_slot,
                eligibility_end_slot = consecutive_eligibility_end,
                "Skipping cached proofs because not enough eligible slots remain for confirmation"
            );
            return Ok(None);
        };

        let cache = match self.processor_pool.get_proof_cache(&tree_pubkey) {
            Some(c) => c,
            None => return Ok(None),
        };

        if cache.is_warming().await {
            debug!(
                event = "cached_proofs_skipped_cache_warming",
                run_id = %self.ctx.run_id,
                tree = %tree_pubkey,
                "Skipping cached proofs because cache is still warming"
            );
            return Ok(None);
        }

        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        let current_root = match self.fetch_current_root(&mut *rpc, tree_accounts).await {
            Ok(root) => root,
            Err(e) => {
                warn!(
                    event = "cached_proofs_root_fetch_failed",
                    run_id = %self.ctx.run_id,
                    tree = %tree_pubkey,
                    error = ?e,
                    "Failed to fetch current root for tree"
                );
                return Ok(None);
            }
        };

        let cached_proofs = match cache.take_if_valid(&current_root).await {
            Some(proofs) => proofs,
            None => {
                debug!(
                    event = "cached_proofs_not_available",
                    run_id = %self.ctx.run_id,
                    tree = %tree_pubkey,
                    root_prefix = ?&current_root[..4],
                    "No valid cached proofs for tree"
                );
                return Ok(None);
            }
        };

        if cached_proofs.is_empty() {
            return Ok(Some(0));
        }

        info!(
            event = "cached_proofs_send_started",
            run_id = %self.ctx.run_id,
            tree = %tree_pubkey,
            proofs = cached_proofs.len(),
            root_prefix = ?&current_root[..4],
            "Sending cached proofs for tree"
        );

        let items_sent = self
            .send_cached_proofs_as_transactions(
                epoch_info,
                tree_accounts,
                cached_proofs,
                confirmation_deadline,
            )
            .await?;

        Ok(Some(items_sent))
    }

    async fn fetch_current_root(
        &self,
        rpc: &mut impl Rpc,
        tree_accounts: &TreeAccounts,
    ) -> crate::Result<[u8; 32]> {
        use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;

        let mut account = rpc
            .get_account(tree_accounts.merkle_tree)
            .await?
            .ok_or_else(|| anyhow!("Tree account not found: {}", tree_accounts.merkle_tree))?;

        let tree = match tree_accounts.tree_type {
            TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
                &mut account.data,
                &tree_accounts.merkle_tree.into(),
            )?,
            TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
                &mut account.data,
                &tree_accounts.merkle_tree.into(),
            )?,
            _ => return Err(anyhow!("Unsupported tree type for root fetch")),
        };

        let root = tree.root_history.last().copied().unwrap_or([0u8; 32]);
        Ok(root)
    }

    async fn send_cached_proofs_as_transactions(
        &self,
        epoch_info: &Epoch,
        tree_accounts: &TreeAccounts,
        cached_proofs: Vec<crate::processor::v2::CachedProof>,
        confirmation_deadline: Instant,
    ) -> crate::Result<usize> {
        let mut total_items = 0;
        let authority = self.ctx.config.payer_keypair.pubkey();
        let derivation = self.ctx.config.derivation_pubkey;

        const PROOFS_PER_TX: usize = 4;
        for chunk in cached_proofs.chunks(PROOFS_PER_TX) {
            let mut instructions = Vec::new();
            let mut chunk_items = 0;

            for proof in chunk {
                match &proof.instruction {
                    BatchInstruction::Append(data) => {
                        for d in data {
                            let serialized = d
                                .try_to_vec()
                                .with_context(|| "Failed to serialize batch append payload")?;
                            instructions.push(create_batch_append_instruction(
                                authority,
                                derivation,
                                tree_accounts.merkle_tree,
                                tree_accounts.queue,
                                epoch_info.epoch,
                                serialized,
                            ));
                        }
                    }
                    BatchInstruction::Nullify(data) => {
                        for d in data {
                            let serialized = d
                                .try_to_vec()
                                .with_context(|| "Failed to serialize batch nullify payload")?;
                            instructions.push(create_batch_nullify_instruction(
                                authority,
                                derivation,
                                tree_accounts.merkle_tree,
                                epoch_info.epoch,
                                serialized,
                            ));
                        }
                    }
                    BatchInstruction::AddressAppend(data) => {
                        for d in data {
                            let serialized = d.try_to_vec().with_context(|| {
                                "Failed to serialize batch address append payload"
                            })?;
                            instructions.push(create_batch_update_address_tree_instruction(
                                authority,
                                derivation,
                                tree_accounts.merkle_tree,
                                epoch_info.epoch,
                                serialized,
                            ));
                        }
                    }
                }
                chunk_items += proof.items;
            }

            if !instructions.is_empty() {
                let mut rpc = self.ctx.rpc_pool.get_connection().await?;
                let priority_fee = self
                    .ctx
                    .resolve_tree_priority_fee(&*rpc, epoch_info.epoch, tree_accounts)
                    .await?;
                let instruction_count = instructions.len();
                let payer = self.ctx.config.payer_keypair.pubkey();
                let signers = [&self.ctx.config.payer_keypair];
                match send_smart_transaction(
                    &mut *rpc,
                    SendSmartTransactionConfig {
                        instructions,
                        payer: &payer,
                        signers: &signers,
                        address_lookup_tables: &self.ctx.address_lookup_tables,
                        compute_budget: ComputeBudgetConfig {
                            compute_unit_price: priority_fee,
                            compute_unit_limit: Some(self.ctx.config.transaction_config.cu_limit),
                        },
                        confirmation: Some(self.ctx.confirmation_config()),
                        confirmation_deadline: Some(confirmation_deadline),
                    },
                )
                .await
                .map_err(light_client::rpc::RpcError::from)
                {
                    Ok(sig) => {
                        info!(
                            event = "cached_proofs_tx_sent",
                            run_id = %self.ctx.run_id,
                            signature = %sig,
                            instruction_count,
                            "Sent cached proofs transaction"
                        );
                        total_items += chunk_items;
                    }
                    Err(e) => {
                        warn!(
                            event = "cached_proofs_tx_send_failed",
                            run_id = %self.ctx.run_id,
                            error = ?e,
                            "Failed to send cached proofs transaction"
                        );
                    }
                }
            }
        }

        Ok(total_items)
    }
}
