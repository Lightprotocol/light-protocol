use std::{sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use bincode::serialized_size;
use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::rpc::Rpc;
use mwmatching::{Matching, SENTINEL};
use solana_program::hash::Hash;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokio::sync::Mutex;
use tracing::{trace, warn};

use crate::{
    epoch_manager::WorkItem,
    processor::{
        tx_cache::ProcessedHashCache,
        v1::{
            config::BuildTransactionBatchConfig,
            helpers::{
                fetch_proofs_and_create_instructions, PreparedV1Instruction,
                StateNullifyInstruction,
            },
        },
    },
    smart_transaction::{create_smart_transaction, CreateSmartTransactionConfig},
    Result,
};

const MAX_PAIRING_INSTRUCTIONS: usize = 100;
const MAX_PAIR_CANDIDATES: usize = 4_950;
const MIN_REMAINING_BLOCKS_FOR_PAIRING: u64 = 25;

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait TransactionBuilder: Send + Sync {
    fn epoch(&self) -> u64;
    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        derivation: &Pubkey,
        recent_blockhash: &Hash,
        last_valid_block_height: u64,
        priority_fee: Option<u64>,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<(Vec<Transaction>, u64)>;
}

pub struct EpochManagerTransactions<R: Rpc> {
    pub pool: Arc<SolanaRpcPool<R>>,
    pub epoch: u64,
    pub phantom: std::marker::PhantomData<R>,
    pub processed_hash_cache: Arc<Mutex<ProcessedHashCache>>,
}

impl<R: Rpc> EpochManagerTransactions<R> {
    pub fn new(
        pool: Arc<SolanaRpcPool<R>>,
        epoch: u64,
        cache: Arc<Mutex<ProcessedHashCache>>,
    ) -> Self {
        Self {
            pool,
            epoch,
            phantom: std::marker::PhantomData,
            processed_hash_cache: cache,
        }
    }

    async fn should_attempt_pairing(
        &self,
        last_valid_block_height: u64,
        state_nullify_count: usize,
    ) -> bool {
        let pair_candidates = pairing_candidate_count(state_nullify_count);
        if !pairing_precheck_passes(state_nullify_count, pair_candidates) {
            warn!(
                "Skipping nullify pairing due to candidate explosion: count={}, pair_candidates={}",
                state_nullify_count, pair_candidates
            );
            return false;
        }

        let conn = match self.pool.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(
                    "Skipping nullify pairing because RPC connection unavailable for block-height check: {}",
                    e
                );
                return false;
            }
        };
        let current_block_height = match conn.get_block_height().await {
            Ok(height) => height,
            Err(e) => {
                warn!(
                    "Skipping nullify pairing because block-height check failed: {}",
                    e
                );
                return false;
            }
        };
        let remaining_blocks = last_valid_block_height.saturating_sub(current_block_height);
        if !remaining_blocks_allows_pairing(remaining_blocks) {
            warn!(
                "Skipping nullify pairing near blockhash expiry: remaining_blocks={}",
                remaining_blocks
            );
            return false;
        }

        true
    }
}

#[async_trait]
impl<R: Rpc> TransactionBuilder for EpochManagerTransactions<R> {
    fn epoch(&self) -> u64 {
        self.epoch
    }

    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        derivation: &Pubkey,
        recent_blockhash: &Hash,
        last_valid_block_height: u64,
        priority_fee: Option<u64>,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<(Vec<Transaction>, u64)> {
        let mut cache = self.processed_hash_cache.lock().await;

        let work_items: Vec<&WorkItem> = work_items
            .iter()
            .filter(|item| {
                let hash_str = bs58::encode(&item.queue_item_data.hash).into_string();
                if cache.contains(&hash_str) {
                    trace!("Skipping already processed hash: {}", hash_str);
                    false
                } else {
                    true
                }
            })
            .collect();

        // Add items with short timeout (30 seconds) for processing
        for item in &work_items {
            let hash_str = bs58::encode(&item.queue_item_data.hash).into_string();
            cache.add_with_timeout(&hash_str, Duration::from_secs(15));
            trace!("Added {} to cache with 15s timeout", hash_str);
        }

        let work_item_hashes: Vec<String> = work_items
            .iter()
            .map(|item| bs58::encode(&item.queue_item_data.hash).into_string())
            .collect();

        drop(cache);

        if work_items.is_empty() {
            trace!("All items in this batch were recently processed, skipping batch");
            return Ok((vec![], last_valid_block_height));
        }

        let work_items = work_items
            .iter()
            .map(|&item| item.clone())
            .collect::<Vec<_>>();

        let mut transactions = vec![];
        let prepared_instructions = match fetch_proofs_and_create_instructions(
            payer.pubkey(),
            *derivation,
            self.pool.clone(),
            self.epoch,
            work_items.as_slice(),
        )
        .await
        {
            Ok((_, instructions)) => instructions,
            Err(e) => {
                // Check if it's a "Record Not Found" error
                let err_str = e.to_string();
                return if err_str.to_lowercase().contains("record not found")
                    || err_str.to_lowercase().contains("not found")
                {
                    warn!("Record not found in indexer, skipping batch: {}", e);
                    // Return empty transactions but don't propagate the error
                    Ok((vec![], last_valid_block_height))
                } else {
                    // For any other error, propagate it
                    Err(e)
                };
            }
        };

        let batch_size = config.batch_size.max(1) as usize;
        let state_nullify_count = prepared_instructions
            .iter()
            .filter(|ix| matches!(ix, PreparedV1Instruction::StateNullify(_)))
            .count();
        let allow_pairing = if batch_size >= 2 {
            self.should_attempt_pairing(last_valid_block_height, state_nullify_count)
                .await
        } else {
            false
        };
        let instruction_batches = build_instruction_batches(
            prepared_instructions,
            batch_size,
            allow_pairing,
            payer,
            recent_blockhash,
            last_valid_block_height,
            priority_fee,
            config.compute_unit_limit,
        )?;

        for instruction_chunk in instruction_batches {
            let (transaction, _) = create_smart_transaction(CreateSmartTransactionConfig {
                payer: payer.insecure_clone(),
                instructions: instruction_chunk,
                recent_blockhash: *recent_blockhash,
                compute_unit_price: priority_fee,
                compute_unit_limit: config.compute_unit_limit,
                last_valid_block_height,
            })
            .await?;
            transactions.push(transaction);
        }

        if !transactions.is_empty() {
            let mut cache = self.processed_hash_cache.lock().await;
            for hash_str in work_item_hashes {
                cache.extend_timeout(&hash_str, Duration::from_secs(30));
                trace!(
                    "Extended cache timeout for {} to 30s after successful transaction creation",
                    hash_str
                );
            }
        }

        Ok((transactions, last_valid_block_height))
    }
}

#[allow(clippy::too_many_arguments)]
fn build_instruction_batches(
    prepared_instructions: Vec<PreparedV1Instruction>,
    batch_size: usize,
    allow_pairing: bool,
    payer: &Keypair,
    recent_blockhash: &Hash,
    last_valid_block_height: u64,
    priority_fee: Option<u64>,
    compute_unit_limit: Option<u32>,
) -> Result<Vec<Vec<solana_program::instruction::Instruction>>> {
    let mut address_instructions = Vec::new();
    let mut state_nullify_instructions = Vec::new();
    for prepared in prepared_instructions {
        match prepared {
            PreparedV1Instruction::AddressUpdate(ix) => address_instructions.push(ix),
            PreparedV1Instruction::StateNullify(ix) => state_nullify_instructions.push(ix),
        }
    }

    let mut batches = Vec::new();
    for chunk in address_instructions.chunks(batch_size) {
        batches.push(chunk.to_vec());
    }

    if state_nullify_instructions.is_empty() {
        return Ok(batches);
    }

    let paired_batches = if batch_size >= 2 && allow_pairing {
        pair_state_nullify_batches(
            state_nullify_instructions,
            payer,
            recent_blockhash,
            priority_fee,
            compute_unit_limit,
        )?
    } else {
        state_nullify_instructions
            .into_iter()
            .map(|ix| vec![ix.instruction])
            .collect()
    };
    batches.extend(paired_batches);
    Ok(batches)
}

fn pair_state_nullify_batches(
    state_nullify_instructions: Vec<StateNullifyInstruction>,
    payer: &Keypair,
    recent_blockhash: &Hash,
    priority_fee: Option<u64>,
    compute_unit_limit: Option<u32>,
) -> Result<Vec<Vec<solana_program::instruction::Instruction>>> {
    let n = state_nullify_instructions.len();
    if n < 2 {
        return Ok(state_nullify_instructions
            .into_iter()
            .map(|ix| vec![ix.instruction])
            .collect());
    }

    let mut edges: Vec<(usize, usize, i32)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if !pair_fits_transaction_size(
                &state_nullify_instructions[i].instruction,
                &state_nullify_instructions[j].instruction,
                payer,
                recent_blockhash,
                priority_fee,
                compute_unit_limit,
            )? {
                continue;
            }
            let overlap = state_nullify_instructions[i]
                .proof_nodes
                .iter()
                .filter(|node| state_nullify_instructions[j].proof_nodes.contains(node))
                .count() as i32;
            // Prioritize pair count first, then maximize proof overlap.
            let weight = 10_000 + overlap;
            edges.push((i, j, weight));
        }
    }

    if edges.is_empty() {
        return Ok(state_nullify_instructions
            .into_iter()
            .map(|ix| vec![ix.instruction])
            .collect());
    }

    let mates = Matching::new(edges).max_cardinality().solve();
    let mut used = vec![false; n];
    let mut paired_batches: Vec<(u64, Vec<solana_program::instruction::Instruction>)> = Vec::new();
    let mut single_batches: Vec<(u64, Vec<solana_program::instruction::Instruction>)> = Vec::new();

    for i in 0..n {
        if used[i] {
            continue;
        }
        let mate = mates.get(i).copied().unwrap_or(SENTINEL);
        if mate != SENTINEL && mate > i && mate < n {
            used[i] = true;
            used[mate] = true;
            let (left, right) = if state_nullify_instructions[i].leaf_index
                <= state_nullify_instructions[mate].leaf_index
            {
                (i, mate)
            } else {
                (mate, i)
            };
            let min_leaf = state_nullify_instructions[left].leaf_index;
            paired_batches.push((
                min_leaf,
                vec![
                    state_nullify_instructions[left].instruction.clone(),
                    state_nullify_instructions[right].instruction.clone(),
                ],
            ));
        }
    }

    for i in 0..n {
        if !used[i] {
            single_batches.push((
                state_nullify_instructions[i].leaf_index,
                vec![state_nullify_instructions[i].instruction.clone()],
            ));
        }
    }

    paired_batches.sort_by_key(|(leaf, _)| *leaf);
    single_batches.sort_by_key(|(leaf, _)| *leaf);
    paired_batches.extend(single_batches);
    Ok(paired_batches.into_iter().map(|(_, batch)| batch).collect())
}

fn pairing_candidate_count(n: usize) -> usize {
    n.saturating_sub(1).saturating_mul(n) / 2
}

fn pairing_precheck_passes(state_nullify_count: usize, pair_candidates: usize) -> bool {
    if state_nullify_count < 2 {
        return false;
    }
    if state_nullify_count > MAX_PAIRING_INSTRUCTIONS {
        return false;
    }
    pair_candidates <= MAX_PAIR_CANDIDATES
}

fn remaining_blocks_allows_pairing(remaining_blocks: u64) -> bool {
    remaining_blocks > MIN_REMAINING_BLOCKS_FOR_PAIRING
}

fn pair_fits_transaction_size(
    ix_a: &solana_program::instruction::Instruction,
    ix_b: &solana_program::instruction::Instruction,
    payer: &Keypair,
    recent_blockhash: &Hash,
    priority_fee: Option<u64>,
    compute_unit_limit: Option<u32>,
) -> Result<bool> {
    let mut instructions = Vec::with_capacity(
        2 + usize::from(priority_fee.is_some()) + usize::from(compute_unit_limit.is_some()),
    );
    if let Some(price) = priority_fee {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
    }
    if let Some(limit) = compute_unit_limit {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    instructions.push(ix_a.clone());
    instructions.push(ix_b.clone());

    let mut tx = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    tx.message.recent_blockhash = *recent_blockhash;
    tx.signatures = vec![
        solana_sdk::signature::Signature::default();
        tx.message.header.num_required_signatures as usize
    ];

    let tx_bytes = serialized_size(&tx)? as usize;
    Ok(tx_bytes <= 1232)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_matching_prioritizes_cardinality() {
        let edges = vec![(0usize, 1usize, 10_100i32), (1usize, 2usize, 10_090i32)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let pairs = mates
            .iter()
            .enumerate()
            .filter_map(|(i, mate)| {
                if *mate != SENTINEL && *mate > i {
                    Some((i, *mate))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn max_matching_handles_disconnected_graph() {
        let edges = vec![(0usize, 1usize, 10_010i32), (2usize, 3usize, 10_005i32)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let matched_vertices = mates.iter().filter(|mate| **mate != SENTINEL).count();
        assert_eq!(matched_vertices, 4);
    }

    #[test]
    fn max_matching_returns_unmatched_for_empty_edges() {
        let mates = Matching::new(vec![]).max_cardinality().solve();
        assert!(mates.is_empty());
    }

    #[test]
    fn pairing_candidate_count_matches_combination_formula() {
        assert_eq!(pairing_candidate_count(0), 0);
        assert_eq!(pairing_candidate_count(1), 0);
        assert_eq!(pairing_candidate_count(2), 1);
        assert_eq!(pairing_candidate_count(3), 3);
        assert_eq!(pairing_candidate_count(10), 45);
    }

    #[test]
    fn pairing_precheck_enforces_instruction_and_candidate_limits() {
        let max_count_by_candidate_limit = 100; // 100 * 99 / 2 = 4950
        assert!(!pairing_precheck_passes(1, pairing_candidate_count(1)));
        assert!(pairing_precheck_passes(2, pairing_candidate_count(2)));
        assert!(pairing_precheck_passes(
            max_count_by_candidate_limit,
            pairing_candidate_count(max_count_by_candidate_limit)
        ));
        assert!(!pairing_precheck_passes(
            max_count_by_candidate_limit + 1,
            pairing_candidate_count(max_count_by_candidate_limit + 1)
        ));
        assert!(pairing_precheck_passes(
            MAX_PAIRING_INSTRUCTIONS,
            pairing_candidate_count(MAX_PAIRING_INSTRUCTIONS)
        ));
        assert!(!pairing_precheck_passes(
            MAX_PAIRING_INSTRUCTIONS + 1,
            pairing_candidate_count(MAX_PAIRING_INSTRUCTIONS + 1)
        ));
        assert!(!pairing_precheck_passes(100, MAX_PAIR_CANDIDATES + 1));
    }

    #[test]
    fn remaining_blocks_guard_is_strictly_greater_than_threshold() {
        assert!(!remaining_blocks_allows_pairing(
            MIN_REMAINING_BLOCKS_FOR_PAIRING - 1
        ));
        assert!(!remaining_blocks_allows_pairing(
            MIN_REMAINING_BLOCKS_FOR_PAIRING
        ));
        assert!(remaining_blocks_allows_pairing(
            MIN_REMAINING_BLOCKS_FOR_PAIRING + 1
        ));
    }
}
