use std::{collections::HashMap, net::SocketAddr, sync::Arc, thread::JoinHandle, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot, watch};
use tracing::{error, info, warn};
use warp::Filter;

use crate::{
    compressible::{CTokenAccountTracker, MintAccountTracker, PdaAccountTracker},
    forester_status::get_forester_status,
    metrics::REGISTRY,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub transactions_processed_total: HashMap<String, u64>,
    pub transaction_rate: HashMap<String, f64>,
    pub last_run_timestamp: i64,
    pub forester_balances: HashMap<String, f64>,
    pub queue_lengths: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleResponse {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctoken_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    #[serde(flatten)]
    pub data: MetricsResponse,
    pub source: String,
    pub cached_at: i64,
}

impl MetricsSnapshot {
    fn empty() -> Self {
        Self {
            data: MetricsResponse::default(),
            source: "none".to_string(),
            cached_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleSnapshot {
    #[serde(flatten)]
    pub data: CompressibleResponse,
    pub source: String,
    pub cached_at: i64,
}

impl CompressibleSnapshot {
    fn empty() -> Self {
        Self {
            data: CompressibleResponse {
                enabled: false,
                ctoken_count: None,
                pda_count: None,
                mint_count: None,
            },
            source: "none".to_string(),
            cached_at: 0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct CompressibleTrackers {
    pub ctoken: Option<Arc<CTokenAccountTracker>>,
    pub pda: Option<Arc<PdaAccountTracker>>,
    pub mint: Option<Arc<MintAccountTracker>>,
}

/// Holds optional references to compressible trackers for the API server.
pub struct CompressibleDashboardState {
    pub ctoken_tracker: Option<Arc<CTokenAccountTracker>>,
    pub pda_tracker: Option<Arc<PdaAccountTracker>>,
    pub mint_tracker: Option<Arc<MintAccountTracker>>,
}

/// Configuration for the HTTP API server.
pub struct ApiServerConfig {
    pub run_id: Arc<str>,
    pub rpc_url: String,
    pub port: u16,
    pub allow_public_bind: bool,
    pub compressible_state: Option<CompressibleDashboardState>,
    pub prometheus_url: Option<String>,
    pub helius_rpc: bool,
}

/// Default timeout for status endpoint in seconds
const STATUS_TIMEOUT_SECS: u64 = 30;

/// Timeout for external HTTP requests (Prometheus queries)
const EXTERNAL_HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// Overall timeout for on-chain compressible count fetch (paginated, can be slow)
const COMPRESSIBLE_FETCH_TIMEOUT: Duration = Duration::from_secs(90);

/// Handle returned by spawn_api_server for graceful shutdown
pub struct ApiServerHandle {
    /// Thread handle for the API server
    pub thread_handle: JoinHandle<()>,
    /// Sender to trigger graceful shutdown
    pub shutdown_tx: oneshot::Sender<()>,
    pub run_id: Arc<str>,
}

impl ApiServerHandle {
    /// Trigger graceful shutdown and wait for the server to stop
    pub fn shutdown(self) {
        let run_id = self.run_id.clone();
        // Send shutdown signal (ignore error if receiver already dropped)
        let _ = self.shutdown_tx.send(());
        // Wait for the thread to finish
        if let Err(e) = self.thread_handle.join() {
            error!(
                event = "api_server_thread_panicked",
                run_id = %run_id,
                error = ?e,
                "API server thread panicked"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Extracted fetch functions (testable, no warp / no Mutex / no cache)
// ---------------------------------------------------------------------------

/// Fetch metrics: try local REGISTRY first, then Prometheus.
pub(crate) async fn fetch_metrics_snapshot(
    client: &reqwest::Client,
    prometheus_url: &Option<String>,
    run_id: &str,
) -> MetricsSnapshot {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // First try local REGISTRY (co-located forester mode)
    if let Ok(metrics) = get_metrics_json() {
        if !is_metrics_empty(&metrics) {
            return MetricsSnapshot {
                data: metrics,
                source: "registry".to_string(),
                cached_at: now,
            };
        }
    }

    // Prometheus fallback
    if let Some(ref url) = prometheus_url {
        match crate::metrics::query_prometheus_metrics(client, url).await {
            Ok(metrics) => {
                return MetricsSnapshot {
                    data: metrics,
                    source: "prometheus".to_string(),
                    cached_at: now,
                };
            }
            Err(e) => {
                warn!(
                    event = "api_server_prometheus_query_failed",
                    run_id = %run_id,
                    error = %e,
                    "Prometheus query failed"
                );
            }
        }
    }

    // No data from any source
    MetricsSnapshot {
        data: MetricsResponse::default(),
        source: "none".to_string(),
        cached_at: now,
    }
}

/// Fetch compressible counts: try in-memory trackers first, then RPC.
pub(crate) async fn fetch_compressible_snapshot(
    trackers: &Option<CompressibleTrackers>,
    rpc_url: &str,
    helius_rpc: bool,
    run_id: &str,
) -> CompressibleSnapshot {
    use crate::compressible::traits::CompressibleTracker;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    if let Some(ref t) = trackers {
        return CompressibleSnapshot {
            data: CompressibleResponse {
                enabled: true,
                ctoken_count: t.ctoken.as_ref().map(|tr| tr.len()),
                pda_count: t.pda.as_ref().map(|tr| tr.len()),
                mint_count: t.mint.as_ref().map(|tr| tr.len()),
            },
            source: "tracker".to_string(),
            cached_at: now,
        };
    }

    // Standalone mode: RPC with timeout
    let fetch_result = tokio::time::timeout(
        COMPRESSIBLE_FETCH_TIMEOUT,
        crate::compressible::count_compressible_accounts(rpc_url, helius_rpc),
    )
    .await;

    match fetch_result {
        Ok(Ok((ctoken_count, mint_count))) => CompressibleSnapshot {
            data: CompressibleResponse {
                enabled: true,
                ctoken_count: Some(ctoken_count),
                pda_count: None,
                mint_count: Some(mint_count),
            },
            source: "rpc".to_string(),
            cached_at: now,
        },
        Ok(Err(e)) => {
            warn!(
                event = "api_server_compressible_count_failed",
                run_id = %run_id,
                error = %e,
                "RPC compressible count failed"
            );
            CompressibleSnapshot {
                data: CompressibleResponse {
                    enabled: false,
                    ctoken_count: None,
                    pda_count: None,
                    mint_count: None,
                },
                source: "none".to_string(),
                cached_at: now,
            }
        }
        Err(_) => {
            warn!(
                event = "api_server_compressible_count_timeout",
                run_id = %run_id,
                timeout_seconds = COMPRESSIBLE_FETCH_TIMEOUT.as_secs(),
                "Compressible count timed out"
            );
            CompressibleSnapshot {
                data: CompressibleResponse {
                    enabled: false,
                    ctoken_count: None,
                    pda_count: None,
                    mint_count: None,
                },
                source: "none".to_string(),
                cached_at: now,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Background provider tasks
// ---------------------------------------------------------------------------

/// Periodically fetches metrics and publishes via watch channel.
async fn run_metrics_provider(
    tx: watch::Sender<MetricsSnapshot>,
    client: reqwest::Client,
    prometheus_url: Option<String>,
    mut shutdown: broadcast::Receiver<()>,
    run_id: Arc<str>,
) {
    loop {
        let snapshot = fetch_metrics_snapshot(&client, &prometheus_url, run_id.as_ref()).await;
        if tx.send(snapshot).is_err() {
            break; // all receivers dropped
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
            _ = shutdown.recv() => break,
        }
    }
    info!(
        event = "api_server_metrics_provider_stopped",
        run_id = %run_id,
        "Metrics provider stopped"
    );
}

/// Periodically fetches compressible counts and publishes via watch channel.
async fn run_compressible_provider(
    tx: watch::Sender<CompressibleSnapshot>,
    trackers: Option<CompressibleTrackers>,
    rpc_url: String,
    mut shutdown: broadcast::Receiver<()>,
    helius_rpc: bool,
    run_id: Arc<str>,
) {
    // In-memory trackers are cheap (.len()); RPC is expensive (getProgramAccounts)
    let interval = if trackers.is_some() {
        Duration::from_secs(5)
    } else {
        Duration::from_secs(30)
    };

    loop {
        let snapshot =
            fetch_compressible_snapshot(&trackers, &rpc_url, helius_rpc, run_id.as_ref()).await;
        if tx.send(snapshot).is_err() {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown.recv() => break,
        }
    }
    info!(
        event = "api_server_compressible_provider_stopped",
        run_id = %run_id,
        "Compressible provider stopped"
    );
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

/// Spawn the HTTP API server with graceful shutdown support.
///
/// # Returns
/// An `ApiServerHandle` that can be used to trigger graceful shutdown
pub fn spawn_api_server(config: ApiServerConfig) -> ApiServerHandle {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let run_id_for_handle = config.run_id.clone();

    let thread_handle = std::thread::spawn(move || {
        let run_id = config.run_id.clone();
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!(
                    event = "api_server_runtime_create_failed",
                    run_id = %run_id,
                    error = %e,
                    "Failed to create tokio runtime for API server"
                );
                return;
            }
        };
        rt.block_on(async move {
            let addr = if config.allow_public_bind {
                warn!(
                    event = "api_server_public_bind_enabled",
                    run_id = %run_id,
                    port = config.port,
                    "API server binding to 0.0.0.0; endpoints will be publicly accessible"
                );
                SocketAddr::from(([0, 0, 0, 0], config.port))
            } else {
                SocketAddr::from(([127, 0, 0, 1], config.port))
            };
            info!(
                event = "api_server_started",
                run_id = %run_id,
                address = %addr,
                "Starting HTTP API server"
            );

            // Shared HTTP client with timeout for external requests (Prometheus)
            let http_client = reqwest::Client::builder()
                .timeout(EXTERNAL_HTTP_TIMEOUT)
                .build()
                .expect("Failed to create HTTP client");

            // Build trackers from config
            let trackers = config
                .compressible_state
                .as_ref()
                .map(|s| CompressibleTrackers {
                    ctoken: s.ctoken_tracker.clone(),
                    pda: s.pda_tracker.clone(),
                    mint: s.mint_tracker.clone(),
                });

            // Create watch channels with empty initial values
            let (metrics_tx, metrics_rx) = watch::channel(MetricsSnapshot::empty());
            let (compressible_tx, compressible_rx) = watch::channel(CompressibleSnapshot::empty());

            // Create shutdown broadcast for providers
            let (provider_shutdown_tx, _) = broadcast::channel::<()>(1);

            // Spawn background providers
            tokio::spawn(run_metrics_provider(
                metrics_tx,
                http_client.clone(),
                config.prometheus_url.clone(),
                provider_shutdown_tx.subscribe(),
                run_id.clone(),
            ));

            tokio::spawn(run_compressible_provider(
                compressible_tx,
                trackers,
                config.rpc_url.clone(),
                provider_shutdown_tx.subscribe(),
                config.helius_rpc,
                run_id.clone(),
            ));

            let cors = warp::cors()
                .allow_any_origin()
                .allow_methods(vec!["GET"])
                .allow_headers(vec!["Content-Type"]);

            let health_route = warp::path("health").and(warp::get()).map(|| {
                warp::reply::json(&HealthResponse {
                    status: "ok".to_string(),
                })
            });

            // --- Status route (unchanged — per-request RPC call) ---
            let rpc_url_for_status = config.rpc_url.clone();
            let run_id_for_status = run_id.clone();
            let status_route = warp::path("status").and(warp::get()).and_then(move || {
                let rpc_url = rpc_url_for_status.clone();
                let run_id = run_id_for_status.clone();
                async move {
                    let timeout_duration = Duration::from_secs(STATUS_TIMEOUT_SECS);
                    match tokio::time::timeout(timeout_duration, get_forester_status(&rpc_url))
                        .await
                    {
                        Ok(Ok(status)) => Ok::<_, warp::Rejection>(warp::reply::with_status(
                            warp::reply::json(&status),
                            warp::http::StatusCode::OK,
                        )),
                        Ok(Err(e)) => {
                            error!(
                                event = "api_server_status_fetch_failed",
                                run_id = %run_id,
                                error = ?e,
                                "Failed to get forester status"
                            );
                            let error_response = ErrorResponse {
                                error: format!("Failed to get forester status: {}", e),
                            };
                            Ok(warp::reply::with_status(
                                warp::reply::json(&error_response),
                                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                            ))
                        }
                        Err(_elapsed) => {
                            error!(
                                event = "api_server_status_timeout",
                                run_id = %run_id,
                                timeout_seconds = STATUS_TIMEOUT_SECS,
                                "Forester status request timed out"
                            );
                            let error_response = ErrorResponse {
                                error: format!(
                                    "Request timed out after {} seconds",
                                    STATUS_TIMEOUT_SECS
                                ),
                            };
                            Ok(warp::reply::with_status(
                                warp::reply::json(&error_response),
                                warp::http::StatusCode::GATEWAY_TIMEOUT,
                            ))
                        }
                    }
                }
            });

            // --- Metrics route (reads latest snapshot from watch channel) ---
            let metrics_rx_clone = metrics_rx.clone();
            let metrics_route = warp::path!("metrics" / "json")
                .and(warp::get())
                .map(move || warp::reply::json(&*metrics_rx_clone.borrow()));

            // --- Compressible route (reads latest snapshot from watch channel) ---
            let compressible_rx_clone = compressible_rx.clone();
            let compressible_route = warp::path("compressible")
                .and(warp::get())
                .map(move || warp::reply::json(&*compressible_rx_clone.borrow()));

            let routes = health_route
                .or(status_route)
                .or(metrics_route)
                .or(compressible_route)
                .with(cors);

            warp::serve(routes)
                .bind(addr)
                .await
                .graceful({
                    let run_id_for_shutdown = run_id.clone();
                    async move {
                        let _ = shutdown_rx.await;
                        info!(
                            event = "api_server_shutdown_signal_received",
                            run_id = %run_id_for_shutdown,
                            "API server received shutdown signal"
                        );
                        // Signal providers to stop
                        let _ = provider_shutdown_tx.send(());
                    }
                })
                .run()
                .await;
            info!(
                event = "api_server_stopped",
                run_id = %run_id,
                "API server shut down gracefully"
            );
        });
    });

    ApiServerHandle {
        thread_handle,
        shutdown_tx,
        run_id: run_id_for_handle,
    }
}

fn is_metrics_empty(m: &MetricsResponse) -> bool {
    m.transactions_processed_total.is_empty()
        && m.forester_balances.is_empty()
        && m.queue_lengths.is_empty()
        && m.last_run_timestamp == 0
}

fn get_metrics_json() -> Result<MetricsResponse, String> {
    use prometheus::proto::MetricType;

    let metric_families = REGISTRY.gather();

    let mut transactions_processed: HashMap<String, u64> = HashMap::new();
    let mut transaction_rate: HashMap<String, f64> = HashMap::new();
    let mut last_run_timestamp: i64 = 0;
    let mut forester_balances: HashMap<String, f64> = HashMap::new();
    let mut queue_lengths: HashMap<String, i64> = HashMap::new();

    for mf in metric_families {
        let name = mf.name();
        let metric_type = mf.get_field_type();

        for metric in mf.get_metric() {
            // Extract labels into a map for easy lookup
            let labels: HashMap<&str, &str> = metric
                .get_label()
                .iter()
                .map(|lp| (lp.name(), lp.value()))
                .collect();

            // Get the metric value based on type
            let value = match metric_type {
                MetricType::COUNTER => metric.get_counter().value(),
                MetricType::GAUGE => metric.get_gauge().value(),
                MetricType::HISTOGRAM => {
                    // For histogram, use sample_sum as a representative value
                    metric.get_histogram().get_sample_sum()
                }
                MetricType::SUMMARY => {
                    // For summary, use sample_sum as a representative value
                    metric.get_summary().sample_sum()
                }
                _ => continue,
            };

            // Skip NaN and Inf values
            if value.is_nan() || value.is_infinite() {
                continue;
            }

            match name {
                "forester_transactions_processed_total" => {
                    if let Some(epoch) = labels.get("epoch") {
                        transactions_processed.insert((*epoch).to_string(), value as u64);
                    }
                }
                "forester_transaction_rate" => {
                    if let Some(epoch) = labels.get("epoch") {
                        transaction_rate.insert((*epoch).to_string(), value);
                    }
                }
                "forester_last_run_timestamp" => {
                    last_run_timestamp = value as i64;
                }
                "forester_sol_balance" => {
                    if let Some(pubkey) = labels.get("pubkey") {
                        forester_balances.insert((*pubkey).to_string(), value);
                    }
                }
                "queue_length" => {
                    if let Some(tree_pubkey) = labels.get("tree_pubkey") {
                        queue_lengths.insert((*tree_pubkey).to_string(), value as i64);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(MetricsResponse {
        transactions_processed_total: transactions_processed,
        transaction_rate,
        last_run_timestamp,
        forester_balances,
        queue_lengths,
    })
}
