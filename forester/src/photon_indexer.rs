use crate::utils::decode_hash;
use account_compression::initialize_address_merkle_tree::Pubkey;
use forester_utils::indexer::{Indexer, IndexerError, MerkleProof, NewAddressProofWithContext};
use forester_utils::rpc::RpcConnection;
use light_system_program::sdk::compressed_account::{
    CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext, MerkleContext,
};
use photon_api::apis::configuration::{ApiKey, Configuration};
use photon_api::models::GetCompressedAccountsByOwnerPostRequestParams;
use solana_sdk::bs58;
use std::fmt::Debug;
use tracing::debug;

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

impl<R: RpcConnection> Indexer<R> for PhotonIndexer<R> {
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        debug!("Getting proofs for {:?}", hashes);
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
                    None => {
                        let error = response.error.unwrap();
                        Err(IndexerError::Custom(error.message.unwrap()))
                    }
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
        _merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError> {
        let addresses_bs58 = addresses
            .iter()
            .map(|x| bs58::encode(x).into_string())
            .collect();

        let request = photon_api::models::GetMultipleNewAddressProofsPostRequest {
            params: addresses_bs58,
            ..Default::default()
        };

        debug!("Request: {:?}", request);

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_post(
            &self.configuration,
            request,
        )
        .await;

        if result.is_err() {
            return Err(IndexerError::Custom(result.err().unwrap().to_string()));
        }

        let photon_proofs = result.unwrap().result.unwrap().value;
        let mut proofs: Vec<NewAddressProofWithContext> = Vec::new();
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

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        let params = GetCompressedAccountsByOwnerPostRequestParams {
            cursor: None,
            limit: None,
            owner: owner.to_string(),
        };
        let request = photon_api::models::GetCompressedAccountsByOwnerPostRequest {
            params: Box::from(params),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_accounts_by_owner_post(
            &self.configuration,
            request,
        )
        .await;
        // let env_accounts = EnvAccounts::get_local_test_validator_accounts();
        match result {
            Ok(response) => match response.result {
                Some(result) => {
                    let accounts = result
                        .value
                        .items
                        .iter()
                        .map(|x| {
                            let address = if let Some(address) = x.address.as_ref() {
                                Some(decode_hash(address))
                            } else {
                                None
                            };
                            let data = if let Some(data) = x.data.as_ref() {
                                Some(CompressedAccountData {
                                    data: bs58::decode(data.data.clone()).into_vec().unwrap(),
                                    discriminator: data.discriminator.to_le_bytes(),
                                    data_hash: decode_hash(&data.data_hash),
                                })
                            } else {
                                None
                            };
                            use std::str::FromStr;
                            let compressed_account = CompressedAccount {
                                address,
                                owner: Pubkey::from_str(x.owner.as_str()).unwrap(),
                                data,
                                lamports: x.lamports.try_into().unwrap(),
                            };
                            let merkle_context = MerkleContext {
                                merkle_tree_pubkey: Pubkey::new_from_array(decode_hash(&x.tree)),
                                leaf_index: x.leaf_index.try_into().unwrap(),
                                queue_index: None,
                                nullifier_queue_pubkey: Pubkey::default(), //env_accounts.nullifier_queue_pubkey, // TODO: make dynamic. Why don't we get this value from the rpc call?
                            };
                            CompressedAccountWithMerkleContext {
                                compressed_account,
                                merkle_context,
                            }
                        })
                        .collect();
                    accounts
                }
                None => panic!("get_compressed_accounts_by_owner No result found"),
            },
            Err(e) => {
                tracing::info!("Error: {:?}", e);
                panic!("get_compressed_accounts_by_owner failed")
            }
        }
    }
}
