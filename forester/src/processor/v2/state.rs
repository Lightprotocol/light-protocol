use std::sync::Arc;

use anyhow::anyhow;
use borsh::BorshSerialize;
use forester_utils::{error::ForesterUtilsError, ParsedMerkleTreeData, ParsedQueueData};
use futures::future::join_all;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs},
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::{get_batch_append_inputs, BatchAppendsCircuitInputs},
        batch_update::{get_batch_update_inputs, BatchUpdateCircuitInputs},
    },
};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use solana_sdk::signer::Signer;
use tracing::{debug, info, instrument};

use super::{
    changelog_cache, context::BatchContext, types::StateConfig, utils::send_transaction_batch,
};
use crate::Result;

#[instrument(
    level = "debug", 
    skip(context, merkle_tree_data, output_queue_data),
    fields(merkle_tree = ?context.merkle_tree)
)]
pub(crate) async fn generate_state_inputs<R: Rpc>(
    context: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
) -> Result<(
    Vec<InstructionDataBatchNullifyInputs>,
    Vec<InstructionDataBatchAppendInputs>,
)> {
    info!("Preparing proofs with sequential changelog calculation and parallel proof generation");

    let state_config = StateConfig {
        rpc_pool: context.rpc_pool.clone(),
        merkle_tree_pubkey: context.merkle_tree,
        output_queue_pubkey: context.output_queue,
        nullify_prover_url: context.config.prover_update_url.clone(),
        append_prover_url: context.config.prover_append_url.clone(),
        prover_api_key: context.config.prover_api_key.clone(),
        polling_interval: context.config.prover_polling_interval,
        max_wait_time: context.config.prover_max_wait_time,
    };

    generate_proofs_with_changelogs(state_config, merkle_tree_data, output_queue_data).await
}

/// Submit nullify transactions with pre-generated proofs
/// Each proof is sent in a separate transaction to handle root updates properly
#[instrument(
    level = "debug",
    skip(context, proofs),
    fields(merkle_tree = ?context.merkle_tree)
)]
pub(crate) async fn submit_nullify_transaction<R: Rpc>(
    context: &BatchContext<R>,
    proofs: Vec<InstructionDataBatchNullifyInputs>,
) -> Result<()> {
    if proofs.is_empty() {
        return Ok(());
    }

    // Send each proof in a separate transaction
    for (i, data) in proofs.iter().enumerate() {
        debug!("Submitting nullify proof {}/{}", i + 1, proofs.len());

        let instruction = create_batch_nullify_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.epoch,
            data.try_to_vec()?,
        );

        send_transaction_batch(context, vec![instruction]).await?;

        // Wait for indexer to catch up before sending next transaction
        if i < proofs.len() - 1 {
            let rpc = context.rpc_pool.get_connection().await?;
            forester_utils::utils::wait_for_indexer(&*rpc)
                .await
                .map_err(|e| anyhow!("Indexer wait error: {:?}", e))?;
        }
    }

    Ok(())
}

/// Submit append transactions with pre-generated proofs
/// Each proof is sent in a separate transaction to handle root updates properly
#[instrument(
    level = "debug",
    skip(context, proofs),
    fields(merkle_tree = ?context.merkle_tree)
)]
pub(crate) async fn submit_append_transaction<R: Rpc>(
    context: &BatchContext<R>,
    proofs: Vec<InstructionDataBatchAppendInputs>,
) -> Result<()> {
    if proofs.is_empty() {
        return Ok(());
    }

    // Send each proof in a separate transaction
    for (i, data) in proofs.iter().enumerate() {
        debug!("Submitting append proof {}/{}", i + 1, proofs.len());

        let instruction = create_batch_append_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.output_queue,
            context.epoch,
            data.try_to_vec()?,
        );

        send_transaction_batch(context, vec![instruction]).await?;

        // Wait for indexer to catch up before sending next transaction
        if i < proofs.len() - 1 {
            let rpc = context.rpc_pool.get_connection().await?;
            forester_utils::utils::wait_for_indexer(&*rpc)
                .await
                .map_err(|e| anyhow!("Indexer wait error: {:?}", e))?;
        }
    }

    Ok(())
}

async fn generate_proofs_with_changelogs<R: Rpc>(
    config: StateConfig<R>,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
) -> Result<(
    Vec<InstructionDataBatchNullifyInputs>,
    Vec<InstructionDataBatchAppendInputs>,
)> {
    info!("Preparing proofs with optimized parallel generation");

    let nullify_zkp_batch_size = merkle_tree_data.zkp_batch_size;
    let append_zkp_batch_size = output_queue_data.zkp_batch_size;
    let nullify_leaves_hash_chains = merkle_tree_data.leaves_hash_chains.clone();
    let append_leaves_hash_chains = output_queue_data.leaves_hash_chains.clone();

    // Early return if nothing to process
    if nullify_leaves_hash_chains.is_empty() && append_leaves_hash_chains.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Step 1: Fetch queue elements in parallel for both operations
    let (nullify_elements, append_elements) = {
        let nullify_future = async {
            if nullify_leaves_hash_chains.is_empty() {
                return Ok(Vec::new());
            }
            let mut connection = config.rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            let total_elements = nullify_zkp_batch_size as usize * nullify_leaves_hash_chains.len();
            let offset = merkle_tree_data.num_inserted_zkps * nullify_zkp_batch_size as u64;

            let res = indexer
                .get_queue_elements(
                    config.merkle_tree_pubkey.to_bytes(),
                    QueueType::InputStateV2,
                    total_elements as u16,
                    Some(offset),
                    None,
                )
                .await?;
            Ok::<_, anyhow::Error>(res.value.0)
        };

        let append_future = async {
            if append_leaves_hash_chains.is_empty() {
                return Ok(Vec::new());
            }
            let mut connection = config.rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            let total_elements = append_zkp_batch_size as usize * append_leaves_hash_chains.len();
            let offset = merkle_tree_data.next_index;

            let res = indexer
                .get_queue_elements(
                    config.merkle_tree_pubkey.to_bytes(),
                    QueueType::OutputStateV2,
                    total_elements as u16,
                    Some(offset),
                    None,
                )
                .await?;
            Ok::<_, anyhow::Error>(res.value.0)
        };

        futures::join!(nullify_future, append_future)
    };

    let nullify_queue_elements = nullify_elements?;
    let append_queue_elements = append_elements?;

    // Step 2: Get cached changelogs
    let changelog_cache = changelog_cache::get_changelog_cache().await;
    let previous_changelogs = changelog_cache
        .get_changelogs(&config.merkle_tree_pubkey)
        .await;
    info!(
        "Starting with {} cached changelogs",
        previous_changelogs.len()
    );

    // Step 3: Calculate nullify changelogs first (sequential)
    let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> =
        previous_changelogs.clone();
    let mut nullify_circuit_inputs = Vec::new();
    let mut current_root = merkle_tree_data.current_root;

    for (batch_offset, leaves_hash_chain) in nullify_leaves_hash_chains.iter().enumerate() {
        let start_idx = batch_offset * nullify_zkp_batch_size as usize;
        let end_idx = start_idx + nullify_zkp_batch_size as usize;
        let batch_elements = &nullify_queue_elements[start_idx..end_idx];

        let mut leaves = Vec::new();
        let mut tx_hashes = Vec::new();
        let mut old_leaves = Vec::new();
        let mut path_indices = Vec::new();
        let mut merkle_proofs = Vec::new();

        for leaf_info in batch_elements.iter() {
            path_indices.push(leaf_info.leaf_index as u32);
            leaves.push(leaf_info.account_hash);
            old_leaves.push(leaf_info.leaf);
            merkle_proofs.push(leaf_info.proof.clone());
            tx_hashes.push(leaf_info.tx_hash.ok_or_else(|| {
                anyhow!("Missing tx_hash for leaf index {}", leaf_info.leaf_index)
            })?);
        }

        let (circuit_inputs, batch_changelog) =
            get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                current_root, // Use the current root, which gets updated after each batch
                tx_hashes,
                leaves,
                *leaves_hash_chain,
                old_leaves,
                merkle_proofs,
                path_indices,
                nullify_zkp_batch_size as u32,
                &all_changelogs, // Use accumulated changelogs
            )?;

        // Update current_root to the new root from this batch for the next iteration
        let new_root_bytes = circuit_inputs.new_root.to_bytes_be().1;
        if new_root_bytes.len() == 32 {
            current_root.copy_from_slice(&new_root_bytes);
            debug!(
                "Updated root after nullify batch {}: {:?}",
                batch_offset, current_root
            );
        } else {
            // Pad or truncate to 32 bytes if necessary
            current_root = [0u8; 32];
            let offset = 32usize.saturating_sub(new_root_bytes.len());
            current_root[offset..].copy_from_slice(&new_root_bytes[..new_root_bytes.len().min(32)]);
            debug!(
                "Updated root after nullify batch {} (padded): {:?}",
                batch_offset, current_root
            );
        }

        all_changelogs.extend(batch_changelog);
        nullify_circuit_inputs.push(circuit_inputs);
    }

    info!(
        "Calculated {} nullify changelogs",
        all_changelogs.len() - previous_changelogs.len()
    );

    // Step 4: Calculate append inputs with nullifies changelogs
    // Continue using the current_root from where nullify left off
    let mut append_circuit_inputs = Vec::new();

    for (batch_idx, leaves_hash_chain) in append_leaves_hash_chains.iter().enumerate() {
        let start_idx = batch_idx * append_zkp_batch_size as usize;
        let end_idx = start_idx + append_zkp_batch_size as usize;
        let batch_elements = &append_queue_elements[start_idx..end_idx];

        let new_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
        let merkle_proofs: Vec<Vec<[u8; 32]>> =
            batch_elements.iter().map(|x| x.proof.clone()).collect();
        let adjusted_start_index = merkle_tree_data.next_index as u32
            + (batch_idx * append_zkp_batch_size as usize) as u32;
        let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();

        let (circuit_inputs, batch_changelog) =
            get_batch_append_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                current_root, // Use the current root, which was updated by nullify operations
                adjusted_start_index,
                new_leaves,
                *leaves_hash_chain,
                old_leaves,
                merkle_proofs,
                append_zkp_batch_size as u32,
                &all_changelogs, // Use changelogs including nullify's
            )?;

        // Update current_root for the next append batch
        let new_root_bytes = circuit_inputs.new_root.to_bytes_be().1;
        if new_root_bytes.len() == 32 {
            current_root.copy_from_slice(&new_root_bytes);
            debug!(
                "Updated root after append batch {}: {:?}",
                batch_idx, current_root
            );
        } else {
            // Pad or truncate to 32 bytes if necessary
            current_root = [0u8; 32];
            let offset = 32usize.saturating_sub(new_root_bytes.len());
            current_root[offset..].copy_from_slice(&new_root_bytes[..new_root_bytes.len().min(32)]);
            debug!(
                "Updated root after append batch {} (padded): {:?}",
                batch_idx, current_root
            );
        }

        all_changelogs.extend(batch_changelog);
        append_circuit_inputs.push(circuit_inputs);
    }

    info!(
        "Calculated {} append changelogs",
        all_changelogs.len() - previous_changelogs.len() - nullify_circuit_inputs.len()
    );

    // Step 5: Generate all proofs in parallel
    let nullify_proof_client = Arc::new(ProofClient::with_config(
        config.nullify_prover_url,
        config.polling_interval,
        config.max_wait_time,
        config.prover_api_key.clone(),
    ));

    let append_proof_client = Arc::new(ProofClient::with_config(
        config.append_prover_url,
        config.polling_interval,
        config.max_wait_time,
        config.prover_api_key,
    ));

    // Generate nullify proofs
    let mut nullify_futures = Vec::new();
    for inputs in nullify_circuit_inputs {
        let client = nullify_proof_client.clone();
        nullify_futures.push(generate_nullify_zkp_proof(inputs, client));
    }

    // Generate append proofs
    let mut append_futures = Vec::new();
    for inputs in append_circuit_inputs {
        let client = append_proof_client.clone();
        append_futures.push(generate_append_zkp_proof(inputs, client));
    }

    info!(
        "Generating {} proofs in parallel ({} nullify, {} append)",
        nullify_futures.len() + append_futures.len(),
        nullify_futures.len(),
        append_futures.len()
    );

    // Execute all proof generation
    let (nullify_results, append_results) =
        futures::join!(join_all(nullify_futures), join_all(append_futures));

    // Collect nullify proofs
    let mut nullify_proofs = Vec::new();
    for result in nullify_results {
        match result {
            Ok(proof) => nullify_proofs.push(proof),
            Err(e) => return Err(e.into()),
        }
    }

    // Collect append proofs
    let mut append_proofs = Vec::new();
    for result in append_results {
        match result {
            Ok(proof) => append_proofs.push(proof),
            Err(e) => return Err(e.into()),
        }
    }

    // Step 6: Cache the new changelogs for future use
    let new_changelogs = all_changelogs
        .into_iter()
        .skip(previous_changelogs.len())
        .collect::<Vec<_>>();
    if !new_changelogs.is_empty() {
        changelog_cache
            .append_changelogs(config.merkle_tree_pubkey, new_changelogs.clone())
            .await?;
        info!(
            "Cached {} new changelogs for future operations",
            new_changelogs.len()
        );
    }

    info!(
        "Generated {} nullify and {} append proofs",
        nullify_proofs.len(),
        append_proofs.len()
    );
    Ok((nullify_proofs, append_proofs))
}

async fn generate_nullify_zkp_proof(
    inputs: BatchUpdateCircuitInputs,
    proof_client: Arc<ProofClient>,
) -> std::result::Result<InstructionDataBatchNullifyInputs, ForesterUtilsError> {
    let (proof, new_root) = proof_client
        .generate_batch_update_proof(inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
    Ok(InstructionDataBatchNullifyInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

async fn generate_append_zkp_proof(
    circuit_inputs: BatchAppendsCircuitInputs,
    proof_client: Arc<ProofClient>,
) -> std::result::Result<InstructionDataBatchAppendInputs, ForesterUtilsError> {
    let (proof, new_root) = proof_client
        .generate_batch_append_proof(circuit_inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}
