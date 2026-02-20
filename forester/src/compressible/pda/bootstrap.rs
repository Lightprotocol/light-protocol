use std::{sync::Arc, time::Duration};

use tokio::sync::oneshot;
use tracing::{debug, error, info};

use super::state::PdaAccountTracker;
use crate::{
    compressible::{
        bootstrap_helpers::{
            bootstrap_standard_api, bootstrap_v2_api, use_helius_rpc, RawAccountData,
        },
        config::PdaProgramConfig,
        traits::CompressibleTracker,
    },
    Result,
};

/// Bootstrap the PDA account tracker by fetching existing accounts for all configured programs
pub async fn bootstrap_pda_accounts(
    rpc_url: String,
    tracker: Arc<PdaAccountTracker>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    helius_rpc: bool,
) -> Result<()> {
    info!("Starting bootstrap of compressible PDA accounts");

    let programs = tracker.programs().to_vec();
    if programs.is_empty() {
        info!("No PDA programs configured, skipping PDA bootstrap");
        return Ok(());
    }

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
        .build()?;

    for program_config in programs {
        if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Shutdown requested, stopping PDA bootstrap");
            break;
        }

        info!(
            "Bootstrapping PDA accounts for program {}",
            program_config.program_id
        );

        let result = bootstrap_program(
            &client,
            &rpc_url,
            &tracker,
            &program_config,
            &shutdown_flag,
            helius_rpc,
        )
        .await;

        if let Err(e) = result {
            error!(
                "Failed to bootstrap program {}: {:?}",
                program_config.program_id, e
            );
            // Continue with other programs
        }
    }

    info!(
        "PDA bootstrap finished: {} total accounts tracked",
        tracker.len()
    );

    Ok(())
}

/// Bootstrap a single program's accounts
async fn bootstrap_program(
    client: &reqwest::Client,
    rpc_url: &str,
    tracker: &PdaAccountTracker,
    program_config: &PdaProgramConfig,
    shutdown_flag: &std::sync::atomic::AtomicBool,
    helius_rpc: bool,
) -> Result<()> {
    let program_id = &program_config.program_id;

    // Process function that updates tracker
    let process_account = |raw_data: RawAccountData| -> bool {
        if let Err(e) = tracker.update_from_account(
            raw_data.pubkey,
            *program_id,
            &raw_data.data,
            raw_data.lamports,
        ) {
            debug!("Failed to insert account {}: {:?}", raw_data.pubkey, e);
            return false;
        }
        true
    };

    // Build memcmp filter for discriminator at offset 0
    let discriminator_base58 = bs58::encode(&program_config.discriminator).into_string();
    let filters = Some(vec![serde_json::json!({
        "memcmp": {
            "offset": 0,
            "bytes": discriminator_base58,
            "encoding": "base58"
        }
    })]);

    if !use_helius_rpc(rpc_url, helius_rpc) {
        let (total_fetched, total_inserted) = bootstrap_standard_api(
            client,
            rpc_url,
            program_id,
            filters,
            Some(shutdown_flag),
            process_account,
        )
        .await?;

        info!(
            "Bootstrap complete for program {}: {} fetched, {} compressible",
            program_id, total_fetched, total_inserted
        );
    } else {
        let (page_count, total_fetched, total_inserted) = bootstrap_v2_api(
            client,
            rpc_url,
            program_id,
            filters,
            Some(shutdown_flag),
            process_account,
        )
        .await?;

        info!(
            "Bootstrap finished for program {}: {} pages, {} fetched, {} compressible",
            program_id, page_count, total_fetched, total_inserted
        );
    }

    Ok(())
}
