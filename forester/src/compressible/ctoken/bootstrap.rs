use std::{sync::Arc, time::Duration};

use borsh::BorshDeserialize;
use light_token_interface::{state::Token, LIGHT_TOKEN_PROGRAM_ID};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::oneshot;
use tracing::{debug, info};

use super::state::CTokenAccountTracker;
use crate::{
    compressible::{
        bootstrap_helpers::{
            bootstrap_standard_api, bootstrap_v2_api, use_helius_rpc, RawAccountData,
        },
        config::{ACCOUNT_TYPE_OFFSET, CTOKEN_ACCOUNT_TYPE_FILTER},
    },
    Result,
};

/// Bootstrap the CToken account tracker by fetching existing accounts
/// Uses standard getProgramAccounts for localhost, getProgramAccountsV2 for remote networks
pub async fn bootstrap_ctoken_accounts(
    rpc_url: String,
    tracker: Arc<CTokenAccountTracker>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    helius_rpc: bool,
) -> Result<()> {
    info!("Starting bootstrap of CToken accounts");

    let program_id = Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);

    // Set up shutdown flag
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

    if let Some(rx) = shutdown_rx {
        let shutdown_flag_clone = shutdown_flag.clone();
        tokio::spawn(async move {
            let _ = rx.await;
            shutdown_flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });
    }

    // Filter for decompressed CToken accounts (account_type = 2)
    let filters = vec![json!({
        "memcmp": {
            "offset": ACCOUNT_TYPE_OFFSET,
            "bytes": CTOKEN_ACCOUNT_TYPE_FILTER,
            "encoding": "base58"
        }
    })];

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Process function that deserializes Token and updates tracker
    let process_account = |raw_data: RawAccountData| -> bool {
        // Deserialize Token
        let ctoken = match Token::try_from_slice(&raw_data.data) {
            Ok(token) => token,
            Err(e) => {
                debug!(
                    "Failed to deserialize Token for account {}: {:?}",
                    raw_data.pubkey, e
                );
                return false;
            }
        };

        // Check if account is a valid Token account (account_type == 2)
        if !ctoken.is_token_account() {
            debug!(
                "Skipping account {} - not a token account (is_token_account() == false)",
                raw_data.pubkey
            );
            return false;
        }

        // Use tracker's update_from_token to avoid re-deserializing the Token
        let account_size = raw_data.data.len();
        if let Err(e) =
            tracker.update_from_token(raw_data.pubkey, ctoken, raw_data.lamports, account_size)
        {
            debug!("Failed to insert account {}: {:?}", raw_data.pubkey, e);
            return false;
        }

        true
    };

    if !use_helius_rpc(&rpc_url, helius_rpc) {
        info!("Using standard getProgramAccounts");
        let (total_fetched, total_inserted) = bootstrap_standard_api(
            &client,
            &rpc_url,
            &program_id,
            Some(filters),
            Some(&shutdown_flag),
            process_account,
        )
        .await?;

        info!(
            "Bootstrap complete: {} total fetched, {} CToken accounts inserted",
            total_fetched, total_inserted
        );
    } else {
        info!("Using getProgramAccountsV2 with pagination");
        let (page_count, total_fetched, total_inserted) = bootstrap_v2_api(
            &client,
            &rpc_url,
            &program_id,
            Some(filters),
            Some(&shutdown_flag),
            process_account,
        )
        .await?;

        info!(
            "Bootstrap finished: {} pages, {} total fetched, {} CToken accounts inserted",
            page_count, total_fetched, total_inserted
        );
    }

    Ok(())
}
