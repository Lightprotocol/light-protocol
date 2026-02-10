use std::sync::Arc;

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
use reqwest::Url;
use solana_program::instruction::Instruction;
use tokio::time::Instant;
use tracing::{info, warn};

use crate::metrics::{update_indexer_proof_count, update_indexer_response_time};

const ADDRESS_PROOF_BATCH_SIZE: usize = 100;
const ADDRESS_PROOF_MAX_RETRIES: u32 = 3;
const ADDRESS_PROOF_RETRY_BASE_DELAY_MS: u64 = 500;

use crate::{
    epoch_manager::{MerkleProofType, WorkItem},
    errors::ForesterError,
    helius_priority_fee_types::{
        GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest,
        GetPriorityFeeEstimateResponse, RpcRequest, RpcResponse,
    },
    processor::v1::config::CapConfig,
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
                "State item has unexpected tree type: {:?}",
                item.tree_account.tree_type
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
        warn!("Indexer not fully caught up, but proceeding anyway: {}", e);
    }

    let address_proofs = if let Some((merkle_tree, addresses)) = address_data {
        let total_addresses = addresses.len();
        info!(
            "Fetching {} address proofs in batches of {}",
            total_addresses, ADDRESS_PROOF_BATCH_SIZE
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
                        "Retrying address proof batch {} (attempt {}/{}), waiting {}ms",
                        batch_idx,
                        attempt + 1,
                        ADDRESS_PROOF_MAX_RETRIES + 1,
                        delay_ms
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
                            "Address proof batch {}: requested={}, received={}, duration={:.3}s{}",
                            batch_idx,
                            batch_size,
                            proofs_received,
                            batch_duration.as_secs_f64(),
                            if attempt > 0 {
                                format!(" (after {} retries)", attempt)
                            } else {
                                String::new()
                            }
                        );

                        if proofs_received != batch_size {
                            warn!(
                                "Address proof count mismatch in batch {}: requested={}, received={}",
                                batch_idx, batch_size, proofs_received
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
                    "Failed to get address proofs for batch {} after {} attempts ({:.3}s): {}",
                    batch_idx,
                    ADDRESS_PROOF_MAX_RETRIES + 1,
                    batch_duration.as_secs_f64(),
                    e
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
            "Address proofs complete: requested={}, received={}, total_duration={:.3}s",
            total_addresses,
            all_proofs.len(),
            total_duration.as_secs_f64()
        );

        update_indexer_response_time(
            "get_multiple_new_address_proofs",
            "AddressV1",
            total_duration.as_secs_f64(),
        );
        update_indexer_proof_count("AddressV1", total_addresses as u64, all_proofs.len() as u64);

        all_proofs
    } else {
        Vec::new()
    };

    let state_proofs = if let Some(states) = state_data {
        let total_states = states.len();
        info!("Fetching {} state proofs", total_states);

        let start_time = Instant::now();

        // Retry loop for transient network errors
        let mut last_error = None;
        let mut proofs = None;

        for attempt in 0..=ADDRESS_PROOF_MAX_RETRIES {
            if attempt > 0 {
                // Exponential backoff: 500ms, 1000ms, 2000ms
                let delay_ms = ADDRESS_PROOF_RETRY_BASE_DELAY_MS * (1 << (attempt - 1));
                warn!(
                    "Retrying state proofs (attempt {}/{}), waiting {}ms",
                    attempt + 1,
                    ADDRESS_PROOF_MAX_RETRIES + 1,
                    delay_ms
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
                        "State proofs complete: requested={}, received={}, duration={:.3}s{}",
                        total_states,
                        proofs_received,
                        duration.as_secs_f64(),
                        if attempt > 0 {
                            format!(" (after {} retries)", attempt)
                        } else {
                            String::new()
                        }
                    );

                    if proofs_received != total_states {
                        warn!(
                            "State proof count mismatch: requested={}, received={}",
                            total_states, proofs_received
                        );
                    }

                    update_indexer_response_time(
                        "get_multiple_compressed_account_proofs",
                        "StateV1",
                        duration.as_secs_f64(),
                    );
                    update_indexer_proof_count(
                        "StateV1",
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
                "Failed to get state proofs after {} attempts ({:.3}s): {}",
                ADDRESS_PROOF_MAX_RETRIES + 1,
                duration.as_secs_f64(),
                e
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

        let _debug = false;
        if _debug {
            let onchain_account = rpc
                .get_account(item.tree_account.merkle_tree)
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("Tree account {} not found", item.tree_account.merkle_tree)
                })?;
            let onchain_tree = match account_compression::state_merkle_tree_from_bytes_zero_copy(
                &onchain_account.data,
            ) {
                Ok(tree) => tree,
                Err(e) => {
                    tracing::error!(
                        "Failed to deserialize onchain tree {}: {}",
                        item.tree_account.merkle_tree,
                        e
                    );
                    return Err(anyhow::anyhow!("Failed to deserialize onchain tree: {}", e));
                }
            };

            let onchain_root = onchain_tree.root();
            let onchain_root_index = onchain_tree.root_index();
            let onchain_changelog_index = onchain_tree.changelog_index();

            tracing::info!(
                "Creating nullify instruction for tree {}: hash={}, leaf_index={}, root_seq={}, changelog_index={}, indexer_root={}",
                item.tree_account.merkle_tree,
                bs58::encode(&item.queue_item_data.hash).into_string(),
                proof.leaf_index,
                proof.root_seq,
                proof.root_seq % STATE_MERKLE_TREE_CHANGELOG,
                bs58::encode(&proof.root).into_string()
            );

            tracing::info!(
                "Onchain tree {} state: current_root={}, root_index={}, changelog_index={}",
                item.tree_account.merkle_tree,
                bs58::encode(&onchain_root).into_string(),
                onchain_root_index,
                onchain_changelog_index
            );

            let capacity = onchain_tree.roots.capacity();
            let first_index = onchain_tree.roots.first_index();

            let root_history: Vec<String> = onchain_tree
                .roots
                .iter()
                .enumerate()
                .map(|(offset, root)| {
                    let buffer_index = (first_index + offset) % capacity.max(1);
                    format!("#{buffer_index}: {}", bs58::encode(root).into_string())
                })
                .collect();

            tracing::info!(
                "Onchain root history (len={}, capacity={}): {:?}",
                onchain_tree.roots.len(),
                capacity,
                root_history,
            );

            let indexer_root_position =
                onchain_tree
                    .roots
                    .iter()
                    .enumerate()
                    .find_map(|(offset, root)| {
                        (root == &proof.root).then_some((first_index + offset) % capacity.max(1))
                    });

            tracing::info!(
                "Indexer root {} present_at_buffer_index={:?}",
                bs58::encode(&proof.root).into_string(),
                indexer_root_position,
            );

            if indexer_root_position.is_none() {
                return Err(anyhow::anyhow!(
                    "Indexer root {} not found in onchain root history for tree {}. Current root: {}, root_index: {}",
                    bs58::encode(&proof.root).into_string(),
                    item.tree_account.merkle_tree,
                    bs58::encode(&onchain_root).into_string(),
                    onchain_root_index
                ));
            }
        }

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

/// Request priority fee estimate from Helius RPC endpoint
pub async fn request_priority_fee_estimate(
    url: &Url,
    account_keys: Vec<Pubkey>,
) -> crate::Result<u64> {
    if url.host_str() != Some("mainnet") {
        return Ok(10_000);
    }

    let priority_fee_request = GetPriorityFeeEstimateRequest {
        transaction: None,
        account_keys: Some(
            account_keys
                .iter()
                .map(|pubkey| bs58::encode(pubkey).into_string())
                .collect(),
        ),
        options: Some(GetPriorityFeeEstimateOptions {
            include_all_priority_fee_levels: None,
            recommended: Some(true),
            include_vote: None,
            lookback_slots: None,
            priority_level: None,
            transaction_encoding: None,
        }),
    };

    let rpc_request = RpcRequest::new(
        "getPriorityFeeEstimate".to_string(),
        serde_json::json!({
            "get_priority_fee_estimate_request": priority_fee_request
        }),
    );

    let client = reqwest::Client::new();
    let response = client
        .post(url.clone())
        .header("Content-Type", "application/json")
        .json(&rpc_request)
        .send()
        .await?;

    let response_text = response.text().await?;

    let response: RpcResponse<GetPriorityFeeEstimateResponse> =
        serde_json::from_str(&response_text)?;

    response
        .result
        .priority_fee_estimate
        .map(|estimate| estimate as u64)
        .ok_or(
            ForesterError::General {
                error: "Priority fee estimate not available".to_string(),
            }
            .into(),
        )
}

/// Calculate the compute unit price in microLamports based on the target lamports and compute units
#[allow(dead_code)]
pub fn calculate_compute_unit_price(target_lamports: u64, compute_units: u64) -> u64 {
    ((target_lamports * 1_000_000) as f64 / compute_units as f64).ceil() as u64
}

/// Get a capped priority fee for transaction between min and max.
#[allow(dead_code)]
pub fn get_capped_priority_fee(cap_config: CapConfig) -> u64 {
    if cap_config.max_fee_lamports < cap_config.min_fee_lamports {
        warn!(
            "Invalid priority fee cap config: max_fee_lamports ({}) < min_fee_lamports ({}); clamping max to min",
            cap_config.max_fee_lamports, cap_config.min_fee_lamports
        );
    }
    let max_fee_lamports = cap_config.max_fee_lamports.max(cap_config.min_fee_lamports);

    let priority_fee_max =
        calculate_compute_unit_price(max_fee_lamports, cap_config.compute_unit_limit);
    let priority_fee_min =
        calculate_compute_unit_price(cap_config.min_fee_lamports, cap_config.compute_unit_limit);
    let capped_fee = std::cmp::min(cap_config.rec_fee_microlamports_per_cu, priority_fee_max);
    std::cmp::max(capped_fee, priority_fee_min)
}
