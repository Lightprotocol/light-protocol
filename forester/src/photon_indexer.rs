use std::fmt::Debug;

use account_compression::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use light_client::{
    indexer::{
        AddressMerkleTreeBundle, Indexer, IndexerError, MerkleProof, NewAddressProofWithContext,
        ProofOfLeaf,
    },
    rpc::RpcConnection,
};
use light_sdk::proof::ProofRpcResult;
use photon_api::{
    apis::configuration::{ApiKey, Configuration},
    models::{AddressWithTree, GetCompressedAccountsByOwnerPostRequestParams},
};
use solana_sdk::bs58;
use tracing::debug;

use crate::utils::decode_hash;

pub struct PhotonIndexer<R: RpcConnection> {
    configuration: Configuration,
    #[allow(dead_code)]
    rpc: R,
}

impl<R: RpcConnection> PhotonIndexer<R> {
    pub fn new(path: String, api_key: Option<String>, rpc: R) -> Self {
        let configuration = Configuration {
            base_path: path,
            api_key: api_key.map(|key| ApiKey {
                prefix: Some("api-key".to_string()),
                key,
            }),
            ..Default::default()
        };

        PhotonIndexer { configuration, rpc }
    }
}

impl<R: RpcConnection> Debug for PhotonIndexer<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotonIndexer")
            .field("configuration", &self.configuration)
            .finish()
    }
}

#[async_trait]
impl<R: RpcConnection> Indexer<R> for PhotonIndexer<R> {
    async fn get_queue_elements(
        &self,
        _pubkey: [u8; 32],
        _batch: u64,
        _start_offset: u64,
        _end_offset: u64,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        unimplemented!()
    }

    fn get_subtrees(&self, _merkle_tree_pubkey: [u8; 32]) -> Result<Vec<[u8; 32]>, IndexerError> {
        unimplemented!()
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        _compressed_accounts: Option<Vec<[u8; 32]>>,
        _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> ProofRpcResult {
        todo!()
    }
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        debug!("Getting proofs for {:?}", hashes);
        let request: photon_api::models::GetMultipleCompressedAccountProofsPostRequest =
            photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
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
                    None => {
                        let error = response.error.unwrap();
                        Err(IndexerError::Custom(error.message.unwrap()))
                    }
                }
            }
            Err(e) => Err(IndexerError::Custom(e.to_string())),
        }
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError> {
        let request = photon_api::models::GetCompressedAccountsByOwnerPostRequest {
            params: Box::from(GetCompressedAccountsByOwnerPostRequestParams {
                cursor: None,
                data_slice: None,
                filters: None,
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
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        let params: Vec<AddressWithTree> = addresses
            .iter()
            .map(|x| AddressWithTree {
                address: bs58::encode(x).into_string(),
                tree: bs58::encode(&merkle_tree_pubkey).into_string(),
            })
            .collect();

        let request = photon_api::models::GetMultipleNewAddressProofsV2PostRequest {
            params,
            ..Default::default()
        };

        debug!("Request: {:?}", request);

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
            &self.configuration,
            request,
        )
        .await;

        debug!("Response: {:?}", result);

        if result.is_err() {
            return Err(IndexerError::Custom(result.err().unwrap().to_string()));
        }

        let photon_proofs = result.unwrap().result.unwrap().value;
        // net height 16 =  height(26) - canopy(10)
        let mut proofs: Vec<NewAddressProofWithContext<16>> = Vec::new();
        for photon_proof in photon_proofs {
            let tree_pubkey = decode_hash(&photon_proof.merkle_tree);
            let low_address_value = decode_hash(&photon_proof.lower_range_address);
            let next_address_value = decode_hash(&photon_proof.higher_range_address);
            let proof = NewAddressProofWithContext {
                merkle_tree: tree_pubkey,
                low_address_index: photon_proof.low_element_leaf_index as u64,
                low_address_value,
                low_address_next_index: photon_proof.next_index as u64,
                low_address_next_value: next_address_value,
                low_address_proof: {
                    let mut proof_vec: Vec<[u8; 32]> = photon_proof
                        .proof
                        .iter()
                        .map(|x: &String| decode_hash(x))
                        .collect();
                    proof_vec.truncate(proof_vec.len() - 10); // Remove canopy
                    let mut proof_arr = [[0u8; 32]; 16];
                    proof_arr.copy_from_slice(&proof_vec);
                    proof_arr
                },
                root: decode_hash(&photon_proof.root),
                root_seq: photon_proof.root_seq,
                new_low_element: None,
                new_element: None,
                new_element_next_value: None,
            };
            proofs.push(proof);
        }

        Ok(proofs)
    }

    async fn get_multiple_new_address_proofs_full(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        _addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError> {
        unimplemented!()
    }

    fn get_proofs_by_indices(
        &mut self,
        _merkle_tree_pubkey: Pubkey,
        _indices: &[u64],
    ) -> Vec<ProofOfLeaf> {
        todo!()
    }

    fn get_leaf_indices_tx_hashes(
        &mut self,
        _merkle_tree_pubkey: Pubkey,
        _zkp_batch_size: usize,
    ) -> Vec<(u32, [u8; 32], [u8; 32])> {
        todo!()
    }

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        todo!()
    }
}
