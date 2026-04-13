//! Epoch registration, recovery, wait-for-active-phase, and re-finalization.

use std::time::Duration;

use anyhow::Context;
use forester_utils::forester_epoch::{get_epoch_phases, Epoch, TreeAccounts, TreeForesterSchedule};
use light_client::{
    indexer::Indexer,
    rpc::{LightClient, LightClientConfig, Rpc, RpcError},
};
use light_compressed_account::TreeType;
use light_registry::{
    sdk::create_finalize_registration_instruction,
    utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority},
    EpochPda, ForesterEpochPda,
};
use solana_sdk::signature::Signer;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

use super::{tracker::RegistrationTracker, EpochManager};
use crate::{
    errors::{ForesterError, RegistrationError},
    pagerduty::send_pagerduty_alert,
    slot_tracker::{slot_duration, wait_until_slot_reached},
    smart_transaction::{send_smart_transaction, ComputeBudgetConfig, SendSmartTransactionConfig},
    transaction_timing::scheduled_confirmation_deadline,
    ForesterEpochInfo,
};

impl<R: Rpc + Indexer> EpochManager<R> {
    pub(crate) async fn recover_registration_info_if_exists(
        &self,
        epoch: u64,
    ) -> std::result::Result<Option<ForesterEpochInfo>, ForesterError> {
        debug!("Recovering registration info for epoch {}", epoch);

        let forester_epoch_pda_pubkey =
            get_forester_epoch_pda_from_authority(&self.ctx.config.derivation_pubkey, epoch).0;

        let existing_pda = {
            let rpc = self.ctx.rpc_pool.get_connection().await?;
            rpc.get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?
        };

        match existing_pda {
            Some(pda) => self
                .recover_registration_info_internal(epoch, forester_epoch_pda_pubkey, pda)
                .await
                .map(Some)
                .map_err(ForesterError::from),
            None => Ok(None),
        }
    }

    async fn recover_registration_info_internal(
        &self,
        epoch: u64,
        forester_epoch_pda_address: solana_program::pubkey::Pubkey,
        forester_epoch_pda: ForesterEpochPda,
    ) -> crate::Result<ForesterEpochInfo> {
        let rpc = self.ctx.rpc_pool.get_connection().await?;

        let phases = get_epoch_phases(&self.ctx.protocol_config, epoch);
        let slot = rpc.get_slot().await?;
        let state = phases.get_current_epoch_state(slot);

        let epoch_pda_address = get_epoch_pda_address(epoch);
        let epoch_pda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await
            .with_context(|| format!("Failed to fetch EpochPda for epoch {}", epoch))?
            .ok_or(RegistrationError::EpochPdaNotFound {
                epoch,
                pda_address: epoch_pda_address,
            })?;

        let epoch_info = Epoch {
            epoch,
            epoch_pda: epoch_pda_address,
            forester_epoch_pda: forester_epoch_pda_address,
            phases,
            state,
            merkle_trees: Vec::new(),
        };

        let forester_epoch_info = ForesterEpochInfo {
            epoch: epoch_info,
            epoch_pda,
            forester_epoch_pda,
            trees: Vec::new(),
        };

        Ok(forester_epoch_info)
    }

    pub(crate) async fn register_for_epoch_with_retry(
        &self,
        epoch: u64,
        max_retries: u32,
        retry_delay: Duration,
    ) -> std::result::Result<ForesterEpochInfo, ForesterError> {
        let rpc = LightClient::new(LightClientConfig {
            url: self.ctx.config.external_services.rpc_url.to_string(),
            photon_url: self.ctx.config.external_services.indexer_url.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            fetch_active_tree: false,
        })
        .await
        .map_err(ForesterError::Rpc)?;
        let slot = rpc.get_slot().await.map_err(ForesterError::Rpc)?;
        let phases = get_epoch_phases(&self.ctx.protocol_config, epoch);

        if slot < phases.registration.start {
            let slots_to_wait = phases.registration.start.saturating_sub(slot);
            info!(
                event = "registration_wait_for_window",
                run_id = %self.ctx.run_id,
                epoch,
                current_slot = slot,
                registration_start_slot = phases.registration.start,
                slots_to_wait,
                "Registration window not open yet; waiting"
            );
            let wait_duration = slot_duration() * slots_to_wait as u32;
            sleep(wait_duration).await;
        }

        for attempt in 0..max_retries {
            match self.recover_registration_info_if_exists(epoch).await {
                Ok(Some(registration_info)) => return Ok(registration_info),
                Ok(None) => {}
                Err(e) => return Err(e),
            }

            match self.register_for_epoch(epoch).await {
                Ok(registration_info) => return Ok(registration_info),
                Err(e) => {
                    warn!(
                        event = "registration_attempt_failed",
                        run_id = %self.ctx.run_id,
                        epoch,
                        attempt = attempt + 1,
                        max_attempts = max_retries,
                        error = ?e,
                        "Failed to register for epoch; retrying"
                    );
                    if attempt < max_retries - 1 {
                        sleep(retry_delay).await;
                    } else {
                        if let Some(pagerduty_key) = self
                            .ctx
                            .config
                            .external_services
                            .pagerduty_routing_key
                            .clone()
                        {
                            if let Err(alert_err) = send_pagerduty_alert(
                                &pagerduty_key,
                                &format!(
                                    "Forester failed to register for epoch {} after {} attempts",
                                    epoch, max_retries
                                ),
                                "critical",
                                &format!("Forester {}", self.ctx.config.payer_keypair.pubkey()),
                            )
                            .await
                            {
                                error!(
                                    event = "pagerduty_alert_failed",
                                    run_id = %self.ctx.run_id,
                                    epoch,
                                    error = ?alert_err,
                                    "Failed to send PagerDuty alert"
                                );
                            }
                        }
                        return Err(ForesterError::Other(e));
                    }
                }
            }
        }
        Err(RegistrationError::MaxRetriesExceeded {
            epoch,
            attempts: max_retries,
        }
        .into())
    }

    #[instrument(level = "debug", skip(self), fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch))]
    async fn register_for_epoch(&self, epoch: u64) -> crate::Result<ForesterEpochInfo> {
        info!(
            event = "registration_attempt_started",
            run_id = %self.ctx.run_id,
            epoch, "Registering for epoch"
        );
        let mut rpc = LightClient::new(LightClientConfig {
            url: self.ctx.config.external_services.rpc_url.to_string(),
            photon_url: self.ctx.config.external_services.indexer_url.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::processed()),
            fetch_active_tree: false,
        })
        .await?;
        let slot = rpc.get_slot().await?;
        let phases = get_epoch_phases(&self.ctx.protocol_config, epoch);

        if slot >= phases.registration.start {
            let forester_epoch_pda_pubkey =
                get_forester_epoch_pda_from_authority(&self.ctx.config.derivation_pubkey, epoch).0;
            let existing_registration = rpc
                .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
                .await?;

            if let Some(existing_pda) = existing_registration {
                info!(
                    event = "registration_already_exists",
                    run_id = %self.ctx.run_id,
                    epoch, "Already registered for epoch; recovering registration info"
                );
                let registration_info = self
                    .recover_registration_info_internal(
                        epoch,
                        forester_epoch_pda_pubkey,
                        existing_pda,
                    )
                    .await?;
                return Ok(registration_info);
            }

            let registration_info = {
                debug!("Registering epoch {}", epoch);
                let registered_epoch = match Epoch::register(
                    &mut rpc,
                    &self.ctx.protocol_config,
                    &self.ctx.config.payer_keypair,
                    &self.ctx.config.derivation_pubkey,
                    Some(epoch),
                )
                .await
                .with_context(|| {
                    format!("Failed to execute epoch registration for epoch {}", epoch)
                })? {
                    Some(epoch) => {
                        debug!("Registered epoch: {:?}", epoch);
                        epoch
                    }
                    None => {
                        return Err(RegistrationError::EmptyRegistration.into());
                    }
                };

                let forester_epoch_pda = rpc
                    .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to fetch ForesterEpochPda from RPC for address {}",
                            registered_epoch.forester_epoch_pda
                        )
                    })?
                    .ok_or(RegistrationError::ForesterEpochPdaNotFound {
                        epoch,
                        pda_address: registered_epoch.forester_epoch_pda,
                    })?;

                let epoch_pda_address = get_epoch_pda_address(epoch);
                let epoch_pda = rpc
                    .get_anchor_account::<EpochPda>(&epoch_pda_address)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to fetch EpochPda from RPC for address {}",
                            epoch_pda_address
                        )
                    })?
                    .ok_or(RegistrationError::EpochPdaNotFound {
                        epoch,
                        pda_address: epoch_pda_address,
                    })?;

                ForesterEpochInfo {
                    epoch: registered_epoch,
                    epoch_pda,
                    forester_epoch_pda,
                    trees: Vec::new(),
                }
            };
            debug!("Registered: {:?}", registration_info);
            Ok(registration_info)
        } else {
            warn!(
                event = "registration_too_early",
                run_id = %self.ctx.run_id,
                epoch,
                current_slot = slot,
                registration_start_slot = phases.registration.start,
                "Too early to register for epoch"
            );
            Err(RegistrationError::RegistrationPhaseNotStarted {
                epoch,
                current_slot: slot,
                registration_start: phases.registration.start,
            }
            .into())
        }
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch))]
    pub(crate) async fn wait_for_active_phase(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> std::result::Result<Option<ForesterEpochInfo>, ForesterError> {
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        let active_phase_start_slot = epoch_info.epoch.phases.active.start;
        let active_phase_end_slot = epoch_info.epoch.phases.active.end;
        let current_slot = self.ctx.slot_tracker.estimated_current_slot();

        if current_slot >= active_phase_start_slot {
            info!(
                event = "active_phase_already_started",
                run_id = %self.ctx.run_id,
                current_slot,
                active_phase_start_slot,
                active_phase_end_slot,
                slots_left = active_phase_end_slot.saturating_sub(current_slot),
                "Active phase has already started"
            );
        } else {
            let waiting_slots = active_phase_start_slot - current_slot;
            let waiting_secs = waiting_slots / 2;
            info!(
                event = "wait_for_active_phase",
                run_id = %self.ctx.run_id,
                current_slot,
                active_phase_start_slot,
                waiting_slots,
                approx_wait_seconds = waiting_secs,
                "Waiting for active phase to start"
            );
        }

        self.prewarm_all_trees_during_wait(epoch_info, active_phase_start_slot)
            .await;

        wait_until_slot_reached(&mut *rpc, &self.ctx.slot_tracker, active_phase_start_slot).await?;

        let forester_epoch_pda_pubkey = get_forester_epoch_pda_from_authority(
            &self.ctx.config.derivation_pubkey,
            epoch_info.epoch.epoch,
        )
        .0;
        let existing_registration = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?;

        if let Some(registration) = existing_registration {
            if registration.total_epoch_weight.is_none() {
                let current_slot = rpc.get_slot().await?;
                if current_slot > epoch_info.epoch.phases.active.end {
                    info!(
                        event = "skip_finalize_registration_phase_ended",
                        run_id = %self.ctx.run_id,
                        epoch = epoch_info.epoch.epoch,
                        current_slot,
                        active_phase_end_slot = epoch_info.epoch.phases.active.end,
                        "Skipping FinalizeRegistration because active phase ended"
                    );
                    return Ok(None);
                }

                let ix = create_finalize_registration_instruction(
                    &self.ctx.config.payer_keypair.pubkey(),
                    &self.ctx.config.derivation_pubkey,
                    epoch_info.epoch.epoch,
                );
                let priority_fee = self
                    .ctx
                    .resolve_epoch_priority_fee(&*rpc, epoch_info.epoch.epoch)
                    .await?;
                let Some(confirmation_deadline) = scheduled_confirmation_deadline(
                    epoch_info
                        .epoch
                        .phases
                        .active
                        .end
                        .saturating_sub(current_slot),
                ) else {
                    info!(
                        event = "skip_finalize_registration_confirmation_budget_exhausted",
                        run_id = %self.ctx.run_id,
                        epoch = epoch_info.epoch.epoch,
                        current_slot,
                        active_phase_end_slot = epoch_info.epoch.phases.active.end,
                        "Skipping FinalizeRegistration because not enough active-phase time remains for confirmation"
                    );
                    return Ok(None);
                };
                let payer = self.ctx.config.payer_keypair.pubkey();
                let signers = [&self.ctx.config.payer_keypair];
                send_smart_transaction(
                    &mut *rpc,
                    SendSmartTransactionConfig {
                        instructions: vec![ix],
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
                .map_err(RpcError::from)?;
            }
        }

        let mut epoch_info = (*epoch_info).clone();
        epoch_info.forester_epoch_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&epoch_info.epoch.forester_epoch_pda)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch ForesterEpochPda for epoch {} at address {}",
                    epoch_info.epoch.epoch, epoch_info.epoch.forester_epoch_pda
                )
            })?
            .ok_or(RegistrationError::ForesterEpochPdaNotFound {
                epoch: epoch_info.epoch.epoch,
                pda_address: epoch_info.epoch.forester_epoch_pda,
            })?;

        let slot = rpc.get_slot().await?;
        let trees = self.trees.lock().await;
        tracing::trace!("Adding schedule for trees: {:?}", *trees);
        epoch_info.add_trees_with_schedule(&trees, slot)?;

        if self.compressible_tracker.is_some() && self.ctx.config.compressible_config.is_some() {
            let compression_tree_accounts = TreeAccounts {
                merkle_tree: solana_sdk::pubkey::Pubkey::default(),
                queue: solana_sdk::pubkey::Pubkey::default(),
                tree_type: TreeType::Unknown,
                is_rolledover: false,
                owner: solana_sdk::pubkey::Pubkey::default(),
            };
            let tree_schedule = TreeForesterSchedule::new_with_schedule(
                &compression_tree_accounts,
                slot,
                &epoch_info.forester_epoch_pda,
                &epoch_info.epoch_pda,
            )
            .map_err(anyhow::Error::from)?;
            epoch_info.trees.insert(0, tree_schedule);
            debug!("Added compression tree to epoch {}", epoch_info.epoch.epoch);
        }

        info!(
            event = "active_phase_ready",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch.epoch,
            "Finished waiting for active phase"
        );
        Ok(Some(epoch_info))
    }

    /// Check if `EpochPda.registered_weight` changed on-chain. If so,
    /// one task sends a `finalize_registration` tx while others wait,
    /// then all tasks refresh their `ForesterEpochPda` and recompute schedules.
    pub(crate) async fn maybe_refinalize(
        &self,
        epoch_info: &Epoch,
        forester_epoch_pda: &mut ForesterEpochPda,
        tree_schedule: &mut TreeForesterSchedule,
        registration_tracker: &RegistrationTracker,
        force: bool,
    ) -> crate::Result<()> {
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        let epoch_pda_address = get_epoch_pda_address(epoch_info.epoch);
        let on_chain_epoch_pda: EpochPda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await?
            .ok_or(RegistrationError::EpochPdaNotFound {
                epoch: epoch_info.epoch,
                pda_address: epoch_pda_address,
            })?;

        let on_chain_weight = on_chain_epoch_pda.registered_weight;
        let cached_weight = registration_tracker.cached_weight();
        let weight_changed = on_chain_weight != cached_weight;

        if !weight_changed && !force {
            return Ok(());
        }

        if weight_changed {
            info!(
                event = "registered_weight_changed",
                run_id = %self.ctx.run_id,
                epoch = epoch_info.epoch,
                old_weight = cached_weight,
                new_weight = on_chain_weight,
                "Detected new forester registration, re-finalizing"
            );

            if registration_tracker.try_claim_refinalize() {
                let ix = create_finalize_registration_instruction(
                    &self.ctx.config.payer_keypair.pubkey(),
                    &self.ctx.config.derivation_pubkey,
                    epoch_info.epoch,
                );
                let priority_fee = self
                    .ctx
                    .resolve_epoch_priority_fee(&*rpc, epoch_info.epoch)
                    .await?;
                let current_slot = rpc.get_slot().await?;
                let Some(confirmation_deadline) = scheduled_confirmation_deadline(
                    epoch_info.phases.active.end.saturating_sub(current_slot),
                ) else {
                    info!(
                        event = "refinalize_registration_skipped_confirmation_budget_exhausted",
                        run_id = %self.ctx.run_id,
                        epoch = epoch_info.epoch,
                        current_slot,
                        active_phase_end_slot = epoch_info.phases.active.end,
                        "Skipping re-finalization because not enough active-phase time remains for confirmation"
                    );
                    registration_tracker.complete_refinalize(cached_weight);
                    return Ok(());
                };
                let payer = self.ctx.config.payer_keypair.pubkey();
                let signers = [&self.ctx.config.payer_keypair];
                match send_smart_transaction(
                    &mut *rpc,
                    SendSmartTransactionConfig {
                        instructions: vec![ix],
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
                .map_err(RpcError::from)
                {
                    Ok(_) => {
                        let post_finalize_weight =
                            match rpc.get_anchor_account::<EpochPda>(&epoch_pda_address).await {
                                Ok(Some(pda)) => pda.registered_weight,
                                _ => on_chain_weight,
                            };
                        info!(
                            event = "refinalize_registration_success",
                            run_id = %self.ctx.run_id,
                            epoch = epoch_info.epoch,
                            new_weight = post_finalize_weight,
                            "Re-finalized registration on-chain"
                        );
                        registration_tracker.complete_refinalize(post_finalize_weight);
                    }
                    Err(e) => {
                        registration_tracker.complete_refinalize(cached_weight);
                        return Err(e.into());
                    }
                }
            } else {
                registration_tracker.wait_for_refinalize().await;
            }
        }

        let refreshed_epoch_pda: EpochPda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_address)
            .await?
            .ok_or(RegistrationError::EpochPdaNotFound {
                epoch: epoch_info.epoch,
                pda_address: epoch_pda_address,
            })?;
        let updated_pda: ForesterEpochPda = rpc
            .get_anchor_account::<ForesterEpochPda>(&epoch_info.forester_epoch_pda)
            .await?
            .ok_or(RegistrationError::ForesterEpochPdaNotFound {
                epoch: epoch_info.epoch,
                pda_address: epoch_info.forester_epoch_pda,
            })?;

        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        let new_schedule = TreeForesterSchedule::new_with_schedule(
            &tree_schedule.tree_accounts,
            current_slot,
            &updated_pda,
            &refreshed_epoch_pda,
        )?;

        *forester_epoch_pda = updated_pda;
        *tree_schedule = new_schedule;

        info!(
            event = "schedule_recomputed_after_refinalize",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch,
            tree = %tree_schedule.tree_accounts.merkle_tree,
            new_eligible_slots = tree_schedule.slots.iter().filter(|s| s.is_some()).count(),
            "Recomputed schedule after re-finalization"
        );

        Ok(())
    }
}
