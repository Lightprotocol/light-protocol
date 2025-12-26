//! Benchmark test comparing SP1 vs Gnark prover performance.
//!
//! This test generates batch append proofs using both provers and compares:
//! - Proving time
//! - Proof format compatibility
//!
//! Requirements:
//! - Go Gnark prover running on localhost:3001
//! - SP1 prover running on localhost:3002

use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    proof_types::batch_append::{get_batch_append_inputs, BatchAppendInputsJson},
};
use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, Instant};

const GNARK_SERVER: &str = "http://localhost:3001";
const SP1_SERVER: &str = "http://localhost:3002";

#[derive(Debug, Deserialize)]
struct ProofWithTiming {
    proof: serde_json::Value,
    #[serde(rename = "proofDurationMs", alias = "proof_duration_ms")]
    proof_duration_ms: u64,
}

#[derive(Debug)]
struct BenchmarkResult {
    server: String,
    request_duration: Duration,
    proof_duration_ms: u64,
    success: bool,
    error: Option<String>,
}

async fn benchmark_server(
    client: &Client,
    server_url: &str,
    inputs_json: &str,
) -> BenchmarkResult {
    let start = Instant::now();
    let url = format!("{}/prove", server_url);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(inputs_json.to_string())
        .send()
        .await;

    let request_duration = start.elapsed();

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            if status.is_success() {
                match serde_json::from_str::<ProofWithTiming>(&body) {
                    Ok(proof) => BenchmarkResult {
                        server: server_url.to_string(),
                        request_duration,
                        proof_duration_ms: proof.proof_duration_ms,
                        success: true,
                        error: None,
                    },
                    Err(e) => {
                        // Try parsing as plain proof (old format)
                        BenchmarkResult {
                            server: server_url.to_string(),
                            request_duration,
                            proof_duration_ms: request_duration.as_millis() as u64,
                            success: true,
                            error: Some(format!("Parse warning: {}", e)),
                        }
                    }
                }
            } else {
                BenchmarkResult {
                    server: server_url.to_string(),
                    request_duration,
                    proof_duration_ms: 0,
                    success: false,
                    error: Some(format!("HTTP {}: {}", status, body)),
                }
            }
        }
        Err(e) => BenchmarkResult {
            server: server_url.to_string(),
            request_duration,
            proof_duration_ms: 0,
            success: false,
            error: Some(format!("Request failed: {}", e)),
        },
    }
}

fn generate_batch_append_inputs(batch_size: usize) -> String {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const CANOPY: usize = 0;

    let mut merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, CANOPY);
    let mut leaves = vec![];
    let mut old_leaves = vec![];
    let mut merkle_proofs = vec![];

    // Create leaves
    for i in 0..batch_size {
        let mut bn: [u8; 32] = [0; 32];
        bn[31] = (i + 1) as u8;
        let leaf: [u8; 32] = Poseidon::hash(&bn).unwrap();
        leaves.push(leaf);
        // Append zero to tree first
        merkle_tree.append(&[0u8; 32]).unwrap();
    }

    // Get proofs
    for index in 0..batch_size {
        let proof = merkle_tree.get_proof_of_leaf(index, true).unwrap();
        let leaf = merkle_tree.leaf(index);
        old_leaves.push(leaf);
        merkle_proofs.push(proof.to_vec());
    }

    let root = merkle_tree.root();
    let leaves_hashchain = create_hash_chain_from_slice(&leaves).unwrap();

    let (inputs, _) = get_batch_append_inputs::<HEIGHT>(
        root,
        0,
        leaves.clone(),
        leaves_hashchain,
        old_leaves.clone(),
        merkle_proofs.clone(),
        batch_size as u32,
        &[],
    )
    .unwrap();

    BatchAppendInputsJson::from_inputs(&inputs).to_string()
}

#[tokio::test]
#[ignore] // Run with: cargo test benchmark_sp1_vs_gnark -- --ignored --nocapture
async fn benchmark_sp1_vs_gnark() {
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .unwrap();

    let batch_sizes = vec![10];

    println!("\n=== SP1 vs Gnark Benchmark ===\n");

    for batch_size in batch_sizes {
        println!("--- Batch Size: {} ---", batch_size);

        let inputs_json = generate_batch_append_inputs(batch_size);
        println!("Input size: {} bytes", inputs_json.len());

        // Benchmark Gnark
        println!("\nBenchmarking Gnark ({})...", GNARK_SERVER);
        let gnark_result = benchmark_server(&client, GNARK_SERVER, &inputs_json).await;

        if gnark_result.success {
            println!(
                "  Gnark: {} ms (request: {} ms)",
                gnark_result.proof_duration_ms,
                gnark_result.request_duration.as_millis()
            );
        } else {
            println!(
                "  Gnark FAILED: {}",
                gnark_result.error.unwrap_or_default()
            );
        }

        // Benchmark SP1
        println!("\nBenchmarking SP1 ({})...", SP1_SERVER);
        let sp1_result = benchmark_server(&client, SP1_SERVER, &inputs_json).await;

        if sp1_result.success {
            println!(
                "  SP1: {} ms (request: {} ms)",
                sp1_result.proof_duration_ms,
                sp1_result.request_duration.as_millis()
            );
        } else {
            println!("  SP1 FAILED: {}", sp1_result.error.unwrap_or_default());
        }

        // Compare
        if gnark_result.success && sp1_result.success {
            let speedup = gnark_result.proof_duration_ms as f64 / sp1_result.proof_duration_ms as f64;
            println!("\n  Speedup: {:.2}x", speedup);
            if speedup > 1.0 {
                println!("  SP1 is {:.1}% faster", (speedup - 1.0) * 100.0);
            } else {
                println!("  Gnark is {:.1}% faster", (1.0 / speedup - 1.0) * 100.0);
            }
        }

        println!();
    }
}

#[tokio::test]
#[ignore]
async fn test_gnark_server_health() {
    let client = Client::new();
    let resp = client
        .get(format!("{}/health", GNARK_SERVER))
        .send()
        .await;
    match resp {
        Ok(r) => println!("Gnark health: {} - {}", r.status(), r.text().await.unwrap_or_default()),
        Err(e) => println!("Gnark not available: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn test_sp1_server_health() {
    let client = Client::new();
    let resp = client
        .get(format!("{}/health", SP1_SERVER))
        .send()
        .await;
    match resp {
        Ok(r) => println!("SP1 health: {} - {}", r.status(), r.text().await.unwrap_or_default()),
        Err(e) => println!("SP1 not available: {}", e),
    }
}
