use std::{collections::HashMap, net::SocketAddr};

use serde::{Deserialize, Serialize};
use tracing::{error, info};
use warp::{http::Response, Filter};

use crate::{forester_status::get_forester_status_blocking, metrics::REGISTRY};

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

pub fn spawn_api_server(rpc_url: String, port: u16) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            info!("Starting HTTP API server on {}", addr);

            let dashboard_route = warp::path::end().and(warp::get()).map(|| {
                Response::builder()
                    .header("content-type", "text/html; charset=utf-8")
                    .body(DASHBOARD_HTML)
            });

            let health_route = warp::path("health").and(warp::get()).map(|| {
                warp::reply::json(&HealthResponse {
                    status: "ok".to_string(),
                })
            });

            let status_route = warp::path("status").and(warp::get()).and_then(move || {
                let rpc_url = rpc_url.clone();
                async move {
                    match tokio::task::spawn_blocking(move || {
                        get_forester_status_blocking(&rpc_url)
                    })
                    .await
                    {
                        Ok(Ok(status)) => Ok::<_, warp::Rejection>(warp::reply::json(&status)),
                        Ok(Err(e)) => {
                            error!("Failed to get forester status: {:?}", e);
                            let error_response = ErrorResponse {
                                error: format!("Failed to get forester status: {}", e),
                            };
                            Ok(warp::reply::json(&error_response))
                        }
                        Err(e) => {
                            error!("Task join error: {:?}", e);
                            let error_response = ErrorResponse {
                                error: format!("Task join error: {}", e),
                            };
                            Ok(warp::reply::json(&error_response))
                        }
                    }
                }
            });

            let metrics_route = warp::path!("metrics" / "json").and(warp::get()).map(|| {
                let metrics = get_metrics_json();
                warp::reply::json(&metrics)
            });

            let routes = dashboard_route
                .or(health_route)
                .or(status_route)
                .or(metrics_route);

            warp::serve(routes).run(addr).await;
        });
    });
}

fn get_metrics_json() -> MetricsResponse {
    use prometheus::Encoder;

    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    let _ = encoder.encode(&metric_families, &mut buffer);
    let text = String::from_utf8_lossy(&buffer);

    let mut transactions_processed: HashMap<String, u64> = HashMap::new();
    let mut transaction_rate: HashMap<String, f64> = HashMap::new();
    let mut last_run_timestamp: i64 = 0;
    let mut forester_balances: HashMap<String, f64> = HashMap::new();
    let mut queue_lengths: HashMap<String, i64> = HashMap::new();

    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((metric_part, value_str)) = line.rsplit_once(' ') {
            let value: f64 = value_str.parse().unwrap_or(0.0);

            if metric_part.starts_with("forester_transactions_processed_total") {
                if let Some(epoch) = extract_label(metric_part, "epoch") {
                    transactions_processed.insert(epoch, value as u64);
                }
            } else if metric_part.starts_with("forester_transaction_rate") {
                if let Some(epoch) = extract_label(metric_part, "epoch") {
                    transaction_rate.insert(epoch, value);
                }
            } else if metric_part.starts_with("forester_last_run_timestamp") {
                last_run_timestamp = value as i64;
            } else if metric_part.starts_with("forester_sol_balance") {
                if let Some(pubkey) = extract_label(metric_part, "pubkey") {
                    forester_balances.insert(pubkey, value);
                }
            } else if metric_part.starts_with("queue_length") {
                if let Some(tree_pubkey) = extract_label(metric_part, "tree_pubkey") {
                    queue_lengths.insert(tree_pubkey, value as i64);
                }
            }
        }
    }

    MetricsResponse {
        transactions_processed_total: transactions_processed,
        transaction_rate,
        last_run_timestamp,
        forester_balances,
        queue_lengths,
    }
}

fn extract_label(metric_part: &str, label_name: &str) -> Option<String> {
    let label_pattern = format!("{}=\"", label_name);
    if let Some(start) = metric_part.find(&label_pattern) {
        let value_start = start + label_pattern.len();
        if let Some(end) = metric_part[value_start..].find('"') {
            return Some(metric_part[value_start..value_start + end].to_string());
        }
    }
    None
}
