use std::sync::Arc;

use clap::Parser;
use forester::{
    api_server::spawn_api_server,
    cli::{Cli, Commands},
    errors::ForesterError,
    forester_status,
    health_check::run_health_check,
    metrics::register_metrics,
    run_pipeline,
    telemetry::setup_telemetry,
    ForesterConfig,
};
use forester_utils::rate_limiter::RateLimiter;
use light_client::rpc::LightClient;
use tokio::{
    signal::ctrl_c,
    sync::{mpsc, oneshot},
};
use tracing::debug;

/// Spawns a task that handles graceful shutdown on Ctrl+C.
///
/// First Ctrl+C triggers graceful shutdown by sending to `service_sender`
/// and calling the optional `additional_shutdown` closure.
/// Second Ctrl+C forces immediate exit.
fn spawn_shutdown_handler<F>(service_sender: oneshot::Sender<()>, additional_shutdown: Option<F>)
where
    F: FnOnce() + Send + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = ctrl_c().await {
            tracing::error!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        if service_sender.send(()).is_err() {
            tracing::warn!("Shutdown signal to service already sent or receiver dropped");
        }
        if let Some(shutdown_fn) = additional_shutdown {
            shutdown_fn();
        }

        // Wait for second Ctrl+C to force exit
        if let Err(e) = ctrl_c().await {
            tracing::warn!("Failed to listen for second Ctrl+C (forcing exit): {}", e);
            std::process::exit(1);
        }
        tracing::warn!("Received second Ctrl+C, forcing exit!");
        std::process::exit(1);
    });
}

#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), ForesterError> {
    dotenvy::dotenv().ok();
    setup_telemetry();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Start(args) => {
            let config = Arc::new(ForesterConfig::new_for_start(args)?);

            if config.general_config.enable_metrics {
                register_metrics();
            }

            let (shutdown_sender_service, shutdown_receiver_service) = oneshot::channel();
            let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

            tokio::spawn(async move {
                while let Some(report) = work_report_receiver.recv().await {
                    debug!("Work Report: {:?}", report);
                }
            });

            let rpc_rate_limiter = config
                .external_services
                .rpc_rate_limit
                .map(RateLimiter::new);
            let send_tx_limiter = config
                .external_services
                .send_tx_rate_limit
                .map(RateLimiter::new);

            let rpc_url_for_api: String = config.external_services.rpc_url.to_string();
            let api_server_handle = spawn_api_server(
                rpc_url_for_api,
                args.api_server_port,
                args.api_server_public_bind,
            );

            // Create compressible shutdown channels if compressible is enabled
            let (shutdown_receiver_compressible, shutdown_receiver_bootstrap) =
                if config.compressible_config.is_some() {
                    let (shutdown_sender_compressible, shutdown_receiver_compressible) =
                        tokio::sync::broadcast::channel(1);
                    let (shutdown_sender_bootstrap, shutdown_receiver_bootstrap) =
                        oneshot::channel();
                    spawn_shutdown_handler(
                        shutdown_sender_service,
                        Some(move || {
                            let _ = shutdown_sender_compressible.send(());
                            let _ = shutdown_sender_bootstrap.send(());
                            api_server_handle.shutdown();
                        }),
                    );
                    (
                        Some(shutdown_receiver_compressible),
                        Some(shutdown_receiver_bootstrap),
                    )
                } else {
                    spawn_shutdown_handler(
                        shutdown_sender_service,
                        Some(move || {
                            api_server_handle.shutdown();
                        }),
                    );
                    (None, None)
                };

            run_pipeline::<LightClient>(
                config,
                rpc_rate_limiter,
                send_tx_limiter,
                shutdown_receiver_service,
                shutdown_receiver_compressible,
                shutdown_receiver_bootstrap,
                work_report_sender,
            )
            .await?
        }
        Commands::Status(args) => {
            forester_status::fetch_forester_status(args).await?;
        }
        Commands::Health(args) => {
            let result = run_health_check(args).await?;
            if !result && args.exit_on_failure {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
