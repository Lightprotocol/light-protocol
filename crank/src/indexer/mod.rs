mod models;
use serde_json::json;
use solana_sdk::bs58;
use crate::constants::{INDEXER_PROOF_PATH, INDEXER_SERVER_ADDRESS};
use crate::indexer::models::PhotonProofJson;

pub fn get_photon_proof(compressed_account: &[u8; 32]) -> Vec<[u8; 32]> {
    let client = reqwest::blocking::Client::new();
    let compressed_account_bs58 = bs58::encode(compressed_account).into_string();

    let payload = json!({
        "id": "test-account",
        "jsonrpc": "2.0",
        "method": "getCompressedAccountProof",
        "params": compressed_account_bs58
    });

    let response_result = client
        .post(format!("{}{}", INDEXER_SERVER_ADDRESS, INDEXER_PROOF_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(serde_json::to_string(&payload).unwrap())
        .send()
        .expect("Failed to execute request.");

    let body = response_result.text().unwrap();
    let proof_json = deserialize_photon_proof_json(&body).unwrap();

    let mut proof_vec = Vec::new();
    for proof in proof_json.value.proof.iter() {
        let bytes = bs58::decode(proof).into_vec().unwrap();
        let mut proof_arr = [0u8; 32];
        proof_arr.copy_from_slice(&bytes);
        proof_vec.push(proof_arr);
    }
    proof_vec
}

fn deserialize_photon_proof_json(json_data: &str) -> serde_json::Result<PhotonProofJson> {
    let deserialized_data: PhotonProofJson = serde_json::from_str(json_data)?;
    Ok(deserialized_data)
}

