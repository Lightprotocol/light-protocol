use std::sync::Arc;

use clap::Parser;
use forester::{
    cli::{Cli, Commands},
    errors::ForesterError,
    forester_status,
    metrics::register_metrics,
    run_pipeline,
    telemetry::setup_telemetry,
    ForesterConfig,
};
use forester_utils::rate_limiter::RateLimiter;
use light_client::{indexer::photon_indexer::PhotonIndexer, rpc::SolanaRpcConnection};
use tokio::{
    signal::ctrl_c,
    sync::{mpsc, oneshot},
};
use tracing::debug;

#[tokio::main]
async fn main() -> Result<(), ForesterError> {
    setup_telemetry();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Start(args) => {
            let config = Arc::new(ForesterConfig::new_for_start(args)?);

            if config.general_config.enable_metrics {
                register_metrics();
            }

            let (shutdown_sender, shutdown_receiver) = oneshot::channel();
            let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

            tokio::spawn(async move {
                ctrl_c().await.expect("Failed to listen for Ctrl+C");
                shutdown_sender
                    .send(())
                    .expect("Failed to send shutdown signal");
            });

            tokio::spawn(async move {
                while let Some(report) = work_report_receiver.recv().await {
                    debug!("Work Report: {:?}", report);
                }
            });

            let mut rpc_rate_limiter = None;
            if let Some(rate_limit) = config.external_services.rpc_rate_limit {
                rpc_rate_limiter = Some(RateLimiter::new(rate_limit));
            }

            let mut send_tx_limiter = None;
            if let Some(rate_limit) = config.external_services.send_tx_rate_limit {
                send_tx_limiter = Some(RateLimiter::new(rate_limit));
            }

            let indexer = PhotonIndexer::new(
                config.external_services.indexer_url.clone().unwrap(),
                config.external_services.photon_api_key.clone(),
            );

            let indexer = Arc::new(tokio::sync::Mutex::new(indexer));

            run_pipeline::<SolanaRpcConnection, PhotonIndexer>(
                config,
                rpc_rate_limiter,
                send_tx_limiter,
                indexer,
                shutdown_receiver,
                work_report_sender,
            )
            .await?
        }
        Commands::Status(args) => {
            forester_status::fetch_forester_status(args).await;
        }
    }
    Ok(())
}
