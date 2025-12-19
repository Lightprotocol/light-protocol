use std::{str::FromStr, sync::Arc};

use futures::StreamExt;
use light_ctoken_interface::{COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, CTOKEN_PROGRAM_ID};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{
        RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcTransactionLogsConfig,
        RpcTransactionLogsFilter,
    },
    rpc_response::{Response as RpcResponse, RpcKeyedAccount, RpcLogsResponse},
};
use solana_rpc_client_api::filter::RpcFilterType;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use super::state::CompressibleAccountTracker;
use crate::Result;

/// Registry program ID for subscribing to compress_and_close logs
const REGISTRY_PROGRAM_ID_STR: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";

/// Log prefix emitted by registry program when closing accounts
const COMPRESS_AND_CLOSE_LOG_PREFIX: &str = "compress_and_close:";

/// Subscribes to account changes for all compressible CToken accounts
pub struct AccountSubscriber {
    ws_url: String,
    tracker: Arc<CompressibleAccountTracker>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl AccountSubscriber {
    pub fn new(
        ws_url: String,
        tracker: Arc<CompressibleAccountTracker>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            ws_url,
            tracker,
            shutdown_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting account subscriber at {}", self.ws_url);

        // Connect to WebSocket
        let pubsub_client = PubsubClient::new(&self.ws_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to WebSocket: {}", e))?;

        let program_id = Pubkey::new_from_array(CTOKEN_PROGRAM_ID);
        // Subscribe to compressed token program accounts with filter for compressible account size
        let (mut subscription, unsubscribe) = pubsub_client
            .program_subscribe(
                &program_id,
                Some(RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::DataSize(
                        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                    )]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        commitment: Some(CommitmentConfig::confirmed()),
                        data_slice: None,
                        min_context_slot: None,
                    },
                    with_context: Some(true),
                    sort_results: None,
                }),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to program accounts: {}", e))?;

        info!(
            "Account subscription established for program {}",
            program_id
        );

        // Process subscription messages
        loop {
            tokio::select! {
                result = subscription.next() => {
                    match result {
                        Some(response) => {
                            self.handle_account_update(response).await;
                        }
                        None => {
                            error!("Account subscription stream closed unexpectedly");
                            unsubscribe().await;
                            return Err(anyhow::anyhow!("Account subscription stream closed"));
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    unsubscribe().await;
                    break;
                }
            }
        }

        info!("Account subscriber stopped");
        Ok(())
    }

    async fn handle_account_update(&self, response: RpcResponse<RpcKeyedAccount>) {
        // Parse pubkey
        let pubkey = match Pubkey::from_str(&response.value.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                error!("Invalid pubkey {}: {}", response.value.pubkey, e);
                return;
            }
        };

        // Decode Base64 account data
        use solana_account_decoder::UiAccountData;
        let account_data = match &response.value.account.data {
            UiAccountData::Binary(data, encoding) => match encoding {
                solana_account_decoder::UiAccountEncoding::Base64 => match base64::decode(data) {
                    Ok(decoded) => decoded,
                    Err(e) => {
                        error!("Failed to decode base64 for {}: {}", pubkey, e);
                        return;
                    }
                },
                _ => {
                    error!("Unexpected encoding for account {}", pubkey);
                    return;
                }
            },
            _ => {
                error!("Unexpected account data format for {}", pubkey);
                return;
            }
        };

        // Update tracker
        match self.tracker.update_from_account(
            pubkey,
            &account_data,
            response.value.account.lamports,
        ) {
            Ok(()) => {
                debug!(
                    "Updated account {} at slot {}",
                    pubkey, response.context.slot
                );
            }
            Err(e) => {
                error!("Failed to update tracker for {}: {}", pubkey, e);
            }
        }
    }
}

/// Subscribes to registry program logs to detect compress_and_close operations
/// and remove closed accounts from the tracker by parsing log messages directly
pub struct LogSubscriber {
    ws_url: String,
    tracker: Arc<CompressibleAccountTracker>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl LogSubscriber {
    pub fn new(
        ws_url: String,
        tracker: Arc<CompressibleAccountTracker>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            ws_url,
            tracker,
            shutdown_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting log subscriber at {}", self.ws_url);

        // Connect to WebSocket
        let pubsub_client = PubsubClient::new(&self.ws_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to WebSocket: {}", e))?;

        let registry_program_id = Pubkey::from_str(REGISTRY_PROGRAM_ID_STR)
            .map_err(|e| anyhow::anyhow!("Invalid registry program ID: {}", e))?;

        // Subscribe to logs mentioning the registry program
        let filter = RpcTransactionLogsFilter::Mentions(vec![registry_program_id.to_string()]);
        let config = RpcTransactionLogsConfig {
            commitment: Some(CommitmentConfig::confirmed()),
        };

        let (mut subscription, unsubscribe) = pubsub_client
            .logs_subscribe(filter, config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to logs: {}", e))?;

        info!(
            "Log subscription established for registry program {}",
            registry_program_id
        );

        // Process subscription messages
        loop {
            tokio::select! {
                result = subscription.next() => {
                    match result {
                        Some(response) => {
                            self.handle_log_notification(response);
                        }
                        None => {
                            error!("Log subscription stream closed unexpectedly");
                            unsubscribe().await;
                            return Err(anyhow::anyhow!("Log subscription stream closed"));
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received for log subscriber");
                    unsubscribe().await;
                    break;
                }
            }
        }

        info!("Log subscriber stopped");
        Ok(())
    }

    fn handle_log_notification(&self, response: RpcResponse<RpcLogsResponse>) {
        let logs_response = response.value;

        // Skip failed transactions
        if logs_response.err.is_some() {
            debug!("Skipping failed transaction {}", logs_response.signature);
            return;
        }

        // Parse logs looking for compress_and_close entries
        let mut removed_count = 0;
        for log in &logs_response.logs {
            // Look for our log prefix: "Program log: compress_and_close:<pubkey>"
            // The actual log format is "Program log: compress_and_close:<pubkey>"
            if let Some(pubkey_str) = log
                .strip_prefix("Program log: ")
                .and_then(|s| s.strip_prefix(COMPRESS_AND_CLOSE_LOG_PREFIX))
            {
                match Pubkey::from_str(pubkey_str) {
                    Ok(pubkey) => {
                        if self.tracker.remove(&pubkey).is_some() {
                            debug!(
                                "Removed closed account {} from tracker (compress_and_close log)",
                                pubkey
                            );
                            removed_count += 1;
                        }
                    }
                    Err(e) => {
                        error!(
                            "Invalid pubkey in compress_and_close log '{}': {}",
                            pubkey_str, e
                        );
                    }
                }
            }
        }

        if removed_count > 0 {
            info!(
                "Removed {} closed accounts from transaction {}",
                removed_count, logs_response.signature
            );
        }
    }
}
