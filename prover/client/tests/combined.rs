use light_prover_client::{
    constants::{PROVE_PATH, SERVER_ADDRESS},
    prover::spawn_prover,
};
use reqwest::Client;
use serial_test::serial;
mod init_merkle_tree;
use crate::init_merkle_tree::{combined_inputs_string_v1, combined_inputs_string_v2};

#[serial]
#[tokio::test]
async fn prove_combined() {
    spawn_prover().await;
    let client = Client::new();
    {
        for i in 1..=4 {
            for non_i in 1..=2 {
                let inputs = combined_inputs_string_v1(i, non_i);
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
    // V2 combined circuits - test 1..4 / 1..4
    {
        for i in 1..=4 {
            for non_i in 1..=4 {
                let inputs = combined_inputs_string_v2(i, non_i);
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
}
