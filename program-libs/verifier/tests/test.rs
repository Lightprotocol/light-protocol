#[cfg(test)]
mod test {

    use light_compressed_account::{
        hash_chain::{
            create_hash_chain_from_array, create_hash_chain_from_slice,
            create_two_inputs_hash_chain,
        },
        instruction_data::compressed_proof::CompressedProof,
    };
    use light_prover_client::{
        gnark::{
            constants::{get_server_address, PROVE_PATH},
            helpers::{kill_prover, spawn_prover, ProverConfig},
            inclusion_json_formatter::inclusion_inputs_string,
            proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
        },
        inclusion::merkle_tree_info::MerkleTreeInfo,
        init_merkle_tree::inclusion_merkle_tree_inputs,
    };
    use light_verifier::{select_verifying_key, verify};
    use reqwest::Client;
    use serial_test::serial;

    #[serial]
    #[tokio::test]
    async fn prove_inclusion() {
        spawn_prover(ProverConfig::default()).await;
        let client = Client::new();
        for number_of_compressed_accounts in &[1usize, 2, 3] {
            let big_int_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H32);

            let inputs = inclusion_inputs_string(*number_of_compressed_accounts);
            let response_result = client
                .post(format!("{}{}", get_server_address(), PROVE_PATH))
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
            let public_input_hash = create_two_inputs_hash_chain(&roots, &leaves).unwrap();
            let vk = select_verifying_key(leaves.len(), 0).unwrap();
            verify::<1>(
                &[public_input_hash],
                &CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
                vk,
            )
            .unwrap();
        }
        kill_prover();
    }

    #[tokio::test]
    #[ignore]
    async fn prove_inclusion_full() {
        spawn_prover(ProverConfig::default()).await;
        let client = Client::new();
        for number_of_compressed_accounts in &[1usize, 2, 3, 4, 8] {
            let big_int_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H26);

            let inputs = inclusion_inputs_string(*number_of_compressed_accounts);
            let response_result = client
                .post(format!("{}{}", get_server_address(), PROVE_PATH))
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

            let roots_hash_chain = create_hash_chain_from_slice(&roots).unwrap();
            let leaves_hash_chain = create_hash_chain_from_slice(&leaves).unwrap();
            let public_input_hash =
                create_hash_chain_from_array([roots_hash_chain, leaves_hash_chain]).unwrap();
            let vk = select_verifying_key(leaves.len(), 0).unwrap();
            verify::<1>(
                &[public_input_hash],
                &CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
                vk,
            )
            .unwrap();
        }
        kill_prover();
    }
}
