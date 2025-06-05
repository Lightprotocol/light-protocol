use light_compressed_account::TreeType;
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_pubkey::Pubkey;

use crate::{
    constants::{
        NULLIFIED_STATE_TREE_LOOKUP_TABLE_DEVNET, NULLIFIED_STATE_TREE_LOOKUP_TABLE_MAINNET,
        STATE_TREE_LOOKUP_TABLE_DEVNET, STATE_TREE_LOOKUP_TABLE_MAINNET,
    },
    indexer::TreeInfo,
    rpc::{errors::RpcError, LightClient, Rpc},
};

/// Represents a pair of state tree lookup tables
pub struct StateTreeLUTPair {
    pub state_tree_lookup_table: Pubkey,
    pub nullify_table: Pubkey,
}

/// Returns the Default Public State Tree LUTs for Devnet and Mainnet-Beta.
pub fn default_state_tree_lookup_tables() -> (Vec<StateTreeLUTPair>, Vec<StateTreeLUTPair>) {
    let mainnet = vec![StateTreeLUTPair {
        state_tree_lookup_table: STATE_TREE_LOOKUP_TABLE_MAINNET,
        nullify_table: NULLIFIED_STATE_TREE_LOOKUP_TABLE_MAINNET,
    }];

    let devnet = vec![StateTreeLUTPair {
        state_tree_lookup_table: STATE_TREE_LOOKUP_TABLE_DEVNET,
        nullify_table: NULLIFIED_STATE_TREE_LOOKUP_TABLE_DEVNET,
    }];

    (mainnet, devnet)
}

/// Get a random tree and queue from the active state tree addresses.
///
/// Prevents write lock contention on state trees.
///
/// # Arguments
/// * `info` - The active state tree addresses
///
/// # Returns
/// A random tree and queue
pub fn pick_random_tree_and_queue(info: &[TreeInfo]) -> Result<(Pubkey, Pubkey), RpcError> {
    let length = info.len();
    if length == 0 {
        return Err(RpcError::StateTreeLookupTableNotFound);
    }

    let index = rand::random::<usize>() % length;

    let tree = info[index].tree;
    let queue = info[index].queue;

    Ok((tree, queue))
}

pub async fn get_light_state_tree_infos(
    rpc_client: &LightClient,
    state_tree_lookup_table_address: &Pubkey,
    nullify_table_address: &Pubkey,
) -> Result<Vec<TreeInfo>, RpcError> {
    let account = rpc_client
        .get_account(*state_tree_lookup_table_address)
        .await
        .map_err(|_| RpcError::StateTreeLookupTableNotFound)?
        .ok_or(RpcError::StateTreeLookupTableNotFound)?;

    let state_tree_lookup_table = AddressLookupTable::deserialize(&account.data)
        .map_err(|_| RpcError::StateTreeLookupTableNotFound)?;
    let state_tree_pubkeys = state_tree_lookup_table.addresses.to_vec();

    if state_tree_pubkeys.len() % 3 != 0 {
        return Err(RpcError::InvalidStateTreeLookupTable);
    }

    let account = rpc_client
        .get_account(*nullify_table_address)
        .await
        .map_err(|_| RpcError::StateTreeLookupTableNotFound)?
        .ok_or(RpcError::StateTreeLookupTableNotFound)?;

    let nullify_table = AddressLookupTable::deserialize(&account.data)
        .map_err(|_| RpcError::StateTreeLookupTableNotFound)?;

    let nullify_table_pubkeys = nullify_table.addresses.to_vec();

    let mut bundles = Vec::new();

    for chunk in state_tree_pubkeys.chunks(3) {
        if let [tree, queue, cpi_context] = chunk {
            if !nullify_table_pubkeys.contains(tree) {
                bundles.push(TreeInfo {
                    tree: *tree,
                    queue: *queue,
                    cpi_context: Some(*cpi_context),
                    next_tree_info: None,
                    tree_type: TreeType::StateV1,
                });
            }
        }
    }

    Ok(bundles)
}
