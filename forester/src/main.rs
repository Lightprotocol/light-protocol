use clap::Parser;
use forester::cli::{Cli, Commands};
use forester::errors::ForesterError;
use forester::metrics::{push_metrics, register_metrics};
use forester::photon_indexer::PhotonIndexer;
use forester::telemetry::setup_telemetry;
use forester::tree_data_sync::fetch_trees;
use forester::{run_pipeline, run_queue_info, ForesterConfig};
use forester_utils::forester_epoch::TreeType;
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use std::sync::Arc;
use tokio::signal::ctrl_c;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

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

            let indexer_rpc =
                SolanaRpcConnection::new(config.external_services.rpc_url.clone(), None);
            let indexer = Arc::new(PhotonIndexer::new(
                config.external_services.indexer_url.clone().unwrap(),
                config.external_services.photon_api_key.clone(),
                indexer_rpc,
            ));

            run_pipeline(config, indexer, shutdown_receiver, work_report_sender).await?
        }
        Commands::Status(args) => {
            let config = Arc::new(ForesterConfig::new_for_status(args)?);

            if config.general_config.enable_metrics {
                register_metrics();
            }

            debug!("Fetching trees...");
            debug!("RPC URL: {}", config.external_services.rpc_url);
            let rpc = SolanaRpcConnection::new(config.external_services.rpc_url.clone(), None);
            let trees = fetch_trees(&rpc).await;
            if trees.is_empty() {
                warn!("No trees found. Exiting.");
            }
            run_queue_info(config.clone(), trees.clone(), TreeType::State).await;
            run_queue_info(config.clone(), trees.clone(), TreeType::Address).await;

            push_metrics(&config.external_services.pushgateway_url).await?;
        }
    }
    Ok(())
}
