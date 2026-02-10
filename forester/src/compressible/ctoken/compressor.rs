use std::sync::Arc;

use anchor_lang::{InstructionData, ToAccountMetas};
use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_token_sdk::compressed_token::compress_and_close::CompressAndCloseAccounts as CTokenAccounts;
use light_compressible::config::CompressibleConfig;
use light_registry::{
    accounts::CompressAndCloseContext, compressible::compressed_token::CompressAndCloseIndices,
    instruction::CompressAndClose,
};
use light_sdk::instruction::PackedAccounts;
use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tracing::{debug, info};

use super::{state::CTokenAccountTracker, types::CTokenAccountState};
use crate::{
    compressible::traits::CompressibleTracker,
    Result,
};

/// Compression executor for CToken accounts via the registry program's compress_and_close instruction.
pub struct CTokenCompressor<R: Rpc + Indexer> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<CTokenAccountTracker>,
    payer_keypair: Keypair,
}

impl<R: Rpc + Indexer> Clone for CTokenCompressor<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: Arc::clone(&self.rpc_pool),
            tracker: Arc::clone(&self.tracker),
            payer_keypair: self.payer_keypair.insecure_clone(),
        }
    }
}

impl<R: Rpc + Indexer> CTokenCompressor<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        tracker: Arc<CTokenAccountTracker>,
        payer_keypair: Keypair,
    ) -> Self {
        Self {
            rpc_pool,
            tracker,
            payer_keypair,
        }
    }

    pub async fn compress_batch(
        &self,
        account_states: &[CTokenAccountState],
        registered_forester_pda: Pubkey,
    ) -> Result<Signature> {
        let registry_program_id = light_registry::ID;
        let compressed_token_program_id = Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);

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
        debug!("Registered forester PDA: {}", registered_forester_pda);

        // Get compressible config PDA
        let (compressible_config, _) =
            CompressibleConfig::derive_v1_config_pda(&registry_program_id);

        debug!("Compressible config: {}", compressible_config);

        // Get output tree from RPC
        let mut rpc = self.rpc_pool.get_connection().await?;

        // Fetch latest active state trees and get a random one
        rpc.get_latest_active_state_trees()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get state tree info: {}", e))?;
        let output_tree_info = rpc
            .get_random_state_tree_info()
            .map_err(|e| anyhow::anyhow!("Failed to get random state tree info: {}", e))?;

        let output_queue = output_tree_info
            .get_output_pubkey()
            .map_err(|e| anyhow::anyhow!("Failed to get output queue: {}", e))?;

        debug!("Output queue: {}", output_queue);

        // Build PackedAccounts
        let mut packed_accounts = PackedAccounts::default();
        packed_accounts.insert_or_get(output_queue);

        let mut indices_vec = Vec::with_capacity(account_states.len());

        for account_state in account_states {
            let source_index = packed_accounts.insert_or_get(account_state.pubkey);

            // Convert mint from light_compressed_account::Pubkey to solana_sdk::Pubkey
            let mint = Pubkey::new_from_array(account_state.account.mint.to_bytes());
            let mint_index = packed_accounts.insert_or_get(mint);

            // Get compression info from Compressible extension
            use light_token_interface::state::extensions::ExtensionStruct;
            let compression = account_state
                .account
                .extensions
                .as_ref()
                .and_then(|exts| {
                    exts.iter().find_map(|ext| match ext {
                        ExtensionStruct::Compressible(comp) => Some(&comp.info),
                        _ => None,
                    })
                })
                .ok_or_else(|| {
                    anyhow::anyhow!("Missing Compressible extension on Light Token account")
                })?;

            // Determine owner based on compress_to_pubkey flag
            let compressed_token_owner = if compression.compress_to_pubkey != 0 {
                account_state.pubkey // Use account pubkey for PDAs
            } else {
                Pubkey::new_from_array(account_state.account.owner.to_bytes()) // Use original owner
            };

            let owner_index = packed_accounts.insert_or_get(compressed_token_owner);

            // Extract rent_sponsor from compression info
            let rent_sponsor = Pubkey::new_from_array(compression.rent_sponsor);
            let rent_sponsor_index = packed_accounts.insert_or_get(rent_sponsor);

            // Handle delegate if present
            let delegate_index = account_state
                .account
                .delegate
                .map(|delegate| {
                    let delegate_pubkey = Pubkey::new_from_array(delegate.to_bytes());
                    packed_accounts.insert_or_get(delegate_pubkey)
                })
                .unwrap_or(0);

            indices_vec.push(CompressAndCloseIndices {
                source_index,
                mint_index,
                owner_index,
                rent_sponsor_index,
                delegate_index,
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

        info!(
            "compress_and_close tx with ({:?}) accounts sent {}",
            account_states.iter().map(|a| a.pubkey.to_string()),
            signature
        );

        // Wait for confirmation before removing from tracker
        let confirmed = rpc
            .confirm_transaction(signature)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {}", e))?;

        if confirmed {
            // Only remove from tracker after confirmed
            for account_state in account_states {
                self.tracker.remove(&account_state.pubkey);
            }
            info!("compress_and_close tx confirmed: {}", signature);
            Ok(signature)
        } else {
            // Transaction not confirmed - keep accounts in tracker for retry
            Err(anyhow::anyhow!(
                "compress_and_close tx not confirmed: {} - accounts kept in tracker for retry",
                signature
            ))
        }
    }
}
