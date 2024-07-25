use crate::config::ForesterConfig;
use crate::errors::ForesterError;
use account_compression::initialize_address_merkle_tree::ProgramError;
use account_compression::utils::check_discrimininator::check_discriminator;
use account_compression::{AddressMerkleTreeAccount, StateMerkleTreeAccount};
use borsh::BorshDeserialize;
use light_test_utils::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts};
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use solana_sdk::pubkey::Pubkey;

const INVALID_MT_PUBKEY: &str = "11111111111111111111111111111111";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TreeData {
    pub tree_pubkey: Pubkey,
    pub queue_pubkey: Pubkey,
    pub tree_type: TreeType,
}

impl TreeData {
    pub fn new(tree_pubkey: Pubkey, queue_pubkey: Pubkey, tree_type: TreeType) -> Self {
        Self {
            tree_pubkey,
            queue_pubkey,
            tree_type,
        }
    }
}

impl From<StateMerkleTreeAccounts> for TreeData {
    fn from(state_merkle_tree_accounts: StateMerkleTreeAccounts) -> Self {
        Self {
            tree_pubkey: state_merkle_tree_accounts.merkle_tree,
            queue_pubkey: state_merkle_tree_accounts.nullifier_queue,
            tree_type: TreeType::State,
        }
    }
}

impl From<AddressMerkleTreeAccounts> for TreeData {
    fn from(address_merkle_tree_accounts: AddressMerkleTreeAccounts) -> Self {
        Self {
            tree_pubkey: address_merkle_tree_accounts.merkle_tree,
            queue_pubkey: address_merkle_tree_accounts.queue,
            tree_type: TreeType::Address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum TreeType {
    State,
    Address,
}

pub async fn fetch_trees(server_url: &str) -> Vec<TreeData> {
    let program_id = account_compression::id();
    let rpc = SolanaRpcConnection::new(server_url, None);
    let mut tree_data_list = Vec::new();
    debug!("Fetching accounts for program: {}", program_id);
    let accounts = rpc.client.get_program_accounts(&program_id).unwrap();
    for (pubkey, account) in accounts {
        let is_state_account = check_discriminator::<StateMerkleTreeAccount>(&account.data)
            .map_err(ProgramError::from);
        if is_state_account.is_ok() {
            let tree_account = StateMerkleTreeAccount::deserialize(&mut &account.data[8..])
                .map_err(RpcError::from)
                .unwrap();
            let queue_pubkey = tree_account.metadata.associated_queue;
            debug!(
                "State Merkle Tree account found. Pubkey: {}. Queue pubkey: {}",
                pubkey, queue_pubkey
            );
            tree_data_list.push(TreeData::new(pubkey, queue_pubkey, TreeType::State));
        } else {
            let is_address_account = check_discriminator::<AddressMerkleTreeAccount>(&account.data)
                .map_err(ProgramError::from);
            if is_address_account.is_ok() {
                let tree_account = AddressMerkleTreeAccount::deserialize(&mut &account.data[8..])
                    .map_err(RpcError::from)
                    .unwrap();
                let queue_pubkey = tree_account.metadata.associated_queue;
                tree_data_list.push(TreeData::new(pubkey, queue_pubkey, TreeType::Address));
                debug!(
                    "Address Merkle Tree account found. Pubkey: {}. Queue pubkey: {}",
                    pubkey, queue_pubkey
                );
            }
        }
    }
    tree_data_list
}

#[allow(dead_code)]
pub async fn sync(
    config: &ForesterConfig,
    server_url: &str,
) -> Result<Vec<TreeData>, ForesterError> {
    let mut solana_rpc = SolanaRpcConnection::new(server_url, None);
    let mut indexed_trees = Vec::new();

    for &data in &config.state_tree_data {
        let tree_data =
            sync_single_tree(&data.tree_pubkey, &mut solana_rpc, TreeType::State).await?;
        indexed_trees.extend(tree_data);
    }

    for &data in &config.address_tree_data {
        let tree_data =
            sync_single_tree(&data.tree_pubkey, &mut solana_rpc, TreeType::Address).await?;
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

#[allow(dead_code)]
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
    let account = rpc
        .get_anchor_account::<StateMerkleTreeAccount>(merkle_tree_pubkey)
        .await?
        .unwrap();
    Ok(account)
}

pub async fn next_merkle_tree_pubkey<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<Pubkey, ForesterError> {
    debug!(
        "Getting next merkle tree pubkey for {:?}",
        merkle_tree_pubkey
    );
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, rpc).await?;
    info!("Metadata: {:?}", merkle_tree_account.metadata);
    Ok(merkle_tree_account.metadata.next_merkle_tree)
}

pub async fn get_nullifier_queue_pubkey<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<Pubkey, ForesterError> {
    debug!(
        "Getting nullifier queue pubkey for {:?}",
        merkle_tree_pubkey
    );
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, rpc).await?;
    let nullifier_queue_pubkey = merkle_tree_account.metadata.associated_queue;
    Ok(nullifier_queue_pubkey)
}
