use std::{
    sync::Once,
    time::{SystemTime, UNIX_EPOCH},
};

use lazy_static::lazy_static;
use prometheus::{Encoder, GaugeVec, IntCounterVec, IntGauge, IntGaugeVec, Registry, TextEncoder};
use reqwest::Client;
use tokio::sync::Mutex;
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
    static ref METRIC_UPDATES: Mutex<Vec<(u64, usize, std::time::Duration)>> =
        Mutex::new(Vec::new());
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

pub async fn queue_metric_update(epoch: u64, count: usize, duration: std::time::Duration) {
    let mut updates = METRIC_UPDATES.lock().await;
    updates.push((epoch, count, duration));
}

pub async fn process_queued_metrics() {
    let mut updates = METRIC_UPDATES.lock().await;
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

pub async fn push_metrics(url: &Option<String>) -> Result<()> {
    let url = match url {
        Some(url) => url,
        None => {
            trace!("Pushgateway URL not set, skipping metrics push");
            return Ok(());
        }
    };

    process_queued_metrics().await;

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
