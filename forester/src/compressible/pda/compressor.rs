use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use borsh::BorshDeserialize;
use forester_utils::rpc_pool::SolanaRpcPool;
use futures::StreamExt;
use light_client::{
    indexer::Indexer,
    interface::instructions::{
        build_compress_accounts_idempotent, COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
    },
    rpc::Rpc,
};
use light_compressed_account::address::derive_address;
use light_sdk::interface::config::LightConfig;
use solana_sdk::{
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tracing::{debug, info};

use super::{state::PdaAccountTracker, types::PdaAccountState};
use crate::{
    compressible::{config::PdaProgramConfig, traits::CompressibleTracker},
    Result,
};

/// Cached program configuration to avoid repeated RPC calls
#[derive(Clone)]
pub struct CachedProgramConfig {
    pub config_pda: Pubkey,
    pub rent_sponsor: Pubkey,
    pub compression_authority: Pubkey,
    pub address_tree: Pubkey,
    pub program_metas: Vec<AccountMeta>,
}

/// Compressor for PDA accounts - builds and sends compress_accounts_idempotent transactions
/// with concurrent execution support and config caching.
pub struct PdaCompressor<R: Rpc + Indexer> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<PdaAccountTracker>,
    payer_keypair: Keypair,
}

impl<R: Rpc + Indexer> Clone for PdaCompressor<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: Arc::clone(&self.rpc_pool),
            tracker: Arc::clone(&self.tracker),
            payer_keypair: self.payer_keypair.insecure_clone(),
        }
    }
}

impl<R: Rpc + Indexer> PdaCompressor<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        tracker: Arc<PdaAccountTracker>,
        payer_keypair: Keypair,
    ) -> Self {
        Self {
            rpc_pool,
            tracker,
            payer_keypair,
        }
    }

    /// Fetch and cache the program configuration.
    /// This should be called once per program before processing accounts.
    pub async fn fetch_program_config(
        &self,
        program_config: &PdaProgramConfig,
    ) -> Result<CachedProgramConfig> {
        let program_id = &program_config.program_id;

        // Get the compressible config PDA for this program (config_bump = 0)
        let (config_pda, _) = LightConfig::derive_pda(program_id, 0);

        // Fetch the config to get rent_sponsor and address_space
        let rpc = self.rpc_pool.get_connection().await?;
        let config_account = rpc
            .get_account(config_pda)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get config account: {:?}", e))?
            .ok_or_else(|| {
                anyhow::anyhow!("Config account not found for program {}", program_id)
            })?;

        // LightConfig is stored with raw Borsh serialization (no Anchor discriminator)
        let config = LightConfig::try_from_slice(&config_account.data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize config: {:?}", e))?;

        let rent_sponsor = config.rent_sponsor;
        let compression_authority = config.compression_authority;
        let address_tree = *config
            .address_space
            .first()
            .ok_or_else(|| anyhow::anyhow!("Config has no address space"))?;

        // CompressAccountsIdempotent expects 4 accounts:
        // 1. fee_payer (signer, writable)
        // 2. config (read-only)
        // 3. rent_sponsor (writable)
        // 4. compression_authority (writable) - must match config.compression_authority
        let program_metas = vec![
            AccountMeta::new(self.payer_keypair.pubkey(), true), // fee_payer
            AccountMeta::new_readonly(config_pda, false),        // config
            AccountMeta::new(rent_sponsor, false),               // rent_sponsor
            AccountMeta::new(compression_authority, false),      // compression_authority
        ];

        Ok(CachedProgramConfig {
            config_pda,
            rent_sponsor,
            compression_authority,
            address_tree,
            program_metas,
        })
    }

    /// Compress a batch of PDA accounts with concurrent execution.
    ///
    /// Successfully compressed accounts are removed from the tracker.
    pub async fn compress_batch_concurrent(
        &self,
        account_states: &[PdaAccountState],
        program_config: &PdaProgramConfig,
        cached_config: &CachedProgramConfig,
        max_concurrent: usize,
        cancelled: Arc<AtomicBool>,
    ) -> Vec<std::result::Result<(Signature, PdaAccountState), (PdaAccountState, anyhow::Error)>>
    {
        if account_states.is_empty() {
            return Vec::new();
        }

        // Create futures for each account
        let compression_futures = account_states.iter().cloned().map(|account_state| {
            let compressor = self.clone();
            let program_config = program_config.clone();
            let cached_config = cached_config.clone();
            let cancelled = cancelled.clone();

            async move {
                // Check cancellation before processing
                if cancelled.load(Ordering::Relaxed) {
                    return Err((account_state, anyhow::anyhow!("Cancelled")));
                }

                match compressor
                    .compress(&account_state, &program_config, &cached_config)
                    .await
                {
                    Ok(sig) => Ok((sig, account_state)),
                    Err(e) => Err((account_state, e)),
                }
            }
        });

        // Execute concurrently with limit
        let results: Vec<_> = futures::stream::iter(compression_futures)
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        // Remove successfully compressed PDAs from tracker
        for (_, pda_state) in results.iter().flatten() {
            self.tracker.remove(&pda_state.pubkey);
        }

        results
    }

    /// Compress multiple PDA accounts in a single transaction.
    ///
    /// This method:
    /// 1. Fetches all compressed accounts in parallel
    /// 2. Gets a single validity proof for all accounts
    /// 3. Builds a single instruction with all accounts
    /// 4. Sends a single transaction
    ///
    /// Returns the transaction signature on success.
    pub async fn compress_batch(
        &self,
        account_states: &[PdaAccountState],
        program_config: &PdaProgramConfig,
    ) -> Result<Signature> {
        if account_states.is_empty() {
            return Err(anyhow::anyhow!("No accounts to compress"));
        }

        let program_id = &program_config.program_id;

        // Fetch and cache config
        let cached_config = self.fetch_program_config(program_config).await?;

        let mut rpc = self.rpc_pool.get_connection().await?;

        // Derive compressed addresses for all accounts
        let compressed_addresses: Vec<[u8; 32]> = account_states
            .iter()
            .map(|state| {
                derive_address(
                    &state.pubkey.to_bytes(),
                    &cached_config.address_tree.to_bytes(),
                    &program_id.to_bytes(),
                )
            })
            .collect();

        // Fetch all compressed accounts in parallel
        let compressed_account_futures = compressed_addresses.iter().map(|addr| {
            let rpc_clone = self.rpc_pool.clone();
            let addr = *addr;
            async move {
                let rpc = rpc_clone.get_connection().await?;
                rpc.get_compressed_account(addr, None)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to get compressed account: {:?}", e))?
                    .value
                    .ok_or_else(|| anyhow::anyhow!("Compressed account not found: {:?}", addr))
            }
        });

        let compressed_accounts: Vec<_> = futures::future::try_join_all(compressed_account_futures)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch compressed accounts: {:?}", e))?;

        // Collect all hashes for a single validity proof request
        let hashes: Vec<[u8; 32]> = compressed_accounts.iter().map(|acc| acc.hash).collect();

        debug!(
            "Fetching batched validity proof for {} accounts",
            hashes.len()
        );

        // Get single validity proof for all accounts
        let proof_with_context = rpc
            .get_validity_proof(hashes, vec![], None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get validity proof: {:?}", e))?
            .value;

        // Build pubkeys array
        let pubkeys: Vec<Pubkey> = account_states.iter().map(|s| s.pubkey).collect();

        // Build single batched instruction
        let ix = build_compress_accounts_idempotent(
            program_id,
            &COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &pubkeys,
            &cached_config.program_metas,
            proof_with_context,
        )
        .map_err(|e| anyhow::anyhow!("Failed to build instruction: {:?}", e))?;

        debug!(
            "Built batched compress_accounts_idempotent for {} PDAs (program {})",
            account_states.len(),
            program_id
        );

        // Send single transaction
        let signature = rpc
            .create_and_send_transaction(
                &[ix],
                &self.payer_keypair.pubkey(),
                &[&self.payer_keypair],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {:?}", e))?;

        info!(
            "Batched compress_accounts_idempotent tx for {} PDAs sent: {}",
            account_states.len(),
            signature
        );

        // Wait for confirmation before removing from tracker
        let confirmed = rpc
            .confirm_transaction(signature)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {:?}", e))?;

        if confirmed {
            // Only remove from tracker after confirmed
            for state in account_states {
                self.tracker.remove(&state.pubkey);
            }
            info!(
                "Batched compress_accounts_idempotent tx confirmed: {}",
                signature
            );
        } else {
            tracing::warn!(
                "compress_accounts_idempotent tx not confirmed: {} - accounts kept in tracker for retry",
                signature
            );
        }

        Ok(signature)
    }

    /// Compress a single PDA account using cached config
    async fn compress(
        &self,
        account_state: &PdaAccountState,
        program_config: &PdaProgramConfig,
        cached_config: &CachedProgramConfig,
    ) -> Result<Signature> {
        let program_id = &program_config.program_id;
        let pda = &account_state.pubkey;

        // Derive the compressed address
        let compressed_address = derive_address(
            &pda.to_bytes(),
            &cached_config.address_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        let mut rpc = self.rpc_pool.get_connection().await?;

        // Get the compressed account
        let compressed_account = rpc
            .get_compressed_account(compressed_address, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get compressed account: {:?}", e))?
            .value
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Compressed account not found for PDA {}. Address: {:?}",
                    pda,
                    compressed_address
                )
            })?;

        // Get validity proof
        let proof_with_context = rpc
            .get_validity_proof(vec![compressed_account.hash], vec![], None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get validity proof: {:?}", e))?
            .value;

        // Build compress_accounts_idempotent instruction
        let ix = build_compress_accounts_idempotent(
            program_id,
            &COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*pda],
            &cached_config.program_metas,
            proof_with_context,
        )
        .map_err(|e| anyhow::anyhow!("Failed to build instruction: {:?}", e))?;

        debug!(
            "Built compress_accounts_idempotent for PDA {} (program {})",
            pda, program_id
        );

        // Send transaction
        let signature = rpc
            .create_and_send_transaction(
                &[ix],
                &self.payer_keypair.pubkey(),
                &[&self.payer_keypair],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {:?}", e))?;

        info!(
            "compress_accounts_idempotent tx for PDA {} sent: {}",
            pda, signature
        );

        // Wait for confirmation
        let confirmed = rpc
            .confirm_transaction(signature)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {:?}", e))?;

        if confirmed {
            info!("compress_accounts_idempotent tx for PDA {} confirmed", pda);
            Ok(signature)
        } else {
            Err(anyhow::anyhow!(
                "Transaction {} not confirmed for PDA {}",
                signature,
                pda
            ))
        }
    }
}
