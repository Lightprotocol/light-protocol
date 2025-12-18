use account_compression::{
    utils::check_discriminator::check_discriminator, AddressMerkleTreeAccount,
    StateMerkleTreeAccount,
};
use base64::{engine::general_purpose, Engine as _};
use borsh::BorshDeserialize;
use forester_utils::forester_epoch::TreeAccounts;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;
use serde_json::json;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tracing::{debug, trace, warn};

use crate::{errors::AccountDeserializationError, Result};

// Discriminators for filtering getProgramAccounts
// BatchedMerkleTreeAccount: b"BatchMta"
const BATCHED_TREE_DISCRIMINATOR: [u8; 8] = [66, 97, 116, 99, 104, 77, 116, 97];
// StateMerkleTreeAccount: sha256("account:StateMerkleTreeAccount")[0..8]
const STATE_V1_DISCRIMINATOR: [u8; 8] = [172, 43, 172, 186, 29, 73, 219, 84];
// AddressMerkleTreeAccount: sha256("account:AddressMerkleTreeAccount")[0..8]
const ADDRESS_V1_DISCRIMINATOR: [u8; 8] = [11, 161, 175, 9, 212, 229, 73, 73];

/// Fetch trees using filtered getProgramAccounts calls (optimized for remote RPCs).
/// Falls back to unfiltered fetch if the filtered approach fails.
pub async fn fetch_trees<R: Rpc>(rpc: &R) -> Result<Vec<TreeAccounts>> {
    let rpc_url = rpc.get_url();

    // Try filtered approach first (much faster for remote RPCs)
    match fetch_trees_filtered(&rpc_url).await {
        Ok(trees) => {
            trace!("Fetched {} trees using filtered queries", trees.len());
            Ok(trees)
        }
        Err(e) => {
            warn!(
                "Filtered tree fetch failed, falling back to unfiltered: {:?}",
                e
            );
            fetch_trees_unfiltered(rpc).await
        }
    }
}

/// Fetch trees without filters (original implementation, slower but more reliable)
pub async fn fetch_trees_unfiltered<R: Rpc>(rpc: &R) -> Result<Vec<TreeAccounts>> {
    let program_id = account_compression::id();
    trace!("Fetching accounts for program (unfiltered): {}", program_id);
    Ok(rpc
        .get_program_accounts(&program_id)
        .await?
        .into_iter()
        .filter_map(|(pubkey, account)| process_account(pubkey, account))
        .collect())
}

/// Fetch trees using filtered getProgramAccounts calls with discriminator memcmp filters.
/// Makes 3 parallel requests (one per tree type) instead of fetching all accounts.
pub async fn fetch_trees_filtered(rpc_url: &str) -> Result<Vec<TreeAccounts>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let program_id = account_compression::id();

    // Fetch all three types in parallel
    let (batched_result, state_v1_result, address_v1_result) = tokio::join!(
        fetch_accounts_with_discriminator(
            &client,
            rpc_url,
            &program_id,
            &BATCHED_TREE_DISCRIMINATOR
        ),
        fetch_accounts_with_discriminator(&client, rpc_url, &program_id, &STATE_V1_DISCRIMINATOR),
        fetch_accounts_with_discriminator(&client, rpc_url, &program_id, &ADDRESS_V1_DISCRIMINATOR),
    );

    let mut all_trees = Vec::new();
    let mut errors = Vec::new();

    // Process batched trees (V2) - need to distinguish state vs address
    match batched_result {
        Ok(accounts) => {
            debug!("Fetched {} batched tree accounts", accounts.len());
            for (pubkey, mut account) in accounts {
                // Try state first, then address
                if let Ok(tree) = process_batch_state_account(&mut account, pubkey) {
                    all_trees.push(tree);
                } else if let Ok(tree) = process_batch_address_account(&mut account, pubkey) {
                    all_trees.push(tree);
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch batched trees: {:?}", e);
            errors.push(format!("batched: {}", e));
        }
    }

    // Process state V1 trees
    match state_v1_result {
        Ok(accounts) => {
            debug!("Fetched {} state V1 tree accounts", accounts.len());
            for (pubkey, account) in accounts {
                if let Ok(tree) = process_state_account(&account, pubkey) {
                    all_trees.push(tree);
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch state V1 trees: {:?}", e);
            errors.push(format!("state_v1: {}", e));
        }
    }

    // Process address V1 trees
    match address_v1_result {
        Ok(accounts) => {
            debug!("Fetched {} address V1 tree accounts", accounts.len());
            for (pubkey, account) in accounts {
                if let Ok(tree) = process_address_account(&account, pubkey) {
                    all_trees.push(tree);
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch address V1 trees: {:?}", e);
            errors.push(format!("address_v1: {}", e));
        }
    }

    // Only return error if all queries failed; empty-but-successful is Ok
    if !errors.is_empty() && all_trees.is_empty() {
        return Err(anyhow::anyhow!(
            "All filtered queries failed: {}",
            errors.join(", ")
        ));
    }

    Ok(all_trees)
}

/// Fetch accounts from a program with a specific discriminator filter
async fn fetch_accounts_with_discriminator(
    client: &reqwest::Client,
    rpc_url: &str,
    program_id: &Pubkey,
    discriminator: &[u8; 8],
) -> Result<Vec<(Pubkey, Account)>> {
    let discriminator_base58 = bs58::encode(discriminator).into_string();

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [
            program_id.to_string(),
            {
                "encoding": "base64",
                "commitment": "confirmed",
                "filters": [
                    {
                        "memcmp": {
                            "offset": 0,
                            "bytes": discriminator_base58
                        }
                    }
                ]
            }
        ]
    });

    let response = client.post(rpc_url).json(&payload).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    let json_response: serde_json::Value = response.json().await?;

    if let Some(error) = json_response.get("error") {
        return Err(anyhow::anyhow!("RPC error: {:?}", error));
    }

    let accounts_array = json_response
        .get("result")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Unexpected response format"))?;

    let mut accounts = Vec::with_capacity(accounts_array.len());

    for account_value in accounts_array {
        if let Some((pubkey, account)) = parse_account_from_json(account_value) {
            accounts.push((pubkey, account));
        }
    }

    Ok(accounts)
}

/// Parse a single account from JSON RPC response
fn parse_account_from_json(value: &serde_json::Value) -> Option<(Pubkey, Account)> {
    let pubkey_str = value.get("pubkey")?.as_str()?;
    let pubkey: Pubkey = pubkey_str.parse().ok()?;

    let account_obj = value.get("account")?;
    let lamports = account_obj.get("lamports")?.as_u64()?;
    let owner_str = account_obj.get("owner")?.as_str()?;
    let owner: Pubkey = owner_str.parse().ok()?;
    let executable = account_obj.get("executable")?.as_bool().unwrap_or(false);
    let rent_epoch = account_obj.get("rentEpoch")?.as_u64().unwrap_or(0);

    let data_array = account_obj.get("data")?.as_array()?;
    let data_str = data_array.first()?.as_str()?;
    let data = general_purpose::STANDARD.decode(data_str).ok()?;

    Some((
        pubkey,
        Account {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        },
    ))
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
        "{:?} Merkle Tree account found. Pubkey: {}. Queue pubkey: {}. Rolledover: {}",
        tree_type,
        pubkey,
        tree_accounts.queue,
        tree_accounts.is_rolledover
    );
    tree_accounts
}
