use std::{sync::Arc, time::Duration};

use tokio::sync::oneshot;
use tracing::{debug, info};

use super::state::MintAccountTracker;
use crate::{
    compressible::{
        bootstrap_helpers::{
            bootstrap_standard_api, bootstrap_v2_api, is_localhost, RawAccountData,
        },
        config::{ACCOUNT_TYPE_OFFSET, MINT_ACCOUNT_TYPE_FILTER},
        traits::CompressibleTracker,
    },
    Result,
};

/// Bootstrap the Mint account tracker by fetching decompressed mints
pub async fn bootstrap_mint_accounts(
    rpc_url: String,
    tracker: Arc<MintAccountTracker>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
) -> Result<()> {
    info!("Starting bootstrap of decompressed Mint accounts");

    // Set up shutdown flag
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

    if let Some(rx) = shutdown_rx {
        let shutdown_flag_clone = shutdown_flag.clone();
        tokio::spawn(async move {
            let _ = rx.await;
            shutdown_flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    // Light Token Program ID
    let program_id =
        solana_sdk::pubkey::Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);

    info!(
        "Bootstrapping decompressed Mint accounts from program {}",
        program_id
    );

    // Process function that updates tracker
    let process_account = |raw_data: RawAccountData| -> bool {
        if let Err(e) =
            tracker.update_from_account(raw_data.pubkey, &raw_data.data, raw_data.lamports)
        {
            debug!("Failed to insert mint {}: {:?}", raw_data.pubkey, e);
            return false;
        }
        true
    };

    // Filter for decompressed Mint accounts (account_type = 1)
    let filters = Some(vec![serde_json::json!({
        "memcmp": {
            "offset": ACCOUNT_TYPE_OFFSET,
            "bytes": MINT_ACCOUNT_TYPE_FILTER,
            "encoding": "base58"
        }
    })]);

    if is_localhost(&rpc_url) {
        let (total_fetched, total_inserted) = bootstrap_standard_api(
            &client,
            &rpc_url,
            &program_id,
            filters,
            Some(&shutdown_flag),
            process_account,
        )
        .await?;

        info!(
            "Mint bootstrap complete: {} fetched, {} decompressed mints tracked",
            total_fetched, total_inserted
        );
    } else {
        let (page_count, total_fetched, total_inserted) = bootstrap_v2_api(
            &client,
            &rpc_url,
            &program_id,
            filters,
            Some(&shutdown_flag),
            process_account,
        )
        .await?;

        info!(
            "Mint bootstrap finished: {} pages, {} fetched, {} decompressed mints tracked",
            page_count, total_fetched, total_inserted
        );
    }

    info!(
        "Mint bootstrap finished: {} total mints tracked",
        tracker.len()
    );

    Ok(())
}
