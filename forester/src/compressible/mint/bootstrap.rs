use std::sync::Arc;

use tokio::sync::oneshot;
use tracing::{debug, info};

use super::state::MintAccountTracker;
use crate::{
    compressible::{
        bootstrap_helpers::{run_bootstrap, RawAccountData},
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
    // Light Token Program ID
    let program_id =
        solana_sdk::pubkey::Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);

    // Filter for decompressed Mint accounts (account_type = 1)
    let filters = Some(vec![serde_json::json!({
        "memcmp": {
            "offset": ACCOUNT_TYPE_OFFSET,
            "bytes": MINT_ACCOUNT_TYPE_FILTER,
            "encoding": "base58"
        }
    })]);

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

    let result = run_bootstrap(
        &rpc_url,
        &program_id,
        filters,
        shutdown_rx,
        process_account,
        "Mint",
    )
    .await?;

    info!(
        "Mint bootstrap finished: {} total mints tracked (fetched: {}, inserted: {}, pages: {})",
        tracker.len(),
        result.fetched,
        result.inserted,
        result.pages
    );

    Ok(())
}
