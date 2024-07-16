use std::str::FromStr;
use crate::config::ForesterConfig;
use crate::errors::ForesterError;
use account_compression::StateMerkleTreeAccount;
use log::{debug, info, warn};
use solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;

const INVALID_MT_PUBKEY: &str = "11111111111111111111111111111111";
const STATE_MERKLE_TREE_PUBKEY: &str = "5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W";
const STATE_NULLIFIER_QUEUE_PUBKEY : &str = "44J4oDXpjPAbzHCSc24q7NEiPekss4sAbLd8ka4gd9CZ";
const ADDRESS_MERKLE_TREE_PUBKEY : &str = "C83cpRN6oaafjNgMQJvaYgAz592EP5wunKvbokeTKPLn";
const ADDRESS_MERKLE_TREE_QUEUE_PUBKEY : &str ="HNjtNrjt6irUPYEgxhx2Vcs42koK9fxzm3aFLHVaaRWz";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TreeData {
    pub tree_pubkey: Pubkey,
    pub queue_pubkey: Pubkey,
    pub tree_type: TreeType,
}

impl TreeData {
    pub fn default_state() -> Self {
        TreeData {
            tree_pubkey: Pubkey::from_str(STATE_MERKLE_TREE_PUBKEY).unwrap(),
            queue_pubkey: Pubkey::from_str(STATE_NULLIFIER_QUEUE_PUBKEY).unwrap(),
            tree_type: TreeType::State,
        }
    }

    pub fn default_address() -> Self {
        TreeData {
            tree_pubkey: Pubkey::from_str(ADDRESS_MERKLE_TREE_PUBKEY).unwrap(),
            queue_pubkey: Pubkey::from_str(ADDRESS_MERKLE_TREE_QUEUE_PUBKEY).unwrap(),
            tree_type: TreeType::Address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum TreeType {
    State,
    Address,
}

pub async fn sync(config: &ForesterConfig, server_url: &str) -> Result<Vec<TreeData>, ForesterError> {
    let mut solana_rpc = SolanaRpcConnection::new(server_url, None);
    let mut indexed_trees = Vec::new();

    for &data in &config.state_tree_data {
        let tree_data = sync_single_tree(&data.tree_pubkey, &mut solana_rpc, TreeType::State).await?;
        indexed_trees.extend(tree_data);
    }

    for &data in &config.address_tree_data {
        let tree_data = sync_single_tree(&data.tree_pubkey, &mut solana_rpc, TreeType::Address).await?;
        indexed_trees.extend(tree_data);
    }

    Ok(indexed_trees)
}


async fn sync_single_tree<R: RpcConnection>(
    start_pubkey: &Pubkey,
    rpc: &mut R,
    tree_type: TreeType,
) -> Result<Vec<TreeData>, ForesterError> {
    debug!("Syncing tree data for {:?}", start_pubkey);
    let mut tree_data = Vec::new();
    let mut current_pubkey = *start_pubkey;

    loop {
        let queue_pubkey = get_nullifier_queue_pubkey(&current_pubkey, rpc).await?;
        info!("Current tree pubkey: {:?}", current_pubkey);
        info!("Current queue pubkey: {:?}", queue_pubkey);
        tree_data.push(TreeData {
            tree_pubkey: current_pubkey,
            queue_pubkey,
            tree_type,
        });

        match next_merkle_tree_pubkey(&current_pubkey, rpc).await {
            Ok(next_pubkey) if next_pubkey.to_string() != INVALID_MT_PUBKEY => {
                info!("Next pubkey: {:?}", next_pubkey);
                current_pubkey = next_pubkey;
            }
            _ => break,
        }
    }

    Ok(tree_data)
}


pub fn serialize_tree_data(trees: &[TreeData]) -> Result<(), ForesterError> {
    let serialized = serde_json::to_string_pretty(trees).map_err(|e| {
        warn!("Failed to serialize indexed trees: {:?}", e);
        ForesterError::Custom("Failed to serialize indexed trees".to_string())
    })?;

    std::fs::write("indexed_trees.json", serialized)?;
    Ok(())
}

pub fn deserialize_tree_data() -> Result<Vec<TreeData>, ForesterError> {
    let serialized = std::fs::read_to_string("indexed_trees.json")?;
    let trees: Vec<TreeData> = from_str(&serialized).map_err(|e| {
        warn!("Failed to deserialize indexed trees: {:?}", e);
        ForesterError::Custom("Failed to deserialize indexed trees".to_string())
    })?;
    Ok(trees)
}


pub async fn merkle_tree_account<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<StateMerkleTreeAccount, ForesterError> {
    debug!("Getting merkle tree account for {:?}", merkle_tree_pubkey);
    let account = rpc.get_anchor_account::<StateMerkleTreeAccount>(merkle_tree_pubkey).await;
    Ok(account)
}

pub async fn next_merkle_tree_pubkey<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<Pubkey, ForesterError> {
    debug!("Getting next merkle tree pubkey for {:?}", merkle_tree_pubkey);
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, rpc).await?;
    info!("Metadata: {:?}", merkle_tree_account.metadata);
    Ok(merkle_tree_account.metadata.next_merkle_tree)
}

pub async fn get_nullifier_queue_pubkey<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<Pubkey, ForesterError> {
    debug!("Getting nullifier queue pubkey for {:?}", merkle_tree_pubkey);
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, rpc).await?;
    let nullifier_queue_pubkey = merkle_tree_account.metadata.associated_queue;
    Ok(nullifier_queue_pubkey)
}
