//! V1 tree processing and rollover.

use std::{sync::Arc, time::Duration};

use forester_utils::forester_epoch::{Epoch, ForesterSlot, TreeAccounts};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::TreeType;
use light_registry::ForesterEpochPda;
use solana_sdk::signature::Keypair;
use tracing::{debug, error, info, warn};

use super::EpochManager;
use crate::{
    errors::ForesterError,
    processor::v1::{
        config::{BuildTransactionBatchConfig, SendBatchedTransactionsConfig},
        send_transaction::send_batched_transactions,
        tx_builder::EpochManagerTransactions,
    },
    rollover::{
        is_tree_ready_for_rollover, perform_address_merkle_tree_rollover,
        perform_state_merkle_tree_rollover_forester,
    },
    transaction_timing::scheduled_v1_batch_timeout,
};

impl<R: Rpc + Indexer> EpochManager<R> {
    pub(crate) async fn process_v1(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        current_solana_slot: u64,
    ) -> std::result::Result<usize, ForesterError> {
        let slots_remaining = forester_slot_details
            .end_solana_slot
            .saturating_sub(current_solana_slot);
        let Some(remaining_time_timeout) = scheduled_v1_batch_timeout(slots_remaining) else {
            debug!(
                event = "v1_tree_skipped_low_slot_budget",
                run_id = %self.ctx.run_id,
                tree = %tree_accounts.merkle_tree,
                slots_remaining,
                "Skipping V1 tree: not enough scheduled slot budget left to confirm a transaction"
            );
            return Ok(0);
        };

        let use_lookup_tables = !self.ctx.address_lookup_tables.is_empty();
        let enable_v1_multi_nullify = self.ctx.config.enable_v1_multi_nullify && use_lookup_tables;

        let batched_tx_config = SendBatchedTransactionsConfig {
            num_batches: 1,
            build_transaction_batch_config: BuildTransactionBatchConfig {
                batch_size: self.ctx.config.transaction_config.legacy_ixs_per_tx as u64,
                compute_unit_price: self
                    .ctx
                    .config
                    .transaction_config
                    .priority_fee_microlamports,
                compute_unit_limit: Some(self.ctx.config.transaction_config.cu_limit),
                enable_priority_fees: self.ctx.config.transaction_config.enable_priority_fees,
                max_concurrent_sends: Some(self.ctx.config.transaction_config.max_concurrent_sends),
            },
            queue_config: self.ctx.config.queue_config,
            retry_config: light_client::rpc::RetryConfig {
                timeout: remaining_time_timeout,
                ..self.ctx.config.retry_config
            },
            light_slot_length: epoch_pda.protocol_config.slot_length,
            confirmation_poll_interval: Duration::from_millis(
                self.ctx
                    .config
                    .transaction_config
                    .confirmation_poll_interval_ms,
            ),
            confirmation_max_attempts: self.ctx.config.transaction_config.confirmation_max_attempts
                as usize,
            min_queue_items: if use_lookup_tables {
                self.ctx.config.min_queue_items
            } else {
                None
            },
            enable_presort: enable_v1_multi_nullify,
            work_item_batch_size: self.ctx.config.work_item_batch_size,
        };

        let transaction_builder = Arc::new(EpochManagerTransactions::new(
            self.ctx.rpc_pool.clone(),
            epoch_info.epoch,
            self.tx_cache.clone(),
            self.ctx.address_lookup_tables.as_ref().clone(),
            enable_v1_multi_nullify,
        ));

        let num_sent = send_batched_transactions(
            &self.ctx.config.payer_keypair,
            &self.ctx.config.derivation_pubkey,
            self.ctx.rpc_pool.clone(),
            &batched_tx_config,
            *tree_accounts,
            transaction_builder,
        )
        .await?;

        if num_sent > 0 {
            debug!(
                event = "v1_tree_items_processed",
                run_id = %self.ctx.run_id,
                tree = %tree_accounts.merkle_tree,
                items = num_sent,
                "Processed items for V1 tree"
            );
        }

        match self.rollover_if_needed(tree_accounts).await {
            Ok(_) => Ok(num_sent),
            Err(e) => {
                error!(
                    event = "tree_rollover_failed",
                    run_id = %self.ctx.run_id,
                    tree = %tree_accounts.merkle_tree,
                    tree_type = ?tree_accounts.tree_type,
                    error = ?e,
                    "Failed to rollover tree"
                );
                Err(e.into())
            }
        }
    }

    async fn rollover_if_needed(&self, tree_account: &TreeAccounts) -> crate::Result<()> {
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        if is_tree_ready_for_rollover(&mut *rpc, tree_account.merkle_tree, tree_account.tree_type)
            .await?
        {
            info!(
                event = "tree_rollover_started",
                run_id = %self.ctx.run_id,
                tree = %tree_account.merkle_tree,
                tree_type = ?tree_account.tree_type,
                "Starting tree rollover"
            );
            self.perform_rollover(tree_account).await?;
        }
        Ok(())
    }

    async fn perform_rollover(&self, tree_account: &TreeAccounts) -> crate::Result<()> {
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        let (_, current_epoch) = self.ctx.get_current_slot_and_epoch().await?;

        let result = match tree_account.tree_type {
            TreeType::AddressV1 => {
                let new_nullifier_queue_keypair = Keypair::new();
                let new_merkle_tree_keypair = Keypair::new();

                let rollover_signature = perform_address_merkle_tree_rollover(
                    &self.ctx.config.payer_keypair,
                    &self.ctx.config.derivation_pubkey,
                    &mut *rpc,
                    &new_nullifier_queue_keypair,
                    &new_merkle_tree_keypair,
                    &tree_account.merkle_tree,
                    &tree_account.queue,
                    current_epoch,
                )
                .await?;

                info!(
                    event = "address_tree_rollover_succeeded",
                    run_id = %self.ctx.run_id,
                    tree = %tree_account.merkle_tree,
                    signature = %rollover_signature,
                    "Address tree rollover succeeded"
                );
                Ok(())
            }
            TreeType::StateV1 => {
                let new_nullifier_queue_keypair = Keypair::new();
                let new_merkle_tree_keypair = Keypair::new();
                let new_cpi_signature_keypair = Keypair::new();

                let rollover_signature = perform_state_merkle_tree_rollover_forester(
                    &self.ctx.config.payer_keypair,
                    &self.ctx.config.derivation_pubkey,
                    &mut *rpc,
                    &new_nullifier_queue_keypair,
                    &new_merkle_tree_keypair,
                    &new_cpi_signature_keypair,
                    &tree_account.merkle_tree,
                    &tree_account.queue,
                    &solana_program::pubkey::Pubkey::default(),
                    current_epoch,
                )
                .await?;

                info!(
                    event = "state_tree_rollover_succeeded",
                    run_id = %self.ctx.run_id,
                    tree = %tree_account.merkle_tree,
                    signature = %rollover_signature,
                    "State tree rollover succeeded"
                );

                Ok(())
            }
            _ => Err(ForesterError::InvalidTreeType(tree_account.tree_type)),
        };

        match result {
            Ok(_) => debug!(
                "{:?} tree rollover completed successfully",
                tree_account.tree_type
            ),
            Err(e) => warn!("{:?} tree rollover failed: {:?}", tree_account.tree_type, e),
        }
        Ok(())
    }
}
