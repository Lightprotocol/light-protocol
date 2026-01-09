use std::{collections::HashMap, net::SocketAddr, thread::JoinHandle, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{error, info, warn};
use warp::{http::StatusCode, Filter};

use crate::{forester_status::get_forester_status, metrics::REGISTRY};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub transactions_processed_total: HashMap<String, u64>,
    pub transaction_rate: HashMap<String, f64>,
    pub last_run_timestamp: i64,
    pub forester_balances: HashMap<String, f64>,
    pub queue_lengths: HashMap<String, i64>,
}

const DASHBOARD_HTML: &str = include_str!("../static/dashboard.html");

/// Default timeout for status endpoint in seconds
const STATUS_TIMEOUT_SECS: u64 = 30;

/// Handle returned by spawn_api_server for graceful shutdown
pub struct ApiServerHandle {
    /// Thread handle for the API server
    pub thread_handle: JoinHandle<()>,
    /// Sender to trigger graceful shutdown
    pub shutdown_tx: oneshot::Sender<()>,
}

impl ApiServerHandle {
    /// Trigger graceful shutdown and wait for the server to stop
    pub fn shutdown(self) {
        // Send shutdown signal (ignore error if receiver already dropped)
        let _ = self.shutdown_tx.send(());
        // Wait for the thread to finish
        if let Err(e) = self.thread_handle.join() {
            error!("API server thread panicked: {:?}", e);
        }
    }
}

/// Spawn the HTTP API server with graceful shutdown support.
///
/// # Arguments
/// * `rpc_url` - RPC URL for forester status endpoint
/// * `port` - Port to bind to
/// * `allow_public_bind` - If true, binds to 0.0.0.0; if false, binds to 127.0.0.1
///
/// # Returns
/// An `ApiServerHandle` that can be used to trigger graceful shutdown
pub fn spawn_api_server(rpc_url: String, port: u16, allow_public_bind: bool) -> ApiServerHandle {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let thread_handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create tokio runtime for API server: {}", e);
                return;
            }
        };
        rt.block_on(async move {
            let addr = if allow_public_bind {
                warn!(
                    "API server binding to 0.0.0.0:{} - endpoints /status and /metrics/json will be publicly accessible",
                    port
                );
                SocketAddr::from(([0, 0, 0, 0], port))
            } else {
                SocketAddr::from(([127, 0, 0, 1], port))
            };
            info!("Starting HTTP API server on {}", addr);

            let dashboard_route = warp::path::end()
                .and(warp::get())
                .map(|| warp::reply::html(DASHBOARD_HTML));

            let health_route = warp::path("health").and(warp::get()).map(|| {
                warp::reply::json(&HealthResponse {
                    status: "ok".to_string(),
                })
            });

            let status_route = warp::path("status").and(warp::get()).and_then(move || {
                let rpc_url = rpc_url.clone();
                async move {
                    let timeout_duration = Duration::from_secs(STATUS_TIMEOUT_SECS);
                    match tokio::time::timeout(timeout_duration, get_forester_status(&rpc_url))
                        .await
                    {
                        Ok(Ok(status)) => Ok::<_, warp::Rejection>(warp::reply::with_status(
                            warp::reply::json(&status),
                            StatusCode::OK,
                        )),
                        Ok(Err(e)) => {
                            error!("Failed to get forester status: {:?}", e);
                            let error_response = ErrorResponse {
                                error: format!("Failed to get forester status: {}", e),
                            };
                            Ok(warp::reply::with_status(
                                warp::reply::json(&error_response),
                                StatusCode::INTERNAL_SERVER_ERROR,
                            ))
                        }
                        Err(_elapsed) => {
                            error!(
                                "Forester status request timed out after {}s",
                                STATUS_TIMEOUT_SECS
                            );
                            let error_response = ErrorResponse {
                                error: format!(
                                    "Request timed out after {} seconds",
                                    STATUS_TIMEOUT_SECS
                                ),
                            };
                            Ok(warp::reply::with_status(
                                warp::reply::json(&error_response),
                                StatusCode::GATEWAY_TIMEOUT,
                            ))
                        }
                    }
                }
            });

            let metrics_route =
                warp::path!("metrics" / "json")
                    .and(warp::get())
                    .and_then(|| async move {
                        match get_metrics_json() {
                            Ok(metrics) => Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(&metrics),
                                StatusCode::OK,
                            )),
                            Err(e) => {
                                error!("Failed to encode metrics: {}", e);
                                let error_response = ErrorResponse {
                                    error: format!("Failed to encode metrics: {}", e),
                                };
                                Ok(warp::reply::with_status(
                                    warp::reply::json(&error_response),
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                ))
                            }
                        }
                    });

            let routes = dashboard_route
                .or(health_route)
                .or(status_route)
                .or(metrics_route);

            warp::serve(routes)
                .bind(addr)
                .await
                .graceful(async {
                    let _ = shutdown_rx.await;
                    info!("API server received shutdown signal");
                })
                .run()
                .await;
            info!("API server shut down gracefully");
        });
    });

    ApiServerHandle {
        thread_handle,
        shutdown_tx,
    }
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
