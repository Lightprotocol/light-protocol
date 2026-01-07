//! Shared bootstrap helpers for fetching program accounts from RPC.
//!
//! This module provides common functionality used by both token and PDA bootstrap:
//! - RPC request sending with shutdown handling and timeout
//! - Account field extraction from JSON responses
//! - Standard and V2 API patterns

use std::time::Duration;

use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use tokio::time::timeout;
use tracing::debug;

use super::config::{DEFAULT_PAGE_SIZE, DEFAULT_PAGINATION_DELAY_MS};
use crate::Result;

const RPC_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Re-export page size for use in other modules
pub const PAGE_SIZE: usize = DEFAULT_PAGE_SIZE;

/// Raw account data extracted from RPC response
pub struct RawAccountData {
    pub pubkey: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
}

pub async fn send_rpc_request(
    client: &reqwest::Client,
    rpc_url: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value> {
    let result = timeout(RPC_REQUEST_TIMEOUT, async {
        let response = client
            .post(rpc_url)
            .json(payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {:?}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        let json_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;

        // Check for RPC error
        if let Some(error) = json_response.get("error") {
            return Err(anyhow::anyhow!("RPC error: {:?}", error));
        }

        json_response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unexpected response format"))
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(anyhow::anyhow!(
            "RPC request timed out after {}s",
            RPC_REQUEST_TIMEOUT.as_secs()
        )),
    }
}

/// Extract raw account data from a JSON account value
/// Returns None if account should be skipped (missing fields, closed, etc.)
pub fn extract_account_fields(account_value: &serde_json::Value) -> Option<RawAccountData> {
    // Extract pubkey
    let pubkey_str = account_value.get("pubkey").and_then(|v| v.as_str())?;
    let pubkey = match pubkey_str.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(e) => {
            debug!("Failed to parse pubkey {}: {:?}", pubkey_str, e);
            return None;
        }
    };

    // Extract account data
    let account_obj = account_value.get("account")?;

    // Check lamports - skip closed accounts (lamports == 0)
    let lamports = match account_obj.get("lamports").and_then(|v| v.as_u64()) {
        Some(0) => {
            debug!("Skipping closed account {} (lamports == 0)", pubkey);
            return None;
        }
        Some(lamports) => lamports,
        None => {
            debug!("Skipping account {} with missing lamports field", pubkey);
            return None;
        }
    };

    let data_array = match account_obj.get("data").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            debug!("Skipping account {} with missing or empty data", pubkey);
            return None;
        }
    };

    let data_str = data_array.first()?.as_str()?;
    let data = match base64::decode(data_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            debug!("Failed to decode base64 for account {}: {:?}", pubkey, e);
            return None;
        }
    };

    Some(RawAccountData {
        pubkey,
        lamports,
        data,
    })
}

/// Fetch current slot from RPC with timeout
pub async fn get_current_slot(client: &reqwest::Client, rpc_url: &str) -> Result<u64> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "getSlot",
        "params": [{"commitment": "confirmed"}]
    });

    let result = timeout(RPC_REQUEST_TIMEOUT, async {
        let response = client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get slot: {:?}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse slot response: {:?}", e))?;

        json.get("result")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract slot from response"))
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(anyhow::anyhow!(
            "getSlot request timed out after {}s",
            RPC_REQUEST_TIMEOUT.as_secs()
        )),
    }
}

/// Extract accounts array from V2 API response (handles various response formats)
pub fn extract_accounts_array(result: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    // Try different possible locations
    if let Some(arr) = result.get("accounts").and_then(|v| v.as_array()) {
        return Some(arr);
    }
    if let Some(arr) = result.as_array() {
        return Some(arr);
    }
    if let Some(value) = result.get("value").and_then(|v| v.as_array()) {
        return Some(value);
    }
    None
}

/// Extract pagination cursor from response
pub fn extract_pagination_cursor(result: &serde_json::Value) -> Option<String> {
    result
        .get("paginationKey")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// Build payload for standard getProgramAccounts request
pub fn build_standard_api_payload(
    program_id: &Pubkey,
    filters: Option<Vec<serde_json::Value>>,
) -> serde_json::Value {
    let mut params = json!({
        "encoding": "base64",
        "commitment": "confirmed"
    });

    if let Some(filters) = filters {
        params["filters"] = json!(filters);
    }

    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [program_id.to_string(), params]
    })
}

/// Build payload for V2 getProgramAccountsV2 request with pagination
pub fn build_v2_api_payload(
    program_id: &Pubkey,
    page_id: i32,
    cursor: Option<&str>,
    filters: Option<Vec<serde_json::Value>>,
) -> serde_json::Value {
    let mut params = json!({
        "encoding": "base64",
        "commitment": "confirmed",
        "limit": PAGE_SIZE
    });

    if let Some(filters) = filters {
        params["filters"] = json!(filters);
    }

    if let Some(c) = cursor {
        params["paginationKey"] = json!(c);
    }

    json!({
        "jsonrpc": "2.0",
        "id": page_id,
        "method": "getProgramAccountsV2",
        "params": [program_id.to_string(), params]
    })
}

/// Check if URL is localhost
pub fn is_localhost(rpc_url: &str) -> bool {
    rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1")
}

/// Generic bootstrap using standard getProgramAccounts API
///
/// Calls `process_fn` for each account that passes initial extraction.
/// Returns (total_fetched, total_inserted) counts.
pub async fn bootstrap_standard_api<F>(
    client: &reqwest::Client,
    rpc_url: &str,
    program_id: &Pubkey,
    filters: Option<Vec<serde_json::Value>>,
    shutdown_flag: Option<&std::sync::atomic::AtomicBool>,
    mut process_fn: F,
) -> Result<(usize, usize)>
where
    F: FnMut(RawAccountData) -> bool,
{
    let payload = build_standard_api_payload(program_id, filters);
    let result = send_rpc_request(client, rpc_url, &payload).await?;

    let accounts_array = result
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Could not find accounts array"))?;

    let total_fetched = accounts_array.len();
    let mut total_inserted = 0;

    for account_value in accounts_array {
        if let Some(flag) = shutdown_flag {
            if flag.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        }

        if let Some(raw_data) = extract_account_fields(account_value) {
            if process_fn(raw_data) {
                total_inserted += 1;
            }
        }
    }

    Ok((total_fetched, total_inserted))
}

/// Generic bootstrap using V2 getProgramAccountsV2 API with pagination
///
/// Calls `process_fn` for each account that passes initial extraction.
/// Returns (total_pages, total_fetched, total_inserted) counts.
pub async fn bootstrap_v2_api<F>(
    client: &reqwest::Client,
    rpc_url: &str,
    program_id: &Pubkey,
    filters: Option<Vec<serde_json::Value>>,
    shutdown_flag: Option<&std::sync::atomic::AtomicBool>,
    mut process_fn: F,
) -> Result<(usize, usize, usize)>
where
    F: FnMut(RawAccountData) -> bool,
{
    let mut total_fetched = 0;
    let mut total_inserted = 0;
    let mut page_count = 0;
    let mut cursor: Option<String> = None;

    // Build the base payload once before the loop to avoid cloning filters on each iteration.
    // We'll update only the page id and pagination cursor per iteration.
    let mut payload = build_v2_api_payload(program_id, 1, None, filters);

    loop {
        if let Some(flag) = shutdown_flag {
            if flag.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        }

        page_count += 1;

        // Update only the page-specific fields
        payload["id"] = json!(page_count as i32);
        if let Some(ref c) = cursor {
            payload["params"][1]["paginationKey"] = json!(c);
        }

        let result = send_rpc_request(client, rpc_url, &payload).await?;

        let accounts_array = extract_accounts_array(&result)
            .ok_or_else(|| anyhow::anyhow!("Could not find accounts array"))?;

        let accounts_count = accounts_array.len();
        if accounts_count == 0 {
            debug!(
                "Pagination returned 0 accounts on page {}, ending pagination",
                page_count
            );
            break;
        }

        total_fetched += accounts_count;

        for account_value in accounts_array {
            if let Some(flag) = shutdown_flag {
                if flag.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
            }

            if let Some(raw_data) = extract_account_fields(account_value) {
                if process_fn(raw_data) {
                    total_inserted += 1;
                }
            }
        }

        // Get cursor for next page
        cursor = extract_pagination_cursor(&result);
        if cursor.is_none() {
            break;
        }

        // Rate limit between paginated requests
        tokio::time::sleep(tokio::time::Duration::from_millis(
            DEFAULT_PAGINATION_DELAY_MS,
        ))
        .await;
    }

    Ok((page_count, total_fetched, total_inserted))
}
