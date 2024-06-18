use log::info;
use std::{
    fmt::{Display, Formatter},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::gnark::constants::{HEALTH_CHECK, SERVER_ADDRESS};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;
use serde_json::json;
use sysinfo::{Signal, System};

static IS_LOADING: AtomicBool = AtomicBool::new(false);
#[derive(Debug, Clone)]
pub enum ProofType {
    Inclusion,
    NonInclusion,
}

impl Display for ProofType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProofType::Inclusion => "inclusion",
                ProofType::NonInclusion => "non-inclusion",
            }
        )
    }
}

pub async fn spawn_prover(restart: bool, proof_types: &[ProofType]) {
    if let Some(project_root) = get_project_root() {
        let path = "circuit-lib/light-prover-client/scripts/prover.sh";
        let absolute_path = format!("{}/{}", project_root.trim(), path);
        if restart {
            kill_prover();
        }

        if !health_check(1, 3).await && !IS_LOADING.load(Ordering::Relaxed) {
            IS_LOADING.store(true, Ordering::Relaxed);

            let proof_type_args: Vec<String> = proof_types.iter().map(|p| p.to_string()).collect();
            let proof_type_str = proof_type_args.join(" ");
            Command::new("sh")
                .arg("-c")
                .arg(format!("{} {}", absolute_path, proof_type_str))
                .spawn()
                .expect("Failed to start server process");
            let health_result = health_check(20, 5).await;
            if health_result {
                info!("Prover started successfully");
            } else {
                panic!("Prover failed to start");
            }
            IS_LOADING.store(false, Ordering::Relaxed);
        }
    } else {
        panic!("Failed to determine the project root directory");
    }
}

pub fn kill_prover() {
    let mut system = System::new_all();
    system.refresh_all();

    for process in system.processes().values() {
        if process.name() == "light-prover" {
            process.kill_with(Signal::Term);
        }
    }
}

pub async fn health_check(retries: usize, timeout: usize) -> bool {
    let client = reqwest::Client::new();
    let mut result = false;
    for _ in 0..retries {
        match client
            .get(&format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK))
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

fn get_project_root() -> Option<String> {
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

pub fn big_int_to_string(big_int: &BigInt) -> String {
    format!("0x{}", big_int.to_str_radix(16))
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
