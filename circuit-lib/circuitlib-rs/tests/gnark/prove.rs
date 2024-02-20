use circuitlib_rs::helpers::init_logger;
use reqwest::header::CONTENT_TYPE;

use crate::{
    constants::{PROVE, SERVER_ADDRESS},
    helpers::{health_check, kill_gnark_server, prepare_inputs, spawn_gnark_server},
};

#[tokio::test]
async fn prove_inclusion() {
    let number_of_utxos = 1;
    init_logger();
    let mut gnark = spawn_gnark_server();
    health_check().await;
    let inputs = prepare_inputs(number_of_utxos);
    let client = reqwest::Client::new();
    let response_result = client
        .post(&format!("{}{}", SERVER_ADDRESS, PROVE))
        .header("Content-Type", CONTENT_TYPE)
        .body(inputs)
        .send()
        .await
        .expect("Failed to execute request.");
    assert!(response_result.status().is_success());
    kill_gnark_server(&mut gnark);
}
