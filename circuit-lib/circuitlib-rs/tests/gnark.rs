use light_circuitlib_rs::{
    gnark::{
        constants::{INCLUSION_PATH, SERVER_ADDRESS},
        helpers::{kill_gnark_server, spawn_gnark_server, ProofType},
        inclusion_json_formatter::inclusion_inputs_string,
    },
    helpers::init_logger,
};
use reqwest::Client;

#[tokio::test]
async fn prove_inclusion() {
    init_logger();
    spawn_gnark_server("scripts/prover.sh", true, &[ProofType::Inclusion]).await;
    let client = Client::new();
    for number_of_utxos in &[1, 2, 3, 4, 8] {
        let (inputs, _) = inclusion_inputs_string(*number_of_utxos as usize);
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
    kill_gnark_server();
}
