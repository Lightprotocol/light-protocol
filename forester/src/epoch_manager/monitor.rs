//! Background monitoring tasks: epoch detection, tree discovery, balance checks.

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use forester_utils::forester_epoch::{get_epoch_phases, TreeAccounts, TreeForesterSchedule};
use light_client::{indexer::Indexer, rpc::Rpc};
use solana_program::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::signature::Signer;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::{context::should_skip_tree, EpochManager};
use crate::{
    metrics::update_forester_sol_balance,
    slot_tracker::wait_until_slot_reached,
    tree_data_sync::{fetch_protocol_group_authority, fetch_trees},
};

impl<R: Rpc + Indexer> EpochManager<R> {
    pub(super) async fn check_sol_balance_periodically(self: Arc<Self>) -> crate::Result<()> {
        let interval_duration = Duration::from_secs(300);
        let mut interval = tokio::time::interval(interval_duration);

        loop {
            interval.tick().await;
            match self.ctx.rpc_pool.get_connection().await {
                Ok(rpc) => match rpc
                    .get_balance(&self.ctx.config.payer_keypair.pubkey())
                    .await
                {
                    Ok(balance) => {
                        let balance_in_sol = balance as f64 / (LAMPORTS_PER_SOL as f64);
                        update_forester_sol_balance(
                            &self.ctx.config.payer_keypair.pubkey().to_string(),
                            balance_in_sol,
                        );
                        debug!(
                            event = "forester_balance_updated",
                            run_id = %self.ctx.run_id,
                            balance_sol = balance_in_sol,
                            "Current SOL balance updated"
                        );
                    }
                    Err(e) => error!(
                        event = "forester_balance_fetch_failed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Failed to get balance"
                    ),
                },
                Err(e) => error!(
                    event = "forester_balance_rpc_connection_failed",
                    run_id = %self.ctx.run_id,
                    error = ?e,
                    "Failed to get RPC connection for balance check"
                ),
            }
        }
    }

    pub(super) async fn discover_trees_periodically(self: Arc<Self>) -> crate::Result<()> {
        let interval_secs = self
            .ctx
            .config
            .general_config
            .tree_discovery_interval_seconds;
        if interval_secs == 0 {
            info!(event = "tree_discovery_disabled", run_id = %self.ctx.run_id, "Tree discovery disabled (interval=0)");
            return Ok(());
        }
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.tick().await;

        info!(
            event = "tree_discovery_started",
            run_id = %self.ctx.run_id,
            interval_secs,
            "Starting periodic tree discovery"
        );

        let mut group_authority: Option<Pubkey> = self.ctx.config.general_config.group_authority;

        loop {
            interval.tick().await;

            let rpc = match self.ctx.rpc_pool.get_connection().await {
                Ok(rpc) => rpc,
                Err(e) => {
                    warn!(event = "tree_discovery_rpc_failed", run_id = %self.ctx.run_id, error = ?e, "Tree discovery: failed to get RPC connection");
                    continue;
                }
            };

            if group_authority.is_none() {
                if let Ok(ga) = fetch_protocol_group_authority(&*rpc, &self.ctx.run_id).await {
                    group_authority = Some(ga);
                    let mut trees = self.trees.lock().await;
                    let before = trees.len();
                    trees.retain(|t| t.owner == ga);
                    if !self.ctx.config.general_config.tree_ids.is_empty() {
                        let tree_ids = &self.ctx.config.general_config.tree_ids;
                        trees.retain(|t| tree_ids.contains(&t.merkle_tree));
                    }
                    if trees.len() < before {
                        info!(
                            event = "tree_discovery_retroactive_filter",
                            run_id = %self.ctx.run_id,
                            group_authority = %ga,
                            trees_before = before,
                            trees_after = trees.len(),
                            "Filtered existing trees after resolving group authority"
                        );
                    }
                }
            }

            let mut fetched_trees = match fetch_trees(&*rpc).await {
                Ok(trees) => trees,
                Err(e) => {
                    warn!(event = "tree_discovery_fetch_failed", run_id = %self.ctx.run_id, error = ?e, "Tree discovery: failed to fetch trees");
                    continue;
                }
            };

            if let Some(ga) = group_authority {
                fetched_trees.retain(|tree| tree.owner == ga);
            }
            if !self.ctx.config.general_config.tree_ids.is_empty() {
                let tree_ids = &self.ctx.config.general_config.tree_ids;
                fetched_trees.retain(|tree| tree_ids.contains(&tree.merkle_tree));
            }

            let known_trees = self.trees.lock().await;
            let known_pubkeys: std::collections::HashSet<Pubkey> =
                known_trees.iter().map(|t| t.merkle_tree).collect();
            drop(known_trees);

            for tree in fetched_trees {
                if known_pubkeys.contains(&tree.merkle_tree) {
                    continue;
                }
                if should_skip_tree(&self.ctx.config, &tree.tree_type) {
                    debug!(
                        event = "tree_discovery_skipped",
                        run_id = %self.ctx.run_id,
                        tree = %tree.merkle_tree,
                        tree_type = ?tree.tree_type,
                        "Skipping tree due to fee filter config"
                    );
                    continue;
                }
                info!(
                    event = "tree_discovery_new_tree",
                    run_id = %self.ctx.run_id,
                    tree = %tree.merkle_tree,
                    tree_type = ?tree.tree_type,
                    queue = %tree.queue,
                    "Discovered new tree"
                );
                if let Err(e) = self.add_new_tree(tree).await {
                    error!(
                        event = "tree_discovery_add_failed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Failed to add discovered tree"
                    );
                }
            }
        }
    }

    async fn add_new_tree(self: &Arc<Self>, new_tree: TreeAccounts) -> crate::Result<()> {
        info!(
            event = "new_tree_add_started",
            run_id = %self.ctx.run_id,
            tree = %new_tree.merkle_tree,
            tree_type = ?new_tree.tree_type,
            "Adding new tree"
        );
        let mut trees = self.trees.lock().await;
        trees.push(new_tree);
        drop(trees);

        info!(
            event = "new_tree_added",
            run_id = %self.ctx.run_id,
            tree = %new_tree.merkle_tree,
            "New tree added to tracked list"
        );

        let (current_slot, current_epoch) = self.ctx.get_current_slot_and_epoch().await?;
        let phases = get_epoch_phases(&self.ctx.protocol_config, current_epoch);

        if current_slot >= phases.active.start && current_slot < phases.active.end {
            info!(
                event = "new_tree_active_phase_injection",
                run_id = %self.ctx.run_id,
                tree = %new_tree.merkle_tree,
                current_slot,
                active_phase_start_slot = phases.active.start,
                active_phase_end_slot = phases.active.end,
                "In active phase; attempting immediate processing for new tree"
            );
            info!(
                event = "new_tree_recover_registration_started",
                run_id = %self.ctx.run_id,
                tree = %new_tree.merkle_tree,
                epoch = current_epoch,
                "Recovering registration info for new tree"
            );
            match self
                .recover_registration_info_if_exists(current_epoch)
                .await
            {
                Ok(Some(mut epoch_info)) => {
                    info!(
                        event = "new_tree_recover_registration_succeeded",
                        run_id = %self.ctx.run_id,
                        tree = %new_tree.merkle_tree,
                        epoch = current_epoch,
                        "Recovered registration info for current epoch"
                    );
                    let tree_schedule = TreeForesterSchedule::new_with_schedule(
                        &new_tree,
                        current_slot,
                        &epoch_info.forester_epoch_pda,
                        &epoch_info.epoch_pda,
                    )?;
                    epoch_info.trees.push(tree_schedule.clone());

                    let self_clone = self.clone();
                    let tracker = self.epoch_tracker.get_or_create_tracker(
                        current_epoch,
                        epoch_info.epoch_pda.registered_weight,
                    );

                    info!(
                        event = "new_tree_processing_task_spawned",
                        run_id = %self.ctx.run_id,
                        tree = %new_tree.merkle_tree,
                        epoch = current_epoch,
                        "Spawning task to process new tree in current epoch"
                    );
                    tokio::spawn(async move {
                        let tree_pubkey = tree_schedule.tree_accounts.merkle_tree;
                        if let Err(e) = self_clone
                            .process_queue(
                                &epoch_info.epoch,
                                epoch_info.forester_epoch_pda.clone(),
                                tree_schedule,
                                tracker,
                            )
                            .await
                        {
                            error!(
                                event = "new_tree_process_queue_failed",
                                run_id = %self_clone.ctx.run_id,
                                tree = %tree_pubkey,
                                error = ?e,
                                "Error processing queue for new tree"
                            );
                        } else {
                            info!(
                                event = "new_tree_process_queue_succeeded",
                                run_id = %self_clone.ctx.run_id,
                                tree = %tree_pubkey,
                                "Successfully processed new tree in current epoch"
                            );
                        }
                    });
                }
                Ok(None) => {
                    debug!(
                        "Not registered for current epoch yet, new tree will be picked up during next registration"
                    );
                }
                Err(e) => {
                    warn!(
                        event = "new_tree_recover_registration_failed",
                        run_id = %self.ctx.run_id,
                        tree = %new_tree.merkle_tree,
                        epoch = current_epoch,
                        error = ?e,
                        "Failed to recover registration info for new tree"
                    );
                }
            }

            info!(
                event = "new_tree_injected_into_current_epoch",
                run_id = %self.ctx.run_id,
                tree = %new_tree.merkle_tree,
                epoch = current_epoch,
                "Injected new tree into current epoch"
            );
        } else {
            info!(
                event = "new_tree_queued_for_next_registration",
                run_id = %self.ctx.run_id,
                tree = %new_tree.merkle_tree,
                current_slot,
                active_phase_start_slot = phases.active.start,
                "Not in active phase; new tree will be picked up in next registration"
            );
        }

        Ok(())
    }

    pub(super) async fn monitor_epochs(&self, tx: Arc<mpsc::Sender<u64>>) -> crate::Result<()> {
        let mut last_epoch: Option<u64> = None;
        let mut consecutive_failures = 0u32;
        const MAX_BACKOFF_SECS: u64 = 60;

        info!(
            event = "epoch_monitor_started",
            run_id = %self.ctx.run_id,
            "Starting epoch monitor"
        );

        loop {
            let (slot, current_epoch) = match self.ctx.get_current_slot_and_epoch().await {
                Ok(result) => {
                    if consecutive_failures > 0 {
                        info!(
                            event = "epoch_monitor_recovered",
                            run_id = %self.ctx.run_id,
                            consecutive_failures, "Epoch monitor recovered after failures"
                        );
                    }
                    consecutive_failures = 0;
                    result
                }
                Err(e) => {
                    consecutive_failures += 1;
                    let backoff_secs = 2u64.pow(consecutive_failures.min(6)).min(MAX_BACKOFF_SECS);
                    let backoff = Duration::from_secs(backoff_secs);

                    if consecutive_failures == 1 {
                        warn!(
                            event = "epoch_monitor_slot_epoch_failed",
                            run_id = %self.ctx.run_id,
                            consecutive_failures,
                            error = ?e,
                            backoff_ms = backoff.as_millis() as u64,
                            "Epoch monitor failed to get slot/epoch; retrying"
                        );
                    } else if consecutive_failures.is_multiple_of(10) {
                        error!(
                            event = "epoch_monitor_slot_epoch_failed_repeated",
                            run_id = %self.ctx.run_id,
                            consecutive_failures,
                            error = ?e,
                            backoff_ms = backoff.as_millis() as u64,
                            "Epoch monitor still failing repeatedly"
                        );
                    }

                    tokio::time::sleep(backoff).await;
                    continue;
                }
            };

            debug!(
                event = "epoch_monitor_tick",
                run_id = %self.ctx.run_id,
                last_epoch = ?last_epoch,
                current_epoch,
                slot,
                "Epoch monitor tick"
            );

            if last_epoch.is_none_or(|last| current_epoch > last) {
                debug!(
                    event = "epoch_monitor_new_epoch_detected",
                    run_id = %self.ctx.run_id,
                    epoch = current_epoch,
                    "New epoch detected; sending for processing"
                );
                if let Err(e) = tx.send(current_epoch).await {
                    error!(
                        event = "epoch_monitor_send_current_epoch_failed",
                        run_id = %self.ctx.run_id,
                        epoch = current_epoch,
                        error = ?e,
                        "Failed to send current epoch for processing; channel closed"
                    );
                    return Err(anyhow!("Epoch channel closed: {}", e));
                }
                last_epoch = Some(current_epoch);
            }

            let target_epoch = current_epoch + 1;
            if last_epoch.is_none_or(|last| target_epoch > last) {
                let target_phases = get_epoch_phases(&self.ctx.protocol_config, target_epoch);

                if slot < target_phases.registration.start {
                    let mut rpc = match self.ctx.rpc_pool.get_connection().await {
                        Ok(rpc) => rpc,
                        Err(e) => {
                            warn!(
                                event = "epoch_monitor_wait_rpc_connection_failed",
                                run_id = %self.ctx.run_id,
                                target_epoch,
                                error = ?e,
                                "Failed to get RPC connection while waiting for registration slot"
                            );
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    };

                    const REGISTRATION_BUFFER_SLOTS: u64 = 30;
                    let wait_target = target_phases
                        .registration
                        .start
                        .saturating_sub(REGISTRATION_BUFFER_SLOTS);
                    let slots_to_wait = wait_target.saturating_sub(slot);

                    debug!(
                        event = "epoch_monitor_wait_for_registration",
                        run_id = %self.ctx.run_id,
                        target_epoch,
                        current_slot = slot,
                        wait_target_slot = wait_target,
                        registration_start_slot = target_phases.registration.start,
                        slots_to_wait,
                        "Waiting for target epoch registration phase"
                    );

                    if let Err(e) =
                        wait_until_slot_reached(&mut *rpc, &self.ctx.slot_tracker, wait_target)
                            .await
                    {
                        error!(
                            event = "epoch_monitor_wait_for_registration_failed",
                            run_id = %self.ctx.run_id,
                            target_epoch,
                            error = ?e,
                            "Error waiting for registration phase"
                        );
                        continue;
                    }
                }

                debug!(
                    event = "epoch_monitor_send_target_epoch",
                    run_id = %self.ctx.run_id,
                    target_epoch,
                    "Sending target epoch for processing"
                );
                if let Err(e) = tx.send(target_epoch).await {
                    error!(
                        event = "epoch_monitor_send_target_epoch_failed",
                        run_id = %self.ctx.run_id,
                        target_epoch,
                        error = ?e,
                        "Failed to send target epoch for processing; channel closed"
                    );
                    return Err(anyhow!("Epoch channel closed: {}", e));
                }
                last_epoch = Some(target_epoch);
                continue;
            } else {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }

    pub(super) async fn process_current_and_previous_epochs(
        &self,
        tx: Arc<mpsc::Sender<u64>>,
    ) -> crate::Result<()> {
        let (slot, current_epoch) = self.ctx.get_current_slot_and_epoch().await?;
        let current_phases = get_epoch_phases(&self.ctx.protocol_config, current_epoch);
        let previous_epoch = current_epoch.saturating_sub(1);

        if slot > current_phases.registration.start {
            debug!("Processing previous epoch: {}", previous_epoch);
            if let Err(e) = tx.send(previous_epoch).await {
                error!(
                    event = "initial_epoch_send_previous_failed",
                    run_id = %self.ctx.run_id,
                    epoch = previous_epoch,
                    error = ?e,
                    "Failed to send previous epoch for processing"
                );
                return Ok(());
            }
        }

        debug!("Processing current epoch: {}", current_epoch);
        if let Err(e) = tx.send(current_epoch).await {
            error!(
                event = "initial_epoch_send_current_failed",
                run_id = %self.ctx.run_id,
                epoch = current_epoch,
                error = ?e,
                "Failed to send current epoch for processing"
            );
            return Ok(());
        }

        debug!("Finished processing current and previous epochs");
        Ok(())
    }
}
