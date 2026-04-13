//! Compression dispatch: ctoken, PDA, and mint compression during active phase.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::anyhow;
use forester_utils::forester_epoch::{Epoch, ForesterSlot};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_registry::ForesterEpochPda;
use solana_program::pubkey::Pubkey;
use tracing::{debug, error, info, trace, warn};

use super::EpochManager;
use crate::compressible::{
    traits::{
        Cancelled, CompressibleState, CompressibleTracker, CompressionOutcome, CompressionTaskError,
    },
    CTokenCompressor, CompressibleConfig,
};

impl<R: Rpc + Indexer> EpochManager<R> {
    pub(crate) async fn dispatch_compression(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        forester_slot_details: &ForesterSlot,
        consecutive_eligibility_end: u64,
    ) -> crate::Result<usize> {
        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                "Skipping compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                current_slot, consecutive_eligibility_end
            );
            return Ok(0);
        }

        if current_slot >= forester_slot_details.end_solana_slot {
            debug!(
                "Skipping compression: forester slot ended (current_slot={}, slot_end={})",
                current_slot, forester_slot_details.end_solana_slot
            );
            return Ok(0);
        }

        let current_light_slot = current_slot.saturating_sub(epoch_info.phases.active.start)
            / epoch_pda.protocol_config.slot_length;
        if !self
            .check_forester_eligibility(
                epoch_pda,
                current_light_slot,
                &Pubkey::default(),
                epoch_info.epoch,
                epoch_info,
            )
            .await?
        {
            debug!(
                "Skipping compression: forester not eligible for current light slot {}",
                current_light_slot
            );
            return Ok(0);
        }

        debug!("Dispatching compression for epoch {}", epoch_info.epoch);

        let tracker = self
            .compressible_tracker
            .as_ref()
            .ok_or_else(|| anyhow!("Compressible tracker not initialized"))?;

        let config = self
            .ctx
            .config
            .compressible_config
            .as_ref()
            .ok_or_else(|| anyhow!("Compressible config not set"))?;

        let pending = tracker.pending();
        let accounts: Vec<_> = tracker
            .accounts()
            .iter()
            .filter(|entry| {
                entry.value().is_ready_to_compress(current_slot) && !pending.contains(entry.key())
            })
            .map(|entry| entry.value().clone())
            .collect();
        let _ = pending;

        if accounts.is_empty() {
            trace!("No compressible accounts ready for compression");
            return Ok(0);
        }

        let num_batches = accounts.len().div_ceil(config.batch_size);
        info!(
            event = "compression_ctoken_started",
            run_id = %self.ctx.run_id,
            accounts = accounts.len(),
            batches = num_batches,
            batch_size = config.batch_size,
            "Starting ctoken compression batches"
        );

        let compressor = CTokenCompressor::new(
            self.ctx.rpc_pool.clone(),
            tracker.clone(),
            self.ctx.authority.clone(),
            self.ctx.transaction_policy(),
        );

        let (registered_forester_pda, _) =
            light_registry::utils::get_forester_epoch_pda_from_authority(
                &self.ctx.config.derivation_pubkey,
                epoch_info.epoch,
            );

        use futures::stream::StreamExt;

        let batches: Vec<(usize, Vec<_>)> = accounts
            .chunks(config.batch_size)
            .enumerate()
            .map(|(idx, chunk)| (idx, chunk.to_vec()))
            .collect();

        let slot_tracker = self.ctx.slot_tracker.clone();
        let cancelled = Arc::new(AtomicBool::new(false));

        let compression_futures = batches.into_iter().map(|(batch_idx, batch)| {
            let compressor = compressor.clone();
            let slot_tracker = slot_tracker.clone();
            let cancelled = cancelled.clone();
            let run_id = self.ctx.run_id.clone();
            async move {
                if cancelled.load(Ordering::Relaxed) {
                    debug!(
                        "Skipping compression batch {}/{}: cancelled",
                        batch_idx + 1,
                        num_batches
                    );
                    return Err((batch_idx, batch.len(), Cancelled.into()));
                }

                let current_slot = slot_tracker.estimated_current_slot();
                if current_slot >= consecutive_eligibility_end {
                    cancelled.store(true, Ordering::Relaxed);
                    warn!(
                        event = "compression_ctoken_cancelled_not_eligible",
                        run_id = %run_id,
                        current_slot,
                        eligibility_end_slot = consecutive_eligibility_end,
                        "Cancelling compression because forester is no longer eligible"
                    );
                    return Err((
                        batch_idx,
                        batch.len(),
                        anyhow!("Forester no longer eligible"),
                    ));
                }

                debug!(
                    "Processing compression batch {}/{} with {} accounts",
                    batch_idx + 1,
                    num_batches,
                    batch.len()
                );

                match compressor
                    .compress_batch(&batch, registered_forester_pda)
                    .await
                {
                    Ok(sig) => {
                        debug!(
                            "Compression batch {}/{} succeeded: {}",
                            batch_idx + 1,
                            num_batches,
                            sig
                        );
                        Ok((batch_idx, batch.len(), sig))
                    }
                    Err(e) => {
                        error!(
                            event = "compression_ctoken_batch_failed",
                            run_id = %run_id,
                            batch = batch_idx + 1,
                            total_batches = num_batches,
                            error = ?e,
                            "Compression batch failed"
                        );
                        Err((batch_idx, batch.len(), e))
                    }
                }
            }
        });

        let results = futures::stream::iter(compression_futures)
            .buffer_unordered(config.max_concurrent_batches)
            .collect::<Vec<_>>()
            .await;

        let mut total_compressed = 0;
        for result in results {
            match result {
                Ok((batch_idx, count, sig)) => {
                    info!(
                        event = "compression_ctoken_batch_succeeded",
                        run_id = %self.ctx.run_id,
                        batch = batch_idx + 1,
                        total_batches = num_batches,
                        accounts = count,
                        signature = %sig,
                        "Compression batch succeeded"
                    );
                    total_compressed += count;
                }
                Err((batch_idx, count, e)) => {
                    error!(
                        event = "compression_ctoken_batch_failed_final",
                        run_id = %self.ctx.run_id,
                        batch = batch_idx + 1,
                        total_batches = num_batches,
                        accounts = count,
                        error = ?e,
                        "Compression batch failed"
                    );
                }
            }
        }

        info!(
            event = "compression_ctoken_completed",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch,
            compressed_accounts = total_compressed,
            "Completed ctoken compression"
        );

        let pda_compressed = self
            .dispatch_pda_compression(epoch_info, epoch_pda, consecutive_eligibility_end)
            .await
            .unwrap_or_else(|e| {
                error!(
                    event = "compression_pda_dispatch_failed",
                    run_id = %self.ctx.run_id,
                    error = ?e,
                    "PDA compression failed"
                );
                0
            });

        let mint_compressed = self
            .dispatch_mint_compression(epoch_info, epoch_pda, consecutive_eligibility_end)
            .await
            .unwrap_or_else(|e| {
                error!(
                    event = "compression_mint_dispatch_failed",
                    run_id = %self.ctx.run_id,
                    error = ?e,
                    "Mint compression failed"
                );
                0
            });

        let total = total_compressed + pda_compressed + mint_compressed;
        info!(
            event = "compression_all_completed",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch,
            ctoken_compressed = total_compressed,
            pda_compressed,
            mint_compressed,
            total_compressed = total,
            "Completed all compression"
        );
        Ok(total)
    }

    async fn dispatch_pda_compression(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        consecutive_eligibility_end: u64,
    ) -> crate::Result<usize> {
        let Some((pda_tracker, config, current_slot)) = self
            .prepare_compression_dispatch(
                self.pda_tracker.as_ref(),
                "PDA",
                epoch_info,
                epoch_pda,
                consecutive_eligibility_end,
            )
            .await?
        else {
            return Ok(0);
        };

        if config.pda_programs.is_empty() {
            return Ok(0);
        }

        let mut total_compressed = 0;
        let cancelled = Arc::new(AtomicBool::new(false));

        for program_config in &config.pda_programs {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }

            let accounts = pda_tracker
                .get_ready_to_compress_for_program(&program_config.program_id, current_slot);

            if accounts.is_empty() {
                trace!(
                    "No compressible PDA accounts ready for program {}",
                    program_config.program_id
                );
                continue;
            }

            info!(
                event = "compression_pda_program_started",
                run_id = %self.ctx.run_id,
                program = %program_config.program_id,
                accounts = accounts.len(),
                "Processing compressible PDA accounts for program"
            );

            let pda_compressor = crate::compressible::pda::PdaCompressor::new(
                self.ctx.rpc_pool.clone(),
                pda_tracker.clone(),
                self.ctx.authority.clone(),
                self.ctx.transaction_policy(),
            );

            let cached_config = match pda_compressor.fetch_program_config(program_config).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!(
                        event = "compression_pda_program_config_failed",
                        run_id = %self.ctx.run_id,
                        program = %program_config.program_id,
                        error = ?e,
                        "Failed to fetch config for PDA program"
                    );
                    continue;
                }
            };

            let current_slot = self.ctx.slot_tracker.estimated_current_slot();
            if current_slot >= consecutive_eligibility_end {
                cancelled.store(true, Ordering::Relaxed);
                warn!(
                    event = "compression_pda_cancelled_not_eligible",
                    run_id = %self.ctx.run_id,
                    current_slot,
                    eligibility_end_slot = consecutive_eligibility_end,
                    "Stopping PDA compression because forester is no longer eligible"
                );
                break;
            }

            let results = pda_compressor
                .compress_batch_concurrent(
                    &accounts,
                    program_config,
                    &cached_config,
                    config.max_concurrent_batches,
                    cancelled.clone(),
                )
                .await;

            for result in results {
                match result {
                    CompressionOutcome::Compressed {
                        signature: sig,
                        pubkey,
                    } => {
                        debug!(
                            "Compressed PDA {} for program {}: {}",
                            pubkey, program_config.program_id, sig
                        );
                        total_compressed += 1;
                    }
                    CompressionOutcome::Failed {
                        error: CompressionTaskError::Cancelled,
                        ..
                    } => {}
                    CompressionOutcome::Failed {
                        pubkey,
                        error: CompressionTaskError::Failed(e),
                    } => {
                        error!(
                            event = "compression_pda_account_failed",
                            run_id = %self.ctx.run_id,
                            account = %pubkey,
                            program = %program_config.program_id,
                            error = ?e,
                            "Failed to compress PDA account"
                        );
                    }
                }
            }
        }

        info!(
            event = "compression_pda_completed",
            run_id = %self.ctx.run_id,
            compressed_accounts = total_compressed,
            "Completed PDA compression"
        );
        Ok(total_compressed)
    }

    async fn dispatch_mint_compression(
        &self,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        consecutive_eligibility_end: u64,
    ) -> crate::Result<usize> {
        let Some((mint_tracker, config, current_slot)) = self
            .prepare_compression_dispatch(
                self.mint_tracker.as_ref(),
                "Mint",
                epoch_info,
                epoch_pda,
                consecutive_eligibility_end,
            )
            .await?
        else {
            return Ok(0);
        };

        let accounts = mint_tracker.get_ready_to_compress(current_slot);

        if accounts.is_empty() {
            trace!("No compressible Mint accounts ready");
            return Ok(0);
        }

        info!(
            event = "compression_mint_started",
            run_id = %self.ctx.run_id,
            accounts = accounts.len(),
            max_concurrent = config.max_concurrent_batches,
            "Processing compressible Mint accounts"
        );

        let mint_compressor = crate::compressible::mint::MintCompressor::new(
            self.ctx.rpc_pool.clone(),
            mint_tracker.clone(),
            self.ctx.authority.clone(),
            self.ctx.transaction_policy(),
        );

        let cancelled = Arc::new(AtomicBool::new(false));

        let results = mint_compressor
            .compress_batch_concurrent(&accounts, config.max_concurrent_batches, cancelled)
            .await;

        let mut total_compressed = 0;
        for result in results {
            match result {
                CompressionOutcome::Compressed {
                    signature: sig,
                    pubkey,
                } => {
                    debug!("Compressed Mint {}: {}", pubkey, sig);
                    total_compressed += 1;
                }
                CompressionOutcome::Failed {
                    error: CompressionTaskError::Cancelled,
                    ..
                } => {}
                CompressionOutcome::Failed {
                    pubkey,
                    error: CompressionTaskError::Failed(e),
                } => {
                    error!(
                        event = "compression_mint_account_failed",
                        run_id = %self.ctx.run_id,
                        mint = %pubkey,
                        error = ?e,
                        "Failed to compress mint account"
                    );
                }
            }
        }

        info!(
            event = "compression_mint_completed",
            run_id = %self.ctx.run_id,
            compressed_accounts = total_compressed,
            "Completed Mint compression"
        );
        Ok(total_compressed)
    }

    async fn prepare_compression_dispatch<'a, T>(
        &'a self,
        tracker: Option<&'a T>,
        label: &'static str,
        epoch_info: &Epoch,
        epoch_pda: &ForesterEpochPda,
        consecutive_eligibility_end: u64,
    ) -> crate::Result<Option<(&'a T, &'a CompressibleConfig, u64)>> {
        let Some(tracker) = tracker else {
            return Ok(None);
        };

        let Some(config) = self.ctx.config.compressible_config.as_ref() else {
            return Ok(None);
        };

        let current_slot = self.ctx.slot_tracker.estimated_current_slot();
        if current_slot >= consecutive_eligibility_end {
            debug!(
                "Skipping {} compression: forester no longer eligible (current_slot={}, eligibility_end={})",
                label, current_slot, consecutive_eligibility_end
            );
            return Ok(None);
        }

        let current_light_slot = current_slot.saturating_sub(epoch_info.phases.active.start)
            / epoch_pda.protocol_config.slot_length;
        if !self
            .check_forester_eligibility(
                epoch_pda,
                current_light_slot,
                &Pubkey::default(),
                epoch_info.epoch,
                epoch_info,
            )
            .await?
        {
            debug!(
                "Skipping {} compression: forester not eligible for current light slot {}",
                label, current_light_slot
            );
            return Ok(None);
        }

        Ok(Some((tracker, config, current_slot)))
    }
}
