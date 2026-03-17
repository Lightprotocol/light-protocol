use std::{collections::HashSet, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::rpc::Rpc;
use solana_program::hash::Hash;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokio::sync::Mutex;
use tracing::{info, trace, warn};

use crate::{
    epoch_manager::WorkItem,
    matching::{Matching, SENTINEL},
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

/// Safety margin subtracted from the Solana packet size (1232 bytes) when
/// checking whether two instructions fit in a single transaction.  This
/// accounts for any minor divergence between the size-check path and the
/// real `create_smart_transaction` path (e.g. signature encoding).
const TX_SIZE_SAFETY_MARGIN: usize = 32;

/// Maximum legacy transaction size (Solana PACKET_DATA_SIZE).
const PACKET_DATA_SIZE: usize = 1232;

/// Maximum allowed serialised transaction size for a paired batch.
const MAX_TRANSACTION_SIZE: usize = PACKET_DATA_SIZE - TX_SIZE_SAFETY_MARGIN;

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

        // Add items with a short timeout (15 seconds) for processing.
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
        let allow_pairing = if state_nullify_count >= 2 {
            self.should_attempt_pairing(last_valid_block_height, state_nullify_count)
                .await
        } else {
            false
        };
        let instruction_batches = build_instruction_batches(
            prepared_instructions,
            batch_size,
            allow_pairing,
            config.pairs_only,
            &payer.pubkey(),
            priority_fee,
            config.compute_unit_limit,
        )?;

        for instruction_chunk in instruction_batches {
            let is_paired = instruction_chunk.len() >= 2;
            let (transaction, _) = create_smart_transaction(CreateSmartTransactionConfig {
                payer: payer.insecure_clone(),
                instructions: instruction_chunk,
                recent_blockhash: *recent_blockhash,
                compute_unit_price: priority_fee,
                compute_unit_limit: config.compute_unit_limit,
                last_valid_block_height,
            })
            .await?;
            if is_paired {
                info!(
                    "Paired nullify_2 tx: sig={}, ixs=2",
                    transaction
                        .signatures
                        .first()
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                );
            }
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

// ---------------------------------------------------------------------------
// Instruction batching with optional pairing
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn build_instruction_batches(
    prepared_instructions: Vec<PreparedV1Instruction>,
    batch_size: usize,
    allow_pairing: bool,
    pairs_only: bool,
    payer: &Pubkey,
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

    // Sort by leaf_index for better proof-node overlap between neighbours.
    state_nullify_instructions.sort_by_key(|ix| ix.leaf_index);

    let paired_batches = if allow_pairing {
        pair_state_nullify_batches(
            state_nullify_instructions,
            payer,
            priority_fee,
            compute_unit_limit,
            pairs_only,
        )?
    } else if !pairs_only {
        state_nullify_instructions
            .into_iter()
            .map(|ix| vec![ix.instruction])
            .collect()
    } else {
        Vec::new()
    };
    batches.extend(paired_batches);
    Ok(batches)
}

fn pair_state_nullify_batches(
    state_nullify_instructions: Vec<StateNullifyInstruction>,
    payer: &Pubkey,
    priority_fee: Option<u64>,
    compute_unit_limit: Option<u32>,
    pairs_only: bool,
) -> Result<Vec<Vec<solana_program::instruction::Instruction>>> {
    let n = state_nullify_instructions.len();
    if n < 2 {
        if pairs_only {
            return Ok(Vec::new());
        }
        return Ok(state_nullify_instructions
            .into_iter()
            .map(|ix| vec![ix.instruction])
            .collect());
    }

    // Pre-compute compute budget instructions once for all pairs.
    let compute_budget_ixs = make_compute_budget_instructions(priority_fee, compute_unit_limit);

    // Pre-compute HashSets for O(1) overlap lookup.
    let proof_sets: Vec<HashSet<[u8; 32]>> = state_nullify_instructions
        .iter()
        .map(|ix| ix.proof_nodes.iter().copied().collect())
        .collect();
    let leaf_indices: Vec<u64> = state_nullify_instructions
        .iter()
        .map(|ix| ix.leaf_index)
        .collect();

    let mut edges: Vec<(usize, usize, i32)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if estimated_tx_size(
                payer,
                &compute_budget_ixs,
                &[
                    &state_nullify_instructions[i].instruction,
                    &state_nullify_instructions[j].instruction,
                ],
            ) > MAX_TRANSACTION_SIZE
            {
                continue;
            }
            let overlap = proof_sets[i].intersection(&proof_sets[j]).count() as i32;
            // Prioritize pair count first, then maximize proof overlap.
            let weight = 10_000 + overlap;
            edges.push((i, j, weight));
        }
    }

    if edges.is_empty() {
        if pairs_only {
            return Ok(Vec::new());
        }
        return Ok(state_nullify_instructions
            .into_iter()
            .map(|ix| vec![ix.instruction])
            .collect());
    }

    let mates = Matching::new(edges).max_cardinality().solve();

    // Move instructions into Options for zero-copy extraction.
    let mut instructions: Vec<Option<solana_program::instruction::Instruction>> =
        state_nullify_instructions
            .into_iter()
            .map(|ix| Some(ix.instruction))
            .collect();

    let mut used = vec![false; n];
    let mut paired_batches: Vec<(u64, Vec<solana_program::instruction::Instruction>)> = Vec::new();

    for i in 0..n {
        if used[i] {
            continue;
        }
        let mate = mates.get(i).copied().unwrap_or(SENTINEL);
        if mate != SENTINEL && mate > i && mate < n {
            used[i] = true;
            used[mate] = true;
            let (left, right) = if leaf_indices[i] <= leaf_indices[mate] {
                (i, mate)
            } else {
                (mate, i)
            };
            let min_leaf = leaf_indices[left];
            paired_batches.push((
                min_leaf,
                vec![
                    instructions[left].take().unwrap(),
                    instructions[right].take().unwrap(),
                ],
            ));
        }
    }

    let mut single_batches: Vec<(u64, Vec<solana_program::instruction::Instruction>)> = Vec::new();
    if !pairs_only {
        for (i, ix) in instructions.into_iter().enumerate() {
            if let Some(ix) = ix {
                single_batches.push((leaf_indices[i], vec![ix]));
            }
        }
    }

    paired_batches.sort_by_key(|(leaf, _)| *leaf);
    single_batches.sort_by_key(|(leaf, _)| *leaf);
    paired_batches.extend(single_batches);
    Ok(paired_batches.into_iter().map(|(_, batch)| batch).collect())
}

// ---------------------------------------------------------------------------
// Transaction-size estimation (zero-copy – no instruction cloning)
// ---------------------------------------------------------------------------

/// Build the compute-budget instructions that `create_smart_transaction` would
/// prepend.  Built once and reused across all pair checks.
fn make_compute_budget_instructions(
    priority_fee: Option<u64>,
    compute_unit_limit: Option<u32>,
) -> Vec<solana_program::instruction::Instruction> {
    let mut ixs = Vec::with_capacity(2);
    if let Some(price) = priority_fee {
        ixs.push(ComputeBudgetInstruction::set_compute_unit_price(price));
    }
    if let Some(limit) = compute_unit_limit {
        ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    ixs
}

/// Estimate the Solana legacy-transaction wire-format size from instruction
/// references, without cloning instructions or constructing a Transaction.
fn estimated_tx_size(
    payer: &Pubkey,
    compute_budget_ixs: &[solana_program::instruction::Instruction],
    main_ixs: &[&solana_program::instruction::Instruction],
) -> usize {
    let mut keys = HashSet::new();
    keys.insert(*payer);

    let mut signer_keys = HashSet::new();
    signer_keys.insert(*payer);

    for ix in compute_budget_ixs {
        keys.insert(ix.program_id);
        for meta in &ix.accounts {
            keys.insert(meta.pubkey);
            if meta.is_signer {
                signer_keys.insert(meta.pubkey);
            }
        }
    }
    for ix in main_ixs {
        keys.insert(ix.program_id);
        for meta in &ix.accounts {
            keys.insert(meta.pubkey);
            if meta.is_signer {
                signer_keys.insert(meta.pubkey);
            }
        }
    }

    let num_keys = keys.len();
    let num_sigs = signer_keys.len();

    // signatures section: compact-u16(count) + count * 64
    let sigs = short_vec_len(num_sigs) + num_sigs * 64;

    // message header (3 bytes)
    let header = 3;

    // account keys: compact-u16(count) + count * 32
    let key_bytes = short_vec_len(num_keys) + num_keys * 32;

    // recent_blockhash
    let blockhash = 32;

    // instructions: compact-u16(count) + each instruction
    let instruction_count = compute_budget_ixs.len() + main_ixs.len();
    let mut ixs = short_vec_len(instruction_count);
    for ix in compute_budget_ixs {
        ixs += 1; // program_id_index (u8)
        ixs += short_vec_len(ix.accounts.len()) + ix.accounts.len();
        ixs += short_vec_len(ix.data.len()) + ix.data.len();
    }
    for ix in main_ixs {
        ixs += 1;
        ixs += short_vec_len(ix.accounts.len()) + ix.accounts.len();
        ixs += short_vec_len(ix.data.len()) + ix.data.len();
    }

    sigs + header + key_bytes + blockhash + ixs
}

/// Compute the Solana legacy-transaction wire-format size from a constructed
/// Transaction.  Used in tests to verify `estimated_tx_size` correctness.
#[cfg(test)]
fn legacy_transaction_size(tx: &Transaction) -> usize {
    let msg = &tx.message;
    let num_sigs = msg.header.num_required_signatures as usize;

    let sigs = short_vec_len(num_sigs) + num_sigs * 64;
    let header = 3;
    let keys = short_vec_len(msg.account_keys.len()) + msg.account_keys.len() * 32;
    let blockhash = 32;

    let mut ixs = short_vec_len(msg.instructions.len());
    for ix in &msg.instructions {
        ixs += 1;
        ixs += short_vec_len(ix.accounts.len()) + ix.accounts.len();
        ixs += short_vec_len(ix.data.len()) + ix.data.len();
    }

    sigs + header + keys + blockhash + ixs
}

/// Length of a Solana ShortVec (compact-u16) encoding.
fn short_vec_len(val: usize) -> usize {
    if val < 0x80 {
        1
    } else if val < 0x4000 {
        2
    } else {
        3
    }
}

// ---------------------------------------------------------------------------
// Guard helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_sdk::signature::Keypair;

    use super::*;

    // -- matching tests (verify our own Blossom impl) --

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

    // -- pairing helper tests --

    #[test]
    fn pairing_candidate_count_matches_combination_formula() {
        assert_eq!(pairing_candidate_count(0), 0);
        assert_eq!(pairing_candidate_count(1), 0);
        assert_eq!(pairing_candidate_count(2), 1);
        assert_eq!(pairing_candidate_count(3), 3);
        assert_eq!(pairing_candidate_count(10), 45);
        assert_eq!(pairing_candidate_count(100), 4950);
    }

    #[test]
    fn pairing_precheck_enforces_instruction_and_candidate_limits() {
        assert!(!pairing_precheck_passes(1, pairing_candidate_count(1)));
        assert!(pairing_precheck_passes(2, pairing_candidate_count(2)));
        assert!(pairing_precheck_passes(
            MAX_PAIRING_INSTRUCTIONS,
            pairing_candidate_count(MAX_PAIRING_INSTRUCTIONS)
        ));
        assert!(!pairing_precheck_passes(
            MAX_PAIRING_INSTRUCTIONS + 1,
            pairing_candidate_count(MAX_PAIRING_INSTRUCTIONS + 1)
        ));
        assert!(!pairing_precheck_passes(90, MAX_PAIR_CANDIDATES + 1));
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

    // -- transaction size tests --

    #[test]
    fn estimated_tx_size_matches_legacy_transaction_size() {
        let payer = Keypair::new();
        let program_id = Pubkey::new_unique();
        let ix = Instruction {
            program_id,
            accounts: vec![AccountMeta::new(payer.pubkey(), true)],
            data: vec![0u8; 100],
        };
        let compute_budget_ixs = make_compute_budget_instructions(Some(1_000), Some(200_000));

        // Estimate without constructing a transaction.
        let estimated = estimated_tx_size(&payer.pubkey(), &compute_budget_ixs, &[&ix]);

        // Build the real transaction for comparison.
        let mut all_ixs = compute_budget_ixs;
        all_ixs.push(ix);
        let tx = Transaction::new_with_payer(&all_ixs, Some(&payer.pubkey()));
        let actual = legacy_transaction_size(&tx);

        assert_eq!(estimated, actual);
    }

    #[test]
    fn estimated_tx_size_with_two_instructions() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);
        let proof: Vec<[u8; 32]> = (0..16).map(shared_proof).collect();
        let ix_a = fx.make_ix(10, proof.clone());
        let ix_b = fx.make_ix(11, proof);
        let compute_budget_ixs = make_compute_budget_instructions(Some(1), Some(200_000));

        let estimated = estimated_tx_size(
            &payer.pubkey(),
            &compute_budget_ixs,
            &[&ix_a.instruction, &ix_b.instruction],
        );

        // Build the real transaction for comparison.
        let mut all_ixs = compute_budget_ixs;
        all_ixs.push(ix_a.instruction);
        all_ixs.push(ix_b.instruction);
        let tx = Transaction::new_with_payer(&all_ixs, Some(&payer.pubkey()));
        let actual = legacy_transaction_size(&tx);

        assert_eq!(estimated, actual);
        // Two nullify_2 instructions with 16 shared proof accounts should fit.
        assert!(
            estimated <= MAX_TRANSACTION_SIZE,
            "estimated={estimated} > MAX_TRANSACTION_SIZE={MAX_TRANSACTION_SIZE}"
        );
    }

    #[test]
    fn legacy_transaction_size_is_consistent() {
        let payer = Keypair::new();
        let ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(payer.pubkey(), true)],
            data: vec![0u8; 100],
        };
        let tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
        let native_size = legacy_transaction_size(&tx);
        // Sanity: a non-trivial tx should be > 200 bytes.
        assert!(native_size > 200, "native_size = {native_size}");
        // And under the packet limit.
        assert!(native_size < PACKET_DATA_SIZE);
    }

    // -- pair_state_nullify_batches integration tests --

    /// Shared test fixtures that mimic real nullify_2 instructions: same
    /// program_id, same queue, same merkle tree, differing only in proof
    /// remaining-accounts and per-leaf instruction data.
    struct TestFixture {
        program_id: Pubkey,
        merkle_tree: Pubkey,
        // Base accounts shared by every nullify_2 instruction.
        base_accounts: Vec<AccountMeta>,
    }

    impl TestFixture {
        fn new(payer: &Keypair) -> Self {
            let program_id = Pubkey::new_unique();
            let queue = Pubkey::new_unique();
            let merkle_tree = Pubkey::new_unique();

            // 8 base accounts: authority, forester_pda, registered_program,
            // queue, merkle_tree, log_wrapper, cpi_authority, acc_compression
            let base_accounts = vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new(queue, false),
                AccountMeta::new(merkle_tree, false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
            ];

            Self {
                program_id,
                merkle_tree,
                base_accounts,
            }
        }

        fn make_ix(&self, leaf_index: u64, proof_nodes: Vec<[u8; 32]>) -> StateNullifyInstruction {
            let mut accounts = self.base_accounts.clone();
            for node in &proof_nodes {
                accounts.push(AccountMeta::new_readonly(
                    Pubkey::new_from_array(*node),
                    false,
                ));
            }
            let instruction = Instruction {
                program_id: self.program_id,
                accounts,
                data: vec![0u8; 27], // 8-byte discriminator + 19-byte scalar payload
            };
            StateNullifyInstruction {
                instruction,
                proof_nodes,
                leaf_index,
                merkle_tree: self.merkle_tree,
            }
        }
    }

    fn shared_proof(prefix: u8) -> [u8; 32] {
        let mut node = [0u8; 32];
        node[0] = prefix;
        node
    }

    fn unique_proof(idx: u16) -> [u8; 32] {
        let mut node = [0xFFu8; 32];
        node[0] = (idx >> 8) as u8;
        node[1] = (idx & 0xFF) as u8;
        node
    }

    #[test]
    fn pair_state_nullify_batches_pairs_overlapping_proofs() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);

        // 4 instructions, each with exactly 16 proof nodes (realistic).
        // ix0 and ix1 share 14/16 nodes (like adjacent leaves in a tree).
        // ix2 and ix3 share 14/16 nodes (different subtree).
        let shared_0_1: Vec<[u8; 32]> = (0..14).map(shared_proof).collect();
        let shared_2_3: Vec<[u8; 32]> = (100..114).map(shared_proof).collect();

        let mut proof_0: Vec<[u8; 32]> = shared_0_1.clone();
        proof_0.extend((0..2).map(unique_proof));
        let mut proof_1: Vec<[u8; 32]> = shared_0_1;
        proof_1.extend((10..12).map(unique_proof));
        let mut proof_2: Vec<[u8; 32]> = shared_2_3.clone();
        proof_2.extend((20..22).map(unique_proof));
        let mut proof_3: Vec<[u8; 32]> = shared_2_3;
        proof_3.extend((40..42).map(unique_proof));

        let ixs = vec![
            fx.make_ix(10, proof_0),
            fx.make_ix(11, proof_1),
            fx.make_ix(50, proof_2),
            fx.make_ix(51, proof_3),
        ];

        let batches =
            pair_state_nullify_batches(ixs, &payer.pubkey(), Some(1), Some(200_000), false)
                .unwrap();

        // All 4 should be paired into 2 batches.
        assert_eq!(batches.len(), 2, "expected 2 paired batches");
        assert_eq!(batches[0].len(), 2, "first batch should have 2 ixs");
        assert_eq!(batches[1].len(), 2, "second batch should have 2 ixs");
    }

    #[test]
    fn pair_state_nullify_batches_single_instruction_no_pairs() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);

        let proof: Vec<[u8; 32]> = (0..16).map(shared_proof).collect();
        let ixs = vec![fx.make_ix(42, proof)];

        let batches =
            pair_state_nullify_batches(ixs, &payer.pubkey(), Some(1), Some(200_000), false)
                .unwrap();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
    }

    #[test]
    fn pair_state_nullify_batches_sorted_by_leaf_index() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);

        // Two instructions with identical proofs → will pair.
        let proof: Vec<[u8; 32]> = (0..16).map(shared_proof).collect();
        let ixs = vec![fx.make_ix(999, proof.clone()), fx.make_ix(1, proof)];

        let batches =
            pair_state_nullify_batches(ixs, &payer.pubkey(), Some(1), Some(200_000), false)
                .unwrap();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
    }

    #[test]
    fn pair_state_nullify_batches_no_edges_falls_back_to_singles() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);

        // Create instructions with huge data that won't fit paired in one tx.
        let make_big_ix = |leaf_index: u64| -> StateNullifyInstruction {
            let proof_nodes: Vec<[u8; 32]> = (0..16)
                .map(|i| unique_proof(leaf_index as u16 * 100 + i))
                .collect();
            let mut accounts = fx.base_accounts.clone();
            for node in &proof_nodes {
                accounts.push(AccountMeta::new_readonly(
                    Pubkey::new_from_array(*node),
                    false,
                ));
            }
            // Large data payload to force tx over size limit when paired.
            let instruction = Instruction {
                program_id: fx.program_id,
                accounts,
                data: vec![0u8; 500],
            };
            StateNullifyInstruction {
                instruction,
                proof_nodes,
                leaf_index,
                merkle_tree: fx.merkle_tree,
            }
        };

        let ixs = vec![make_big_ix(1), make_big_ix(2)];
        let batches =
            pair_state_nullify_batches(ixs, &payer.pubkey(), Some(1), Some(200_000), false)
                .unwrap();

        // Both should be singles since pairing exceeds tx size.
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[1].len(), 1);
    }

    #[test]
    fn build_instruction_batches_separates_address_and_state() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);

        let addr_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(payer.pubkey(), true)],
            data: vec![0u8; 50],
        };

        let proof: Vec<[u8; 32]> = (0..16).map(shared_proof).collect();
        let state_ix_0 = fx.make_ix(10, proof.clone());
        let state_ix_1 = fx.make_ix(11, proof);

        let prepared = vec![
            PreparedV1Instruction::AddressUpdate(addr_ix),
            PreparedV1Instruction::StateNullify(state_ix_0),
            PreparedV1Instruction::StateNullify(state_ix_1),
        ];

        let batches = build_instruction_batches(
            prepared,
            2,
            true,
            false,
            &payer.pubkey(),
            Some(1),
            Some(200_000),
        )
        .unwrap();

        // 1 address batch + 1 paired state batch.
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 1, "address batch should have 1 ix");
        assert_eq!(batches[1].len(), 2, "state batch should be paired");
    }

    #[test]
    fn build_instruction_batches_no_pairing_when_disabled() {
        let payer = Keypair::new();
        let fx = TestFixture::new(&payer);

        let proof: Vec<[u8; 32]> = (0..16).map(shared_proof).collect();
        let state_ix_0 = fx.make_ix(10, proof.clone());
        let state_ix_1 = fx.make_ix(11, proof);

        let prepared = vec![
            PreparedV1Instruction::StateNullify(state_ix_0),
            PreparedV1Instruction::StateNullify(state_ix_1),
        ];

        let batches = build_instruction_batches(
            prepared,
            2,
            false, // pairing disabled
            false,
            &payer.pubkey(),
            Some(1),
            Some(200_000),
        )
        .unwrap();

        // Each state nullify should be a separate batch.
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[1].len(), 1);
    }
}
