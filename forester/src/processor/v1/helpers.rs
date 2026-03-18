use std::{sync::Arc, time::Duration};

use account_compression::{
    processor::initialize_address_merkle_tree::Pubkey,
    utils::constants::{
        ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
        STATE_MERKLE_TREE_CHANGELOG,
    },
};
use forester_utils::{rpc_pool::SolanaRpcPool, utils::wait_for_indexer};
use light_client::{
    indexer::{Indexer, MerkleProof},
    rpc::Rpc,
};
use light_compressed_account::TreeType;
use light_registry::account_compression_cpi::sdk::{
    compress_proofs, create_nullify_instruction, create_nullify_state_v1_multi_instruction,
    create_update_address_merkle_tree_instruction, CompressedProofs,
    CreateNullifyInstructionInputs, CreateNullifyStateV1MultiInstructionInputs,
    UpdateAddressMerkleTreeInstructionInputs,
};
use solana_program::instruction::Instruction;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::{
    logging::should_emit_rate_limited_warning,
    metrics::{update_indexer_proof_count, update_indexer_response_time},
};

const ADDRESS_PROOF_BATCH_SIZE: usize = 100;
const ADDRESS_PROOF_MAX_RETRIES: u32 = 3;
const ADDRESS_PROOF_RETRY_BASE_DELAY_MS: u64 = 500;

use crate::{
    epoch_manager::{MerkleProofType, WorkItem},
    errors::ForesterError,
};

/// A labeled instruction for logging purposes.
#[derive(Clone)]
pub struct LabeledInstruction {
    pub instruction: Instruction,
    /// Label for logging, e.g. "StateV1Nullify" or "StateV1MultiNullify(3)"
    pub label: String,
}

/// Work items should be of only one type and tree
pub async fn fetch_proofs_and_create_instructions<R: Rpc>(
    authority: Pubkey,
    derivation: Pubkey,
    pool: Arc<SolanaRpcPool<R>>,
    epoch: u64,
    work_items: &[WorkItem],
    use_multi_nullify: bool,
) -> crate::Result<(Vec<MerkleProofType>, Vec<LabeledInstruction>)> {
    let mut proofs = Vec::new();
    let mut instructions = vec![];

    let (address_items, state_items): (Vec<_>, Vec<_>) = work_items
        .iter()
        .partition(|item| matches!(item.tree_account.tree_type, TreeType::AddressV1));

    for item in state_items.iter() {
        if item.tree_account.tree_type != TreeType::StateV1 {
            warn!(
                event = "v1_state_item_unexpected_tree_type",
                tree_type = ?item.tree_account.tree_type,
                "State item has unexpected tree type"
            );
        }
    }
    let state_items = state_items
        .into_iter()
        .filter(|item| item.tree_account.tree_type == TreeType::StateV1)
        .collect::<Vec<_>>();

    let address_data = if !address_items.is_empty() {
        let merkle_tree = address_items
            .first()
            .ok_or_else(|| ForesterError::General {
                error: "No address items found".to_string(),
            })?
            .tree_account
            .merkle_tree
            .to_bytes();
        let addresses: Vec<[u8; 32]> = address_items
            .iter()
            .map(|item| item.queue_item_data.hash)
            .collect();
        Some((merkle_tree, addresses))
    } else {
        None
    };

    let rpc = pool.get_connection().await?;
    if let Err(e) = wait_for_indexer(&*rpc).await {
        if should_emit_rate_limited_warning("v1_wait_for_indexer", Duration::from_secs(30)) {
            warn!(
                event = "v1_wait_for_indexer_error",
                error = %e,
                "Indexer not fully caught up, but proceeding anyway"
            );
        }
    }

    let address_proofs = if let Some((merkle_tree, addresses)) = address_data {
        let total_addresses = addresses.len();
        info!(
            event = "v1_address_proofs_fetch_started",
            requested = total_addresses,
            batch_size = ADDRESS_PROOF_BATCH_SIZE,
            "Fetching address proofs in batches"
        );

        let start_time = Instant::now();
        let mut all_proofs = Vec::with_capacity(total_addresses);

        for (batch_idx, batch) in addresses.chunks(ADDRESS_PROOF_BATCH_SIZE).enumerate() {
            let batch_start = Instant::now();
            // Pass slice directly if indexer accepts it, otherwise clone
            let batch_addresses: Vec<[u8; 32]> = batch.to_vec();
            let batch_size = batch_addresses.len();

            // Retry loop for transient network errors
            let mut last_error = None;
            for attempt in 0..=ADDRESS_PROOF_MAX_RETRIES {
                if attempt > 0 {
                    // Exponential backoff: 500ms, 1000ms, 2000ms
                    let delay_ms = ADDRESS_PROOF_RETRY_BASE_DELAY_MS * (1 << (attempt - 1));
                    warn!(
                        event = "v1_address_proof_batch_retrying",
                        batch_index = batch_idx,
                        attempt = attempt + 1,
                        max_attempts = ADDRESS_PROOF_MAX_RETRIES + 1,
                        delay_ms,
                        "Retrying address proof batch"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }

                match rpc
                    .indexer()?
                    .get_multiple_new_address_proofs(merkle_tree, batch_addresses.clone(), None)
                    .await
                {
                    Ok(response) => {
                        let batch_duration = batch_start.elapsed();
                        let proofs_received = response.value.items.len();

                        info!(
                            event = "v1_address_proof_batch_completed",
                            batch_index = batch_idx,
                            requested = batch_size,
                            received = proofs_received,
                            duration_s = batch_duration.as_secs_f64(),
                            retries = attempt,
                            "Address proof batch completed"
                        );

                        if proofs_received != batch_size {
                            warn!(
                                event = "v1_address_proof_batch_count_mismatch",
                                batch_index = batch_idx,
                                requested = batch_size,
                                received = proofs_received,
                                "Address proof count mismatch in batch"
                            );
                        }

                        all_proofs.extend(response.value.items);
                        last_error = None;
                        break;
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }

            // If we exhausted all retries, return the last error
            if let Some(e) = last_error {
                let batch_duration = batch_start.elapsed();
                warn!(
                    event = "v1_address_proof_batch_failed",
                    batch_index = batch_idx,
                    attempts = ADDRESS_PROOF_MAX_RETRIES + 1,
                    duration_s = batch_duration.as_secs_f64(),
                    error = %e,
                    "Failed to get address proofs for batch"
                );
                return Err(anyhow::anyhow!(
                    "Failed to get address proofs for batch {} after {} retries: {}",
                    batch_idx,
                    ADDRESS_PROOF_MAX_RETRIES,
                    e
                ));
            }
        }

        let total_duration = start_time.elapsed();
        info!(
            event = "v1_address_proofs_fetch_completed",
            requested = total_addresses,
            received = all_proofs.len(),
            duration_s = total_duration.as_secs_f64(),
            "Address proofs fetch completed"
        );

        update_indexer_response_time(
            "get_multiple_new_address_proofs",
            "AddressV1",
            total_duration.as_secs_f64(),
        );
        let tree_pubkey_str = address_items
            .first()
            .map(|item| item.tree_account.merkle_tree.to_string())
            .unwrap_or_default();
        update_indexer_proof_count(
            "AddressV1",
            &tree_pubkey_str,
            total_addresses as u64,
            all_proofs.len() as u64,
        );

        all_proofs
    } else {
        Vec::new()
    };

    let state_data = if !state_items.is_empty() {
        let states: Vec<[u8; 32]> = state_items
            .iter()
            .map(|item| item.queue_item_data.hash)
            .collect();
        Some(states)
    } else {
        None
    };

    let state_proofs = if let Some(states) = state_data {
        let total_states = states.len();
        info!(
            event = "v1_state_proofs_fetch_started",
            requested = total_states,
            "Fetching state proofs"
        );

        let start_time = Instant::now();

        // Retry loop for transient network errors
        let mut last_error = None;
        let mut proofs = None;

        for attempt in 0..=ADDRESS_PROOF_MAX_RETRIES {
            if attempt > 0 {
                // Exponential backoff: 500ms, 1000ms, 2000ms
                let delay_ms = ADDRESS_PROOF_RETRY_BASE_DELAY_MS * (1 << (attempt - 1));
                warn!(
                    event = "v1_state_proofs_retrying",
                    attempt = attempt + 1,
                    max_attempts = ADDRESS_PROOF_MAX_RETRIES + 1,
                    delay_ms,
                    "Retrying state proofs"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            match rpc
                .indexer()?
                .get_multiple_compressed_account_proofs(states.clone(), None)
                .await
            {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    let proofs_received = response.value.items.len();

                    info!(
                        event = "v1_state_proofs_fetch_completed",
                        requested = total_states,
                        received = proofs_received,
                        duration_s = duration.as_secs_f64(),
                        retries = attempt,
                        "State proofs fetch completed"
                    );

                    if proofs_received != total_states {
                        warn!(
                            event = "v1_state_proof_count_mismatch",
                            requested = total_states,
                            received = proofs_received,
                            "State proof count mismatch"
                        );
                    }

                    update_indexer_response_time(
                        "get_multiple_compressed_account_proofs",
                        "StateV1",
                        duration.as_secs_f64(),
                    );
                    let state_tree_pubkey_str = state_items
                        .first()
                        .map(|item| item.tree_account.merkle_tree.to_string())
                        .unwrap_or_default();
                    update_indexer_proof_count(
                        "StateV1",
                        &state_tree_pubkey_str,
                        total_states as u64,
                        proofs_received as u64,
                    );

                    proofs = Some(response.value.items);
                    last_error = None;
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        // If we exhausted all retries, return the last error
        if let Some(e) = last_error {
            let duration = start_time.elapsed();
            warn!(
                event = "v1_state_proofs_fetch_failed",
                attempts = ADDRESS_PROOF_MAX_RETRIES + 1,
                duration_s = duration.as_secs_f64(),
                error = %e,
                "Failed to get state proofs"
            );
            return Err(anyhow::anyhow!(
                "Failed to get state proofs after {} retries: {}",
                ADDRESS_PROOF_MAX_RETRIES,
                e
            ));
        }

        proofs.unwrap_or_default()
    } else {
        Vec::new()
    };

    if address_proofs.len() != address_items.len() {
        return Err(anyhow::anyhow!(
            "Address proof count mismatch: requested={}, received={}",
            address_items.len(),
            address_proofs.len()
        ));
    }

    for (item, proof) in address_items.iter().zip(address_proofs.into_iter()) {
        proofs.push(MerkleProofType::AddressProof(proof.clone()));
        let instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority,
                derivation,
                address_merkle_tree: item.tree_account.merkle_tree,
                address_queue: item.tree_account.queue,
                value: item.queue_item_data.index as u16,
                low_address_index: proof.low_address_index,
                low_address_value: proof.low_address_value,
                low_address_next_index: proof.low_address_next_index,
                low_address_next_value: proof.low_address_next_value,
                low_address_proof: proof.low_address_proof.try_into().map_err(|_| {
                    ForesterError::General {
                        error: "Failed to convert proof to fixed array".to_string(),
                    }
                })?,
                changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
                indexed_changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG)
                    as u16,
                is_metadata_forester: false,
            },
            epoch,
        );
        instructions.push(LabeledInstruction {
            instruction,
            label: "AddressV1Update".to_string(),
        });
    }

    // Process state proofs and create instructions
    if state_proofs.len() != state_items.len() {
        return Err(anyhow::anyhow!(
            "State proof count mismatch: requested={}, received={}",
            state_items.len(),
            state_proofs.len()
        ));
    }

    let mut items_with_proofs: Vec<(&WorkItem, MerkleProof)> = state_items
        .iter()
        .zip(state_proofs.into_iter())
        .map(|(item, proof)| (*item, proof))
        .collect();

    if use_multi_nullify && items_with_proofs.len() >= 2 {
        let groups = group_state_items_for_dedup(&mut items_with_proofs);

        // Push proofs in sorted order (after grouping may have sorted)
        for (_, proof) in items_with_proofs.iter() {
            proofs.push(MerkleProofType::StateProof(proof.clone()));
        }

        let mut count_1 = 0usize;
        let mut count_2 = 0usize;
        let mut count_3 = 0usize;
        let mut count_4 = 0usize;
        for g in &groups {
            match g.len() {
                1 => count_1 += 1,
                2 => count_2 += 1,
                3 => count_3 += 1,
                4 => count_4 += 1,
                _ => {}
            }
        }
        let total_leaves = items_with_proofs.len();
        let total_instructions = groups.len();
        let dedup_savings_pct = if total_leaves > 0 {
            ((total_leaves - total_instructions) as f64 / total_leaves as f64 * 100.0) as u32
        } else {
            0
        };
        info!(
            event = "v1_nullify_state_v1_multi_grouping",
            total_leaves,
            groups_of_4 = count_4,
            groups_of_3 = count_3,
            groups_of_2 = count_2,
            singletons = count_1,
            total_instructions,
            dedup_savings_pct,
            "State nullify dedup grouping complete"
        );

        for group_indices in groups {
            if group_indices.len() == 1 {
                let (item, proof) = &items_with_proofs[group_indices[0]];
                instructions.push(LabeledInstruction {
                    instruction: build_nullify_instruction(
                        item, proof, authority, derivation, epoch,
                    ),
                    label: "StateV1Nullify".to_string(),
                });
            } else {
                let group_proofs: Vec<[[u8; 32]; 16]> = group_indices
                    .iter()
                    .map(|&idx| {
                        let proof = &items_with_proofs[idx].1.proof;
                        let arr: [[u8; 32]; 16] = proof.as_slice().try_into().map_err(|_| {
                            anyhow::anyhow!("proof has {} nodes, expected 16", proof.len())
                        })?;
                        Ok(arr)
                    })
                    .collect::<crate::Result<Vec<_>>>()?;
                let proof_refs: Vec<&[[u8; 32]; 16]> = group_proofs.iter().collect();
                let CompressedProofs {
                    proof_bitvecs,
                    nodes,
                } = compress_proofs(&proof_refs).ok_or_else(|| {
                    anyhow::anyhow!(
                        "compress_proofs failed for group that passed try_compress_group"
                    )
                })?;

                let first_item = &items_with_proofs[group_indices[0]];
                let change_log_index = (first_item.1.root_seq % STATE_MERKLE_TREE_CHANGELOG) as u16;

                let mut queue_indices = [0u16; 4];
                let mut leaf_indices = [u32::MAX; 4];
                for (slot, &idx) in group_indices.iter().enumerate() {
                    let (item, proof) = &items_with_proofs[idx];
                    queue_indices[slot] = item.queue_item_data.index as u16;
                    leaf_indices[slot] = proof.leaf_index as u32;
                }

                let node_count = nodes.len();
                let instruction = create_nullify_state_v1_multi_instruction(
                    CreateNullifyStateV1MultiInstructionInputs {
                        authority,
                        nullifier_queue: first_item.0.tree_account.queue,
                        merkle_tree: first_item.0.tree_account.merkle_tree,
                        change_log_index,
                        queue_indices,
                        leaf_indices,
                        proof_bitvecs,
                        nodes,
                        derivation,
                        is_metadata_forester: false,
                    },
                    epoch,
                );
                let group_size = group_indices.len();
                debug!(
                    event = "v1_nullify_state_v1_multi_instruction",
                    group_size,
                    node_count,
                    ix_data_bytes = instruction.data.len(),
                    "Created nullify_state_v1_multi instruction"
                );
                instructions.push(LabeledInstruction {
                    instruction,
                    label: format!("StateV1MultiNullify({})", group_size),
                });
            }
        }
    } else {
        for (_, proof) in items_with_proofs.iter() {
            proofs.push(MerkleProofType::StateProof(proof.clone()));
        }
        for (item, proof) in items_with_proofs.iter() {
            instructions.push(LabeledInstruction {
                instruction: build_nullify_instruction(item, proof, authority, derivation, epoch),
                label: "StateV1Nullify".to_string(),
            });
        }
    }

    Ok((proofs, instructions))
}

fn build_nullify_instruction(
    item: &WorkItem,
    proof: &MerkleProof,
    authority: Pubkey,
    derivation: Pubkey,
    epoch: u64,
) -> Instruction {
    create_nullify_instruction(
        CreateNullifyInstructionInputs {
            nullifier_queue: item.tree_account.queue,
            merkle_tree: item.tree_account.merkle_tree,
            change_log_indices: vec![proof.root_seq % STATE_MERKLE_TREE_CHANGELOG],
            leaves_queue_indices: vec![item.queue_item_data.index as u16],
            indices: vec![proof.leaf_index],
            proofs: vec![proof.proof.clone()],
            authority,
            derivation,
            is_metadata_forester: false,
        },
        epoch,
    )
}

/// Groups sorted (WorkItem, MerkleProof) pairs for dedup nullification.
/// Returns a vec of groups: each group is a vec of indices into `items_with_proofs`
/// that can be packed into a single nullify_state_v1_multi instruction (2-4 items),
/// or a singleton for regular nullify.
fn group_state_items_for_dedup(
    items_with_proofs: &mut [(&WorkItem, MerkleProof)],
) -> Vec<Vec<usize>> {
    items_with_proofs.sort_by_key(|(_, proof)| proof.leaf_index);

    let n = items_with_proofs.len();
    let mut groups = Vec::new();
    let mut i = 0;

    while i < n {
        if i + 4 <= n && try_compress_group(items_with_proofs, i, 4).is_some() {
            groups.push((i..i + 4).collect());
            i += 4;
        } else if i + 3 <= n && try_compress_group(items_with_proofs, i, 3).is_some() {
            groups.push((i..i + 3).collect());
            i += 3;
        } else if i + 2 <= n && try_compress_group(items_with_proofs, i, 2).is_some() {
            groups.push((i..i + 2).collect());
            i += 2;
        } else {
            groups.push(vec![i]);
            i += 1;
        }
    }

    groups
}

/// Attempt to compress a group of proofs starting at `start` with `count` items.
/// Returns the compression result if successful.
fn try_compress_group(
    items_with_proofs: &[(&WorkItem, MerkleProof)],
    start: usize,
    count: usize,
) -> Option<CompressedProofs> {
    let proof_arrays: Vec<[[u8; 32]; 16]> = (start..start + count)
        .map(|idx| items_with_proofs[idx].1.proof.as_slice().try_into().ok())
        .collect::<Option<Vec<_>>>()?;
    let refs: Vec<&[[u8; 32]; 16]> = proof_arrays.iter().collect();
    compress_proofs(&refs)
}

#[cfg(test)]
mod tests {
    use forester_utils::forester_epoch::TreeAccounts;
    use light_compressed_account::TreeType;
    use solana_sdk::pubkey::Pubkey;

    use super::*;
    use crate::queue_helpers::QueueItemData;

    fn make_work_item() -> WorkItem {
        WorkItem {
            tree_account: TreeAccounts {
                merkle_tree: Pubkey::new_unique(),
                queue: Pubkey::new_unique(),
                tree_type: TreeType::StateV1,
                is_rolledover: false,
                owner: Pubkey::new_unique(),
            },
            queue_item_data: QueueItemData {
                hash: [0u8; 32],
                index: 0,
                leaf_index: None,
            },
        }
    }

    /// Create a 16-node proof where all proofs share the same top node (index 15)
    /// but lower nodes differ unless leaves are in the same subtree.
    fn make_proof(leaf_index: u64, shared_top: [u8; 32]) -> MerkleProof {
        let mut proof = [[0u8; 32]; 16];
        // Set unique values per leaf for levels 0..15
        for (level, slot) in proof.iter_mut().enumerate().take(15) {
            let mut node = [0u8; 32];
            node[0..8].copy_from_slice(&leaf_index.to_le_bytes());
            node[8] = level as u8;
            *slot = node;
        }
        // All proofs share the same top node
        proof[15] = shared_top;
        MerkleProof {
            hash: [0u8; 32],
            leaf_index,
            merkle_tree: Pubkey::new_unique(),
            proof: proof.to_vec(),
            root_seq: 100,
            root: [0u8; 32],
        }
    }

    /// Create proofs that share sibling nodes so compress_proofs succeeds.
    /// Adjacent leaves (leaf_index differing only in low bits) share many proof nodes.
    fn make_compressible_proofs(leaf_indices: &[u64]) -> Vec<MerkleProof> {
        let shared_top = [0xFFu8; 32];
        let base_proof = {
            let mut p = [[0u8; 32]; 16];
            for (level, slot) in p.iter_mut().enumerate().take(15) {
                let mut node = [0u8; 32];
                node[0] = level as u8;
                node[1] = 0xAA;
                *slot = node;
            }
            p[15] = shared_top;
            p
        };

        leaf_indices
            .iter()
            .map(|&li| {
                // All proofs share the same nodes (maximally compressible).
                // Only the leaf_index differs.
                MerkleProof {
                    hash: [0u8; 32],
                    leaf_index: li,
                    merkle_tree: Pubkey::new_unique(),
                    proof: base_proof.to_vec(),
                    root_seq: 100,
                    root: [0u8; 32],
                }
            })
            .collect()
    }

    /// Describes expected grouping result for assertion.
    #[derive(Debug, PartialEq)]
    struct GroupingResult {
        group_sizes: Vec<usize>,
    }

    impl GroupingResult {
        fn from_groups(groups: &[Vec<usize>]) -> Self {
            Self {
                group_sizes: groups.iter().map(|g| g.len()).collect(),
            }
        }
    }

    #[test]
    fn test_group_dedup_empty() {
        let mut items: Vec<(&WorkItem, MerkleProof)> = vec![];
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![]
            },
            "Empty input should produce empty grouping"
        );
    }

    #[test]
    fn test_group_dedup_single_item() {
        let work_item = make_work_item();
        let proof = make_proof(0, [0xFFu8; 32]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = vec![(&work_item, proof)];
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![1]
            },
            "Single item should produce one singleton group"
        );
    }

    #[test]
    fn test_group_dedup_2_compressible() {
        let work_items: Vec<WorkItem> = (0..2).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[0, 1]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![2]
            },
            "2 compressible leaves should form 1 group of 2"
        );
    }

    #[test]
    fn test_group_dedup_3_compressible() {
        let work_items: Vec<WorkItem> = (0..3).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[0, 1, 2]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![3]
            },
            "3 compressible leaves should form 1 group of 3"
        );
    }

    #[test]
    fn test_group_dedup_4_compressible() {
        let work_items: Vec<WorkItem> = (0..4).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[0, 1, 2, 3]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![4]
            },
            "4 compressible leaves should form 1 group of 4"
        );
    }

    #[test]
    fn test_group_dedup_5_compressible_makes_4_plus_1() {
        let work_items: Vec<WorkItem> = (0..5).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[0, 1, 2, 3, 4]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![4, 1]
            },
            "5 compressible leaves should form group of 4 + singleton"
        );
    }

    #[test]
    fn test_group_dedup_6_compressible_makes_4_plus_2() {
        let work_items: Vec<WorkItem> = (0..6).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[0, 1, 2, 3, 4, 5]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![4, 2]
            },
            "6 compressible leaves should form group of 4 + group of 2"
        );
    }

    #[test]
    fn test_group_dedup_incompressible_becomes_singletons() {
        let shared_top = [0xFFu8; 32];
        let work_items: Vec<WorkItem> = (0..3).map(|_| make_work_item()).collect();
        // Each proof has unique nodes per leaf, so compress_proofs fails when
        // total unique nodes exceed NULLIFY_STATE_V1_MULTI_MAX_NODES (26).
        // proof_1 contributes 15 nodes; proof_2 has 15 unique => 30 total > 26.
        let proofs: Vec<MerkleProof> = (0..3).map(|i| make_proof(i * 1000, shared_top)).collect();
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        // 15 (proof1) + 15 (proof2 unique) = 30 > 28 max, so pairs fail.
        // All 3 become singletons.
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![1, 1, 1]
            },
            "Incompressible proofs (30 nodes > 28 max) should all become singletons"
        );
    }

    #[test]
    fn test_group_dedup_sorts_by_leaf_index() {
        let work_items: Vec<WorkItem> = (0..4).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[100, 3, 50, 1]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);

        let sorted_leaf_indices: Vec<u64> =
            items.iter().map(|(_, proof)| proof.leaf_index).collect();
        assert_eq!(
            sorted_leaf_indices,
            vec![1, 3, 50, 100],
            "Items should be sorted by leaf_index after grouping"
        );
        assert_eq!(
            GroupingResult::from_groups(&groups),
            GroupingResult {
                group_sizes: vec![4]
            },
            "All compressible, should form 1 group of 4"
        );
    }

    #[test]
    fn test_group_dedup_indices_reference_sorted_positions() {
        let work_items: Vec<WorkItem> = (0..4).map(|_| make_work_item()).collect();
        let proofs = make_compressible_proofs(&[0, 1, 2, 3]);
        let mut items: Vec<(&WorkItem, MerkleProof)> = work_items.iter().zip(proofs).collect();
        let groups = group_state_items_for_dedup(&mut items);
        assert_eq!(
            groups,
            vec![vec![0, 1, 2, 3]],
            "Group indices should reference positions in the sorted items array"
        );
    }
}
