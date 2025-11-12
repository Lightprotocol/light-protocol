use std::{str::FromStr, sync::Arc, time::Duration};

use anchor_lang::{InstructionData, ToAccountMetas};
use forester_utils::{forester_epoch::EpochPhases, rpc_pool::SolanaRpcPool};
use light_client::rpc::Rpc;
use light_compressed_token_sdk::instructions::compress_and_close::CompressAndCloseAccounts as CTokenAccounts;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_registry::{
    accounts::CompressAndCloseContext, compressible::compressed_token::CompressAndCloseIndices,
    instruction::CompressAndClose, protocol_config::state::EpochState,
    utils::get_forester_epoch_pda_from_authority, ForesterEpochPda,
};
use light_sdk::instruction::PackedAccounts;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tracing::{debug, error, info, warn};

use super::{state::CompressibleAccountTracker, types::CompressibleAccountState};
use crate::{slot_tracker::SlotTracker, Result};

const REGISTRY_PROGRAM_ID_STR: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";

/// Compression executor that builds and sends compress_and_close transactions via registry program
pub struct Compressor<R: Rpc> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<CompressibleAccountTracker>,
    payer_keypair: Keypair,
    slot_tracker: Arc<SlotTracker>,
    batch_size: usize,
}

impl<R: Rpc> Compressor<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        tracker: Arc<CompressibleAccountTracker>,
        payer_keypair: Keypair,
        slot_tracker: Arc<SlotTracker>,
        batch_size: usize,
    ) -> Self {
        Self {
            rpc_pool,
            tracker,
            payer_keypair,
            slot_tracker,
            batch_size,
        }
    }

    /// Run compression for a specific epoch during the active phase
    pub async fn run_for_epoch(
        &mut self,
        current_epoch: u64,
        active_phase_end_slot: u64,
        epoch_phases: EpochPhases,
        sleep_after_processing_ms: u64,
        sleep_when_idle_ms: u64,
    ) -> Result<()> {
        info!(
            "Starting compression for epoch {} (active phase ends at slot {})",
            current_epoch, active_phase_end_slot
        );

        while self.slot_tracker.estimated_current_slot() < active_phase_end_slot {
            let current_slot = self.slot_tracker.estimated_current_slot();

            // Check if still in active phase
            let current_phase = epoch_phases.get_current_epoch_state(current_slot);
            if current_phase != EpochState::Active {
                info!(
                    "No longer in active phase (current phase: {:?}), exiting compression",
                    current_phase
                );
                break;
            }

            // Check forester eligibility
            if !self
                .check_compression_eligibility(current_epoch, current_slot, &epoch_phases)
                .await?
            {
                warn!(
                    "Forester no longer eligible for compression in epoch {}",
                    current_epoch
                );
                break;
            }

            // Get accounts that are ready to be compressed
            let accounts = self.tracker.get_ready_to_compress(current_slot);

            if accounts.is_empty() {
                debug!("No compressible accounts found");
                tokio::time::sleep(Duration::from_millis(sleep_when_idle_ms)).await;
                continue;
            }

            info!("Found {} compressible accounts", accounts.len());

            let mut total_compressed = 0;

            // Process in batches
            for (batch_num, batch) in accounts.chunks(self.batch_size).enumerate() {
                info!(
                    "Processing batch {} with {} accounts",
                    batch_num + 1,
                    batch.len()
                );

                match self.compress_batch(batch, current_epoch).await {
                    Ok(sig) => {
                        info!(
                            "Successfully compressed {} accounts in batch {}: {}",
                            batch.len(),
                            batch_num + 1,
                            sig
                        );
                        total_compressed += batch.len();
                    }
                    Err(e) => {
                        error!("Failed to compress batch {}: {:?}", batch_num + 1, e);
                        // Keep accounts in tracker for retry
                    }
                }
            }

            // Sleep based on whether we did work
            let sleep_duration_ms = if total_compressed > 0 {
                sleep_after_processing_ms
            } else {
                sleep_when_idle_ms
            };
            tokio::time::sleep(Duration::from_millis(sleep_duration_ms)).await;
        }

        info!("Compression for epoch {} completed", current_epoch);
        Ok(())
    }

    /// Check if forester is eligible for compression in the current epoch
    async fn check_compression_eligibility(
        &self,
        current_epoch: u64,
        current_slot: u64,
        epoch_phases: &EpochPhases,
    ) -> Result<bool> {
        // Check if in active phase
        let current_phase = epoch_phases.get_current_epoch_state(current_slot);
        if current_phase != EpochState::Active {
            return Ok(false);
        }

        // Check if forester is registered for this epoch
        let (forester_epoch_pda_pubkey, _) =
            get_forester_epoch_pda_from_authority(&self.payer_keypair.pubkey(), current_epoch);

        let rpc = self.rpc_pool.get_connection().await?;
        let forester_epoch_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
            .await?;

        if forester_epoch_pda.is_none() {
            return Ok(false);
        }

        let pda = forester_epoch_pda.unwrap();

        // Get total epoch weight
        let total_epoch_weight = match pda.total_epoch_weight {
            Some(weight) => weight,
            None => {
                debug!(
                    "Total epoch weight not yet available for epoch {}",
                    current_epoch
                );
                return Ok(false);
            }
        };

        // Calculate current light slot
        let current_light_slot =
            (current_slot - epoch_phases.active.start) / pda.protocol_config.slot_length;

        // Check eligibility using Pubkey::default() (epoch-level, not tree-specific)
        let eligible_forester_slot_index = ForesterEpochPda::get_eligible_forester_index(
            current_light_slot,
            &Pubkey::default(),
            total_epoch_weight,
            current_epoch,
        )
        .map_err(|e| anyhow::anyhow!("Failed to calculate eligible forester index: {:?}", e))?;

        Ok(pda.is_eligible(eligible_forester_slot_index))
    }

    pub async fn compress_batch(
        &self,
        account_states: &[CompressibleAccountState],
        current_epoch: u64,
    ) -> Result<Signature> {
        let registry_program_id = Pubkey::from_str(REGISTRY_PROGRAM_ID_STR)?;
        let compressed_token_program_id = Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID);

        // Derive compression_authority PDA deterministically (version = 1)
        let compression_authority_seeds = CompressibleConfig::get_compression_authority_seeds(1);
        let (compression_authority, _) = Pubkey::find_program_address(
            &compression_authority_seeds
                .iter()
                .map(|v| v.as_slice())
                .collect::<Vec<_>>(),
            &registry_program_id,
        );

        debug!("Compression authority: {}", compression_authority);

        // Get registered forester PDA
        let (registered_forester_pda, _) =
            get_forester_epoch_pda_from_authority(&self.payer_keypair.pubkey(), current_epoch);

        debug!("Registered forester PDA: {}", registered_forester_pda);

        // Get compressible config PDA
        let (compressible_config, _) =
            CompressibleConfig::derive_v1_config_pda(&registry_program_id);

        debug!("Compressible config: {}", compressible_config);

        // Get output tree from RPC
        let mut rpc = self.rpc_pool.get_connection().await?;
        // TODO: use a tree from config.
        rpc.get_latest_active_state_trees()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get state tree info: {}", e))?;

        let output_tree_info = rpc
            .get_random_state_tree_info()
            .map_err(|e| anyhow::anyhow!("Failed to get state tree info: {}", e))?;
        let output_queue = output_tree_info
            .get_output_pubkey()
            .map_err(|e| anyhow::anyhow!("Failed to get output queue: {}", e))?;

        debug!("Output queue: {}", output_queue);

        // Build PackedAccounts
        let mut packed_accounts = PackedAccounts::default();
        packed_accounts.insert_or_get(output_queue);

        let mut indices_vec = Vec::new();

        for account_state in account_states {
            let source_index = packed_accounts.insert_or_get(account_state.pubkey);

            // Convert mint from light_compressed_account::Pubkey to solana_sdk::Pubkey
            let mint = Pubkey::new_from_array(account_state.account.mint.to_bytes());
            let mint_index = packed_accounts.insert_or_get(mint);

            // Get compressible extension to extract rent_sponsor and compress_to_pubkey
            let compressible_ext = account_state
                .account
                .extensions
                .as_ref()
                .and_then(|exts| {
                    exts.iter().find_map(|ext| {
                        if let light_ctoken_types::state::ExtensionStruct::Compressible(comp) = ext
                        {
                            Some(comp)
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| anyhow::anyhow!("Account missing compressible extension"))?;

            // Determine owner based on compress_to_pubkey flag
            let compressed_token_owner = if compressible_ext.compress_to_pubkey != 0 {
                account_state.pubkey // Use account pubkey for PDAs
            } else {
                Pubkey::new_from_array(account_state.account.owner.to_bytes()) // Use original owner
            };

            let owner_index = packed_accounts.insert_or_get(compressed_token_owner);

            // Extract rent_sponsor from extension
            let rent_sponsor = Pubkey::new_from_array(compressible_ext.rent_sponsor);
            let rent_sponsor_index = packed_accounts.insert_or_get(rent_sponsor);

            indices_vec.push(CompressAndCloseIndices {
                source_index,
                mint_index,
                owner_index,
                rent_sponsor_index,
            });
        }

        // Add destination (receives compression incentive)
        let destination_index =
            packed_accounts.insert_or_get_config(self.payer_keypair.pubkey(), false, true);

        // Add authority
        let authority_index =
            packed_accounts.insert_or_get_config(compression_authority, false, true);

        // Add system accounts
        let ctoken_config = CTokenAccounts {
            compressed_token_program: compressed_token_program_id,
            cpi_authority_pda: Pubkey::find_program_address(
                &[b"cpi_authority"],
                &compressed_token_program_id,
            )
            .0,
            cpi_context: None,
            self_program: None, // Critical: None = no light_system_cpi_authority
        };

        packed_accounts
            .add_custom_system_accounts(ctoken_config)
            .map_err(|e| anyhow::anyhow!("Failed to add system accounts: {:?}", e))?;

        // Build instruction
        let (remaining_account_metas, _, _) = packed_accounts.to_account_metas();

        let registry_accounts = CompressAndCloseContext {
            authority: self.payer_keypair.pubkey(),
            registered_forester_pda,
            compression_authority,
            compressible_config,
        };

        let mut accounts = registry_accounts.to_account_metas(Some(true));
        accounts.extend(remaining_account_metas);

        let instruction = CompressAndClose {
            authority_index,
            destination_index,
            indices: indices_vec,
        };

        debug!(
            "Built compress_and_close instruction with {} accounts",
            accounts.len()
        );

        // Collect pubkeys for sync before creating instruction
        let pubkeys: Vec<_> = account_states.iter().map(|state| state.pubkey).collect();

        let ix = Instruction {
            program_id: registry_program_id,
            accounts,
            data: instruction.data(),
        };

        // Send transaction
        let signature = rpc
            .create_and_send_transaction(
                &[ix],
                &self.payer_keypair.pubkey(),
                &[&self.payer_keypair],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {}", e))?;

        // Sync accounts to verify they're closed
        if let Err(e) = self.tracker.sync_accounts(&*rpc, &pubkeys).await {
            error!("Failed to sync accounts after compression: {:?}", e);
        }

        Ok(signature)
    }
}
