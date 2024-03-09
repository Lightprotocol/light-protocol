use std::{
    process::{Child, Command},
    thread,
    time::Duration,
};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;
use serde_json::json;

use crate::constants::{HEALTH_CHECK, SERVER_ADDRESS};

pub fn spawn_gnark_server() -> Child {
    let server_process = Command::new("sh")
        .arg("-c")
        .arg("scripts/prover.sh")
        .spawn()
        .expect("Failed to start server process");

    // Wait for the server to launch before proceeding.
    thread::sleep(Duration::from_secs(5));

    server_process
}

pub fn kill_gnark_server(gnark: &mut Child) {
    Command::new("sh")
        .arg("-c")
        .arg("killall light-prover")
        .spawn()
        .unwrap();
    gnark.kill().unwrap();
}

pub async fn health_check() {
    const MAX_RETRIES: usize = 20;
    const TIMEOUT: usize = 5;

    let client = reqwest::Client::new();

    for _ in 0..MAX_RETRIES {
        match client
            .get(&format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK))
            .send()
            .await
        {
            Ok(_) => break,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(TIMEOUT as u64)).await;
            }
        }
    }
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
