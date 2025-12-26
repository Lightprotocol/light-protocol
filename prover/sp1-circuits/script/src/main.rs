//! SP1 Proof Generation Script
//!
//! This script generates SP1 proofs for batch circuits with Groth16 wrapping.
//!
//! Usage:
//!   prove --circuit batch-append --input inputs.json --output proof.json

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use sp1_circuits_lib::{BatchAddressAppendInputs, BatchAppendInputs, BatchUpdateInputs};
use sp1_sdk::{ProverClient, SP1Stdin};
use std::fs;
use std::path::PathBuf;

/// ELF binaries for SP1 programs (built by sp1-build)
const BATCH_APPEND_ELF: &[u8] =
    include_bytes!("../../programs/batch-append/elf/batch-append-program");
const BATCH_UPDATE_ELF: &[u8] =
    include_bytes!("../../programs/batch-update/elf/batch-update-program");
const BATCH_ADDRESS_APPEND_ELF: &[u8] =
    include_bytes!("../../programs/batch-address-append/elf/batch-address-append-program");

#[derive(Debug, Clone, ValueEnum)]
enum CircuitType {
    BatchAppend,
    BatchUpdate,
    BatchAddressAppend,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Circuit type to prove
    #[arg(short, long)]
    circuit: CircuitType,

    /// Input JSON file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output proof JSON file path
    #[arg(short, long)]
    output: PathBuf,

    /// Use Groth16 wrapping for on-chain verification
    #[arg(long, default_value = "true")]
    groth16: bool,
}

fn main() -> Result<()> {
    // Initialize logging
    sp1_sdk::utils::setup_logger();

    let args = Args::parse();

    // Read input JSON
    let input_json = fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {:?}", args.input))?;

    // Generate proof based on circuit type
    let proof_json = match args.circuit {
        CircuitType::BatchAppend => prove_batch_append(&input_json, args.groth16)?,
        CircuitType::BatchUpdate => prove_batch_update(&input_json, args.groth16)?,
        CircuitType::BatchAddressAppend => prove_batch_address_append(&input_json, args.groth16)?,
    };

    // Write output
    fs::write(&args.output, &proof_json)
        .with_context(|| format!("Failed to write output file: {:?}", args.output))?;

    println!("Proof written to {:?}", args.output);
    Ok(())
}

fn prove_batch_append(input_json: &str, use_groth16: bool) -> Result<String> {
    // Parse input
    let inputs: BatchAppendInputs =
        serde_json::from_str(input_json).context("Failed to parse BatchAppend inputs")?;

    // Create SP1 stdin with inputs
    let mut stdin = SP1Stdin::new();
    stdin.write(&inputs);

    // Create prover client
    let client = ProverClient::from_env();

    // Setup (compile program to proving key)
    let (pk, vk) = client.setup(BATCH_APPEND_ELF);

    // Generate proof
    let proof = if use_groth16 {
        println!("Generating Groth16 proof (this may take a while)...");
        client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .context("Failed to generate Groth16 proof")?
    } else {
        println!("Generating compressed STARK proof...");
        client
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .context("Failed to generate compressed proof")?
    };

    // Verify proof
    client
        .verify(&proof, &vk)
        .context("Failed to verify proof")?;
    println!("Proof verified successfully!");

    // Serialize proof to JSON
    // Note: SP1 Groth16 proof format needs to be converted to match Gnark format
    let proof_bytes = bincode::serialize(&proof).context("Failed to serialize proof")?;

    // Create output JSON matching Gnark format
    // The actual format conversion will be done in the HTTP server
    let output = serde_json::json!({
        "proof": hex::encode(&proof_bytes),
        "proofType": if use_groth16 { "groth16" } else { "compressed" },
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

fn prove_batch_update(input_json: &str, use_groth16: bool) -> Result<String> {
    // Parse input
    let inputs: BatchUpdateInputs =
        serde_json::from_str(input_json).context("Failed to parse BatchUpdate inputs")?;

    // Create SP1 stdin with inputs
    let mut stdin = SP1Stdin::new();
    stdin.write(&inputs);

    // Create prover client
    let client = ProverClient::from_env();

    // Setup (compile program to proving key)
    let (pk, vk) = client.setup(BATCH_UPDATE_ELF);

    // Generate proof
    let proof = if use_groth16 {
        println!("Generating Groth16 proof for BatchUpdate...");
        client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .context("Failed to generate Groth16 proof")?
    } else {
        println!("Generating compressed STARK proof for BatchUpdate...");
        client
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .context("Failed to generate compressed proof")?
    };

    // Verify proof
    client
        .verify(&proof, &vk)
        .context("Failed to verify proof")?;
    println!("Proof verified successfully!");

    // Serialize proof to JSON
    let proof_bytes = bincode::serialize(&proof).context("Failed to serialize proof")?;

    let output = serde_json::json!({
        "proof": hex::encode(&proof_bytes),
        "proofType": if use_groth16 { "groth16" } else { "compressed" },
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

fn prove_batch_address_append(input_json: &str, use_groth16: bool) -> Result<String> {
    // Parse input
    let inputs: BatchAddressAppendInputs =
        serde_json::from_str(input_json).context("Failed to parse BatchAddressAppend inputs")?;

    // Create SP1 stdin with inputs
    let mut stdin = SP1Stdin::new();
    stdin.write(&inputs);

    // Create prover client
    let client = ProverClient::from_env();

    // Setup (compile program to proving key)
    let (pk, vk) = client.setup(BATCH_ADDRESS_APPEND_ELF);

    // Generate proof
    let proof = if use_groth16 {
        println!("Generating Groth16 proof for BatchAddressAppend...");
        client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .context("Failed to generate Groth16 proof")?
    } else {
        println!("Generating compressed STARK proof for BatchAddressAppend...");
        client
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .context("Failed to generate compressed proof")?
    };

    // Verify proof
    client
        .verify(&proof, &vk)
        .context("Failed to verify proof")?;
    println!("Proof verified successfully!");

    // Serialize proof to JSON
    let proof_bytes = bincode::serialize(&proof).context("Failed to serialize proof")?;

    let output = serde_json::json!({
        "proof": hex::encode(&proof_bytes),
        "proofType": if use_groth16 { "groth16" } else { "compressed" },
    });

    Ok(serde_json::to_string_pretty(&output)?)
}
