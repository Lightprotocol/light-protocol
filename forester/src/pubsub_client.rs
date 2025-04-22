use std::{str::FromStr, thread};

use futures::StreamExt;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio::{runtime::Builder, sync::mpsc};
use tracing::{debug, error};

use crate::{
    errors::{ChannelError, ForesterError},
    queue_helpers::QueueUpdate,
    ForesterConfig, Result,
};

pub async fn setup_pubsub_client(
    config: &ForesterConfig,
    queue_pubkeys: std::collections::HashSet<Pubkey>,
) -> Result<(mpsc::Receiver<QueueUpdate>, mpsc::Sender<()>)> {
    let ws_url = config
        .external_services
        .ws_rpc_url
        .as_ref()
        .ok_or_else(|| {
            ForesterError::Channel(ChannelError::General {
                error: "PubSub client requires a WebSocket URL".to_string(),
            })
        })?
        .clone();

    debug!(
        "Setting up pubsub client for {} queues",
        queue_pubkeys.len()
    );
    let (update_tx, update_rx) = mpsc::channel(100);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    let handle = spawn_pubsub_client(ws_url, queue_pubkeys, update_tx, shutdown_rx);

    tokio::spawn(async move {
        match handle.join() {
            Ok(result) => {
                if let Err(e) = result {
                    error!("PubSub client error: {:?}", e);
                } else {
                    debug!("PubSub client thread completed successfully");
                }
            }
            Err(e) => error!("Failed to join PubSub client thread: {:?}", e),
        }
    });

    Ok((update_rx, shutdown_tx))
}

fn spawn_pubsub_client(
    ws_url: String,
    queue_pubkeys: std::collections::HashSet<Pubkey>,
    update_tx: mpsc::Sender<QueueUpdate>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> thread::JoinHandle<Result<()>> {
    thread::spawn(move || {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                ForesterError::Channel(ChannelError::General {
                    error: format!("Failed to build runtime: {}", e),
                })
            })?;

        rt.block_on(async {
            debug!("Connecting to PubSub at {}", ws_url);
            let pubsub_client = PubsubClient::new(&ws_url)
                .await
                .map_err(|e| ForesterError::Channel(ChannelError::General {
                    error: format!("Failed to create PubsubClient: {}", e)
                }))?;

            debug!("PubSub connection established");

            let (mut subscription, _) = pubsub_client
                .program_subscribe(
                    &account_compression::id(),
                    Some(RpcProgramAccountsConfig {
                        sort_results: None,
                        filters: None,
                        account_config: RpcAccountInfoConfig {
                            encoding: Some(UiAccountEncoding::Base64),
                            commitment: Some(CommitmentConfig::confirmed()),
                            data_slice: None,
                            min_context_slot: None,
                        },
                        with_context: Some(true),
                    }),
                )
                .await
                .map_err(|e| ForesterError::Channel(ChannelError::General {
                    error: format!("Failed to subscribe to program: {}", e)
                }))?;

            loop {
                tokio::select! {
                    Some(update) = subscription.next() => {
                        if let Ok(pubkey) = Pubkey::from_str(&update.value.pubkey) {
                            if queue_pubkeys.contains(&pubkey) {
                                debug!("Received update for queue {}", pubkey);
                                 if update_tx.send(QueueUpdate {
                                        pubkey,
                                        slot: update.context.slot,
                                    }).await.is_err() {
                                    debug!("Failed to send update, receiver might have been dropped");
                                    break;
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Received shutdown signal");
                        break;
                    }
                }
            }
            debug!("PubSub client loop ended");
            Ok(())
        })
    })
}
