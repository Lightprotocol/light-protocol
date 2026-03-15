use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use forester_utils::{
    instructions::create_compress_and_close_mint_instruction, rpc_pool::SolanaRpcPool,
};
use futures::StreamExt;
use light_client::{indexer::Indexer, rpc::Rpc};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tracing::{debug, info};

use super::{state::MintAccountTracker, types::MintAccountState};
use crate::{
    compressible::traits::{
        send_and_confirm_with_tracking, CompressibleTracker, CompressionOutcome,
        CompressionOutcomes, CompressionTaskError,
    },
    smart_transaction::TransactionPolicy,
    Result,
};

/// Compressor for decompressed Mint accounts - builds and sends CompressAndCloseMint transactions.
pub struct MintCompressor<R: Rpc + Indexer> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    tracker: Arc<MintAccountTracker>,
    payer_keypair: Arc<Keypair>,
    transaction_policy: TransactionPolicy,
}

impl<R: Rpc + Indexer> Clone for MintCompressor<R> {
    fn clone(&self) -> Self {
        Self {
            rpc_pool: Arc::clone(&self.rpc_pool),
            tracker: Arc::clone(&self.tracker),
            payer_keypair: Arc::clone(&self.payer_keypair),
            transaction_policy: self.transaction_policy,
        }
    }
}

impl<R: Rpc + Indexer> MintCompressor<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        tracker: Arc<MintAccountTracker>,
        payer_keypair: Keypair,
        transaction_policy: TransactionPolicy,
    ) -> Self {
        Self {
            rpc_pool,
            tracker,
            payer_keypair: Arc::new(payer_keypair),
            transaction_policy,
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

                let ix = create_compress_and_close_mint_instruction(
                    &mut *rpc,
                    payer,
                    compressed_address,
                    mint_seed,
                    true, // idempotent
                )
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

        let pubkeys: Vec<Pubkey> = mint_states.iter().map(|s| s.pubkey).collect();

        let mut rpc = self.rpc_pool.get_connection().await?;
        send_and_confirm_with_tracking(
            &mut *rpc,
            &instructions,
            self.payer_keypair.as_ref(),
            self.transaction_policy,
            &*self.tracker,
            &pubkeys,
            "CompressAndCloseMint",
        )
        .await
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
    ) -> CompressionOutcomes<MintAccountState> {
        if mint_states.is_empty() {
            return Vec::new();
        }

        // Guard against max_concurrent == 0 to avoid buffer_unordered panic
        if max_concurrent == 0 {
            return mint_states
                .iter()
                .cloned()
                .map(|mint_state| CompressionOutcome::Failed {
                    state: mint_state,
                    error: CompressionTaskError::Failed(anyhow::anyhow!(
                        "max_concurrent must be > 0"
                    )),
                })
                .collect();
        }

        // Mark all as pending upfront
        let all_pubkeys: Vec<Pubkey> = mint_states.iter().map(|s| s.pubkey).collect();
        self.tracker.mark_pending(&all_pubkeys);

        // Create futures for each mint
        let compression_futures = mint_states.iter().map(|mint_state| {
            let compressor = self.clone();
            let cancelled = cancelled.clone();
            async move {
                // Check cancellation before processing
                if cancelled.load(Ordering::Relaxed) {
                    compressor.tracker.unmark_pending(&[mint_state.pubkey]);
                    return CompressionOutcome::Failed {
                        state: mint_state.clone(),
                        error: CompressionTaskError::Cancelled,
                    };
                }

                match compressor.compress(mint_state).await {
                    Ok(sig) => CompressionOutcome::Compressed {
                        signature: sig,
                        state: mint_state.clone(),
                    },
                    Err(e) => CompressionOutcome::Failed {
                        state: mint_state.clone(),
                        error: e.into(),
                    },
                }
            }
        });

        // Execute concurrently with limit
        let results: Vec<_> = futures::stream::iter(compression_futures)
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        // Remove successfully compressed mints; unmark failed ones
        for result in &results {
            match result {
                CompressionOutcome::Compressed { state, .. } => {
                    self.tracker.remove_compressed(&state.pubkey);
                }
                CompressionOutcome::Failed { state, .. } => {
                    self.tracker.unmark_pending(&[state.pubkey]);
                }
            }
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

        // Pre-check: verify the Mint PDA still exists on-chain to avoid no-op txs
        let account_info = rpc
            .get_account(*mint_pda)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to check Mint PDA {}: {:?}", mint_pda, e))?;
        if account_info.is_none() {
            debug!(
                "Mint PDA {} no longer exists on-chain, removing from tracker",
                mint_pda
            );
            self.tracker.remove(mint_pda);
            return Err(anyhow::anyhow!(
                "Mint PDA {} already closed, skipping",
                mint_pda
            ));
        }

        // Build the CompressAndCloseMint instruction
        let ix = create_compress_and_close_mint_instruction(
            &mut *rpc,
            self.payer_keypair.pubkey(),
            compressed_address,
            *mint_seed,
            true, // idempotent
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Failed to build CompressAndCloseMint instruction: {:?}", e)
        })?;

        debug!(
            "Built CompressAndCloseMint instruction for Mint {}",
            mint_pda
        );

        let tracked_pubkeys = [*mint_pda];
        let signature = send_and_confirm_with_tracking(
            &mut *rpc,
            &[ix],
            self.payer_keypair.as_ref(),
            self.transaction_policy,
            &*self.tracker,
            &tracked_pubkeys,
            "CompressAndCloseMint",
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send CompressAndCloseMint transaction: {:?}", e))?;

        info!(
            "CompressAndCloseMint tx for Mint {} confirmed: {}",
            mint_pda, signature
        );
        Ok(signature)
    }
}
