use light_prover_client::{
    constants::{PROVE_PATH, SERVER_ADDRESS},
    prover::{spawn_prover, ProverConfig},
};
use reqwest::Client;
use serial_test::serial;
mod init_merkle_tree;
use crate::init_merkle_tree::{non_inclusion_inputs_string_v1, non_inclusion_inputs_string_v2};

#[serial]
#[tokio::test]
async fn prove_non_inclusion() {
    spawn_prover(ProverConfig::default()).await;
    let client = Client::new();
    // legacy height 26
    {
        for i in 1..=2 {
            let (inputs, _) = non_inclusion_inputs_string_v1(i);
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
    // height 40
    {
        for i in [1, 2].iter() {
            let inputs = non_inclusion_inputs_string_v2(i.to_owned());

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
