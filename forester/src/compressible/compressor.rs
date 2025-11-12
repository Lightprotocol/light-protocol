use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::rpc::Rpc;
use light_compressed_token_sdk::instructions::compress_and_close::CompressAndCloseAccounts as CTokenAccounts;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_registry::{
    accounts::CompressAndCloseContext, compressible::compressed_token::CompressAndCloseIndices,
    instruction::CompressAndClose, utils::get_forester_epoch_pda_from_authority,
};
use light_sdk::instruction::PackedAccounts;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signature::Signature,
    signer::Signer,
};
use tracing::{debug, error, info};

use forester_utils::rpc_pool::SolanaRpcPool;

use super::{state::CompressibleAccountTracker, types::CompressibleAccountState};
use crate::Result;

const REGISTRY_PROGRAM_ID_STR: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";
const BATCH_SIZE: usize = 10;
const COMPRESSION_LOOP_INTERVAL_SECS: u64 = 10;

/// Compression executor that builds and sends compress_and_close transactions via registry program
pub struct Compressor<R: Rpc> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<CompressibleAccountTracker>,
    payer_keypair: Keypair,
}

impl<R: Rpc> Compressor<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        tracker: Arc<CompressibleAccountTracker>,
        payer_keypair: Keypair,
    ) -> Self {
        Self {
            rpc_pool,
            tracker,
            payer_keypair,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting compression executor");

        loop {
            // Wait between compression attempts
            tokio::time::sleep(Duration::from_secs(COMPRESSION_LOOP_INTERVAL_SECS)).await;

            // Get all compressible accounts
            let accounts = self.tracker.get_compressible_accounts();

            if accounts.is_empty() {
                debug!("No compressible accounts found");
                continue;
            }

            info!("Found {} compressible accounts", accounts.len());

            // TODO: Check forester eligibility before compressing
            // This requires access to current epoch info from EpochManager

            // Process in batches
            for (batch_num, batch) in accounts.chunks(BATCH_SIZE).enumerate() {
                debug!(
                    "Processing batch {} with {} accounts",
                    batch_num + 1,
                    batch.len()
                );

                match self.compress_batch(batch).await {
                    Ok(sig) => {
                        info!(
                            "Successfully compressed {} accounts in batch {}: {}",
                            batch.len(),
                            batch_num + 1,
                            sig
                        );

                        // Remove successfully compressed accounts from tracker
                        for account in batch {
                            self.tracker.remove(&account.pubkey);
                        }
                    }
                    Err(e) => {
                        error!("Failed to compress batch {}: {:?}", batch_num + 1, e);
                        // Keep accounts in tracker for retry
                    }
                }
            }
        }
    }

    async fn compress_batch(&self, accounts: &[CompressibleAccountState]) -> Result<Signature> {
        // TODO: Get current epoch from EpochManager
        let current_epoch = 0u64;

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

        for account_state in accounts {
            let source_index = packed_accounts.insert_or_get(account_state.pubkey);
            let mint_index = packed_accounts.insert_or_get(account_state.mint);

            // Determine owner based on compress_to_pubkey flag
            let compressed_token_owner = if account_state.compress_to_pubkey {
                account_state.pubkey // Use account pubkey for PDAs
            } else {
                account_state.owner // Use original owner
            };

            let owner_index = packed_accounts.insert_or_get(compressed_token_owner);
            let rent_sponsor_index = packed_accounts.insert_or_get(account_state.rent_sponsor);

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

        Ok(signature)
    }
}
