use crate::utils::decode_hash;
use account_compression::initialize_address_merkle_tree::Pubkey;
use light_test_utils::indexer::{Indexer, IndexerError, MerkleProof, NewAddressProofWithContext};
use log::info;
use photon_api::apis::configuration::Configuration;
use photon_api::models::GetCompressedAccountsByOwnerPostRequestParams;
use solana_sdk::bs58;
use std::fmt::Debug;

pub struct PhotonIndexer {
    configuration: Configuration,
}

impl PhotonIndexer {
    pub fn new(path: String) -> Self {
        let configuration = Configuration {
            base_path: path,
            ..Default::default()
        };

        PhotonIndexer { configuration }
    }
}

impl Debug for PhotonIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotonIndexer")
            .field("configuration", &self.configuration)
            .field("configuration", &self.configuration)
            .finish()
    }
}

impl Clone for PhotonIndexer {
    fn clone(&self) -> Self {
        PhotonIndexer {
            configuration: self.configuration.clone(),
        }
    }
}

impl Indexer for PhotonIndexer {
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        info!("Getting proofs for {:?}", hashes);
        let request = photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
            params: hashes,
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
            &self.configuration,
            request,
        )
        .await;

        match result {
            Ok(response) => {
                match response.result {
                    Some(result) => {
                        let proofs = result
                            .value
                            .iter()
                            .map(|x| {
                                let mut proof_result_value = x.proof.clone();
                                proof_result_value.truncate(proof_result_value.len() - 10); // Remove canopy
                                let proof: Vec<[u8; 32]> =
                                    proof_result_value.iter().map(|x| decode_hash(x)).collect();
                                MerkleProof {
                                    hash: x.hash.clone(),
                                    leaf_index: x.leaf_index,
                                    merkle_tree: x.merkle_tree.clone(),
                                    proof,
                                    root_seq: x.root_seq,
                                }
                            })
                            .collect();

                        Ok(proofs)
                    }
                    None => Err(IndexerError::Custom("No result".to_string())),
                }
            }
            Err(e) => Err(IndexerError::Custom(e.to_string())),
        }
    }

    async fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError> {
        let request = photon_api::models::GetCompressedAccountsByOwnerPostRequest {
            params: Box::from(GetCompressedAccountsByOwnerPostRequestParams {
                cursor: None,
                limit: None,
                owner: owner.to_string(),
            }),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_accounts_by_owner_post(
            &self.configuration,
            request,
        )
        .await
        .unwrap();

        let accs = result.result.unwrap().value;
        let mut hashes = Vec::new();
        for acc in accs.items {
            hashes.push(acc.hash);
        }

        Ok(hashes)
    }

    async fn get_multiple_new_address_proofs(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        address: [u8; 32],
    ) -> Result<NewAddressProofWithContext, IndexerError> {
        let request = photon_api::models::GetMultipleNewAddressProofsPostRequest {
            params: vec![bs58::encode(address).into_string()],
            ..Default::default()
        };

        info!("Request: {:?}", request);

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_post(
            &self.configuration,
            request,
        )
        .await;

        if result.is_err() {
            return Err(IndexerError::Custom(result.err().unwrap().to_string()));
        }

        info!("Result: {:?}", result);
        let proofs: photon_api::models::MerkleContextWithNewAddressProof =
            result.unwrap().result.unwrap().value[0].clone();

        let tree_pubkey = decode_hash(&proofs.merkle_tree);
        let low_address_value = decode_hash(&proofs.lower_range_address);
        let next_address_value = decode_hash(&proofs.higher_range_address);
        Ok(NewAddressProofWithContext {
            merkle_tree: tree_pubkey,
            low_address_index: proofs.low_element_leaf_index as u64,
            low_address_value,
            low_address_next_index: proofs.next_index as u64,
            low_address_next_value: next_address_value,
            low_address_proof: {
                let mut proof_vec: Vec<[u8; 32]> = proofs
                    .proof
                    .iter()
                    .map(|x: &String| decode_hash(x))
                    .collect();
                proof_vec.truncate(proof_vec.len() - 10); // Remove canopy
                let mut proof_arr = [[0u8; 32]; 16];
                proof_arr.copy_from_slice(&proof_vec);
                proof_arr
            },
            root: decode_hash(&proofs.root),
            root_seq: proofs.root_seq,
            new_low_element: None,
            new_element: None,
            new_element_next_value: None,
        })
    }
}
