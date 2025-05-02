use std::sync::Arc;

use account_compression::{
    processor::initialize_address_merkle_tree::Pubkey,
    utils::constants::{
        ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
        STATE_MERKLE_TREE_CHANGELOG,
    },
};
use forester_utils::{rpc_pool::SolanaRpcPool, utils::wait_for_indexer};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::TreeType;
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use reqwest::Url;
use solana_program::instruction::Instruction;
use tokio::{join, sync::Mutex};
use tracing::warn;

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
pub async fn fetch_proofs_and_create_instructions<R: RpcConnection, I: Indexer>(
    authority: Pubkey,
    derivation: Pubkey,
    pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
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
        let states: Vec<String> = state_items
            .iter()
            .map(|item| bs58::encode(&item.queue_item_data.hash).into_string())
            .collect();
        Some(states)
    } else {
        None
    };

    let indexer_guard = indexer.lock().await;
    let mut rpc = pool.get_connection().await?;

    if let Err(e) = wait_for_indexer(&mut *rpc, &*indexer_guard).await {
        warn!("Indexer not fully caught up, but proceeding anyway: {}", e);
    }

    let (address_proofs_result, state_proofs_result) = {
        let address_future = async {
            if let Some((merkle_tree, addresses)) = address_data {
                indexer_guard
                    .get_multiple_new_address_proofs(merkle_tree, addresses)
                    .await
            } else {
                Ok(vec![])
            }
        };

        let state_future = async {
            if let Some(states) = state_data {
                indexer_guard
                    .get_multiple_compressed_account_proofs(states)
                    .await
            } else {
                Ok(vec![])
            }
        };

        join!(address_future, state_future)
    };

    let address_proofs = match address_proofs_result {
        Ok(proofs) => proofs,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to get address proofs: {}", e));
        }
    };

    let state_proofs = match state_proofs_result {
        Ok(proofs) => proofs,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to get state proofs: {}", e));
        }
    };

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
                low_address_proof: proof.low_address_proof,
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
        panic!("Max fee is less than min fee");
    }

    let priority_fee_max =
        calculate_compute_unit_price(cap_config.max_fee_lamports, cap_config.compute_unit_limit);
    let priority_fee_min =
        calculate_compute_unit_price(cap_config.min_fee_lamports, cap_config.compute_unit_limit);
    let capped_fee = std::cmp::min(cap_config.rec_fee_microlamports_per_cu, priority_fee_max);
    std::cmp::max(capped_fee, priority_fee_min)
}
