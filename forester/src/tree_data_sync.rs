use account_compression::initialize_address_merkle_tree::ProgramError;
use account_compression::utils::check_discrimininator::check_discriminator;
use account_compression::{AddressMerkleTreeAccount, MerkleTreeMetadata, StateMerkleTreeAccount};
use borsh::BorshDeserialize;
use forester_utils::forester_epoch::{TreeAccounts, TreeType};
use forester_utils::rpc::RpcConnection;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use tracing::debug;

pub async fn fetch_trees<R: RpcConnection>(rpc: &R) -> Vec<TreeAccounts> {
    let program_id = account_compression::id();
    debug!("Fetching accounts for program: {}", program_id);
    rpc.get_program_accounts(&program_id)
        .unwrap()
        .into_iter()
        .filter_map(|(pubkey, account)| process_account(pubkey, account))
        .collect()
}

fn process_account(pubkey: Pubkey, account: Account) -> Option<TreeAccounts> {
    process_state_account(&account, pubkey)
        .or_else(|_| process_address_account(&account, pubkey))
        .ok()
}

fn process_state_account(account: &Account, pubkey: Pubkey) -> Result<TreeAccounts, ProgramError> {
    check_discriminator::<StateMerkleTreeAccount>(&account.data)?;
    let tree_account = StateMerkleTreeAccount::deserialize(&mut &account.data[8..])?;
    Ok(create_tree_accounts(
        pubkey,
        &tree_account.metadata,
        TreeType::State,
    ))
}

fn process_address_account(
    account: &Account,
    pubkey: Pubkey,
) -> Result<TreeAccounts, ProgramError> {
    check_discriminator::<AddressMerkleTreeAccount>(&account.data)?;
    let tree_account = AddressMerkleTreeAccount::deserialize(&mut &account.data[8..])?;
    Ok(create_tree_accounts(
        pubkey,
        &tree_account.metadata,
        TreeType::Address,
    ))
}

fn create_tree_accounts(
    pubkey: Pubkey,
    metadata: &MerkleTreeMetadata,
    tree_type: TreeType,
) -> TreeAccounts {
    let tree_accounts = TreeAccounts::new(
        pubkey,
        metadata.associated_queue,
        tree_type,
        metadata.rollover_metadata.rolledover_slot != u64::MAX,
    );

    debug!(
        "{:?} Merkle Tree account found. Pubkey: {}. Queue pubkey: {}",
        tree_type, pubkey, tree_accounts.queue
    );
    tree_accounts
}
