use crate::config::ForesterConfig;
use crate::errors::ForesterError;
use account_compression::initialize_address_merkle_tree::ProgramError;
use account_compression::utils::check_discrimininator::check_discriminator;
use account_compression::{AddressMerkleTreeAccount, MerkleTreeMetadata, StateMerkleTreeAccount};
use borsh::BorshDeserialize;
use light_test_utils::forester_epoch::{TreeAccounts, TreeType};
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::debug;
use solana_sdk::pubkey::Pubkey;

pub async fn fetch_trees(server_url: &str) -> Vec<TreeAccounts> {
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
            let is_rolled_over =
                tree_account.metadata.rollover_metadata.rolledover_slot != u64::MAX;
            tree_data_list.push(TreeAccounts::new(
                pubkey,
                queue_pubkey,
                TreeType::State,
                is_rolled_over,
            ));
            debug!(
                "State Merkle Tree account found. Pubkey: {}. Queue pubkey: {}",
                pubkey, queue_pubkey
            );
        } else {
            let is_address_account = check_discriminator::<AddressMerkleTreeAccount>(&account.data)
                .map_err(ProgramError::from);
            if is_address_account.is_ok() {
                let tree_account = AddressMerkleTreeAccount::deserialize(&mut &account.data[8..])
                    .map_err(RpcError::from)
                    .unwrap();
                let queue_pubkey = tree_account.metadata.associated_queue;
                let is_rolled_over =
                    tree_account.metadata.rollover_metadata.rolledover_slot != u64::MAX;
                tree_data_list.push(TreeAccounts::new(
                    pubkey,
                    queue_pubkey,
                    TreeType::Address,
                    is_rolled_over,
                ));
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
) -> Result<Vec<TreeAccounts>, ForesterError> {
    let mut solana_rpc = SolanaRpcConnection::new(server_url, None);
    let mut indexed_trees = Vec::new();

    for data in config.state_tree_data.as_slice() {
        let tree_data =
            sync_single_tree(&data.merkle_tree, &mut solana_rpc, TreeType::State).await?;
        indexed_trees.extend(tree_data);
    }

    for data in config.address_tree_data.as_slice() {
        let tree_data =
            sync_single_tree(&data.merkle_tree, &mut solana_rpc, TreeType::Address).await?;
        indexed_trees.extend(tree_data);
    }

    Ok(indexed_trees)
}

async fn sync_single_tree<R: RpcConnection>(
    start_pubkey: &Pubkey,
    rpc: &mut R,
    tree_type: TreeType,
) -> Result<Vec<TreeAccounts>, ForesterError> {
    let mut tree_data = Vec::new();
    let mut current_pubkey = *start_pubkey;

    loop {
        let metadata = get_tree_metadata(&current_pubkey, rpc).await?;
        tree_data.push(TreeAccounts::new(
            current_pubkey,
            metadata.associated_queue,
            tree_type,
            false,
        ));

        match next_merkle_tree_pubkey(&current_pubkey, rpc).await {
            Ok(next_pubkey) if next_pubkey != Pubkey::default() => {
                current_pubkey = next_pubkey;
            }
            _ => break,
        }
    }

    Ok(tree_data)
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
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, rpc).await?;
    Ok(merkle_tree_account.metadata.next_merkle_tree)
}

pub async fn get_tree_metadata<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<MerkleTreeMetadata, ForesterError> {
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, rpc).await?;
    Ok(merkle_tree_account.metadata)
}
