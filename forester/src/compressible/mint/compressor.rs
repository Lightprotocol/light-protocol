use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use forester_utils::rpc_pool::SolanaRpcPool;
use futures::StreamExt;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_test_utils::actions::legacy::instructions::mint_action::{
    create_mint_action_instruction, MintActionParams, MintActionType,
};
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tracing::{debug, info};

use super::{state::MintAccountTracker, types::MintAccountState};
use crate::{compressible::traits::CompressibleTracker, Result};

/// Compressor for decompressed Mint accounts - builds and sends CompressAndCloseMint transactions.
pub struct MintCompressor<R: Rpc + Indexer> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<MintAccountTracker>,
    payer_keypair: Keypair,
}

impl<R: Rpc + Indexer> Clone for MintCompressor<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: Arc::clone(&self.rpc_pool),
            tracker: Arc::clone(&self.tracker),
            payer_keypair: self.payer_keypair.insecure_clone(),
        }
    }
}

impl<R: Rpc + Indexer> MintCompressor<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        tracker: Arc<MintAccountTracker>,
        payer_keypair: Keypair,
    ) -> Self {
        Self {
            rpc_pool,
            tracker,
            payer_keypair,
        }
    }

    /// Compress multiple Mint accounts in a single transaction.
    pub async fn compress_batch(&self, mint_states: &[MintAccountState]) -> Result<Signature> {
        if mint_states.is_empty() {
            return Err(anyhow::anyhow!("No mints to compress"));
        }

        debug!(
            "Building {} CompressAndCloseMint instructions in parallel",
            mint_states.len()
        );

        // Build all instructions in parallel
        let instruction_futures = mint_states.iter().map(|mint_state| {
            let rpc_pool = self.rpc_pool.clone();
            let payer = self.payer_keypair.pubkey();
            let mint_seed = mint_state.mint_seed;
            let compressed_address = mint_state.compressed_address;
            let mint_pda = mint_state.pubkey;

            async move {
                let mut rpc = rpc_pool.get_connection().await?;

                let params = MintActionParams {
                    compressed_mint_address: compressed_address,
                    mint_seed,
                    authority: payer,
                    payer,
                    actions: vec![MintActionType::CompressAndCloseMint { idempotent: true }],
                    new_mint: None,
                };

                let ix = create_mint_action_instruction(&mut *rpc, params)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to build CompressAndCloseMint instruction for {}: {:?}",
                            mint_pda,
                            e
                        )
                    })?;

                Ok::<Instruction, anyhow::Error>(ix)
            }
        });

        // Wait for all instructions to be built
        let instructions: Vec<Instruction> =
            futures::future::try_join_all(instruction_futures).await?;

        debug!(
            "Built {} instructions, sending in single transaction",
            instructions.len()
        );

        // Send all instructions in a single transaction
        let mut rpc = self.rpc_pool.get_connection().await?;
        let signature = rpc
            .create_and_send_transaction(
                &instructions,
                &self.payer_keypair.pubkey(),
                &[&self.payer_keypair],
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to send batched CompressAndCloseMint transaction: {:?}",
                    e
                )
            })?;

        info!(
            "Batched CompressAndCloseMint tx for {} mints sent: {}",
            mint_states.len(),
            signature
        );

        // Wait for confirmation before removing from tracker
        let confirmed = rpc
            .confirm_transaction(signature)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {:?}", e))?;

        if confirmed {
            // Only remove from tracker after confirmed
            for mint_state in mint_states {
                self.tracker.remove(&mint_state.pubkey);
            }
            info!("Batched CompressAndCloseMint tx confirmed: {}", signature);
            Ok(signature)
        } else {
            tracing::warn!(
                "Batch CompressAndCloseMint tx not confirmed: {} - accounts kept in tracker for retry",
                signature
            );
            Err(anyhow::anyhow!(
                "Batch CompressAndCloseMint tx not confirmed: {}",
                signature
            ))
        }
    }

    /// Compress a batch of decompressed Mint accounts with concurrent execution.
    ///
    /// Each mint gets its own transaction, executed concurrently with cancellation support.
    /// Successfully compressed accounts are removed from the tracker.
    /// Use this when you need fine-grained control over individual compressions.
    pub async fn compress_batch_concurrent(
        &self,
        mint_states: &[MintAccountState],
        max_concurrent: usize,
        cancelled: Arc<AtomicBool>,
    ) -> Vec<std::result::Result<(Signature, MintAccountState), (MintAccountState, anyhow::Error)>>
    {
        if mint_states.is_empty() {
            return Vec::new();
        }

        // Guard against max_concurrent == 0 to avoid buffer_unordered panic
        if max_concurrent == 0 {
            return mint_states
                .iter()
                .cloned()
                .map(|mint_state| Err((mint_state, anyhow::anyhow!("max_concurrent must be > 0"))))
                .collect();
        }

        // Create futures for each mint
        let compression_futures = mint_states.iter().cloned().map(|mint_state| {
            let compressor = self.clone();
            let cancelled = cancelled.clone();
            async move {
                // Check cancellation before processing
                if cancelled.load(Ordering::Relaxed) {
                    return Err((mint_state, anyhow::anyhow!("Cancelled")));
                }

                match compressor.compress(&mint_state).await {
                    Ok(sig) => Ok((sig, mint_state)),
                    Err(e) => Err((mint_state, e)),
                }
            }
        });

        // Execute concurrently with limit
        let results: Vec<_> = futures::stream::iter(compression_futures)
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        // Remove successfully compressed mints from tracker
        for (_, mint_state) in results.iter().flatten() {
            self.tracker.remove(&mint_state.pubkey);
        }

        results
    }

    /// Compress a single decompressed Mint account.
    async fn compress(&self, mint_state: &MintAccountState) -> Result<Signature> {
        let mint_pda = &mint_state.pubkey;
        let mint_seed = &mint_state.mint_seed;
        let compressed_address = mint_state.compressed_address;

        debug!(
            "Compressing Mint PDA {} (seed: {}, compressed_address: {:?})",
            mint_pda, mint_seed, compressed_address
        );

        let mut rpc = self.rpc_pool.get_connection().await?;

        // Build the CompressAndCloseMint instruction using the mint action builder
        // This is idempotent - succeeds silently if mint doesn't exist or is already compressed
        let params = MintActionParams {
            compressed_mint_address: compressed_address,
            mint_seed: *mint_seed,
            authority: self.payer_keypair.pubkey(),
            payer: self.payer_keypair.pubkey(),
            actions: vec![MintActionType::CompressAndCloseMint { idempotent: true }],
            new_mint: None,
        };

        let ix = create_mint_action_instruction(&mut *rpc, params)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to build CompressAndCloseMint instruction: {:?}", e)
            })?;

        debug!(
            "Built CompressAndCloseMint instruction for Mint {}",
            mint_pda
        );

        // Send transaction
        let signature = rpc
            .create_and_send_transaction(
                &[ix],
                &self.payer_keypair.pubkey(),
                &[&self.payer_keypair],
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to send CompressAndCloseMint transaction: {:?}", e)
            })?;

        info!(
            "CompressAndCloseMint tx for Mint {} sent: {}",
            mint_pda, signature
        );

        // Wait for confirmation
        let confirmed = rpc
            .confirm_transaction(signature)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {:?}", e))?;

        if confirmed {
            info!("CompressAndCloseMint tx for Mint {} confirmed", mint_pda);
            Ok(signature)
        } else {
            Err(anyhow::anyhow!(
                "Transaction {} not confirmed for Mint {}",
                signature,
                mint_pda
            ))
        }
    }
}
