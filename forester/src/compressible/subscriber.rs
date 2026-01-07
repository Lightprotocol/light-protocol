use std::{str::FromStr, sync::Arc};

use futures::StreamExt;
use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_response::{Response as RpcResponse, RpcKeyedAccount},
};
use solana_rpc_client_api::filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use super::{
    config::{ACCOUNT_TYPE_OFFSET, CTOKEN_ACCOUNT_TYPE_FILTER, MINT_ACCOUNT_TYPE_FILTER},
    traits::SubscriptionHandler,
};
use crate::Result;

/// Configuration for a program subscription
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Program ID to subscribe to
    pub program_id: Pubkey,
    /// Optional memcmp filter (offset and base58-encoded bytes)
    /// None means no filter (subscribe to all accounts)
    pub filter: Option<MemcmpFilter>,
    /// Human-readable name for logging
    pub name: String,
}

/// Memcmp filter configuration
#[derive(Debug, Clone)]
pub struct MemcmpFilter {
    pub offset: usize,
    pub bytes: String, // Base58-encoded
}

impl SubscriptionConfig {
    /// Create subscription config for Light Token accounts (ctokens)
    pub fn ctoken() -> Self {
        Self {
            program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            filter: Some(MemcmpFilter {
                offset: ACCOUNT_TYPE_OFFSET,
                bytes: CTOKEN_ACCOUNT_TYPE_FILTER.to_string(),
            }),
            name: "ctoken".to_string(),
        }
    }

    /// Create subscription config for Light Mint accounts
    pub fn mint() -> Self {
        Self {
            program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            filter: Some(MemcmpFilter {
                offset: ACCOUNT_TYPE_OFFSET,
                bytes: MINT_ACCOUNT_TYPE_FILTER.to_string(),
            }),
            name: "mint".to_string(),
        }
    }

    /// Create subscription config for a PDA program with discriminator filter.
    /// The discriminator is an 8-byte value at the start of the account data (offset 0).
    pub fn pda(program_id: Pubkey, discriminator: [u8; 8], name: String) -> Self {
        // Convert discriminator to base58 for the memcmp filter
        let discriminator_base58 = bs58::encode(&discriminator).into_string();

        Self {
            program_id,
            filter: Some(MemcmpFilter {
                offset: 0,
                bytes: discriminator_base58,
            }),
            name,
        }
    }
}

/// Generic subscriber for account changes.
/// Works with any tracker that implements SubscriptionHandler.
pub struct AccountSubscriber<H: SubscriptionHandler> {
    ws_url: String,
    handler: Arc<H>,
    config: SubscriptionConfig,
    shutdown_rx: broadcast::Receiver<()>,
}

impl<H: SubscriptionHandler + 'static> AccountSubscriber<H> {
    pub fn new(
        ws_url: String,
        handler: Arc<H>,
        config: SubscriptionConfig,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            ws_url,
            handler,
            config,
            shutdown_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Starting {} account subscriber at {}",
            self.config.name, self.ws_url
        );

        // Connect to WebSocket
        let pubsub_client = PubsubClient::new(&self.ws_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to WebSocket: {}", e))?;

        // Build filters based on config
        let filters = self.config.filter.as_ref().map(|f| {
            vec![RpcFilterType::Memcmp(Memcmp::new(
                f.offset,
                MemcmpEncodedBytes::Base58(f.bytes.clone()),
            ))]
        });

        let (mut subscription, unsubscribe) = pubsub_client
            .program_subscribe(
                &self.config.program_id,
                Some(RpcProgramAccountsConfig {
                    filters,
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
            "{} subscription established for program {}",
            self.config.name, self.config.program_id
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
                            error!("{} subscription stream closed unexpectedly", self.config.name);
                            unsubscribe().await;
                            return Err(anyhow::anyhow!("{} subscription stream closed", self.config.name));
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received for {} subscriber", self.config.name);
                    unsubscribe().await;
                    break;
                }
            }
        }

        info!("{} subscriber stopped", self.config.name);
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

        // Call handler
        match self.handler.handle_update(
            pubkey,
            self.config.program_id,
            &account_data,
            response.value.account.lamports,
        ) {
            Ok(()) => {
                debug!(
                    "Updated {} account {} at slot {}",
                    self.config.name, pubkey, response.context.slot
                );
            }
            Err(e) => {
                error!(
                    "Failed to update {} tracker for {}: {}",
                    self.config.name, pubkey, e
                );
            }
        }
    }
}
