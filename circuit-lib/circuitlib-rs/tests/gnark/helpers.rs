use std::{
    process::{Child, Command},
    thread,
    time::Duration,
};

use circuitlib_rs::{init_merkle_tree::merkle_tree_inputs, merkle_proof_inputs::MerkleTreeInfo};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;
use serde_json::json;

use crate::constants::{HEALTH_CHECK, SERVER_ADDRESS};

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct JsonStruct {
    root: Vec<String>,
    leaf: Vec<String>,
    inPathIndices: Vec<u32>,
    inPathElements: Vec<Vec<String>>,
}

impl JsonStruct {
    fn new(number_of_utxos: usize) -> Self {
        let merkle_inputs = merkle_tree_inputs(MerkleTreeInfo::H26);
        let roots = create_vec_of_string(number_of_utxos, &merkle_inputs.root);
        let leafs = create_vec_of_string(number_of_utxos, &merkle_inputs.leaf);
        let in_path_indices = create_vec_of_u32(number_of_utxos, &merkle_inputs.in_path_indices);
        let in_path_elements =
            create_vec_of_vec_of_string(number_of_utxos, &merkle_inputs.in_path_elements);
        Self {
            root: roots,
            leaf: leafs,
            inPathIndices: in_path_indices,
            inPathElements: in_path_elements,
        }
    }
}
pub fn prepare_inputs(number_of_utxos: usize) -> String {
    let json_struct = JsonStruct::new(number_of_utxos);
    create_json_from_struct(&json_struct)
}

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

pub fn create_json_from_struct(json_struct: &JsonStruct) -> String {
    let json = json!(json_struct);
    match serde_json::to_string_pretty(&json) {
        Ok(json) => json,
        Err(_) => panic!("Merkle tree data invalid"),
    }
}
