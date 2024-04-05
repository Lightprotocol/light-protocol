use std::{
    fmt::{Display, Formatter},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;
use serde_json::json;
use sysinfo::{Signal, System};

use crate::gnark::constants::{HEALTH_CHECK, SERVER_ADDRESS};

static IS_LOADING: AtomicBool = AtomicBool::new(false);

pub enum ProofType {
    Inclusion,
    NonInclusion,
    Combined,
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
            }
        )
    }
}
pub async fn spawn_gnark_server(path: &str, restart: bool, proof_type: ProofType) {
    if restart {
        kill_gnark_server();
    }
    if !health_check(1, 3).await && !IS_LOADING.load(Ordering::Relaxed) {
        IS_LOADING.store(true, Ordering::Relaxed);
        Command::new("sh")
            .arg("-c")
            .arg(format!("{} {}", path, proof_type))
            .spawn()
            .expect("Failed to start server process");
        health_check(20, 5).await;
        IS_LOADING.store(false, Ordering::Relaxed);
    }
}

pub fn kill_gnark_server() {
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

pub fn create_vec_of_string(number_of_utxos: usize, element: &BigInt) -> Vec<String> {
    vec![format!("0x{}", element.to_str_radix(16)); number_of_utxos]
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
