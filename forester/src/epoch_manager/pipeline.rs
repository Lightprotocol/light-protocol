//! Epoch processing pipeline: process_epoch → perform_active_work → process_queue → slot processing.

use std::{sync::Arc, time::Duration};

use forester_utils::forester_epoch::{
    get_epoch_phases, Epoch, ForesterSlot, TreeAccounts, TreeForesterSchedule,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::TreeType;
use light_registry::{protocol_config::state::EpochState, ForesterEpochPda};
use solana_sdk::signature::Signer;
use tokio::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

use super::{context::should_skip_tree, tracker::RegistrationTracker, EpochManager};
use crate::{
    errors::ForesterError,
    logging::should_emit_rate_limited_warning,
    metrics::{push_metrics, queue_metric_update, update_epoch_detected, update_epoch_registered},
    slot_tracker::wait_until_slot_reached,
    ForesterEpochInfo,
};

impl<R: Rpc + Indexer> EpochManager<R> {
    #[instrument(level = "debug", skip(self), fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch))]
    pub(super) async fn process_epoch(self: Arc<Self>, epoch: u64) -> crate::Result<()> {
        let _epoch_guard = match self.epoch_tracker.try_claim_epoch(epoch) {
            Some(guard) => guard,
            None => {
                debug!("Epoch {} is already being processed, skipping", epoch);
                return Ok(());
            }
        };

        let phases = get_epoch_phases(&self.ctx.protocol_config, epoch);
        update_epoch_detected(epoch);

        debug!("Recovering registration info for epoch {}", epoch);
        let mut registration_info = match self.recover_registration_info_if_exists(epoch).await {
            Ok(Some(info)) => info,
            Ok(None) => {
                debug!(
                    "No existing registration found for epoch {}, will register fresh",
                    epoch
                );
                match self
                    .register_for_epoch_with_retry(epoch, 100, Duration::from_millis(1000))
                    .await
                {
                    Ok(info) => info,
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => {
                warn!(
                    event = "recover_registration_info_failed",
                    run_id = %self.ctx.run_id,
                    epoch,
                    error = ?e,
                    "Failed to recover registration info"
                );
                return Err(e.into());
            }
        };
        debug!("Recovered registration info for epoch {}", epoch);
        update_epoch_registered(epoch);

        registration_info = match self.wait_for_active_phase(&registration_info).await? {
            Some(info) => info,
            None => {
                let current_slot = self.ctx.slot_tracker.estimated_current_slot();
                debug!(
                    event = "epoch_processing_skipped_finalize_registration_phase_ended",
                    run_id = %self.ctx.run_id,
                    epoch,
                    current_slot,
                    active_phase_end_slot = registration_info.epoch.phases.active.end,
                    "Skipping epoch processing because FinalizeRegistration is no longer possible"
                );
                return Ok(());
            }
        };

        if self.ctx.sync_slot().await? < phases.active.end {
            self.clone().perform_active_work(&registration_info).await?;
        }
        if self.ctx.sync_slot().await? < phases.report_work.start {
            self.wait_for_report_work_phase(&registration_info).await?;
        }

        self.send_work_report(&registration_info).await?;

        if self.ctx.sync_slot().await? < phases.report_work.end {
            self.report_work_onchain(&registration_info).await?;
        } else {
            let current_slot = self.ctx.slot_tracker.estimated_current_slot();
            info!(
                event = "skip_onchain_work_report_phase_ended",
                run_id = %self.ctx.run_id,
                epoch = registration_info.epoch.epoch,
                current_slot,
                report_work_end_slot = phases.report_work.end,
                "Skipping on-chain work report because report_work phase has ended"
            );
        }

        self.epoch_tracker.cleanup(epoch).await;

        info!(
            event = "process_epoch_completed",
            run_id = %self.ctx.run_id,
            epoch, "Exiting process_epoch"
        );
        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info),
        fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch)
    )]
    pub(super) async fn perform_active_work(
        self: Arc<Self>,
        epoch_info: &ForesterEpochInfo,
    ) -> crate::Result<()> {
        self.ctx.heartbeat.increment_active_cycle();

        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        let active_phase_end = epoch_info.epoch.phases.active.end;

        if !self.is_in_active_phase(current_slot, epoch_info)? {
            info!(
                event = "active_work_skipped_not_in_phase",
                run_id = %self.ctx.run_id,
                current_slot,
                active_phase_end,
                "No longer in active phase. Skipping work."
            );
            return Ok(());
        }

        self.ctx.sync_slot().await?;

        let trees_to_process: Vec<_> = epoch_info
            .trees
            .iter()
            .filter(|tree| !should_skip_tree(&self.ctx.config, &tree.tree_accounts.tree_type))
            .cloned()
            .collect();

        if trees_to_process.is_empty() {
            debug!(
                event = "active_work_cycle_no_trees",
                run_id = %self.ctx.run_id,
                "No trees to process this cycle"
            );
            let mut rpc = self.ctx.rpc_pool.get_connection().await?;
            wait_until_slot_reached(&mut *rpc, &self.ctx.slot_tracker, active_phase_end).await?;
            return Ok(());
        }

        info!(
            event = "active_work_cycle_started",
            run_id = %self.ctx.run_id,
            current_slot,
            active_phase_end,
            tree_count = trees_to_process.len(),
            "Starting active work cycle"
        );

        let registration_tracker = self.epoch_tracker.get_or_create_tracker(
            epoch_info.epoch.epoch,
            epoch_info.epoch_pda.registered_weight,
        );

        let mut tree_tasks = tokio::task::JoinSet::new();

        for tree in trees_to_process {
            debug!(
                event = "tree_processing_task_spawned",
                run_id = %self.ctx.run_id,
                tree = %tree.tree_accounts.merkle_tree,
                tree_type = ?tree.tree_accounts.tree_type,
                "Spawning tree processing task"
            );
            self.ctx.heartbeat.add_tree_tasks_spawned(1);

            let self_clone = self.clone();
            let epoch_clone = epoch_info.epoch.clone();
            let forester_epoch_pda = epoch_info.forester_epoch_pda.clone();
            let tracker = registration_tracker.clone();
            tree_tasks.spawn(async move {
                self_clone
                    .process_queue(&epoch_clone, forester_epoch_pda, tree, tracker)
                    .await
            });
        }

        let mut success_count = 0usize;
        let mut error_count = 0usize;
        let mut panic_count = 0usize;
        while let Some(join_result) = tree_tasks.join_next().await {
            match join_result {
                Ok(Ok(())) => success_count += 1,
                Ok(Err(e)) => {
                    error_count += 1;
                    error!(
                        event = "tree_processing_task_failed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Error processing queue"
                    );
                }
                Err(join_error) => {
                    if join_error.is_panic() {
                        panic_count += 1;
                        error!(
                            event = "tree_processing_task_panicked",
                            run_id = %self.ctx.run_id,
                            error = %join_error,
                            "Tree processing task panicked"
                        );
                    }
                }
            }
        }
        info!(
            event = "active_work_cycle_completed",
            run_id = %self.ctx.run_id,
            tree_tasks = success_count + error_count + panic_count,
            succeeded = success_count,
            failed = error_count,
            panicked = panic_count,
            "Active work cycle completed"
        );

        debug!("Waiting for active phase to end");
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        wait_until_slot_reached(&mut *rpc, &self.ctx.slot_tracker, active_phase_end).await?;
        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, forester_epoch_pda, tree_schedule, registration_tracker),
        fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree_schedule.tree_accounts.merkle_tree)
    )]
    pub(crate) async fn process_queue(
        &self,
        epoch_info: &Epoch,
        mut forester_epoch_pda: ForesterEpochPda,
        mut tree_schedule: TreeForesterSchedule,
        registration_tracker: Arc<RegistrationTracker>,
    ) -> crate::Result<()> {
        self.ctx.heartbeat.increment_queue_started();
        let mut current_slot = self.ctx.slot_tracker.estimated_current_slot();

        let total_slots = tree_schedule.slots.len();
        let eligible_slots = tree_schedule.slots.iter().filter(|s| s.is_some()).count();
        let tree_type = tree_schedule.tree_accounts.tree_type;

        debug!(
            event = "process_queue_started",
            run_id = %self.ctx.run_id,
            tree = %tree_schedule.tree_accounts.merkle_tree,
            tree_type = ?tree_type,
            total_slots,
            eligible_slots,
            current_slot,
            active_phase_end = epoch_info.phases.active.end,
            "Processing queue for tree"
        );

        let mut last_weight_check = Instant::now();
        const WEIGHT_CHECK_INTERVAL: Duration = Duration::from_secs(30);

        'outer_slot_loop: while current_slot < epoch_info.phases.active.end {
            let next_slot_to_process = tree_schedule
                .slots
                .iter_mut()
                .enumerate()
                .find_map(|(idx, opt_slot)| opt_slot.as_ref().map(|s| (idx, s.clone())));

            if let Some((slot_idx, light_slot_details)) = next_slot_to_process {
                let result = match tree_type {
                    TreeType::StateV1 | TreeType::AddressV1 | TreeType::Unknown => {
                        self.process_light_slot(
                            epoch_info,
                            &forester_epoch_pda,
                            &tree_schedule.tree_accounts,
                            &light_slot_details,
                        )
                        .await
                    }
                    TreeType::StateV2 | TreeType::AddressV2 => {
                        let consecutive_end = tree_schedule
                            .get_consecutive_eligibility_end(slot_idx)
                            .unwrap_or(light_slot_details.end_solana_slot);
                        self.process_light_slot_v2(
                            epoch_info,
                            &forester_epoch_pda,
                            &tree_schedule.tree_accounts,
                            &light_slot_details,
                            consecutive_end,
                        )
                        .await
                    }
                };

                let mut force_refinalize = false;
                match result {
                    Ok(_) => {
                        trace!(
                            "Successfully processed light slot {:?}",
                            light_slot_details.slot
                        );
                    }
                    Err(e) => {
                        force_refinalize = e.is_forester_not_eligible();
                        if force_refinalize {
                            warn!(
                                event = "light_slot_processing_stale_eligibility",
                                run_id = %self.ctx.run_id,
                                tree = %tree_schedule.tree_accounts.merkle_tree,
                                light_slot = light_slot_details.slot,
                                "Detected ForesterNotEligible; forcing immediate re-finalization"
                            );
                        }
                        error!(
                            event = "light_slot_processing_error",
                            run_id = %self.ctx.run_id,
                            light_slot = light_slot_details.slot,
                            error = ?e,
                            "Error processing light slot"
                        );
                    }
                }
                tree_schedule.slots[slot_idx] = None;

                if force_refinalize || last_weight_check.elapsed() >= WEIGHT_CHECK_INTERVAL {
                    last_weight_check = Instant::now();
                    if let Err(e) = self
                        .maybe_refinalize(
                            epoch_info,
                            &mut forester_epoch_pda,
                            &mut tree_schedule,
                            &registration_tracker,
                            force_refinalize,
                        )
                        .await
                    {
                        warn!(
                            event = "refinalize_check_failed",
                            run_id = %self.ctx.run_id,
                            forced = force_refinalize,
                            error = ?e,
                            "Failed to check/perform re-finalization"
                        );
                    }
                }
            } else {
                debug!(
                    event = "process_queue_no_eligible_slots",
                    run_id = %self.ctx.run_id,
                    tree = %tree_schedule.tree_accounts.merkle_tree,
                    "No further eligible slots in schedule"
                );
                break 'outer_slot_loop;
            }

            current_slot = self.ctx.slot_tracker.estimated_current_slot();
        }

        self.ctx.heartbeat.increment_queue_finished();
        debug!(
            event = "process_queue_finished",
            run_id = %self.ctx.run_id,
            tree = %tree_schedule.tree_accounts.merkle_tree,
            "Exiting process_queue"
        );
        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree_accounts, forester_slot_details),
        fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch,
        tree = %tree_accounts.merkle_tree)
    )]
    async fn process_light_slot(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
    ) -> std::result::Result<(), ForesterError> {
        debug!(
            event = "light_slot_processing_started",
            run_id = %self.ctx.run_id,
            tree = %tree_accounts.merkle_tree,
            epoch = epoch_info.epoch,
            light_slot = forester_slot_details.slot,
            slot_start = forester_slot_details.start_solana_slot,
            slot_end = forester_slot_details.end_solana_slot,
            "Processing light slot"
        );
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        wait_until_slot_reached(
            &mut *rpc,
            &self.ctx.slot_tracker,
            forester_slot_details.start_solana_slot,
        )
        .await?;
        let mut estimated_slot = self.ctx.slot_tracker.estimated_current_slot();

        'inner_processing_loop: loop {
            if estimated_slot >= forester_slot_details.end_solana_slot {
                trace!(
                    "Ending processing for slot {:?} due to time limit.",
                    forester_slot_details.slot
                );
                break 'inner_processing_loop;
            }

            let current_light_slot = (estimated_slot - epoch_info.phases.active.start)
                / epoch_pda.protocol_config.slot_length;
            if current_light_slot != forester_slot_details.slot {
                warn!(
                    event = "light_slot_mismatch",
                    run_id = %self.ctx.run_id,
                    tree = %tree_accounts.merkle_tree,
                    expected_light_slot = forester_slot_details.slot,
                    actual_light_slot = current_light_slot,
                    estimated_slot,
                    "Light slot mismatch; exiting processing for this slot"
                );
                break 'inner_processing_loop;
            }

            if !self
                .check_forester_eligibility(
                    epoch_pda,
                    current_light_slot,
                    &tree_accounts.queue,
                    epoch_info.epoch,
                    epoch_info,
                )
                .await?
            {
                break 'inner_processing_loop;
            }

            let processing_start_time = Instant::now();
            let items_processed_this_iteration = match self
                .dispatch_tree_processing(
                    epoch_info,
                    epoch_pda,
                    tree_accounts,
                    forester_slot_details,
                    forester_slot_details.end_solana_slot,
                    estimated_slot,
                )
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    if e.is_forester_not_eligible() {
                        return Err(e);
                    }
                    error!(
                        event = "light_slot_processing_failed",
                        run_id = %self.ctx.run_id,
                        tree = %tree_accounts.merkle_tree,
                        light_slot = forester_slot_details.slot,
                        error = ?e,
                        "Failed processing in light slot"
                    );
                    break 'inner_processing_loop;
                }
            };
            if items_processed_this_iteration > 0 {
                debug!(
                    event = "light_slot_items_processed",
                    run_id = %self.ctx.run_id,
                    light_slot = forester_slot_details.slot,
                    items = items_processed_this_iteration,
                    "Processed items in light slot"
                );
            }

            self.update_metrics_and_counts(
                epoch_info.epoch,
                items_processed_this_iteration,
                processing_start_time.elapsed(),
            )
            .await;

            if let Err(e) = push_metrics(&self.ctx.config.external_services.pushgateway_url).await {
                if should_emit_rate_limited_warning("push_metrics_v1", Duration::from_secs(30)) {
                    warn!(
                        event = "metrics_push_failed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Failed to push metrics"
                    );
                } else {
                    debug!(
                        event = "metrics_push_failed_suppressed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Suppressing repeated metrics push failure"
                    );
                }
            }
            estimated_slot = self.ctx.slot_tracker.estimated_current_slot();

            let sleep_duration_ms = if items_processed_this_iteration > 0 {
                self.ctx.config.general_config.sleep_after_processing_ms
            } else {
                self.ctx.config.general_config.sleep_when_idle_ms
            };

            tokio::time::sleep(Duration::from_millis(sleep_duration_ms)).await;
        }
        Ok(())
    }

    #[instrument(
        level = "debug",
        skip(self, epoch_info, epoch_pda, tree_accounts, forester_slot_details, consecutive_eligibility_end),
        fields(tree = %tree_accounts.merkle_tree)
    )]
    async fn process_light_slot_v2(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        consecutive_eligibility_end: u64,
    ) -> std::result::Result<(), ForesterError> {
        debug!(
            event = "v2_light_slot_processing_started",
            run_id = %self.ctx.run_id,
            tree = %tree_accounts.merkle_tree,
            light_slot = forester_slot_details.slot,
            slot_start = forester_slot_details.start_solana_slot,
            slot_end = forester_slot_details.end_solana_slot,
            consecutive_eligibility_end_slot = consecutive_eligibility_end,
            "Processing V2 light slot"
        );

        let tree_pubkey = tree_accounts.merkle_tree;

        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        wait_until_slot_reached(
            &mut *rpc,
            &self.ctx.slot_tracker,
            forester_slot_details.start_solana_slot,
        )
        .await?;

        let cached_send_start = Instant::now();
        if let Some(items_sent) = self
            .try_send_cached_proofs(epoch_info, tree_accounts, consecutive_eligibility_end)
            .await?
        {
            if items_sent > 0 {
                let cached_send_duration = cached_send_start.elapsed();
                info!(
                    event = "cached_proofs_sent",
                    run_id = %self.ctx.run_id,
                    tree = %tree_pubkey,
                    items = items_sent,
                    duration_ms = cached_send_duration.as_millis() as u64,
                    "Sent items from proof cache"
                );
                self.update_metrics_and_counts(epoch_info.epoch, items_sent, cached_send_duration)
                    .await;
            }
        }

        let mut estimated_slot = self.ctx.slot_tracker.estimated_current_slot();

        const POLL_INTERVAL: Duration = Duration::from_millis(200);

        'inner_processing_loop: loop {
            if estimated_slot >= forester_slot_details.end_solana_slot {
                trace!(
                    "Ending V2 processing for slot {:?}",
                    forester_slot_details.slot
                );
                break 'inner_processing_loop;
            }

            let current_light_slot = (estimated_slot - epoch_info.phases.active.start)
                / epoch_pda.protocol_config.slot_length;
            if current_light_slot != forester_slot_details.slot {
                warn!(
                    event = "v2_light_slot_mismatch",
                    run_id = %self.ctx.run_id,
                    tree = %tree_pubkey,
                    expected_light_slot = forester_slot_details.slot,
                    actual_light_slot = current_light_slot,
                    estimated_slot,
                    "V2 slot mismatch; exiting processing"
                );
                break 'inner_processing_loop;
            }

            if !self
                .check_forester_eligibility(
                    epoch_pda,
                    current_light_slot,
                    &tree_accounts.merkle_tree,
                    epoch_info.epoch,
                    epoch_info,
                )
                .await?
            {
                break 'inner_processing_loop;
            }

            let processing_start_time = Instant::now();
            match self
                .dispatch_tree_processing(
                    epoch_info,
                    epoch_pda,
                    tree_accounts,
                    forester_slot_details,
                    consecutive_eligibility_end,
                    estimated_slot,
                )
                .await
            {
                Ok(count) => {
                    if count > 0 {
                        info!(
                            event = "v2_tree_processed_items",
                            run_id = %self.ctx.run_id,
                            tree = %tree_pubkey,
                            items = count,
                            epoch = epoch_info.epoch,
                            "V2 processed items for tree"
                        );
                        self.update_metrics_and_counts(
                            epoch_info.epoch,
                            count,
                            processing_start_time.elapsed(),
                        )
                        .await;
                    } else {
                        tokio::time::sleep(POLL_INTERVAL).await;
                    }
                }
                Err(e) => {
                    if e.is_forester_not_eligible() {
                        return Err(e);
                    }
                    error!(
                        event = "v2_tree_processing_failed",
                        run_id = %self.ctx.run_id,
                        tree = %tree_pubkey,
                        error = ?e,
                        "V2 processing failed for tree"
                    );
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            }

            if let Err(e) = push_metrics(&self.ctx.config.external_services.pushgateway_url).await {
                if should_emit_rate_limited_warning("push_metrics_v2", Duration::from_secs(30)) {
                    warn!(
                        event = "metrics_push_failed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Failed to push metrics"
                    );
                } else {
                    debug!(
                        event = "metrics_push_failed_suppressed",
                        run_id = %self.ctx.run_id,
                        error = ?e,
                        "Suppressing repeated metrics push failure"
                    );
                }
            }
            estimated_slot = self.ctx.slot_tracker.estimated_current_slot();
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn dispatch_tree_processing(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        tree_accounts: &TreeAccounts,
        forester_slot_details: &ForesterSlot,
        consecutive_eligibility_end: u64,
        current_solana_slot: u64,
    ) -> std::result::Result<usize, ForesterError> {
        match tree_accounts.tree_type {
            TreeType::Unknown => self
                .dispatch_compression(
                    epoch_info,
                    epoch_pda,
                    forester_slot_details,
                    consecutive_eligibility_end,
                )
                .await
                .map_err(ForesterError::from),
            TreeType::StateV1 | TreeType::AddressV1 => {
                self.process_v1(
                    epoch_info,
                    epoch_pda,
                    tree_accounts,
                    forester_slot_details,
                    current_solana_slot,
                )
                .await
            }
            TreeType::StateV2 | TreeType::AddressV2 => {
                let result = self
                    .process_v2(epoch_info, tree_accounts, consecutive_eligibility_end)
                    .await?;
                self.epoch_tracker
                    .add_processing_metrics(epoch_info.epoch, result.metrics)
                    .await;
                Ok(result.items_processed)
            }
        }
    }

    pub(crate) async fn check_forester_eligibility(
        &self,
        epoch_pda: &ForesterEpochPda,
        current_light_slot: u64,
        queue_pubkey: &solana_program::pubkey::Pubkey,
        current_epoch_num: u64,
        epoch_info: &Epoch,
    ) -> crate::Result<bool> {
        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        let current_phase_state = epoch_info.phases.get_current_epoch_state(current_slot);

        if current_phase_state != EpochState::Active {
            trace!(
                "Skipping processing: not in active phase (current phase: {:?}, slot: {})",
                current_phase_state,
                current_slot
            );
            return Ok(false);
        }

        let total_epoch_weight = epoch_pda.total_epoch_weight.ok_or_else(|| {
            anyhow::anyhow!(
                "Total epoch weight not available in ForesterEpochPda for epoch {}",
                current_epoch_num
            )
        })?;

        let eligible_forester_slot_index = ForesterEpochPda::get_eligible_forester_index(
            current_light_slot,
            queue_pubkey,
            total_epoch_weight,
            current_epoch_num,
        )
        .map_err(|e| {
            error!(
                event = "eligibility_index_calculation_failed",
                run_id = %self.ctx.run_id,
                queue = %queue_pubkey,
                epoch = current_epoch_num,
                light_slot = current_light_slot,
                error = ?e,
                "Failed to calculate eligible forester index"
            );
            anyhow::anyhow!("Eligibility calculation failed: {}", e)
        })?;

        if !epoch_pda.is_eligible(eligible_forester_slot_index) {
            warn!(
                event = "forester_not_eligible_for_slot",
                run_id = %self.ctx.run_id,
                forester = %self.ctx.config.payer_keypair.pubkey(),
                queue = %queue_pubkey,
                light_slot = current_light_slot,
                "Forester is no longer eligible to process this queue in current light slot"
            );
            return Ok(false);
        }
        Ok(true)
    }

    pub(crate) async fn update_metrics_and_counts(
        &self,
        epoch_num: u64,
        items_processed: usize,
        duration: Duration,
    ) {
        if items_processed > 0 {
            trace!(
                "{} items processed in this iteration, duration: {:?}",
                items_processed,
                duration
            );
            queue_metric_update(epoch_num, items_processed, duration);
            self.epoch_tracker
                .increment_processed_items_count(epoch_num, items_processed)
                .await;
            self.ctx.heartbeat.add_items_processed(items_processed);
        }
    }

    pub(crate) fn is_in_active_phase(
        &self,
        slot: u64,
        epoch_info: &ForesterEpochInfo,
    ) -> crate::Result<bool> {
        let current_epoch = self.ctx.protocol_config.get_current_active_epoch(slot)?;
        if current_epoch != epoch_info.epoch.epoch {
            return Ok(false);
        }

        Ok(self
            .ctx
            .protocol_config
            .is_active_phase(slot, epoch_info.epoch.epoch)
            .is_ok())
    }
}
