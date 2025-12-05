use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use borsh::BorshDeserialize;
use light_ctoken_interface::{
    state::{extensions::ExtensionStruct, CToken},
    COMPRESSED_TOKEN_PROGRAM_ID, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::oneshot;
use tracing::{debug, error, info};

use super::state::CompressibleAccountTracker;
use crate::Result;

const PAGE_SIZE: usize = 10_000;

/// Bootstrap the compressible account tracker by fetching existing accounts
/// Uses standard getProgramAccounts for localhost, getProgramAccountsV2 for remote networks
pub async fn bootstrap_compressible_accounts(
    rpc_url: String,
    tracker: Arc<CompressibleAccountTracker>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    info!("Starting bootstrap of compressible accounts");

    let is_localhost = rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1");

    if is_localhost {
        info!("Detected localhost, using standard getProgramAccounts");
        bootstrap_with_standard_api(rpc_url, tracker, shutdown_rx).await
    } else {
        info!("Using getProgramAccountsV2 with pagination");
        bootstrap_with_v2_api(rpc_url, tracker, shutdown_rx).await
    }
}

/// Process a single account from RPC response
/// Returns Ok(true) if account was inserted, Ok(false) if skipped, Err on critical failure
fn process_account(
    account_value: &serde_json::Value,
    tracker: &CompressibleAccountTracker,
) -> Result<bool> {
    // Extract pubkey
    let pubkey_str = match account_value.get("pubkey").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            debug!("Skipping account with missing pubkey");
            return Ok(false);
        }
    };

    let pubkey = match pubkey_str.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(e) => {
            debug!("Failed to parse pubkey {}: {:?}", pubkey_str, e);
            return Ok(false);
        }
    };

    // Extract account data
    let account_obj = match account_value.get("account") {
        Some(obj) => obj,
        None => {
            debug!("Skipping account {} with missing account object", pubkey);
            return Ok(false);
        }
    };

    // Check lamports - skip closed accounts (lamports == 0)
    let lamports = match account_obj.get("lamports").and_then(|v| v.as_u64()) {
        Some(0) => {
            debug!("Skipping closed account {} (lamports == 0)", pubkey);
            return Ok(false);
        }
        Some(lamports) => lamports,
        None => {
            debug!("Skipping account {} with missing lamports field", pubkey);
            return Ok(false);
        }
    };

    let data_array = match account_obj.get("data").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            debug!("Skipping account {} with missing or empty data", pubkey);
            return Ok(false);
        }
    };

    let data_str = match data_array[0].as_str() {
        Some(s) => s,
        None => {
            debug!("Skipping account {} with invalid data format", pubkey);
            return Ok(false);
        }
    };

    let data_bytes = match general_purpose::STANDARD.decode(data_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            debug!("Failed to decode base64 for account {}: {:?}", pubkey, e);
            return Ok(false);
        }
    };

    // Deserialize CToken
    let ctoken = match CToken::try_from_slice(&data_bytes) {
        Ok(token) => token,
        Err(e) => {
            debug!(
                "Failed to deserialize CToken for account {}: {:?}",
                pubkey, e
            );
            return Ok(false);
        }
    };

    // Check for Compressible extension
    let has_compressible = ctoken.extensions.as_ref().is_some_and(|exts| {
        exts.iter()
            .any(|ext| matches!(ext, ExtensionStruct::Compressible(_)))
    });

    if !has_compressible {
        debug!("Skipping account {} without Compressible extension", pubkey);
        return Ok(false);
    }

    // Use tracker's update_from_account to calculate compressible_slot
    if let Err(e) = tracker.update_from_account(pubkey, &data_bytes, lamports) {
        debug!("Failed to insert account {}: {:?}", pubkey, e);
        return Ok(false);
    }

    Ok(true)
}

/// Send RPC request with shutdown handling
async fn send_rpc_request(
    client: &reqwest::Client,
    rpc_url: &str,
    payload: &serde_json::Value,
    shutdown_rx: &mut oneshot::Receiver<()>,
) -> Result<serde_json::Value> {
    let response_result = tokio::select! {
        response = client.post(rpc_url).json(payload).send() => response,
        _ = shutdown_rx => {
            return Err(anyhow::anyhow!("Shutdown requested"));
        }
    };

    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            error!("Bootstrap request error: {:?}", e);
            return Err(anyhow::anyhow!("Request failed: {:?}", e));
        }
    };

    if !response.status().is_success() {
        error!("Bootstrap HTTP error: {}", response.status());
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    let json_response: serde_json::Value = match response.json().await {
        Ok(json) => json,
        Err(e) => {
            error!("Bootstrap failed to parse response: {:?}", e);
            return Err(anyhow::anyhow!("Parse error: {:?}", e));
        }
    };

    // Check for RPC error
    if let Some(error) = json_response.get("error") {
        error!("Bootstrap RPC error: {:?}", error);
        return Err(anyhow::anyhow!("RPC error: {:?}", error));
    }

    json_response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Unexpected response format"))
}

/// Bootstrap using Helius getProgramAccountsV2 with cursor pagination
async fn bootstrap_with_v2_api(
    rpc_url: String,
    tracker: Arc<CompressibleAccountTracker>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let program_id = Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID);

    let mut total_fetched = 0;
    let mut total_inserted = 0;
    let mut page_count = 0;
    let mut cursor: Option<String> = None;

    loop {
        page_count += 1;

        // Build request payload
        let mut params = json!([
            program_id.to_string(),
            {
                "encoding": "base64",
                "commitment": "confirmed",
                "filters": [
                    {"dataSize": COMPRESSIBLE_TOKEN_ACCOUNT_SIZE}
                ],
                "limit": PAGE_SIZE
            }
        ]);

        // Add cursor for pagination
        if let Some(ref c) = cursor {
            params[1]["paginationKey"] = json!(c);
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "id": page_count,
            "method": "getProgramAccountsV2",
            "params": params
        });

        // Send request
        let result = match send_rpc_request(&client, &rpc_url, &payload, &mut shutdown_rx).await {
            Ok(res) => res,
            Err(e) if e.to_string().contains("Shutdown requested") => {
                info!(
                    "Bootstrap shutting down at page {}, {} accounts inserted",
                    page_count, total_inserted
                );
                return Ok(());
            }
            Err(e) => {
                error!("Bootstrap failed on page {}: {:?}", page_count, e);
                return Err(e);
            }
        };

        // Extract accounts array
        let accounts_array = if let Some(arr) = result.get("accounts").and_then(|v| v.as_array()) {
            arr
        } else if let Some(arr) = result.as_array() {
            arr
        } else if let Some(value) = result.get("value").and_then(|v| v.as_array()) {
            value
        } else {
            error!(
                "Bootstrap could not find accounts array on page {}",
                page_count
            );
            return Err(anyhow::anyhow!("Could not find accounts array"));
        };

        let accounts_count = accounts_array.len();

        if accounts_count == 0 {
            info!("Bootstrap complete: no more accounts (page {})", page_count);
            break;
        }

        total_fetched += accounts_count;

        // Process each account
        let mut page_inserted = 0;
        for account_value in accounts_array {
            if let Ok(true) = process_account(account_value, &tracker) {
                page_inserted += 1;
                total_inserted += 1;
            }
        }

        info!(
            "Bootstrap page {}: fetched {} accounts, inserted {} compressible accounts (total: {})",
            page_count, accounts_count, page_inserted, total_inserted
        );

        // Get cursor for next page
        cursor = result
            .get("paginationKey")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        // If no cursor, we've reached the end
        if cursor.is_none() {
            info!(
                "Bootstrap complete: reached end of results at page {}",
                page_count
            );
            break;
        }

        // Add small delay between requests to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    info!(
        "Bootstrap finished: {} pages, {} total fetched, {} compressible accounts inserted",
        page_count, total_fetched, total_inserted
    );

    Ok(())
}

/// Bootstrap using standard getProgramAccounts (for localhost/test validator)
async fn bootstrap_with_standard_api(
    rpc_url: String,
    tracker: Arc<CompressibleAccountTracker>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let program_id = Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID);

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
                    {"dataSize": COMPRESSIBLE_TOKEN_ACCOUNT_SIZE}
                ]
            }
        ]
    });

    // Send request
    let result = match send_rpc_request(&client, &rpc_url, &payload, &mut shutdown_rx).await {
        Ok(res) => res,
        Err(e) if e.to_string().contains("Shutdown requested") => {
            info!("Bootstrap shutting down before request");
            return Ok(());
        }
        Err(e) => {
            error!("Bootstrap failed: {:?}", e);
            return Err(e);
        }
    };

    // Standard API returns array directly
    let accounts_array = match result.as_array() {
        Some(arr) => arr,
        None => {
            error!("Bootstrap could not find accounts array");
            return Err(anyhow::anyhow!("Could not find accounts array"));
        }
    };

    let total_fetched = accounts_array.len();
    let mut total_inserted = 0;

    info!("Bootstrap fetched {} total accounts", total_fetched);

    // Process each account
    for account_value in accounts_array {
        if let Ok(true) = process_account(account_value, &tracker) {
            total_inserted += 1;
        }
    }

    info!(
        "Bootstrap complete: {} total fetched, {} compressible accounts inserted",
        total_fetched, total_inserted
    );

    Ok(())
}
