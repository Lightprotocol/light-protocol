//! SP1 Prover HTTP Server
//!
//! A standalone HTTP server for generating SP1 proofs with Groth16 wrapping.
//! Designed to work alongside the existing Go prover server.
//!
//! Endpoints:
//! - POST /prove - Generate a proof
//! - GET /health - Health check

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sp1_circuits_lib::{BatchAddressAppendInputs, BatchAppendInputs, BatchUpdateInputs};
use sp1_sdk::{EnvProver, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

/// ELF binaries for SP1 programs
const BATCH_APPEND_ELF: &[u8] =
    include_bytes!("../../sp1-circuits/programs/batch-append/elf/batch-append-program");
const BATCH_UPDATE_ELF: &[u8] =
    include_bytes!("../../sp1-circuits/programs/batch-update/elf/batch-update-program");
const BATCH_ADDRESS_APPEND_ELF: &[u8] =
    include_bytes!("../../sp1-circuits/programs/batch-address-append/elf/batch-address-append-program");

#[derive(Parser, Debug)]
#[command(author, version, about = "SP1 Prover HTTP Server")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "3002")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

/// Shared state containing preloaded proving keys
struct AppState {
    client: EnvProver,
    batch_append_keys: RwLock<Option<(SP1ProvingKey, SP1VerifyingKey)>>,
    batch_update_keys: RwLock<Option<(SP1ProvingKey, SP1VerifyingKey)>>,
    batch_address_append_keys: RwLock<Option<(SP1ProvingKey, SP1VerifyingKey)>>,
}

impl AppState {
    fn new() -> Self {
        // Log environment configuration for debugging
        let prover_mode = std::env::var("SP1_PROVER").unwrap_or_else(|_| "cpu".to_string());
        let rpc_url = std::env::var("NETWORK_RPC_URL").unwrap_or_else(|_| "not set".to_string());
        let has_private_key = std::env::var("NETWORK_PRIVATE_KEY").is_ok();

        info!("SP1 Prover Configuration:");
        info!("  SP1_PROVER: {}", prover_mode);
        info!("  NETWORK_RPC_URL: {}", rpc_url);
        info!("  NETWORK_PRIVATE_KEY: {}", if has_private_key { "set" } else { "not set" });

        Self {
            client: EnvProver::new(),
            batch_append_keys: RwLock::new(None),
            batch_update_keys: RwLock::new(None),
            batch_address_append_keys: RwLock::new(None),
        }
    }

    async fn get_batch_append_keys(&self) -> (SP1ProvingKey, SP1VerifyingKey) {
        {
            let keys = self.batch_append_keys.read().await;
            if let Some(ref keys) = *keys {
                return keys.clone();
            }
        }

        let (pk, vk) = self.client.setup(BATCH_APPEND_ELF);

        {
            let mut keys = self.batch_append_keys.write().await;
            *keys = Some((pk.clone(), vk.clone()));
        }

        (pk, vk)
    }

    async fn get_batch_update_keys(&self) -> (SP1ProvingKey, SP1VerifyingKey) {
        {
            let keys = self.batch_update_keys.read().await;
            if let Some(ref keys) = *keys {
                return keys.clone();
            }
        }

        let (pk, vk) = self.client.setup(BATCH_UPDATE_ELF);

        {
            let mut keys = self.batch_update_keys.write().await;
            *keys = Some((pk.clone(), vk.clone()));
        }

        (pk, vk)
    }

    async fn get_batch_address_append_keys(&self) -> (SP1ProvingKey, SP1VerifyingKey) {
        {
            let keys = self.batch_address_append_keys.read().await;
            if let Some(ref keys) = *keys {
                return keys.clone();
            }
        }

        let (pk, vk) = self.client.setup(BATCH_ADDRESS_APPEND_ELF);

        {
            let mut keys = self.batch_address_append_keys.write().await;
            *keys = Some((pk.clone(), vk.clone()));
        }

        (pk, vk)
    }
}

/// Helper to extract circuit type from request JSON
fn extract_circuit_type(value: &serde_json::Value) -> Option<String> {
    value.get("circuitType")
        .or_else(|| value.get("circuit_type"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProofResponse {
    /// Proof in Gnark-compatible format
    ar: Vec<String>,
    bs: Vec<Vec<String>>,
    krs: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProofWithTiming {
    proof: ProofResponse,
    proof_duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    // Create shared state
    let state = Arc::new(AppState::new());

    // Build router
    let app = Router::new()
        .route("/prove", post(handle_prove))
        .route("/health", get(handle_health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", args.host, args.port);
    info!("SP1 Prover Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn handle_prove(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<ProofWithTiming>, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();

    let circuit_type = extract_circuit_type(&request).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing circuitType field".to_string(),
            }),
        )
    })?;

    let result = match circuit_type.as_str() {
        "append" => prove_batch_append(&state, request).await,
        "update" => prove_batch_update(&state, request).await,
        "address-append" => prove_batch_address_append(&state, request).await,
        other => Err(anyhow::anyhow!("Unknown circuit type: {}", other)),
    };

    match result {
        Ok(proof) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            info!("Proof generated in {}ms", duration_ms);
            Ok(Json(ProofWithTiming {
                proof,
                proof_duration_ms: duration_ms,
            }))
        }
        Err(e) => {
            error!("Proof generation failed: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

async fn prove_batch_append(
    state: &AppState,
    inputs_json: serde_json::Value,
) -> Result<ProofResponse> {
    let inputs: BatchAppendInputs = serde_json::from_value(inputs_json)?;

    let mut stdin = SP1Stdin::new();
    stdin.write(&inputs);

    let (pk, vk) = state.get_batch_append_keys().await;

    info!("Generating Groth16 proof for BatchAppend...");
    let proof = state
        .client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .map_err(|e| anyhow::anyhow!("Proof generation failed: {}", e))?;

    // Verify proof
    state
        .client
        .verify(&proof, &vk)
        .map_err(|e| anyhow::anyhow!("Proof verification failed: {}", e))?;

    info!("Proof verified successfully");

    convert_sp1_proof_to_gnark(&proof)
}

async fn prove_batch_update(
    state: &AppState,
    inputs_json: serde_json::Value,
) -> Result<ProofResponse> {
    let inputs: BatchUpdateInputs = serde_json::from_value(inputs_json)?;

    let mut stdin = SP1Stdin::new();
    stdin.write(&inputs);

    let (pk, vk) = state.get_batch_update_keys().await;

    info!("Generating Groth16 proof for BatchUpdate...");
    let proof = state
        .client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .map_err(|e| anyhow::anyhow!("Proof generation failed: {}", e))?;

    state
        .client
        .verify(&proof, &vk)
        .map_err(|e| anyhow::anyhow!("Proof verification failed: {}", e))?;

    info!("Proof verified successfully");

    convert_sp1_proof_to_gnark(&proof)
}

async fn prove_batch_address_append(
    state: &AppState,
    inputs_json: serde_json::Value,
) -> Result<ProofResponse> {
    let inputs: BatchAddressAppendInputs = serde_json::from_value(inputs_json)?;

    let mut stdin = SP1Stdin::new();
    stdin.write(&inputs);

    let (pk, vk) = state.get_batch_address_append_keys().await;

    info!("Generating Groth16 proof for BatchAddressAppend...");
    let proof = state
        .client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .map_err(|e| anyhow::anyhow!("Proof generation failed: {}", e))?;

    state
        .client
        .verify(&proof, &vk)
        .map_err(|e| anyhow::anyhow!("Proof verification failed: {}", e))?;

    info!("Proof verified successfully");

    convert_sp1_proof_to_gnark(&proof)
}

/// Convert SP1 Groth16 proof to Gnark-compatible JSON format.
///
/// SP1's Groth16 proof uses BN254 curve, same as Gnark.
/// The proof bytes layout is:
/// - A point (G1): 64 bytes (32 bytes x, 32 bytes y) - big endian
/// - B point (G2): 128 bytes (32 bytes x0, 32 bytes x1, 32 bytes y0, 32 bytes y1) - big endian
/// - C point (G1): 64 bytes (32 bytes x, 32 bytes y) - big endian
///
/// The client handles negation of A and compression, so we just return raw hex strings.
fn convert_sp1_proof_to_gnark(
    proof: &sp1_sdk::SP1ProofWithPublicValues,
) -> Result<ProofResponse> {
    // Get the raw proof bytes from SP1
    let proof_bytes = proof.bytes();

    info!("SP1 proof bytes length: {}", proof_bytes.len());

    // SP1 Groth16 proof structure (256 bytes for the proof points):
    // Offset 0-63: A point (G1) - 64 bytes
    // Offset 64-191: B point (G2) - 128 bytes
    // Offset 192-255: C point (G1) - 64 bytes

    if proof_bytes.len() < 256 {
        return Err(anyhow::anyhow!(
            "SP1 proof too short: expected at least 256 bytes, got {}",
            proof_bytes.len()
        ));
    }

    // Extract and convert A point (G1) to hex strings
    // ar: [x, y] where x and y are 32-byte big-endian hex strings
    let ar = vec![
        format!("0x{}", hex::encode(&proof_bytes[0..32])),
        format!("0x{}", hex::encode(&proof_bytes[32..64])),
    ];

    // Extract and convert B point (G2) to hex strings
    // bs: [[x0, x1], [y0, y1]] where each component is 32-byte big-endian hex
    // Note: G2 point layout is x = x0 + x1*u, y = y0 + y1*u (extension field)
    let bs = vec![
        vec![
            format!("0x{}", hex::encode(&proof_bytes[64..96])),
            format!("0x{}", hex::encode(&proof_bytes[96..128])),
        ],
        vec![
            format!("0x{}", hex::encode(&proof_bytes[128..160])),
            format!("0x{}", hex::encode(&proof_bytes[160..192])),
        ],
    ];

    // Extract and convert C point (G1) to hex strings
    // krs: [x, y] where x and y are 32-byte big-endian hex strings
    let krs = vec![
        format!("0x{}", hex::encode(&proof_bytes[192..224])),
        format!("0x{}", hex::encode(&proof_bytes[224..256])),
    ];

    Ok(ProofResponse { ar, bs, krs })
}
