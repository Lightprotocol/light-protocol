use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for CompressedProof {}

#[derive(Debug)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub address_root_indices: Vec<u16>,
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for ProofRpcResult {}

async fn create_proof_for_compressed_accounts(
    compressed_accounts: Option<&[[u8; 32]]>,
    state_merkle_tree_pubkeys: Option<&[Pubkey]>,
    new_addresses: Option<&[[u8; 32]]>,
    address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
    rpc: &mut R,
) -> ProofRpcResult {
    if compressed_accounts.is_some()
        && ![1usize, 2usize, 3usize, 4usize, 8usize].contains(&compressed_accounts.unwrap().len())
    {
        panic!(
            "compressed_accounts must be of length 1, 2, 3, 4 or 8 != {}",
            compressed_accounts.unwrap().len()
        )
    }
    if new_addresses.is_some() && ![1usize, 2usize].contains(&new_addresses.unwrap().len()) {
        panic!("new_addresses must be of length 1, 2")
    }
    let client = Client::new();
    let (root_indices, address_root_indices, json_payload) =
        match (compressed_accounts, new_addresses) {
            (Some(accounts), None) => {
                let (payload, indices) = self
                    .process_inclusion_proofs(state_merkle_tree_pubkeys.unwrap(), accounts, rpc)
                    .await;
                (indices, Vec::new(), payload.to_string())
            }
            (None, Some(addresses)) => {
                let (payload, indices) = self
                    .process_non_inclusion_proofs(
                        address_merkle_tree_pubkeys.unwrap().as_slice(),
                        addresses,
                        rpc,
                    )
                    .await;
                (Vec::<u16>::new(), indices, payload.to_string())
            }
            (Some(accounts), Some(addresses)) => {
                let (inclusion_payload, inclusion_indices) = self
                    .process_inclusion_proofs(state_merkle_tree_pubkeys.unwrap(), accounts, rpc)
                    .await;
                let (non_inclusion_payload, non_inclusion_indices) = self
                    .process_non_inclusion_proofs(
                        address_merkle_tree_pubkeys.unwrap().as_slice(),
                        addresses,
                        rpc,
                    )
                    .await;

                let combined_payload = CombinedJsonStruct {
                    inclusion: inclusion_payload.inputs,
                    non_inclusion: non_inclusion_payload.inputs,
                }
                .to_string();
                (inclusion_indices, non_inclusion_indices, combined_payload)
            }
            _ => {
                panic!("At least one of compressed_accounts or new_addresses must be provided")
            }
        };

    let mut retries = 3;
    while retries > 0 {
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(json_payload.clone())
            .send()
            .await
            .expect("Failed to execute request.");
        if response_result.status().is_success() {
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            return ProofRpcResult {
                root_indices,
                address_root_indices,
                proof: CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
            };
        } else {
            warn!("Error: {}", response_result.text().await.unwrap());
            tokio::time::sleep(Duration::from_secs(1)).await;
            spawn_prover(true, self.proof_types.as_slice()).await;
            retries -= 1;
        }
    }
    panic!("Failed to get proof from server");
}
