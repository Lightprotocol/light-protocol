use std::{str::FromStr, sync::Arc};

use anchor_lang::{InstructionData, ToAccountMetas};
use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::{indexer::TreeInfo, rpc::Rpc};
use light_compressed_account::TreeType;
use light_compressible::config::CompressibleConfig;
use light_ctoken_interface::CTOKEN_PROGRAM_ID;
use light_ctoken_sdk::compressed_token::compress_and_close::CompressAndCloseAccounts as CTokenAccounts;
use light_registry::{
    accounts::CompressAndCloseContext, compressible::compressed_token::CompressAndCloseIndices,
    instruction::CompressAndClose,
};
use light_sdk::instruction::PackedAccounts;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tracing::{debug, info, warn};
use solana_pubkey::{pubkey, Pubkey};

use super::{state::CompressibleAccountTracker, types::CompressibleAccountState};
use crate::Result;

const REGISTRY_PROGRAM_ID_STR: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";

/// Compression executor that builds and sends compress_and_close transactions via registry program
pub struct Compressor<R: Rpc> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<CompressibleAccountTracker>,
    payer_keypair: Keypair,
}

impl<R: Rpc> Clone for Compressor<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: Arc::clone(&self.rpc_pool),
            tracker: Arc::clone(&self.tracker),
            payer_keypair: self.payer_keypair.insecure_clone(),
        }
    }
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

    pub async fn compress_batch(
        &self,
        account_states: &[CompressibleAccountState],
        registered_forester_pda: Pubkey,
    ) -> Result<Signature> {

        let registry_program_id = Pubkey::from_str(REGISTRY_PROGRAM_ID_STR)?;
        let compressed_token_program_id = Pubkey::new_from_array(CTOKEN_PROGRAM_ID);

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
        
        // FIXME: Use latest active state tree after updating lookup tables
        // rpc.get_latest_active_state_trees()
        //     .await
        //     .map_err(|e| anyhow::anyhow!("Failed to get state tree info: {}", e))?;
        // let output_tree_info = rpc
        //     .get_random_state_tree_info()
        //     .map_err(|e| anyhow::anyhow!("Failed to get state tree info: {}", e))?;

         let output_tree_info = TreeInfo {
            tree: pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
            queue: pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
            cpi_context: Some(pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y")),
            tree_type: TreeType::StateV2,
            next_tree_info: None,
        };

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

            // Get compressible extension to extract rent_sponsor and compress_to_pubkey
            let compressible_ext = account_state
                .account
                .extensions
                .as_ref()
                .and_then(|exts| {
                    exts.iter().find_map(|ext| {
                        if let light_ctoken_interface::state::ExtensionStruct::Compressible(comp) =
                            ext
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

        let ix = Instruction {
            program_id: registry_program_id,
            accounts,
            data: instruction.data(),
        };

        // Send transaction
        // Note: Account removal from tracker is handled by LogSubscriber which parses
        // the "compress_and_close:<pubkey>" logs emitted by the registry program
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


        Ok(signature)
    }
}
