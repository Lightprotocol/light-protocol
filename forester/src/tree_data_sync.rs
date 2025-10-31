use account_compression::{
    utils::check_discriminator::check_discriminator, AddressMerkleTreeAccount,
    StateMerkleTreeAccount,
};
use borsh::BorshDeserialize;
use forester_utils::forester_epoch::TreeAccounts;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tracing::trace;

use crate::{errors::AccountDeserializationError, Result};

pub async fn fetch_trees<R: Rpc>(rpc: &R) -> Result<Vec<TreeAccounts>> {
    let program_id = account_compression::id();
    trace!("Fetching accounts for program: {}", program_id);
    Ok(rpc
        .get_program_accounts(&program_id)
        .await?
        .into_iter()
        .filter_map(|(pubkey, account)| process_account(pubkey, account))
        .collect())
}

fn process_account(pubkey: Pubkey, mut account: Account) -> Option<TreeAccounts> {
    process_state_account(&account, pubkey)
        .or_else(|_| process_batch_state_account(&mut account, pubkey))
        .or_else(|_| process_address_account(&account, pubkey))
        .or_else(|_| process_batch_address_account(&mut account, pubkey))
        .ok()
}

fn process_state_account(account: &Account, pubkey: Pubkey) -> Result<TreeAccounts> {
    check_discriminator::<StateMerkleTreeAccount>(&account.data)?;
    let tree_account = StateMerkleTreeAccount::deserialize(&mut &account.data[8..])?;
    Ok(create_tree_accounts(
        pubkey,
        &tree_account.metadata,
        TreeType::StateV1,
    ))
}

fn process_address_account(account: &Account, pubkey: Pubkey) -> Result<TreeAccounts> {
    check_discriminator::<AddressMerkleTreeAccount>(&account.data)?;
    let tree_account = AddressMerkleTreeAccount::deserialize(&mut &account.data[8..])?;
    Ok(create_tree_accounts(
        pubkey,
        &tree_account.metadata,
        TreeType::AddressV1,
    ))
}

fn process_batch_state_account(account: &mut Account, pubkey: Pubkey) -> Result<TreeAccounts> {
    light_account_checks::checks::check_discriminator::<BatchedMerkleTreeAccount>(&account.data)
        .map_err(|_| AccountDeserializationError::BatchStateMerkleTree {
            error: "Invalid discriminator".to_string(),
        })?;

    let tree_account =
        BatchedMerkleTreeAccount::state_from_bytes(&mut account.data, &pubkey.into()).map_err(
            |e| AccountDeserializationError::BatchStateMerkleTree {
                error: e.to_string(),
            },
        )?;
    Ok(create_tree_accounts(
        pubkey,
        &tree_account.metadata,
        TreeType::StateV2,
    ))
}

fn process_batch_address_account(account: &mut Account, pubkey: Pubkey) -> Result<TreeAccounts> {
    light_account_checks::checks::check_discriminator::<BatchedMerkleTreeAccount>(&account.data)
        .map_err(|_| AccountDeserializationError::BatchAddressMerkleTree {
            error: "Invalid discriminator".to_string(),
        })?;

    let tree_account =
        BatchedMerkleTreeAccount::address_from_bytes(&mut account.data, &pubkey.into()).map_err(
            |e| AccountDeserializationError::BatchAddressMerkleTree {
                error: e.to_string(),
            },
        )?;
    Ok(create_tree_accounts(
        pubkey,
        &tree_account.metadata,
        TreeType::AddressV2,
    ))
}

fn create_tree_accounts(
    pubkey: Pubkey,
    metadata: &MerkleTreeMetadata,
    tree_type: TreeType,
) -> TreeAccounts {
    let tree_accounts = TreeAccounts::new(
        pubkey,
        metadata.associated_queue.into(),
        tree_type,
        metadata.rollover_metadata.rolledover_slot != u64::MAX,
    );

    trace!(
        "{:?} Merkle Tree account found. Pubkey: {}. Queue pubkey: {}",
        tree_type,
        pubkey,
        tree_accounts.queue
    );
    tree_accounts
}
