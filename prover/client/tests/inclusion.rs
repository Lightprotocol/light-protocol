use light_prover_client::{
    constants::{PROVE_PATH, SERVER_ADDRESS},
    prover::spawn_prover,
};
use reqwest::Client;
use serial_test::serial;
mod init_merkle_tree;
use crate::init_merkle_tree::{inclusion_inputs_string_v1, inclusion_inputs_string_v2};

#[serial]
#[tokio::test]
async fn prove_inclusion() {
    spawn_prover().await;
    let client = Client::new();

    // v2 - test all keys from 1 to 20
    for number_of_utxos in 1..=20 {
        let inputs = inclusion_inputs_string_v2(number_of_utxos);
        let response_result = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
    }

    // v1 height 26
    {
        for number_of_utxos in &[1, 2, 3, 4, 8] {
            let inputs = inclusion_inputs_string_v1(*number_of_utxos as usize);
            let response_result = client
                .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs)
                .send()
                .await
                .expect("Failed to execute request.");
            assert!(response_result.status().is_success());
        }
    }
}
