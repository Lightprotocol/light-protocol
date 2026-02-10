use std::{
    sync::Once,
    time::{SystemTime, UNIX_EPOCH},
};

use lazy_static::lazy_static;
use prometheus::{
    Encoder, GaugeVec, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Registry, TextEncoder,
};
use reqwest::Client;
use tracing::{debug, error, log::trace};

use crate::Result;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref QUEUE_LENGTH: IntGaugeVec = IntGaugeVec::new(
        prometheus::opts!("queue_length", "Length of the queue"),
        &["tree_type", "tree_pubkey"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric QUEUE_LENGTH: {:?}", e);
        std::process::exit(1);
    });
    pub static ref LAST_RUN_TIMESTAMP: IntGauge = IntGauge::new(
        "forester_last_run_timestamp",
        "Timestamp of the last Forester run"
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric LAST_RUN_TIMESTAMP: {:?}", e);
        std::process::exit(1);
    });
    pub static ref TRANSACTIONS_PROCESSED: IntCounterVec = IntCounterVec::new(
        prometheus::opts!(
            "forester_transactions_processed_total",
            "Total number of transactions processed"
        ),
        &["epoch"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric TRANSACTIONS_PROCESSED: {:?}", e);
        std::process::exit(1);
    });
    pub static ref TRANSACTION_TIMESTAMP: GaugeVec = GaugeVec::new(
        prometheus::opts!(
            "forester_transaction_timestamp",
            "Timestamp of the last processed transaction"
        ),
        &["epoch"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric TRANSACTION_TIMESTAMP: {:?}", e);
        std::process::exit(1);
    });
    pub static ref TRANSACTION_RATE: GaugeVec = GaugeVec::new(
        prometheus::opts!(
            "forester_transaction_rate",
            "Rate of transactions processed per second"
        ),
        &["epoch"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric TRANSACTION_RATE: {:?}", e);
        std::process::exit(1);
    });
    pub static ref FORESTER_SOL_BALANCE: GaugeVec = GaugeVec::new(
        prometheus::opts!(
            "forester_sol_balance",
            "Current SOL balance of the forester"
        ),
        &["pubkey"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric FORESTER_SOL_BALANCE: {:?}", e);
        std::process::exit(1);
    });
    pub static ref REGISTERED_FORESTERS: GaugeVec = GaugeVec::new(
        prometheus::opts!("registered_foresters", "Foresters registered per epoch"),
        &["epoch", "authority"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric REGISTERED_FORESTERS: {:?}", e);
        std::process::exit(1);
    });
    pub static ref INDEXER_RESPONSE_TIME: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "forester_indexer_response_time_seconds",
            "Response time for indexer proof requests in seconds"
        )
        .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        &["operation", "tree_type"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric INDEXER_RESPONSE_TIME: {:?}", e);
        std::process::exit(1);
    });
    pub static ref INDEXER_PROOF_COUNT: IntCounterVec = IntCounterVec::new(
        prometheus::opts!(
            "forester_indexer_proof_count",
            "Number of proofs requested vs received from indexer"
        ),
        &["tree_type", "metric"]
    )
    .unwrap_or_else(|e| {
        error!("Failed to create metric INDEXER_PROOF_COUNT: {:?}", e);
        std::process::exit(1);
    });
    static ref METRIC_UPDATES: std::sync::Mutex<Vec<(u64, usize, std::time::Duration)>> =
        std::sync::Mutex::new(Vec::new());
}

static INIT: Once = Once::new();
pub fn register_metrics() {
    INIT.call_once(|| {
        if let Err(e) = REGISTRY.register(Box::new(QUEUE_LENGTH.clone())) {
            error!("Failed to register metric QUEUE_LENGTH: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(LAST_RUN_TIMESTAMP.clone())) {
            error!("Failed to register metric LAST_RUN_TIMESTAMP: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(TRANSACTIONS_PROCESSED.clone())) {
            error!("Failed to register metric TRANSACTIONS_PROCESSED: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(TRANSACTION_TIMESTAMP.clone())) {
            error!("Failed to register metric TRANSACTION_TIMESTAMP: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(TRANSACTION_RATE.clone())) {
            error!("Failed to register metric TRANSACTION_RATE: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(FORESTER_SOL_BALANCE.clone())) {
            error!("Failed to register metric FORESTER_SOL_BALANCE: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(REGISTERED_FORESTERS.clone())) {
            error!("Failed to register metric REGISTERED_FORESTERS: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(INDEXER_RESPONSE_TIME.clone())) {
            error!("Failed to register metric INDEXER_RESPONSE_TIME: {:?}", e);
        }
        if let Err(e) = REGISTRY.register(Box::new(INDEXER_PROOF_COUNT.clone())) {
            error!("Failed to register metric INDEXER_PROOF_COUNT: {:?}", e);
        }
    });
}

pub fn update_last_run_timestamp() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_else(|e| {
            error!("Failed to compute last run timestamp: {}", e);
            0
        });
    LAST_RUN_TIMESTAMP.set(now);
}

pub fn update_transactions_processed(epoch: u64, count: usize, duration: std::time::Duration) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or_else(|e| {
            error!("Failed to compute transaction timestamp: {}", e);
            0.0
        });

    TRANSACTIONS_PROCESSED
        .with_label_values(&[&epoch.to_string()])
        .inc_by(count as u64);
    TRANSACTION_TIMESTAMP
        .with_label_values(&[&epoch.to_string()])
        .set(now);

    let rate = count as f64 / duration.as_secs_f64();
    TRANSACTION_RATE
        .with_label_values(&[&epoch.to_string()])
        .set(rate);

    debug!(
        "Updated metrics for epoch {}: processed = {}, rate = {} tx/s",
        epoch, count, rate
    );
}

pub fn queue_metric_update(epoch: u64, count: usize, duration: std::time::Duration) {
    let mut updates = METRIC_UPDATES
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    updates.push((epoch, count, duration));
}

pub fn process_queued_metrics() {
    let mut updates = METRIC_UPDATES
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for (epoch, count, duration) in updates.drain(..) {
        update_transactions_processed(epoch, count, duration);
    }
}

pub fn update_forester_sol_balance(pubkey: &str, balance: f64) {
    FORESTER_SOL_BALANCE
        .with_label_values(&[pubkey])
        .set(balance);
    debug!(
        "Updated SOL balance for forester {}: {} SOL",
        pubkey, balance
    );
}

pub fn update_registered_foresters(epoch: u64, authority: &str) {
    let epoch_str = epoch.to_string();
    let authority_str = authority.to_string();
    REGISTERED_FORESTERS
        .with_label_values(&[epoch_str.as_str(), authority_str.as_str()])
        .set(1.0);
}

pub fn update_indexer_response_time(operation: &str, tree_type: &str, duration_secs: f64) {
    // Ensure metrics are registered before updating (idempotent via Once)
    register_metrics();
    INDEXER_RESPONSE_TIME
        .with_label_values(&[operation, tree_type])
        .observe(duration_secs);
    debug!(
        "Indexer {} for {} took {:.3}s",
        operation, tree_type, duration_secs
    );
}

pub fn update_indexer_proof_count(tree_type: &str, requested: u64, received: u64) {
    // Ensure metrics are registered before updating (idempotent via Once)
    register_metrics();
    INDEXER_PROOF_COUNT
        .with_label_values(&[tree_type, "requested"])
        .inc_by(requested);
    INDEXER_PROOF_COUNT
        .with_label_values(&[tree_type, "received"])
        .inc_by(received);
}

pub async fn push_metrics(url: &Option<String>) -> Result<()> {
    let url = match url {
        Some(url) => url,
        None => {
            trace!("Pushgateway URL not set, skipping metrics push");
            return Ok(());
        }
    };

    process_queued_metrics();

    update_last_run_timestamp();

    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;

    let client = Client::new();
    let res = client.post(url).body(buffer).send().await?;

    if res.status().is_success() {
        Ok(())
    } else {
        let error_message = format!(
            "Failed to push metrics. Status: {}, Body: {}",
            res.status(),
            res.text().await?
        );
        Err(anyhow::anyhow!(error_message))
    }
}

/// Query a Prometheus server for forester metrics and return a MetricsResponse.
///
/// Runs PromQL instant queries for the same metrics the in-memory REGISTRY
/// exposes, so the dashboard can show aggregated data from all foresters
/// even when running in standalone mode.
///
/// Accepts a shared `reqwest::Client` (with timeout pre-configured) to reuse
/// connection pools across calls.
pub async fn query_prometheus_metrics(
    client: &Client,
    prometheus_url: &str,
) -> Result<crate::api_server::MetricsResponse> {
    use std::collections::HashMap;

    let base = prometheus_url.trim_end_matches('/');

    async fn query_instant(
        client: &Client,
        base: &str,
        promql: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/api/v1/query", base);
        let resp = client
            .get(&url)
            .query(&[("query", promql)])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Prometheus request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Prometheus HTTP error: {}",
                resp.status()
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Prometheus JSON parse error: {}", e))?;

        if body.get("status").and_then(|s| s.as_str()) != Some("success") {
            return Err(anyhow::anyhow!("Prometheus query failed: {:?}", body));
        }

        Ok(body["data"]["result"].clone())
    }

    /// Extract label→value pairs from a Prometheus vector result.
    fn extract_label_values(
        result: &serde_json::Value,
        label_key: &str,
    ) -> Vec<(String, f64)> {
        let arr = match result.as_array() {
            Some(a) => a,
            None => return Vec::new(),
        };
        arr.iter()
            .filter_map(|entry| {
                let label = entry["metric"][label_key].as_str()?.to_string();
                let val_str = entry["value"]
                    .as_array()?
                    .get(1)?
                    .as_str()?;
                let val: f64 = val_str.parse().ok()?;
                Some((label, val))
            })
            .collect()
    }

    // Run all queries concurrently
    let (tx_total, tx_rate, last_run, balances, queues) = tokio::join!(
        query_instant(
            client,
            base,
            "sum(forester_transactions_processed_total) by (epoch)"
        ),
        query_instant(
            client,
            base,
            "sum(forester_transaction_rate) by (epoch)"
        ),
        query_instant(client, base, "max(forester_last_run_timestamp)"),
        query_instant(client, base, "forester_sol_balance"),
        query_instant(client, base, "queue_length"),
    );

    let mut transactions_processed_total: HashMap<String, u64> = HashMap::new();
    if let Ok(ref v) = tx_total {
        for (epoch, val) in extract_label_values(v, "epoch") {
            transactions_processed_total.insert(epoch, val as u64);
        }
    }

    let mut transaction_rate: HashMap<String, f64> = HashMap::new();
    if let Ok(ref v) = tx_rate {
        for (epoch, val) in extract_label_values(v, "epoch") {
            transaction_rate.insert(epoch, val);
        }
    }

    let last_run_timestamp: i64 = if let Ok(ref v) = last_run {
        v.as_array()
            .and_then(|arr| arr.first())
            .and_then(|entry| entry["value"].as_array())
            .and_then(|pair| pair.get(1))
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as i64)
            .unwrap_or(0)
    } else {
        0
    };

    let mut forester_balances: HashMap<String, f64> = HashMap::new();
    if let Ok(ref v) = balances {
        for (pubkey, val) in extract_label_values(v, "pubkey") {
            forester_balances.insert(pubkey, val);
        }
    }

    let mut queue_lengths: HashMap<String, i64> = HashMap::new();
    if let Ok(ref v) = queues {
        for (tree_pubkey, val) in extract_label_values(v, "tree_pubkey") {
            queue_lengths.insert(tree_pubkey, val as i64);
        }
    }

    Ok(crate::api_server::MetricsResponse {
        transactions_processed_total,
        transaction_rate,
        last_run_timestamp,
        forester_balances,
        queue_lengths,
    })
}

pub async fn metrics_handler() -> Result<impl warp::Reply> {
    use prometheus::Encoder;
    let encoder = TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        error!("could not encode custom metrics: {}", e);
    };
    let mut res = String::from_utf8(buffer.clone()).unwrap_or_else(|e| {
        error!("custom metrics could not be from_utf8'd: {}", e);
        String::new()
    });
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        error!("could not encode prometheus metrics: {}", e);
    };
    let res_prometheus = String::from_utf8(buffer.clone()).unwrap_or_else(|e| {
        error!("prometheus metrics could not be from_utf8'd: {}", e);
        String::new()
    });
    buffer.clear();

    res.push_str(&res_prometheus);
    Ok(res)
}
