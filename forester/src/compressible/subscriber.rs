use std::{str::FromStr, sync::Arc, time::Duration};

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
use tracing::{debug, error, info, warn};

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

/// Configuration for WebSocket reconnection with exponential backoff
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before first reconnection attempt
    pub initial_delay: Duration,
    /// Maximum delay between reconnection attempts
    pub max_delay: Duration,
    /// Multiplier for exponential backoff (e.g., 2.0 doubles delay each attempt)
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }
}

/// Result of a single connection session
enum ConnectionResult {
    /// Shutdown signal received
    Shutdown,
    /// Stream closed unexpectedly (should reconnect)
    StreamClosed,
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
/// Automatically reconnects with exponential backoff on connection loss.
pub struct AccountSubscriber<H: SubscriptionHandler> {
    ws_url: String,
    handler: Arc<H>,
    config: SubscriptionConfig,
    reconnect_config: ReconnectConfig,
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
            reconnect_config: ReconnectConfig::default(),
            shutdown_rx,
        }
    }

    pub fn with_reconnect_config(mut self, reconnect_config: ReconnectConfig) -> Self {
        self.reconnect_config = reconnect_config;
        self
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Starting {} account subscriber at {}",
            self.config.name, self.ws_url
        );

        let mut current_delay = self.reconnect_config.initial_delay;
        let mut attempt: u32 = 0;

        loop {
            match self.run_connection().await {
                Ok(ConnectionResult::Shutdown) => {
                    info!("{} subscriber stopped", self.config.name);
                    return Ok(());
                }
                Ok(ConnectionResult::StreamClosed) => {
                    attempt += 1;
                    warn!(
                        "{} connection lost (attempt {}), reconnecting in {:?}...",
                        self.config.name, attempt, current_delay
                    );
                }
                Err(e) => {
                    attempt += 1;
                    warn!(
                        "{} connection error (attempt {}): {:?}, reconnecting in {:?}...",
                        self.config.name, attempt, e, current_delay
                    );
                }
            }

            // Wait with backoff, but check for shutdown signal
            tokio::select! {
                _ = tokio::time::sleep(current_delay) => {}
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received for {} subscriber during reconnect backoff", self.config.name);
                    return Ok(());
                }
            }

            // Exponential backoff
            current_delay = Duration::from_secs_f64(
                (current_delay.as_secs_f64() * self.reconnect_config.backoff_multiplier)
                    .min(self.reconnect_config.max_delay.as_secs_f64()),
            );
        }
    }

    /// Runs a single connection session. Returns when:
    /// - Shutdown signal received (Ok(Shutdown))
    /// - Stream closed unexpectedly (Ok(StreamClosed))
    /// - Connection/subscription error (Err)
    async fn run_connection(&mut self) -> Result<ConnectionResult> {
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
                            return Ok(ConnectionResult::StreamClosed);
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received for {} subscriber", self.config.name);
                    unsubscribe().await;
                    return Ok(ConnectionResult::Shutdown);
                }
            }
        }
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
