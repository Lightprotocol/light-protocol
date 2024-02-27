use circuitlib_rs::helpers::init_logger;

use crate::{
    constants::{PROVE, SERVER_ADDRESS},
    helpers::{health_check, kill_gnark_server, prepare_inputs, spawn_gnark_server},
};

#[tokio::test]
async fn prove_inclusion() {
    init_logger();
    let mut gnark = spawn_gnark_server();
    health_check().await;
    let client = reqwest::Client::new();
    for number_of_utxos in &[1, 2, 3, 4, 8] {
        let inputs = prepare_inputs(*number_of_utxos as usize);
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
    }

    kill_gnark_server(&mut gnark);
}
