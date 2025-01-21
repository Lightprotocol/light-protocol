use std::sync::Arc;

use clap::Parser;
use forester::{
    cli::{Cli, Commands}, errors::ForesterError, forester_status, photon_indexer::PhotonIndexer, run_pipeline, telemetry::setup_telemetry, ForesterConfig
};
use forester_utils::{metrics::helpers::register_metrics, SolanaRpcConnection};
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

            let indexer_rpc = SolanaRpcConnection::new_with_retry(
                config.external_services.rpc_url.clone(),
                None,
                None,
            );
            
            let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
                config.external_services.indexer_url.clone().unwrap(),
                config.external_services.photon_api_key.clone(),
                indexer_rpc,
            )));

            run_pipeline::<SolanaRpcConnection, PhotonIndexer<SolanaRpcConnection>>(
                config,
                indexer,
                shutdown_receiver,
                work_report_sender,
            ).await?
        }
        Commands::Status(args) => {
            forester_status::fetch_forester_status(args).await;
        }
    }
    Ok(())
}
