use anyhow::anyhow;
use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
use light_batched_merkle_tree::{
    batch::BatchState, merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_compressed_account::TreeType;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, trace};

use super::types::BatchReadyState;
use crate::{errors::ForesterError, Result};

pub fn parse_merkle_tree_account(
    tree_type: TreeType,
    merkle_tree_pubkey: &Pubkey,
    account: &mut solana_sdk::account::Account,
) -> Result<(ParsedMerkleTreeData, bool)> {
    let merkle_tree = match tree_type {
        TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        ),
        TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        ),
        _ => return Err(ForesterError::InvalidTreeType(tree_type).into()),
    }?;

    let batch_index = merkle_tree.queue_batches.pending_batch_index;
    let batch = merkle_tree
        .queue_batches
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    let num_inserted_zkps = batch.get_num_inserted_zkps();
    let current_zkp_batch_index = batch.get_current_zkp_batch_index();

    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..current_zkp_batch_index {
        leaves_hash_chains.push(merkle_tree.hash_chain_stores[batch_index as usize][i as usize]);
    }

    let parsed_data = ParsedMerkleTreeData {
        next_index: merkle_tree.next_index,
        current_root: *merkle_tree.root_history.last().unwrap(),
        root_history: merkle_tree.root_history.to_vec(),
        zkp_batch_size: batch.zkp_batch_size as u16,
        pending_batch_index: batch_index as u32,
        num_inserted_zkps,
        current_zkp_batch_index,
        leaves_hash_chains,
    };

    let is_ready = batch.get_state() != BatchState::Inserted
        && batch.get_current_zkp_batch_index() > batch.get_num_inserted_zkps();

    Ok((parsed_data, is_ready))
}

pub fn parse_output_queue_account(
    account: &mut solana_sdk::account::Account,
) -> Result<(ParsedQueueData, bool)> {
    let output_queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;

    let batch_index = output_queue.batch_metadata.pending_batch_index;
    let batch = output_queue
        .batch_metadata
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    let num_inserted_zkps = batch.get_num_inserted_zkps();
    let current_zkp_batch_index = batch.get_current_zkp_batch_index();

    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..current_zkp_batch_index {
        leaves_hash_chains.push(output_queue.hash_chain_stores[batch_index as usize][i as usize]);
    }

    let parsed_data = ParsedQueueData {
        zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
        pending_batch_index: batch_index as u32,
        num_inserted_zkps,
        current_zkp_batch_index,
        leaves_hash_chains,
    };

    let is_ready = batch.get_state() != BatchState::Inserted
        && batch.get_current_zkp_batch_index() > batch.get_num_inserted_zkps();

    Ok((parsed_data, is_ready))
}

pub fn determine_batch_state(
    tree_type: TreeType,
    merkle_tree_pubkey: Pubkey,
    merkle_tree_account: Option<solana_sdk::account::Account>,
    output_queue_account: Option<solana_sdk::account::Account>,
) -> BatchReadyState {
    let (merkle_tree_data, input_ready) = if let Some(mut account) = merkle_tree_account {
        match parse_merkle_tree_account(tree_type, &merkle_tree_pubkey, &mut account) {
            Ok((data, ready)) => (Some(data), ready),
            Err(_) => (None, false),
        }
    } else {
        (None, false)
    };

    let (output_queue_data, output_ready) = if tree_type == TreeType::StateV2 {
        if let Some(mut account) = output_queue_account {
            match parse_output_queue_account(&mut account) {
                Ok((data, ready)) => (Some(data), ready),
                Err(_) => (None, false),
            }
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    trace!(
        "tree_type: {}, input_ready: {}, output_ready: {}",
        tree_type,
        input_ready,
        output_ready
    );

    if tree_type == TreeType::AddressV2 {
        return if input_ready {
            if let Some(mt_data) = merkle_tree_data {
                BatchReadyState::AddressReadyForAppend {
                    merkle_tree_data: mt_data,
                }
            } else {
                BatchReadyState::NotReady
            }
        } else {
            BatchReadyState::NotReady
        };
    }

    match (input_ready, output_ready) {
        (true, true) => {
            if let (Some(mt_data), Some(oq_data)) = (merkle_tree_data, output_queue_data) {
                debug!(
                    "Both input and output queues ready for tree {}",
                    merkle_tree_pubkey
                );
                BatchReadyState::BothReady {
                    merkle_tree_data: mt_data,
                    output_queue_data: oq_data,
                }
            } else {
                BatchReadyState::NotReady
            }
        }
        (true, false) => {
            if let Some(mt_data) = merkle_tree_data {
                BatchReadyState::StateReadyForNullify {
                    merkle_tree_data: mt_data,
                }
            } else {
                BatchReadyState::NotReady
            }
        }
        (false, true) => {
            if let (Some(mt_data), Some(oq_data)) = (merkle_tree_data, output_queue_data) {
                BatchReadyState::StateReadyForAppend {
                    merkle_tree_data: mt_data,
                    output_queue_data: oq_data,
                }
            } else {
                BatchReadyState::NotReady
            }
        }
        (false, false) => BatchReadyState::NotReady,
    }
}
