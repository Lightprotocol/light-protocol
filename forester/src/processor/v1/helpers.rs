use std::{sync::Arc, time::Duration};

use account_compression::{
    processor::initialize_address_merkle_tree::Pubkey,
    utils::constants::{
        ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
        STATE_MERKLE_TREE_CHANGELOG,
    },
};
use forester_utils::{rpc_pool::SolanaRpcPool, utils::wait_for_indexer};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::TreeType;
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use solana_program::instruction::Instruction;
use tokio::time::Instant;
use tracing::{info, warn};

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

/// Work items should be of only one type and tree
pub async fn fetch_proofs_and_create_instructions<R: Rpc>(
    authority: Pubkey,
    derivation: Pubkey,
    pool: Arc<SolanaRpcPool<R>>,
    epoch: u64,
    work_items: &[WorkItem],
) -> crate::Result<(Vec<MerkleProofType>, Vec<Instruction>)> {
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

    let state_data = if !state_items.is_empty() {
        let states: Vec<[u8; 32]> = state_items
            .iter()
            .map(|item| item.queue_item_data.hash)
            .collect();
        Some(states)
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
        instructions.push(instruction);
    }

    // Process state proofs and create instructions
    if state_proofs.len() != state_items.len() {
        return Err(anyhow::anyhow!(
            "State proof count mismatch: requested={}, received={}",
            state_items.len(),
            state_proofs.len()
        ));
    }

    for (item, proof) in state_items.iter().zip(state_proofs.into_iter()) {
        proofs.push(MerkleProofType::StateProof(proof.clone()));

        let instruction = create_nullify_instruction(
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
        );
        instructions.push(instruction);
    }

    Ok((proofs, instructions))
}
