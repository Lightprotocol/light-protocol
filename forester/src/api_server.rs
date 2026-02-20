use std::{collections::HashMap, net::SocketAddr, sync::Arc, thread::JoinHandle, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot, watch};
use tracing::{error, info, warn};
use warp::Filter;

use crate::{
    compressible::{
        traits::{CompressibleState, CompressibleTracker},
        CTokenAccountState, CTokenAccountTracker, MintAccountTracker, PdaAccountTracker,
    },
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
    pub ata_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_slot: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tracked: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ready: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_waiting: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctoken: Option<CompressibleTypeStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ata: Option<CompressibleTypeStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda: Option<CompressibleTypeStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<CompressibleTypeStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_programs: Option<Vec<PdaProgramStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstreams: Option<Vec<CompressibleUpstreamStatus>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleTypeStats {
    pub tracked: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_ready_slot: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdaProgramStats {
    pub program_id: String,
    pub tracked: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_ready_slot: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleUpstreamStatus {
    pub base_url: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
                ata_count: None,
                pda_count: None,
                mint_count: None,
                current_slot: None,
                total_tracked: None,
                total_ready: None,
                total_waiting: None,
                ctoken: None,
                ata: None,
                pda: None,
                mint: None,
                pda_programs: None,
                upstreams: None,
                note: None,
                error: None,
                refresh_interval_secs: None,
            },
            source: "none".to_string(),
            cached_at: 0,
        }
    }
}

fn summarize_slots<I>(slots: I, current_slot: Option<u64>) -> CompressibleTypeStats
where
    I: IntoIterator<Item = u64>,
{
    let all_slots: Vec<u64> = slots.into_iter().collect();
    let tracked = all_slots.len();

    if let Some(slot) = current_slot {
        let mut ready = 0usize;
        let mut next_ready_slot: Option<u64> = None;
        for compressible_slot in all_slots {
            if slot > compressible_slot {
                ready += 1;
            } else {
                next_ready_slot = Some(
                    next_ready_slot
                        .map(|current_min| current_min.min(compressible_slot))
                        .unwrap_or(compressible_slot),
                );
            }
        }
        CompressibleTypeStats {
            tracked,
            ready: Some(ready),
            waiting: Some(tracked.saturating_sub(ready)),
            next_ready_slot,
        }
    } else {
        CompressibleTypeStats {
            tracked,
            ready: None,
            waiting: None,
            next_ready_slot: None,
        }
    }
}

fn is_ctoken_ata(state: &CTokenAccountState) -> bool {
    let owner = solana_sdk::pubkey::Pubkey::new_from_array(state.account.owner.to_bytes());
    let mint = solana_sdk::pubkey::Pubkey::new_from_array(state.account.mint.to_bytes());
    let light_token_program_id =
        solana_sdk::pubkey::Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);

    let expected_ata = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            owner.as_ref(),
            light_token_program_id.as_ref(),
            mint.as_ref(),
        ],
        &light_token_program_id,
    )
    .0;

    state.pubkey == expected_ata
}

fn summarize_ctoken_and_ata_slots(
    tracker: &CTokenAccountTracker,
    current_slot: Option<u64>,
) -> (CompressibleTypeStats, CompressibleTypeStats) {
    let mut ctoken_slots: Vec<u64> = Vec::new();
    let mut ata_slots: Vec<u64> = Vec::new();

    for entry in tracker.accounts().iter() {
        let state = entry.value();
        let slot = state.compressible_slot();
        ctoken_slots.push(slot);
        if is_ctoken_ata(state) {
            ata_slots.push(slot);
        }
    }

    (
        summarize_slots(ctoken_slots, current_slot),
        summarize_slots(ata_slots, current_slot),
    )
}

fn aggregate_optional_sum(values: impl Iterator<Item = Option<usize>>) -> Option<usize> {
    let mut sum = 0usize;
    let mut seen = false;
    for value in values {
        if let Some(v) = value {
            sum = sum.saturating_add(v);
            seen = true;
        }
    }
    if seen {
        Some(sum)
    } else {
        None
    }
}

fn aggregate_type_stats(
    stats: impl Iterator<Item = Option<CompressibleTypeStats>>,
) -> Option<CompressibleTypeStats> {
    let mut tracked = 0usize;
    let mut ready = 0usize;
    let mut waiting = 0usize;
    let mut ready_seen = false;
    let mut waiting_seen = false;
    let mut next_ready_slot: Option<u64> = None;
    let mut any = false;

    for stat in stats.flatten() {
        any = true;
        tracked = tracked.saturating_add(stat.tracked);
        if let Some(v) = stat.ready {
            ready = ready.saturating_add(v);
            ready_seen = true;
        }
        if let Some(v) = stat.waiting {
            waiting = waiting.saturating_add(v);
            waiting_seen = true;
        }
        if let Some(slot) = stat.next_ready_slot {
            next_ready_slot = Some(
                next_ready_slot
                    .map(|current| current.min(slot))
                    .unwrap_or(slot),
            );
        }
    }

    if !any {
        return None;
    }

    Some(CompressibleTypeStats {
        tracked,
        ready: ready_seen.then_some(ready),
        waiting: waiting_seen.then_some(waiting),
        next_ready_slot,
    })
}

async fn fetch_upstream_compressible(
    client: &reqwest::Client,
    base_url: String,
) -> (CompressibleUpstreamStatus, Option<CompressibleSnapshot>) {
    let normalized = base_url.trim_end_matches('/').to_string();
    let endpoint = format!("{}/compressible", normalized);

    let response =
        match tokio::time::timeout(Duration::from_secs(6), client.get(&endpoint).send()).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                return (
                    CompressibleUpstreamStatus {
                        base_url: normalized,
                        ok: false,
                        source: None,
                        cached_at: None,
                        error: Some(format!("request failed: {}", e)),
                    },
                    None,
                );
            }
            Err(_) => {
                return (
                    CompressibleUpstreamStatus {
                        base_url: normalized,
                        ok: false,
                        source: None,
                        cached_at: None,
                        error: Some("request timed out after 6s".to_string()),
                    },
                    None,
                );
            }
        };

    if !response.status().is_success() {
        let status = response.status();
        return (
            CompressibleUpstreamStatus {
                base_url: normalized,
                ok: false,
                source: None,
                cached_at: None,
                error: Some(format!("upstream returned {}", status)),
            },
            None,
        );
    }

    let snapshot = match response.json::<CompressibleSnapshot>().await {
        Ok(snapshot) => snapshot,
        Err(e) => {
            return (
                CompressibleUpstreamStatus {
                    base_url: normalized,
                    ok: false,
                    source: None,
                    cached_at: None,
                    error: Some(format!("invalid json payload: {}", e)),
                },
                None,
            );
        }
    };

    (
        CompressibleUpstreamStatus {
            base_url: normalized,
            ok: true,
            source: Some(snapshot.source.clone()),
            cached_at: Some(snapshot.cached_at),
            error: None,
        },
        Some(snapshot),
    )
}

async fn fetch_compressible_snapshot_from_foresters(
    forester_api_urls: &[String],
    run_id: &str,
) -> Option<CompressibleSnapshot> {
    if forester_api_urls.is_empty() {
        return None;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let client = reqwest::Client::new();
    let calls = forester_api_urls
        .iter()
        .cloned()
        .map(|url| fetch_upstream_compressible(&client, url));

    let responses = futures::future::join_all(calls).await;
    let mut upstreams: Vec<CompressibleUpstreamStatus> = Vec::with_capacity(responses.len());
    let mut snapshots: Vec<CompressibleSnapshot> = Vec::new();

    for (status, snapshot) in responses {
        upstreams.push(status);
        if let Some(snapshot) = snapshot {
            snapshots.push(snapshot);
        }
    }

    let ok_count = snapshots.len();
    let fail_count = upstreams.len().saturating_sub(ok_count);

    if snapshots.is_empty() {
        warn!(
            event = "api_server_upstream_compressible_all_failed",
            run_id = %run_id,
            total_upstreams = upstreams.len(),
            "All upstream forester compressible endpoints failed"
        );
        return Some(CompressibleSnapshot {
            data: CompressibleResponse {
                enabled: false,
                ctoken_count: None,
                ata_count: None,
                pda_count: None,
                mint_count: None,
                current_slot: None,
                total_tracked: None,
                total_ready: None,
                total_waiting: None,
                ctoken: None,
                ata: None,
                pda: None,
                mint: None,
                pda_programs: None,
                upstreams: Some(upstreams),
                note: None,
                error: Some(
                    "Failed to fetch /compressible from all configured forester APIs".to_string(),
                ),
                refresh_interval_secs: Some(5),
            },
            source: "forester-apis".to_string(),
            cached_at: now,
        });
    }

    let ctoken_count = aggregate_optional_sum(snapshots.iter().map(|s| s.data.ctoken_count));
    let ata_count = aggregate_optional_sum(snapshots.iter().map(|s| s.data.ata_count));
    let pda_count = aggregate_optional_sum(snapshots.iter().map(|s| s.data.pda_count));
    let mint_count = aggregate_optional_sum(snapshots.iter().map(|s| s.data.mint_count));

    let ctoken = aggregate_type_stats(snapshots.iter().map(|s| s.data.ctoken.clone()));
    let ata = aggregate_type_stats(snapshots.iter().map(|s| s.data.ata.clone()));
    let pda = aggregate_type_stats(snapshots.iter().map(|s| s.data.pda.clone()));
    let mint = aggregate_type_stats(snapshots.iter().map(|s| s.data.mint.clone()));

    let mut pda_program_map: HashMap<String, CompressibleTypeStats> = HashMap::new();
    for snapshot in &snapshots {
        if let Some(programs) = &snapshot.data.pda_programs {
            for row in programs {
                let entry = pda_program_map.entry(row.program_id.clone()).or_insert(
                    CompressibleTypeStats {
                        tracked: 0,
                        ready: Some(0),
                        waiting: Some(0),
                        next_ready_slot: None,
                    },
                );
                entry.tracked = entry.tracked.saturating_add(row.tracked);
                entry.ready = Some(
                    entry
                        .ready
                        .unwrap_or(0)
                        .saturating_add(row.ready.unwrap_or(0)),
                );
                entry.waiting = Some(
                    entry
                        .waiting
                        .unwrap_or(0)
                        .saturating_add(row.waiting.unwrap_or(0)),
                );
                if let Some(slot) = row.next_ready_slot {
                    entry.next_ready_slot = Some(
                        entry
                            .next_ready_slot
                            .map(|current| current.min(slot))
                            .unwrap_or(slot),
                    );
                }
            }
        }
    }

    let mut pda_programs: Vec<PdaProgramStats> = pda_program_map
        .into_iter()
        .map(|(program_id, stats)| PdaProgramStats {
            program_id,
            tracked: stats.tracked,
            ready: stats.ready,
            waiting: stats.waiting,
            next_ready_slot: stats.next_ready_slot,
        })
        .collect();
    pda_programs.sort_by(|a, b| a.program_id.cmp(&b.program_id));

    let current_slot = snapshots.iter().filter_map(|s| s.data.current_slot).max();
    let total_tracked = aggregate_optional_sum(snapshots.iter().map(|s| s.data.total_tracked))
        .or_else(|| {
            Some(ctoken_count.unwrap_or(0) + pda_count.unwrap_or(0) + mint_count.unwrap_or(0))
        });
    let total_ready = aggregate_optional_sum(snapshots.iter().map(|s| s.data.total_ready));
    let total_waiting = aggregate_optional_sum(snapshots.iter().map(|s| s.data.total_waiting));

    let note = if fail_count > 0 {
        Some(format!(
            "Aggregated from {}/{} forester API endpoints ({} unavailable).",
            ok_count,
            upstreams.len(),
            fail_count
        ))
    } else {
        Some(format!(
            "Aggregated from {} forester API endpoint(s).",
            ok_count
        ))
    };

    Some(CompressibleSnapshot {
        data: CompressibleResponse {
            enabled: true,
            ctoken_count,
            ata_count,
            pda_count,
            mint_count,
            current_slot,
            total_tracked,
            total_ready,
            total_waiting,
            ctoken,
            ata,
            pda,
            mint,
            pda_programs: (!pda_programs.is_empty()).then_some(pda_programs),
            upstreams: Some(upstreams),
            note,
            error: None,
            refresh_interval_secs: Some(5),
        },
        source: "forester-apis".to_string(),
        cached_at: now,
    })
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
    pub forester_api_urls: Vec<String>,
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
    forester_api_urls: &[String],
    rpc_url: &str,
    helius_rpc: bool,
    run_id: &str,
) -> CompressibleSnapshot {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    if let Some(ref t) = trackers {
        let client = reqwest::Client::new();
        let current_slot = match crate::compressible::bootstrap_helpers::get_current_slot(
            &client, rpc_url,
        )
        .await
        {
            Ok(slot) => Some(slot),
            Err(e) => {
                warn!(
                    event = "api_server_compressible_slot_fetch_failed",
                    run_id = %run_id,
                    error = %e,
                    "Failed to fetch current slot for compressible readiness breakdown"
                );
                None
            }
        };
        let (ctoken, ata) = if let Some(tracker) = t.ctoken.as_ref() {
            let (ctoken_stats, ata_stats) = summarize_ctoken_and_ata_slots(tracker, current_slot);
            (Some(ctoken_stats), Some(ata_stats))
        } else {
            (None, None)
        };

        let pda = t.pda.as_ref().map(|tracker| {
            summarize_slots(
                tracker
                    .accounts()
                    .iter()
                    .map(|entry| entry.value().compressible_slot()),
                current_slot,
            )
        });

        let mint = t.mint.as_ref().map(|tracker| {
            summarize_slots(
                tracker
                    .accounts()
                    .iter()
                    .map(|entry| entry.value().compressible_slot()),
                current_slot,
            )
        });

        let mut pda_programs: Option<Vec<PdaProgramStats>> = None;
        if let Some(pda_tracker) = t.pda.as_ref() {
            let mut by_program: HashMap<String, Vec<u64>> = HashMap::new();
            for entry in pda_tracker.accounts().iter() {
                let state = entry.value();
                by_program
                    .entry(state.program_id.to_string())
                    .or_default()
                    .push(state.compressible_slot());
            }
            let mut rows: Vec<PdaProgramStats> = by_program
                .into_iter()
                .map(|(program_id, slots)| {
                    let stats = summarize_slots(slots, current_slot);
                    PdaProgramStats {
                        program_id,
                        tracked: stats.tracked,
                        ready: stats.ready,
                        waiting: stats.waiting,
                        next_ready_slot: stats.next_ready_slot,
                    }
                })
                .collect();
            rows.sort_by(|a, b| a.program_id.cmp(&b.program_id));
            pda_programs = Some(rows);
        }

        let total_tracked = ctoken.as_ref().map(|s| s.tracked).unwrap_or(0)
            + pda.as_ref().map(|s| s.tracked).unwrap_or(0)
            + mint.as_ref().map(|s| s.tracked).unwrap_or(0);

        let (total_ready, total_waiting, note) = if current_slot.is_some() {
            (
                Some(
                    ctoken.as_ref().and_then(|s| s.ready).unwrap_or(0)
                        + pda.as_ref().and_then(|s| s.ready).unwrap_or(0)
                        + mint.as_ref().and_then(|s| s.ready).unwrap_or(0),
                ),
                Some(
                    ctoken.as_ref().and_then(|s| s.waiting).unwrap_or(0)
                        + pda.as_ref().and_then(|s| s.waiting).unwrap_or(0)
                        + mint.as_ref().and_then(|s| s.waiting).unwrap_or(0),
                ),
                None,
            )
        } else {
            (
                None,
                None,
                Some(
                    "Current slot is unavailable; readiness breakdown is temporarily unknown."
                        .to_string(),
                ),
            )
        };

        return CompressibleSnapshot {
            data: CompressibleResponse {
                enabled: true,
                ctoken_count: ctoken.as_ref().map(|s| s.tracked),
                ata_count: ata.as_ref().map(|s| s.tracked),
                pda_count: pda.as_ref().map(|s| s.tracked),
                mint_count: mint.as_ref().map(|s| s.tracked),
                current_slot,
                total_tracked: Some(total_tracked),
                total_ready,
                total_waiting,
                ctoken,
                ata,
                pda,
                mint,
                pda_programs,
                upstreams: None,
                note,
                error: None,
                refresh_interval_secs: Some(5),
            },
            source: "tracker".to_string(),
            cached_at: now,
        };
    }

    if let Some(snapshot) =
        fetch_compressible_snapshot_from_foresters(forester_api_urls, run_id).await
    {
        return snapshot;
    }

    // Standalone mode: RPC with timeout
    let fetch_result = tokio::time::timeout(
        COMPRESSIBLE_FETCH_TIMEOUT,
        crate::compressible::count_compressible_accounts(rpc_url, helius_rpc),
    )
    .await;

    match fetch_result {
        Ok(Ok((ctoken_count, mint_count))) => {
            let ctoken = CompressibleTypeStats {
                tracked: ctoken_count,
                ready: None,
                waiting: None,
                next_ready_slot: None,
            };
            let mint = CompressibleTypeStats {
                tracked: mint_count,
                ready: None,
                waiting: None,
                next_ready_slot: None,
            };
            let client = reqwest::Client::new();
            let current_slot =
                crate::compressible::bootstrap_helpers::get_current_slot(&client, rpc_url)
                    .await
                    .ok();

            CompressibleSnapshot {
                data: CompressibleResponse {
                    enabled: true,
                    ctoken_count: Some(ctoken_count),
                    ata_count: None,
                    pda_count: None,
                    mint_count: Some(mint_count),
                    current_slot,
                    total_tracked: Some(ctoken_count + mint_count),
                    total_ready: None,
                    total_waiting: None,
                    ctoken: Some(ctoken),
                    ata: None,
                    pda: None,
                    mint: Some(mint),
                    pda_programs: None,
                    upstreams: None,
                    note: Some(
                        "Standalone API mode: readiness and PDA breakdown require in-memory trackers inside this process, or --forester-api-url upstream(s).".to_string(),
                    ),
                    error: None,
                    refresh_interval_secs: Some(30),
                },
                source: "rpc".to_string(),
                cached_at: now,
            }
        }
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
                    ata_count: None,
                    pda_count: None,
                    mint_count: None,
                    current_slot: None,
                    total_tracked: None,
                    total_ready: None,
                    total_waiting: None,
                    ctoken: None,
                    ata: None,
                    pda: None,
                    mint: None,
                    pda_programs: None,
                    upstreams: None,
                    note: None,
                    error: Some(format!("RPC compressible count failed: {}", e)),
                    refresh_interval_secs: Some(30),
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
                    ata_count: None,
                    pda_count: None,
                    mint_count: None,
                    current_slot: None,
                    total_tracked: None,
                    total_ready: None,
                    total_waiting: None,
                    ctoken: None,
                    ata: None,
                    pda: None,
                    mint: None,
                    pda_programs: None,
                    upstreams: None,
                    note: None,
                    error: Some(format!(
                        "Compressible count timed out after {} seconds",
                        COMPRESSIBLE_FETCH_TIMEOUT.as_secs()
                    )),
                    refresh_interval_secs: Some(30),
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
    forester_api_urls: Vec<String>,
    rpc_url: String,
    mut shutdown: broadcast::Receiver<()>,
    helius_rpc: bool,
    run_id: Arc<str>,
) {
    // In-memory trackers and upstream forester APIs are cheap compared to full RPC scans.
    let interval = if trackers.is_some() || !forester_api_urls.is_empty() {
        Duration::from_secs(5)
    } else {
        Duration::from_secs(30)
    };

    loop {
        let snapshot = fetch_compressible_snapshot(
            &trackers,
            &forester_api_urls,
            &rpc_url,
            helius_rpc,
            run_id.as_ref(),
        )
        .await;
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
                config.forester_api_urls.clone(),
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
