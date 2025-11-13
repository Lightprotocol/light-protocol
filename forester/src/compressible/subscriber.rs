use std::{str::FromStr, sync::Arc};

use futures::StreamExt;
use light_ctoken_types::{COMPRESSED_TOKEN_PROGRAM_ID, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_response::{Response as RpcResponse, RpcKeyedAccount},
};
use solana_rpc_client_api::filter::RpcFilterType;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio::sync::oneshot;
use tracing::{debug, error, info};

use super::state::CompressibleAccountTracker;
use crate::Result;

/// Subscribes to account changes for all compressible CToken accounts
pub struct AccountSubscriber {
    ws_url: String,
    tracker: Arc<CompressibleAccountTracker>,
    shutdown_rx: oneshot::Receiver<()>,
}

impl AccountSubscriber {
    pub fn new(
        ws_url: String,
        tracker: Arc<CompressibleAccountTracker>,
        shutdown_rx: oneshot::Receiver<()>,
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

        let program_id = Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID);

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
                Some(response) = subscription.next() => {
                    self.handle_account_update(response).await;
                }
                _ = &mut self.shutdown_rx => {
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
                solana_account_decoder::UiAccountEncoding::Base64 => {
                    match base64::engine::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        data,
                    ) {
                        Ok(decoded) => decoded,
                        Err(e) => {
                            error!("Failed to decode base64 for {}: {}", pubkey, e);
                            return;
                        }
                    }
                }
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
