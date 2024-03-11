use circuitlib_rs::helpers::init_logger;

use crate::{
    constants::{INCLUSION_PATH, SERVER_ADDRESS},
    helpers::{health_check, kill_gnark_server, spawn_gnark_server},
    inclusion_json_formatter::inclusion_inputs_string,
};

#[tokio::test]
async fn prove_inclusion() {
    init_logger();
    let mut gnark = spawn_gnark_server();
    health_check().await;
    let client = reqwest::Client::new();
    for number_of_utxos in &[1, 2, 3, 4, 8] {
        let inputs = inclusion_inputs_string(*number_of_utxos as usize);
        println!("Inputs utxo {} inclusion: {}", number_of_utxos, inputs);
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, INCLUSION_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
    }
    kill_gnark_server(&mut gnark);
}
