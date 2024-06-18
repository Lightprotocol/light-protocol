use light_prover_client::gnark::helpers::{spawn_prover, ProofType};
use light_prover_client::{
    gnark::{
        constants::{PROVE_PATH, SERVER_ADDRESS},
        inclusion_json_formatter::inclusion_inputs_string,
    },
    helpers::init_logger,
};
use reqwest::Client;

#[tokio::test]
async fn prove_inclusion() {
    init_logger();
    spawn_prover(false, &[ProofType::Inclusion]).await;
    let client = Client::new();
    for number_of_utxos in &[1, 2, 3, 4, 8] {
        let (inputs, _) = inclusion_inputs_string(*number_of_utxos as usize);
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
    }
}
