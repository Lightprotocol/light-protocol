use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::stream::{FuturesUnordered, Stream};
use futures::{FutureExt, StreamExt};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
};
use light_client::indexer::QueueElementsV2Options;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::{Hasher, Poseidon};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::get_batch_append_inputs_v2, batch_update::get_batch_update_inputs_v2,
    },
};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use crate::{
    error::ForesterUtilsError, rpc_pool::SolanaRpcPool, staging_tree::StagingTree,
    ParsedMerkleTreeData, ParsedQueueData,
};

#[derive(Debug)]
pub enum BatchInstruction {
    Append(Vec<InstructionDataBatchAppendInputs>),
    Nullify(Vec<InstructionDataBatchNullifyInputs>),
}

#[allow(clippy::too_many_arguments)]
pub async fn get_state_update_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_append_url: String,
    prover_update_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: Option<ParsedQueueData>,
    staging_tree_cache: Arc<Mutex<Option<StagingTree>>>,
) -> Result<
    (
        Pin<Box<dyn Stream<Item = Result<BatchInstruction, ForesterUtilsError>> + Send + 'a>>,
        u16,
    ),
    ForesterUtilsError,
> {
    let (merkle_tree_next_index, current_root, _) = (
        merkle_tree_data.next_index,
        merkle_tree_data.current_root,
        merkle_tree_data.root_history,
    );

    let append_hash_chains = output_queue_data
        .as_ref()
        .map(|q| q.leaves_hash_chains.clone())
        .unwrap_or_default();
    let nullify_hash_chains = merkle_tree_data.leaves_hash_chains.clone();

    let zkp_batch_size = output_queue_data
        .as_ref()
        .map(|q| q.zkp_batch_size)
        .unwrap_or(merkle_tree_data.zkp_batch_size);

    if append_hash_chains.is_empty() && nullify_hash_chains.is_empty() {
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let stream = stream! {
        let mut staging_tree: Option<StagingTree> = {
            let cache = staging_tree_cache.lock().await;
            if let Some(cached_tree) = cache.as_ref() {
                let cached_root = cached_tree.current_root();
                debug!("📍 Cache check: on-chain root={:?}[..4] cached root={:?}[..4]", &current_root[..4], &cached_root[..4]);
                debug!("✅ Cache loaded (root validation deferred to indexer fetch)");
                Some(cached_tree.clone())
            } else {
                debug!("📍 No cache found, will initialize fresh tree");
                None
            }
        };
        let mut expected_indexer_root = staging_tree
            .as_ref()
            .map(|t| t.current_root())
            .unwrap_or(current_root);

        let append_proof_client = Arc::new(ProofClient::with_config(
            prover_append_url.clone(),
            polling_interval,
            max_wait_time,
            prover_api_key.clone(),
        ));

        let nullify_proof_client = Arc::new(ProofClient::with_config(
            prover_update_url.clone(),
            polling_interval,
            max_wait_time,
            prover_api_key.clone(),
        ));

        let mut last_processed_root: Option<[u8; 32]> = None;

        // Prefetch queues once with larger limits (multiple batches).
        let (prefetched_output_queue, prefetched_input_queue) = {
            let mut connection = rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;

            let mut options = QueueElementsV2Options::default();

            if !append_hash_chains.is_empty() {
                let limit = (append_hash_chains.len() as u16).saturating_mul(zkp_batch_size);
                options = options
                    .with_output_queue(None, Some(limit))
                    .with_output_queue_batch_size(Some(zkp_batch_size));
            }

            if !nullify_hash_chains.is_empty() {
                let limit = (nullify_hash_chains.len() as u16).saturating_mul(zkp_batch_size);
                options = options
                    .with_input_queue(None, Some(limit))
                    .with_input_queue_batch_size(Some(zkp_batch_size));
            }

            let queue_elements_result = indexer
                .get_queue_elements_v2(
                    merkle_tree_pubkey.to_bytes(),
                    options,
                    None,
                )
                .await;

            match queue_elements_result {
                Ok(res) => (res.value.output_queue, res.value.input_queue),
                Err(e) => {
                    yield Err(ForesterUtilsError::Indexer(format!("Failed to prefetch queue elements: {}", e)));
                    return;
                }
            }
        };

        // Concurrent proof generation with ordered emission.
        let mut proof_tasks: FuturesUnordered<_> = FuturesUnordered::new();
        let mut buffered_results = std::collections::BTreeMap::new();
        let mut expected_seq: usize = 0;
        let total_batches = append_hash_chains.len() + nullify_hash_chains.len();

        for (batch_idx, leaves_hash_chain) in append_hash_chains.iter().enumerate() {
            let batch_data = match prefetched_output_queue.as_ref() {
                Some(data) => {
                    let expected_len = append_hash_chains.len() * zkp_batch_size as usize;
                    if data.leaf_indices.len() < expected_len || data.account_hashes.len() < expected_len || data.old_leaves.len() < expected_len {
                        warn!(
                            "Prefetched output queue has insufficient elements (have {}, need {}). Stopping APPEND phase.",
                            data.leaf_indices.len(),
                            expected_len
                        );
                        break;
                    }
                    data
                }
                None => {
                    yield Err(ForesterUtilsError::Indexer("No output queue data in V2 response".into()));
                    return;
                }
            };

            let batch_initial_root = if batch_idx == 0 {
                batch_data.initial_root
            } else {
                expected_indexer_root
            };

            debug!(
                "📍 APPEND batch {} roots: indexer initial={:?}[..4] expected={:?}[..4]",
                batch_idx,
                &batch_initial_root[..4],
                &expected_indexer_root[..4],
            );
            // Ensure indexer initial_root matches on-chain current root before processing
            let onchain_root_check = {
                let rpc = rpc_pool.get_connection().await?;
                let account = rpc
                    .get_account(merkle_tree_pubkey)
                    .await
                    .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to fetch tree account: {}", e)))?
                    .ok_or_else(|| ForesterUtilsError::Indexer("Tree account missing".into()))?;
                let mut data = account.data.clone();
                let tree = BatchedMerkleTreeAccount::state_from_bytes(&mut data, &merkle_tree_pubkey.into())
                    .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to parse tree account: {:?}", e)))?;
                tree.get_root().ok_or_else(|| ForesterUtilsError::Indexer("Tree has no root".into()))
            };
            if let Ok(onchain_root) = onchain_root_check {
                if onchain_root != batch_initial_root {
                    warn!(
                        "📍 On-chain root mismatch for APPEND batch {}: on-chain={:?}[..4] indexer initial={:?}[..4]. Skipping batch.",
                        batch_idx,
                        &onchain_root[..4],
                        &batch_initial_root[..4],
                    );
                    break;
                }
            }
            if batch_initial_root != expected_indexer_root {
                // Check if we just processed a batch and are ahead of the indexer
                if batch_idx > 0 {
                    // We've already processed at least one batch this slot, so we're ahead of indexer
                    debug!(
                        "📍 Staging tree ahead of indexer after batch {} (our root: {:?}[..4] vs indexer: {:?}[..4]). Stopping APPEND phase - indexer needs to catch up.",
                        batch_idx,
                        &expected_indexer_root[..4],
                        &batch_initial_root[..4]
                    );
                    break;
                } else if staging_tree.is_none() {
                    // First batch, no cache - trust the indexer's root to initialize
                    debug!(
                        "📍 No cache, initializing with indexer root: {:?}[..4] (on-chain root: {:?}[..4])",
                        &batch_initial_root[..4],
                        &current_root[..4]
                    );
                } else {
                    // First batch but we have a cache that disagrees with indexer - treat as invalidation: drop cache and wait for indexer to catch up.
                    warn!(
                        "📍 Cache root mismatch with indexer at batch 0! Cached: {:?}[..4], Indexer: {:?}[..4]. Dropping cache and skipping APPEND to wait for indexer.",
                        &expected_indexer_root[..4],
                        &batch_data.initial_root[..4]
                    );
                    // Check on-chain root to realign expected root for next attempt
                    if let Ok(rpc) = rpc_pool.get_connection().await {
                        if let Ok(onchain_root) = async {
                            let account = rpc
                                .get_account(merkle_tree_pubkey)
                                .await
                                .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to fetch tree account: {}", e)))?
                                .ok_or_else(|| ForesterUtilsError::Indexer("Tree account missing".into()))?;
                            let mut data = account.data.clone();
                            let tree = BatchedMerkleTreeAccount::state_from_bytes(&mut data, &merkle_tree_pubkey.into())
                                .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to parse tree account: {:?}", e)))?;
                            tree.get_root().ok_or_else(|| ForesterUtilsError::Indexer("Tree has no root".into()))
                        }.await {
                            if onchain_root != expected_indexer_root {
                                debug!(
                                    "On-chain root after mismatch: {:?}[..4], updating expected_indexer_root",
                                    &onchain_root[..4]
                                );
                                expected_indexer_root = onchain_root;
                            }
                        }
                    }
                    {
                        let mut cache = staging_tree_cache.lock().await;
                        *cache = None;
                    }
                    staging_tree = None;
                    // Stop processing this phase; retry later when indexer catches up.
                    break;
                }
            }

            if staging_tree.is_none() {
                debug!("📍  Initializing APPEND tree from indexer data:");
                debug!(" 📍   - {} leaf indices: {:?}", batch_data.leaf_indices.len(), batch_data.leaf_indices);
                debug!(" 📍   - {} old leaves (first 4 bytes): {:?}",
                    batch_data.old_leaves.len(),
                    batch_data.old_leaves.iter().map(|l| &l[..4]).collect::<Vec<_>>()
                );
                debug!("📍    - initial_root: {:?}[..4]", &batch_data.initial_root[..4]);

                match StagingTree::from_v2_output_queue(
                    &batch_data.leaf_indices,
                    &batch_data.old_leaves,
                    &batch_data.nodes,
                    &batch_data.node_hashes,
                    batch_data.initial_root,
                ) {
                    Ok(tree) => {
                        staging_tree = Some(tree);
                        debug!("📍  APPEND tree initialized successfully");
                    },
                    Err(e) => {
                        yield Err(ForesterUtilsError::Prover(format!("Failed to initialize staging tree: {}", e)));
                        return;
                    }
                }
            }

            let staging = staging_tree.as_mut().unwrap();
            if staging.current_root() != batch_initial_root {
                warn!(
                    "📍 Staging current_root mismatch after init: staging={:?}[..4] indexer_initial={:?}[..4]. Skipping APPEND batch {} to re-fetch.",
                    &staging.current_root()[..4],
                    &batch_initial_root[..4],
                    batch_idx
                );
                break;
            }
            let start = batch_idx * zkp_batch_size as usize;
            let end = start + zkp_batch_size as usize;
            let leaves: Vec<[u8; 32]> = batch_data.account_hashes[start..end].to_vec();

            let (old_leaves, merkle_proofs, old_root, new_root) = match staging.process_batch_updates(
                &batch_data.leaf_indices[start..end],
                &leaves,
                "APPEND",
                batch_idx,
            ) {
                Ok(result) => result,
                Err(e) => {
                    yield Err(ForesterUtilsError::StagingTree(format!("📍 Failed to process APPEND batch {}: {}", batch_idx, e)));
                    return;
                }
            };
            debug!(
                "📍 APPEND batch {} roots: old={:?}[..4] new={:?}[..4]",
                batch_idx,
                &old_root[..4],
                &new_root[..4],
            );
            if let Some(last_root) = last_processed_root {
                if old_root != last_root {
                    warn!(
                        "📍 Root regression detected before APPEND batch {}: expected old_root={:?}[..4], last_new_root={:?}[..4]. Skipping batch.",
                        batch_idx,
                        &old_root[..4],
                        &last_root[..4],
                    );
                    break;
                }
            }
            let adjusted_start_index = merkle_tree_next_index as u32 + (batch_idx * zkp_batch_size as usize) as u32;

            debug!("📍 Using start_index: {} (min leaf_index from batch)", adjusted_start_index);

            use light_hasher::hash_chain::create_hash_chain_from_slice;
            let indexer_hashchain = create_hash_chain_from_slice(&leaves)
                .map_err(|e| ForesterUtilsError::Prover(format!("Failed to calculate hashchain: {}", e)))?;

            debug!(
                "📍 APPEND batch {} hashchain check: on-chain={:?}[..4] computed={:?}[..4]",
                batch_idx,
                &leaves_hash_chain[..4],
                &indexer_hashchain[..4],
            );
            if indexer_hashchain != *leaves_hash_chain {
                error!(
                    "📍 Hashchain mismatch! On-chain: {:?}[..4], indexer recomputed: {:?}[..4] (batch {} indices: {:?})",
                    &leaves_hash_chain[..4],
                    &indexer_hashchain[..4],
                    batch_idx,
                    &batch_data.leaf_indices[start..end],
                );
                yield Err(ForesterUtilsError::Indexer("Hashchain mismatch between indexer and on-chain state".into()))
            }

            let circuit_inputs = match get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                adjusted_start_index,
                leaves.clone(),
                *leaves_hash_chain,
                old_leaves.clone(),
                merkle_proofs.clone(),
                zkp_batch_size as u32,
                new_root,
            ) {
                Ok(inputs) => {
                    debug!("📍 APPEND batch {} circuit inputs: old_root={:?}[..4] new_root={:?}[..4] start_index={} leaves={}",
                        batch_idx, &old_root[..4], &new_root[..4], adjusted_start_index, leaves.len());
                    inputs
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get circuit inputs: {}", e)));
                    return;
                }
            };

            expected_indexer_root = new_root;

            {
                let mut cache = staging_tree_cache.lock().await;
                *cache = staging_tree.clone();
            }

            let seq = batch_idx;
            let client = Arc::clone(&append_proof_client);
            let fut = async move {
                let res = client.generate_batch_append_proof(circuit_inputs).await
                    .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
                let (proof, new_root) = res;
                let instruction_data = InstructionDataBatchAppendInputs {
                    new_root,
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };
                debug!("📍 Generated APPEND instruction data for batch {}", batch_idx);
                Ok::<_, ForesterUtilsError>((seq, BatchInstruction::Append(vec![instruction_data])))
            };
            proof_tasks.push(fut.boxed());

            last_processed_root = Some(new_root);
        }

        for (batch_idx, leaves_hash_chain) in nullify_hash_chains.iter().enumerate() {
            let batch_data = match prefetched_input_queue.as_ref() {
                Some(data) => {
                    let expected_len = nullify_hash_chains.len() * zkp_batch_size as usize;
                    if data.leaf_indices.len() < expected_len || data.account_hashes.len() < expected_len || data.current_leaves.len() < expected_len || data.tx_hashes.len() < expected_len {
                        warn!(
                            "📍 Prefetched input queue has insufficient elements (have {}, need {}). Stopping NULLIFY phase.",
                            data.leaf_indices.len(),
                            expected_len
                        );
                        break;
                    }
                    data
                }
                None => {
                    yield Err(ForesterUtilsError::Indexer("No input queue data in V2 response".into()));
                    return;
                }
            };

            let batch_initial_root = if batch_idx == 0 {
                batch_data.initial_root
            } else {
                expected_indexer_root
            };

            debug!(
                "📍 NULLIFY batch {} roots: indexer initial={:?}[..4] expected={:?}[..4]",
                batch_idx,
                &batch_initial_root[..4],
                &expected_indexer_root[..4],
            );
            // Ensure indexer initial_root matches on-chain current root before processing
            let onchain_root_check = {
                let rpc = rpc_pool.get_connection().await?;
                let account = rpc
                    .get_account(merkle_tree_pubkey)
                    .await
                    .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to fetch tree account: {}", e)))?
                    .ok_or_else(|| ForesterUtilsError::Indexer("Tree account missing".into()))?;
                let mut data = account.data.clone();
                let tree = BatchedMerkleTreeAccount::state_from_bytes(&mut data, &merkle_tree_pubkey.into())
                    .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to parse tree account: {:?}", e)))?;
                tree.get_root().ok_or_else(|| ForesterUtilsError::Indexer("Tree has no root".into()))
            };
            if let Ok(onchain_root) = onchain_root_check {
                if onchain_root != batch_initial_root {
                    warn!(
                        "📍 On-chain root mismatch for NULLIFY batch {}: on-chain={:?}[..4] indexer initial={:?}[..4]. Skipping batch.",
                        batch_idx,
                        &onchain_root[..4],
                        &batch_initial_root[..4],
                    );
                    break;
                }
            }
            if batch_initial_root != expected_indexer_root {
                // Check if we just processed a batch and are ahead of the indexer
                if batch_idx > 0 {
                    // We've already processed at least one batch this slot, so we're ahead of indexer
                    debug!(
                        "📍 Staging tree ahead of indexer after batch {} (our root: {:?}[..4] vs indexer: {:?}[..4]). Stopping NULLIFY phase - indexer needs to catch up.",
                        batch_idx,
                        &expected_indexer_root[..4],
                        &batch_initial_root[..4]
                    );
                    break;
                } else if staging_tree.is_none() {
                    // First batch, no cache - trust the indexer's root to initialize
                    debug!(
                        "📍 No cache for NULLIFY, initializing with indexer root: {:?}[..4]",
                        &batch_initial_root[..4]
                    );
                } else {
                    // First batch but we have a cache that disagrees with indexer - treat as invalidation: drop cache and wait for indexer to catch up.
                    warn!(
                        "📍 Cache root mismatch with indexer at NULLIFY batch 0! Cached: {:?}[..4], Indexer: {:?}[..4]. Dropping cache and skipping NULLIFY to wait for indexer.",
                        &expected_indexer_root[..4],
                        &batch_data.initial_root[..4]
                    );
                    // Check on-chain root to realign expected root for next attempt
                    if let Ok(rpc) = rpc_pool.get_connection().await {
                        if let Ok(onchain_root) = async {
                            let account = rpc
                                .get_account(merkle_tree_pubkey)
                                .await
                                .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to fetch tree account: {}", e)))?
                                .ok_or_else(|| ForesterUtilsError::Indexer("Tree account missing".into()))?;
                            let mut data = account.data.clone();
                            let tree = BatchedMerkleTreeAccount::state_from_bytes(&mut data, &merkle_tree_pubkey.into())
                                .map_err(|e| ForesterUtilsError::Indexer(format!("Failed to parse tree account: {:?}", e)))?;
                            tree.get_root().ok_or_else(|| ForesterUtilsError::Indexer("Tree has no root".into()))
                        }.await {
                            debug!(
                                "📍 On-chain root after mismatch: {:?}[..4]",
                                &onchain_root[..4]
                            );
                        }
                    }
                    {
                        let mut cache = staging_tree_cache.lock().await;
                        *cache = None;
                    }
                    // Stop processing this phase; retry later when indexer catches up.
                    break;
                }
            }

            if staging_tree.is_none() {
                debug!("📍  Initializing NULLIFY tree from indexer data:");
                debug!("📍    - {} leaf indices: {:?}", batch_data.leaf_indices.len(), batch_data.leaf_indices);
                debug!("📍    - {} current leaves (first 4 bytes): {:?}",
                    batch_data.current_leaves.len(),
                    batch_data.current_leaves.iter().map(|l| &l[..4]).collect::<Vec<_>>()
                );
                debug!("📍   - initial_root: {:?}[..4]", &batch_data.initial_root[..4]);

                match StagingTree::from_v2_input_queue(
                    &batch_data.leaf_indices,
                    &batch_data.current_leaves,
                    &batch_data.nodes,
                    &batch_data.node_hashes,
                    batch_data.initial_root,
                ) {
                    Ok(tree) => {
                        staging_tree = Some(tree);
                        debug!("📍  NULLIFY tree initialized successfully");
                    },
                    Err(e) => {
                        yield Err(ForesterUtilsError::Prover(format!("Failed to initialize staging tree: {}", e)));
                        return;
                    }
                }
            }

            let staging = staging_tree.as_mut().unwrap();
            if staging.current_root() != batch_initial_root {
                warn!(
                    "📍 Staging current_root mismatch after init: staging={:?}[..4] indexer_initial={:?}[..4]. Skipping NULLIFY batch {} to re-fetch.",
                    &staging.current_root()[..4],
                    &batch_initial_root[..4],
                    batch_idx
                );
                break;
            }

            let start = batch_idx * zkp_batch_size as usize;
            let end = start + zkp_batch_size as usize;
            let account_hashes: Vec<[u8; 32]> = batch_data.account_hashes[start..end].to_vec();
            let tx_hashes = batch_data.tx_hashes[start..end].to_vec();
            let leaf_indices_slice = &batch_data.leaf_indices[start..end];
            let path_indices: Vec<u32> = leaf_indices_slice.iter().map(|&idx| idx as u32).collect();

            // Compute nullifiers: Hash(account_hash, leaf_index, tx_hash)
            let mut nullifiers = Vec::with_capacity(account_hashes.len());
            for (i, account_hash) in account_hashes.iter().enumerate() {
                let mut leaf_index_bytes = [0u8; 32];
                leaf_index_bytes[24..].copy_from_slice(&leaf_indices_slice[i].to_be_bytes());
                let nullifier = Poseidon::hashv(&[account_hash.as_slice(), &leaf_index_bytes, &tx_hashes[i]])
                    .map_err(|e| ForesterUtilsError::Prover(format!("📍 Failed to compute nullifier {}: {}", i, e)))?;
                nullifiers.push(nullifier);
            }

            let (old_leaves, merkle_proofs, old_root, new_root) = match staging.process_batch_updates(
                leaf_indices_slice,
                &nullifiers,
                "NULLIFY",
                batch_idx,
            ) {
                Ok(result) => result,
                Err(e) => {
                    yield Err(ForesterUtilsError::StagingTree(format!("📍 Failed to process NULLIFY batch {}: {}", batch_idx, e)));
                    return;
                }
            };
            debug!(
                "📍 NULLIFY batch {} roots: old={:?}[..4] new={:?}[..4]",
                batch_idx,
                &old_root[..4],
                &new_root[..4],
            );
            if let Some(last_root) = last_processed_root {
                if old_root != last_root {
                    warn!("Root regression detected before NULLIFY batch {}: expected old_root={:?}[..4], last_new_root={:?}[..4]. Skipping batch.",
                        batch_idx,
                        &old_root[..4],
                        &last_root[..4],
                    );
                    break;
                }
            }

            debug!("📍 NULLIFY batch {} old_leaves[0]={:?}[..4] nullifiers[0]={:?}[..4]",
                batch_idx, &old_leaves[0][..4], &nullifiers[0][..4]);

            // Compute hash_chain(nullifiers) - the on-chain leaves_hash_chain is computed from nullifiers
            use light_hasher::hash_chain::create_hash_chain_from_slice;
            let nullifiers_hashchain = create_hash_chain_from_slice(&nullifiers)
                .map_err(|e| ForesterUtilsError::Prover(format!("Failed to calculate nullifiers hashchain: {}", e)))?;

            debug!("📍 NULLIFY batch {} on-chain hashchain={:?}[..4] computed={:?}[..4]",
                batch_idx, &leaves_hash_chain[..4], &nullifiers_hashchain[..4]);
            if nullifiers_hashchain != *leaves_hash_chain {
                error!(
                    "📍 NULLIFY hashchain mismatch! On-chain: {:?}[..4], recomputed: {:?}[..4] (batch {} indices: {:?})",
                    &leaves_hash_chain[..4],
                    &nullifiers_hashchain[..4],
                    batch_idx,
                    leaf_indices_slice,
                );
                yield Err(ForesterUtilsError::Indexer("Nullify hashchain mismatch between indexer and on-chain state".into()));
                return;
            }

            let circuit_inputs = match get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                tx_hashes.clone(),
                account_hashes.clone(),
                nullifiers_hashchain,
                old_leaves.clone(),
                merkle_proofs.clone(),
                path_indices.clone(),
                zkp_batch_size as u32,
                new_root,
            ) {
                Ok(inputs) => {
                    debug!("📍 NULLIFY batch {} circuit inputs: old_root={:?}[..4] new_root={:?}[..4] leaves={}",
                        batch_idx, &old_root[..4], &new_root[..4], nullifiers.len());
                    inputs
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get batch update inputs: {}", e)));
                    return;
                }
            };

            expected_indexer_root = new_root;

            {
                let mut cache = staging_tree_cache.lock().await;
                *cache = staging_tree.clone();
            }

            let seq = append_hash_chains.len() + batch_idx;
            let client = Arc::clone(&nullify_proof_client);
            let fut = async move {
                let res = client.generate_batch_update_proof(circuit_inputs).await
                    .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
                let (proof, new_root) = res;
                let instruction_data = InstructionDataBatchNullifyInputs {
                    new_root,
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };
                Ok::<_, ForesterUtilsError>((seq, BatchInstruction::Nullify(vec![instruction_data])))
            };
            proof_tasks.push(fut.boxed());

            last_processed_root = Some(new_root);
        }

        // Drain proofs as they complete, but emit in order.
        while expected_seq < total_batches {
            match proof_tasks.next().await {
                Some(Ok((seq, instr))) => {
                    buffered_results.insert(seq, instr);
                    while let Some(instr) = buffered_results.remove(&expected_seq) {
                        yield Ok(instr);
                        expected_seq += 1;
                    }
                }
                Some(Err(e)) => {
                    yield Err(e);
                    return;
                }
                None => break,
            }
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}
