use std::{collections::HashMap, fs, path::PathBuf};

use light_client::rpc::RpcError;
use serde::{Deserialize, Serialize};
use solana_sdk::{account::Account, pubkey::Pubkey};

#[derive(Debug, Serialize, Deserialize)]
struct AccountData {
    pubkey: String,
    account: AccountInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountInfo {
    lamports: u64,
    data: (String, String), // (data, encoding) where encoding is typically "base64"
    owner: String,
    executable: bool,
    #[serde(rename = "rentEpoch")]
    rent_epoch: u64,
}

pub fn find_accounts_dir() -> Option<PathBuf> {
    #[cfg(not(feature = "devenv"))]
    {
        use std::process::Command;
        let output = Command::new("which")
            .arg("light")
            .output()
            .expect("Failed to execute 'which light'");

        if !output.status.success() {
            return None;
        }

        let light_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let mut light_bin_path = PathBuf::from(light_path);
        light_bin_path.pop();

        let accounts_dir =
            light_bin_path.join("../lib/node_modules/@lightprotocol/zk-compression-cli/accounts");

        Some(accounts_dir.canonicalize().unwrap_or(accounts_dir))
    }
    #[cfg(feature = "devenv")]
    {
        println!("Use only in light protocol monorepo. Using 'git rev-parse --show-toplevel' to find the accounts directory");
        let light_protocol_toplevel = String::from_utf8_lossy(
            &std::process::Command::new("git")
                .arg("rev-parse")
                .arg("--show-toplevel")
                .output()
                .expect("Failed to get top-level directory")
                .stdout,
        )
        .trim()
        .to_string();

        // In devenv mode, we don't load accounts from directory
        // This path won't be used as we initialize accounts directly
        let accounts_path = PathBuf::from(format!("{}/cli/accounts/", light_protocol_toplevel));
        Some(accounts_path)
    }
}

/// Load all accounts from the accounts directory
/// Returns a HashMap of Pubkey -> Account
pub fn load_all_accounts_from_dir() -> Result<HashMap<Pubkey, Account>, RpcError> {
    let accounts_dir = find_accounts_dir().ok_or_else(|| {
        RpcError::CustomError(
            "Failed to find accounts directory. Make sure light CLI is installed.".to_string(),
        )
    })?;

    let mut accounts = HashMap::new();

    let entries = fs::read_dir(&accounts_dir).map_err(|e| {
        RpcError::CustomError(format!(
            "Failed to read accounts directory at {:?}: {}",
            accounts_dir, e
        ))
    })?;

    for entry in entries {
        let entry = entry
            .map_err(|e| RpcError::CustomError(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path).map_err(|e| {
                RpcError::CustomError(format!("Failed to read file {:?}: {}", path, e))
            })?;

            let account_data: AccountData = serde_json::from_str(&contents).map_err(|e| {
                RpcError::CustomError(format!(
                    "Failed to parse account JSON from {:?}: {}",
                    path, e
                ))
            })?;

            let pubkey = account_data
                .pubkey
                .parse::<Pubkey>()
                .map_err(|e| RpcError::CustomError(format!("Invalid pubkey: {}", e)))?;

            let owner = account_data
                .account
                .owner
                .parse::<Pubkey>()
                .map_err(|e| RpcError::CustomError(format!("Invalid owner pubkey: {}", e)))?;

            // Decode base64 data
            let data = if account_data.account.data.1 == "base64" {
                use base64::{engine::general_purpose, Engine as _};
                general_purpose::STANDARD
                    .decode(&account_data.account.data.0)
                    .map_err(|e| {
                        RpcError::CustomError(format!("Failed to decode base64 data: {}", e))
                    })?
            } else {
                return Err(RpcError::CustomError(format!(
                    "Unsupported encoding: {}",
                    account_data.account.data.1
                )));
            };

            let account = Account {
                lamports: account_data.account.lamports,
                data,
                owner,
                executable: account_data.account.executable,
                rent_epoch: account_data.account.rent_epoch,
            };

            accounts.insert(pubkey, account);
        }
    }

    Ok(accounts)
}

/// Load a specific account by pubkey from the accounts directory
/// Optionally provide a prefix for the filename (e.g. "address_merkle_tree")
pub fn load_account_from_dir(pubkey: &Pubkey, prefix: Option<&str>) -> Result<Account, RpcError> {
    let accounts_dir = find_accounts_dir().ok_or_else(|| {
        RpcError::CustomError(
            "Failed to find accounts directory. Make sure light CLI is installed.".to_string(),
        )
    })?;

    let filename = if let Some(prefix) = prefix {
        format!("{}_{}.json", prefix, pubkey)
    } else {
        format!("{}.json", pubkey)
    };
    let path = accounts_dir.join(&filename);

    let contents = fs::read_to_string(&path).map_err(|e| {
        RpcError::CustomError(format!("Failed to read account file {:?}: {}", path, e))
    })?;

    let account_data: AccountData = serde_json::from_str(&contents).map_err(|e| {
        RpcError::CustomError(format!(
            "Failed to parse account JSON from {:?}: {}",
            path, e
        ))
    })?;

    let owner = account_data
        .account
        .owner
        .parse::<Pubkey>()
        .map_err(|e| RpcError::CustomError(format!("Invalid owner pubkey: {}", e)))?;

    // Decode base64 data
    let data = if account_data.account.data.1 == "base64" {
        use base64::{engine::general_purpose, Engine as _};
        general_purpose::STANDARD
            .decode(&account_data.account.data.0)
            .map_err(|e| RpcError::CustomError(format!("Failed to decode base64 data: {}", e)))?
    } else {
        return Err(RpcError::CustomError(format!(
            "Unsupported encoding: {}",
            account_data.account.data.1
        )));
    };

    Ok(Account {
        lamports: account_data.account.lamports,
        data,
        owner,
        executable: account_data.account.executable,
        rent_epoch: account_data.account.rent_epoch,
    })
}
