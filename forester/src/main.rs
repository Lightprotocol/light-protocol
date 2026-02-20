use std::sync::Arc;

use clap::Parser;
use forester::{
    api_server::{spawn_api_server, ApiServerConfig, CompressibleDashboardState},
    cli::{Cli, Commands},
    epoch_manager::generate_run_id,
    errors::ForesterError,
    forester_status,
    health_check::run_health_check,
    initialize_compressible_trackers,
    metrics::register_metrics,
    run_pipeline_with_run_id,
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

            let mut shutdown_sender_compressible: Option<tokio::sync::broadcast::Sender<()>> = None;
            let mut shutdown_sender_bootstrap: Option<oneshot::Sender<()>> = None;

            let preconfigured_trackers = if config.compressible_config.is_some() {
                let (sender, receiver) = tokio::sync::broadcast::channel(1);
                let (bootstrap_sender, bootstrap_receiver) = oneshot::channel();
                shutdown_sender_compressible = Some(sender);
                shutdown_sender_bootstrap = Some(bootstrap_sender);
                Some(
                    initialize_compressible_trackers(
                        config.clone(),
                        Some(receiver),
                        Some(bootstrap_receiver),
                    )
                    .await?,
                )
            } else {
                None
            };

            let compressible_state = preconfigured_trackers.as_ref().and_then(|trackers| {
                if trackers.ctoken_tracker.is_none()
                    && trackers.pda_tracker.is_none()
                    && trackers.mint_tracker.is_none()
                {
                    None
                } else {
                    Some(CompressibleDashboardState {
                        ctoken_tracker: trackers.ctoken_tracker.clone(),
                        pda_tracker: trackers.pda_tracker.clone(),
                        mint_tracker: trackers.mint_tracker.clone(),
                    })
                }
            });

            let rpc_url_for_api: String = config.external_services.rpc_url.to_string();
            let run_id = generate_run_id();
            let api_server_handle = spawn_api_server(ApiServerConfig {
                run_id: Arc::<str>::from(run_id.clone()),
                rpc_url: rpc_url_for_api,
                port: args.api_server_port,
                allow_public_bind: args.api_server_public_bind,
                compressible_state,
                prometheus_url: args.prometheus_url.clone(),
                helius_rpc: args.helius_rpc,
                forester_api_urls: Vec::new(),
            });

            spawn_shutdown_handler(
                shutdown_sender_service,
                Some(move || {
                    if let Some(sender) = shutdown_sender_compressible {
                        let _ = sender.send(());
                    }
                    if let Some(sender) = shutdown_sender_bootstrap {
                        let _ = sender.send(());
                    }
                    api_server_handle.shutdown();
                }),
            );

            run_pipeline_with_run_id::<LightClient>(
                config,
                rpc_rate_limiter,
                send_tx_limiter,
                shutdown_receiver_service,
                None,
                None,
                work_report_sender,
                run_id,
                preconfigured_trackers,
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
        Commands::Dashboard(args) => {
            tracing::info!(
                event = "dashboard_server_starting",
                port = args.port,
                "Starting standalone dashboard API server"
            );
            let run_id = generate_run_id();
            let api_server_handle = spawn_api_server(ApiServerConfig {
                run_id: Arc::<str>::from(run_id),
                rpc_url: args.rpc_url.clone(),
                port: args.port,
                allow_public_bind: args.public_bind,
                compressible_state: None,
                prometheus_url: args.prometheus_url.clone(),
                helius_rpc: false,
                forester_api_urls: args.forester_api_urls.clone(),
            });

            // Block until Ctrl+C
            if let Err(e) = ctrl_c().await {
                tracing::error!(
                    event = "dashboard_ctrlc_listener_failed",
                    error = %e,
                    "Failed to listen for Ctrl+C"
                );
            }
            tracing::info!(
                event = "dashboard_shutdown_signal_received",
                "Received Ctrl+C, shutting down dashboard API server"
            );
            api_server_handle.shutdown();
        }
    }
    Ok(())
}
