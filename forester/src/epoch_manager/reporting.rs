//! Work reporting: send metrics to channel and report on-chain.

use light_client::{
    indexer::Indexer,
    rpc::{LightClient, LightClientConfig, Rpc, RpcError},
};
use light_registry::{
    sdk::create_report_work_instruction, utils::get_forester_epoch_pda_from_authority,
    ForesterEpochPda,
};
use solana_program::instruction::InstructionError;
use solana_sdk::{signature::Signer, transaction::TransactionError};
use tracing::{info, instrument};

use super::{EpochManager, WorkReport};
use crate::{
    errors::{rpc_is_already_processed, ChannelError, ForesterError, WorkReportError},
    slot_tracker::wait_until_slot_reached,
    smart_transaction::{send_smart_transaction, ComputeBudgetConfig, SendSmartTransactionConfig},
    ForesterEpochInfo,
};

impl<R: Rpc + Indexer> EpochManager<R> {
    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch))]
    pub(crate) async fn wait_for_report_work_phase(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> crate::Result<()> {
        info!(
            event = "wait_for_report_work_phase",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch.epoch,
            report_work_start_slot = epoch_info.epoch.phases.report_work.start,
            "Waiting for report work phase"
        );
        let mut rpc = self.ctx.rpc_pool.get_connection().await?;
        let report_work_start_slot = epoch_info.epoch.phases.report_work.start;
        wait_until_slot_reached(&mut *rpc, &self.ctx.slot_tracker, report_work_start_slot).await?;

        info!(
            event = "report_work_phase_ready",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch.epoch,
            "Finished waiting for report work phase"
        );
        Ok(())
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch))]
    pub(crate) async fn send_work_report(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> crate::Result<()> {
        let report = WorkReport {
            epoch: epoch_info.epoch.epoch,
            processed_items: self
                .epoch_tracker
                .get_processed_items_count(epoch_info.epoch.epoch)
                .await,
            metrics: self
                .epoch_tracker
                .get_processing_metrics(epoch_info.epoch.epoch)
                .await,
        };

        info!(
            event = "work_report_sent_to_channel",
            run_id = %self.ctx.run_id,
            epoch = report.epoch,
            items = report.processed_items,
            total_circuit_inputs_ms = report.metrics.total_circuit_inputs().as_millis() as u64,
            total_proof_generation_ms = report.metrics.total_proof_generation().as_millis() as u64,
            total_round_trip_ms = report.metrics.total_round_trip().as_millis() as u64,
            tx_sending_ms = report.metrics.tx_sending_duration.as_millis() as u64,
            "Sending work report to channel"
        );

        self.work_report_sender
            .send(report)
            .await
            .map_err(|e| ChannelError::WorkReportSend {
                epoch: report.epoch,
                error: e.to_string(),
            })?;
        self.ctx.heartbeat.increment_work_report_sent();

        Ok(())
    }

    #[instrument(level = "debug", skip(self, epoch_info), fields(forester = %self.ctx.config.payer_keypair.pubkey(), epoch = epoch_info.epoch.epoch))]
    pub(crate) async fn report_work_onchain(
        &self,
        epoch_info: &ForesterEpochInfo,
    ) -> crate::Result<()> {
        info!(
            event = "work_report_onchain_started",
            run_id = %self.ctx.run_id,
            epoch = epoch_info.epoch.epoch,
            "Reporting work on-chain"
        );
        let mut rpc = LightClient::new(LightClientConfig {
            url: self.ctx.config.external_services.rpc_url.to_string(),
            photon_url: self.ctx.config.external_services.indexer_url.clone(),
            commitment_config: Some(solana_sdk::commitment_config::CommitmentConfig::processed()),
            fetch_active_tree: false,
        })
        .await?;

        let forester_epoch_pda_pubkey = get_forester_epoch_pda_from_authority(
            &self.ctx.config.derivation_pubkey,
            epoch_info.epoch.epoch,
        )
        .0;
        if let Some(forester_epoch_pda) = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?
        {
            if forester_epoch_pda.has_reported_work {
                return Ok(());
            }
        }

        let forester_epoch_pda = &epoch_info.forester_epoch_pda;
        if forester_epoch_pda.has_reported_work {
            return Ok(());
        }

        let ix = create_report_work_instruction(
            &self.ctx.config.payer_keypair.pubkey(),
            &self.ctx.config.derivation_pubkey,
            epoch_info.epoch.epoch,
        );

        let priority_fee = self
            .ctx
            .resolve_epoch_priority_fee(&rpc, epoch_info.epoch.epoch)
            .await?;
        let payer = self.ctx.config.payer_keypair.pubkey();
        let signers = [&self.ctx.config.payer_keypair];
        match send_smart_transaction(
            &mut rpc,
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
                confirmation_deadline: None,
            },
        )
        .await
        .map_err(RpcError::from)
        {
            Ok(_) => {
                info!(
                    event = "work_report_onchain_succeeded",
                    run_id = %self.ctx.run_id,
                    epoch = epoch_info.epoch.epoch,
                    "Work reported on-chain"
                );
            }
            Err(e) => {
                if rpc_is_already_processed(&e) {
                    info!(
                        event = "work_report_onchain_already_reported",
                        run_id = %self.ctx.run_id,
                        epoch = epoch_info.epoch.epoch,
                        "Work already reported on-chain for epoch"
                    );
                    return Ok(());
                }
                if let RpcError::ClientError(client_error) = &e {
                    if let Some(TransactionError::InstructionError(
                        _,
                        InstructionError::Custom(error_code),
                    )) = client_error.get_transaction_error()
                    {
                        return WorkReportError::from_registry_error(
                            error_code,
                            epoch_info.epoch.epoch,
                        )
                        .map_err(|e| anyhow::Error::from(ForesterError::from(e)));
                    }
                }
                return Err(anyhow::Error::from(WorkReportError::Transaction(Box::new(
                    e,
                ))));
            }
        }

        Ok(())
    }
}
