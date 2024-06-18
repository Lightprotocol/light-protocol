#[cfg(test)]
mod test {
    use light_prover_client::{
        gnark::{
            constants::{PROVE_PATH, SERVER_ADDRESS},
            helpers::{kill_prover, spawn_prover, ProofType},
            inclusion_json_formatter::inclusion_inputs_string,
            proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
        },
        helpers::init_logger,
    };
    use light_verifier::{verify_merkle_proof_zkp, CompressedProof};
    use reqwest::Client;

    #[tokio::test]
    async fn prove_inclusion() {
        init_logger();
        spawn_prover(false, &[ProofType::Inclusion]).await;
        let client = Client::new();
        for number_of_compressed_accounts in &[1usize, 2, 3, 4, 8] {
            let (inputs, big_int_inputs) = inclusion_inputs_string(*number_of_compressed_accounts);
            let response_result = client
                .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs)
                .send()
                .await
                .expect("Failed to execute request.");
            assert!(response_result.status().is_success());
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            let mut roots = Vec::<[u8; 32]>::new();
            let mut leaves = Vec::<[u8; 32]>::new();

            for _ in 0..*number_of_compressed_accounts {
                roots.push(big_int_inputs.root.to_bytes_be().1.try_into().unwrap());
                leaves.push(big_int_inputs.leaf.to_bytes_be().1.try_into().unwrap());
            }

            verify_merkle_proof_zkp(
                &roots,
                &leaves,
                &CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
            )
            .unwrap();
        }
        kill_prover();
    }
}
