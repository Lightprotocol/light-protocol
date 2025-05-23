use std::{
    fmt::{Display, Formatter},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use num_bigint::{BigInt, BigUint};
use num_traits::{Num, ToPrimitive};
use serde::Serialize;
use serde_json::json;
use tracing::info;

use crate::gnark::constants::{HEALTH_CHECK, SERVER_ADDRESS};

static IS_LOADING: AtomicBool = AtomicBool::new(false);
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProverMode {
    Rpc,
    Forester,
    ForesterTest,
    Full,
    FullTest,
}

impl Display for ProverMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProverMode::Rpc => "rpc",
                ProverMode::Forester => "forester",
                ProverMode::ForesterTest => "forester-test",
                ProverMode::Full => "full",
                ProverMode::FullTest => "full-test",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProofType {
    Inclusion,
    NonInclusion,
    Combined,
    BatchAppendWithProofs,
    BatchUpdate,
    BatchAddressAppend,
    BatchUpdateTest,
    BatchAppendWithProofsTest,
    BatchAddressAppendTest,
}

impl Display for ProofType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProofType::Inclusion => "inclusion",
                ProofType::NonInclusion => "non-inclusion",
                ProofType::Combined => "combined",
                ProofType::BatchAppendWithProofs => "append-with-proofs",
                ProofType::BatchUpdate => "update",
                ProofType::BatchUpdateTest => "update-test",
                ProofType::BatchAppendWithProofsTest => "append-with-proofs-test",
                ProofType::BatchAddressAppend => "addressAppend",
                ProofType::BatchAddressAppendTest => "address-append-test",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProverConfig {
    pub run_mode: Option<ProverMode>,
    pub circuits: Vec<ProofType>,
}

impl Default for ProverConfig {
    #[cfg(feature = "devenv")]
    fn default() -> Self {
        Self {
            run_mode: Some(ProverMode::ForesterTest),
            circuits: vec![],
        }
    }
    #[cfg(not(feature = "devenv"))]
    fn default() -> Self {
        Self {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }
    }
}

impl ProverConfig {
    pub fn rpc_no_restart() -> Self {
        Self {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }
    }
}

pub async fn spawn_prover(config: ProverConfig) {
    if let Some(_project_root) = get_project_root() {
        let prover_path: &str = {
            #[cfg(feature = "devenv")]
            {
                &format!("{}/{}", _project_root.trim(), "cli/test_bin/run")
            }
            #[cfg(not(feature = "devenv"))]
            {
                println!("Running in production mode, using prover binary");
                "light"
            }
        };

        if !health_check(10, 1).await && !IS_LOADING.load(Ordering::Relaxed) {
            IS_LOADING.store(true, Ordering::Relaxed);

            let mut command = Command::new(prover_path);
            command.arg("start-prover");

            if let Some(ref mode) = config.run_mode {
                command.arg("--run-mode").arg(mode.to_string());
            }

            for circuit in config.circuits.clone() {
                command.arg("--circuit").arg(circuit.to_string());
            }

            println!("Starting prover with command: {:?}", command);

            let _ = command
                .spawn()
                .expect("Failed to start prover process")
                .wait();

            let health_result = health_check(120, 1).await;
            if health_result {
                info!("Prover started successfully");
            } else {
                panic!("Failed to start prover, health check failed.");
            }
        }
        #[cfg(not(feature = "devenv"))]
        {
            "light"
        }
    } else {
        panic!("Failed to find project root.");
    };
}

pub async fn health_check(retries: usize, timeout: usize) -> bool {
    let client = reqwest::Client::new();
    let mut result = false;
    for _ in 0..retries {
        match client
            .get(format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK))
            .send()
            .await
        {
            Ok(_) => {
                result = true;
                break;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(timeout as u64)).await;
            }
        }
    }
    result
}

pub fn get_project_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

pub fn big_uint_to_string(big_uint: &BigUint) -> String {
    format!("0x{}", big_uint.to_str_radix(16))
}

pub fn big_int_to_string(big_int: &BigInt) -> String {
    format!("0x{}", big_int.to_str_radix(16))
}
pub fn string_to_big_int(hex_str: &str) -> Option<BigInt> {
    if hex_str.starts_with("0x") || hex_str.starts_with("0X") {
        BigInt::from_str_radix(&hex_str[2..], 16).ok()
    } else {
        None
    }
}

pub fn create_vec_of_string(number_of_utxos: usize, element: &BigInt) -> Vec<String> {
    vec![big_int_to_string(element); number_of_utxos]
}

pub fn create_vec_of_u32(number_of_utxos: usize, element: &BigInt) -> Vec<u32> {
    vec![element.to_u32().unwrap(); number_of_utxos]
}

pub fn create_vec_of_vec_of_string(
    number_of_utxos: usize,
    elements: &[BigInt],
) -> Vec<Vec<String>> {
    let vec: Vec<String> = elements
        .iter()
        .map(|e| format!("0x{}", e.to_str_radix(16)))
        .collect();
    vec![vec; number_of_utxos]
}

pub fn create_json_from_struct<T>(json_struct: &T) -> String
where
    T: Serialize,
{
    let json = json!(json_struct);
    match serde_json::to_string_pretty(&json) {
        Ok(json) => json,
        Err(_) => panic!("Merkle tree data invalid"),
    }
}

#[derive(Debug)]
pub struct LightValidatorConfig {
    pub enable_indexer: bool,
    pub prover_config: Option<ProverConfig>,
    pub wait_time: u64,
    pub sbf_programs: Vec<(String, String)>,
    pub limit_ledger_size: Option<u64>,
}

impl Default for LightValidatorConfig {
    fn default() -> Self {
        Self {
            enable_indexer: false,
            prover_config: None,
            wait_time: 35,
            sbf_programs: vec![],
            limit_ledger_size: None,
        }
    }
}

// TODO: move to light-client
pub async fn spawn_validator(config: LightValidatorConfig) {
    if let Some(project_root) = get_project_root() {
        let path = "cli/test_bin/run test-validator";
        let mut path = format!("{}/{}", project_root.trim(), path);
        if !config.enable_indexer {
            path.push_str(" --skip-indexer");
        }

        if let Some(limit_ledger_size) = config.limit_ledger_size {
            path.push_str(&format!(" --limit-ledger-size {}", limit_ledger_size));
        }

        for sbf_program in config.sbf_programs.iter() {
            path.push_str(&format!(
                " --sbf-program {} {}",
                sbf_program.0, sbf_program.1
            ));
        }

        if let Some(prover_config) = config.prover_config {
            prover_config.circuits.iter().for_each(|circuit| {
                path.push_str(&format!(" --circuit {}", circuit));
            });
            if let Some(prover_mode) = prover_config.run_mode {
                path.push_str(&format!(" --prover-run-mode {}", prover_mode));
            }
        } else {
            path.push_str(" --skip-prover");
        }

        println!("Starting validator with command: {}", path);

        let child = Command::new("sh")
            .arg("-c")
            .arg(path)
            .stdin(Stdio::null()) // Detach from stdin
            .stdout(Stdio::null()) // Detach from stdout
            .stderr(Stdio::null()) // Detach from stderr
            .spawn()
            .expect("Failed to start server process");

        // Explicitly `drop` the process to ensure we don't wait on it
        std::mem::drop(child);

        tokio::time::sleep(tokio::time::Duration::from_secs(config.wait_time)).await;
    }
}
