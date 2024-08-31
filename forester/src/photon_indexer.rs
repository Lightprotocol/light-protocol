use crate::utils::decode_hash;
use account_compression::initialize_address_merkle_tree::Pubkey;
use account_compression::utils::constants::{
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexedArray;
use light_indexed_merkle_tree::reference;
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::gnark::combined_json_formatter::CombinedJsonStruct;
use light_prover_client::gnark::constants::{PROVE_PATH, SERVER_ADDRESS};
use light_prover_client::gnark::inclusion_json_formatter::BatchInclusionJsonStruct;
use light_prover_client::gnark::non_inclusion_json_formatter::BatchNonInclusionJsonStruct;
use light_prover_client::gnark::proof_helpers::{
    compress_proof, deserialize_gnark_proof_json, proof_from_json_struct,
};
use light_prover_client::inclusion::merkle_inclusion_proof_inputs::{
    InclusionMerkleProofInputs, InclusionProofInputs,
};
use light_prover_client::non_inclusion::merkle_non_inclusion_proof_inputs::{
    NonInclusionMerkleProofInputs, NonInclusionProofInputs,
};
use light_system_program::invoke::processor::CompressedProof;
use light_system_program::sdk::compressed_account::{
    CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext, MerkleContext,
};
use light_test_utils::indexer::test_indexer::ProofRpcResult;
use light_test_utils::indexer::{
    AddressMerkleTreeAccounts, AddressMerkleTreeBundle, Indexer, IndexerError, MerkleProof,
    NewAddressProofWithContext, StateMerkleTreeAccounts, StateMerkleTreeBundle,
};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::test_env::EnvAccounts;
use light_test_utils::transaction_params::FeeConfig;
use log::{debug, info, warn};
use num_bigint::BigInt;
use photon_api::apis::configuration::{ApiKey, Configuration};
use photon_api::models::GetCompressedAccountsByOwnerPostRequestParams;
use reqwest::Client;
use solana_sdk::bs58;
use std::fmt::Debug;
use std::str::FromStr;

pub struct PhotonIndexer<R: RpcConnection> {
    configuration: Configuration,
    #[allow(dead_code)]
    rpc: R,
}

impl<R: RpcConnection> PhotonIndexer<R> {
    pub fn new(base_url: String, api_key: Option<String>, rpc: R) -> Self {
        let configuration = Configuration {
            base_path: base_url,
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
use num_traits::ops::bytes::FromBytes;
async fn process_inclusion_proofs<R: RpcConnection>(
    indexer: &PhotonIndexer<R>,
    accounts: &[[u8; 32]],
) -> (BatchInclusionJsonStruct, Vec<u16>) {
    let mut inclusion_proofs = Vec::new();
    let mut root_indices = Vec::new();
    let proofs = indexer
        .get_multiple_compressed_account_proofs(
            accounts
                .iter()
                .map(|x| bs58::encode(x).into_string())
                .collect(),
        )
        .await
        .unwrap();

    for (i, proof) in proofs.iter().enumerate() {
        inclusion_proofs.push(InclusionMerkleProofInputs {
            root: BigInt::from_be_bytes(proof.root.as_slice()),
            leaf: BigInt::from_be_bytes(accounts[i].as_slice()),
            path_index: BigInt::from_be_bytes(proof.leaf_index.to_be_bytes().as_slice()),
            path_elements: proof
                .proof
                .iter()
                .map(|x| BigInt::from_be_bytes(x))
                .collect(),
        });
        root_indices.push((proof.root_seq % 2400) as u16);
    }

    let inclusion_proof_inputs = InclusionProofInputs(inclusion_proofs.as_slice());
    let batch_inclusion_proof_inputs =
        BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs);

    (batch_inclusion_proof_inputs, root_indices)
}

async fn process_non_inclusion_proofs<R: RpcConnection>(
    indexer: &PhotonIndexer<R>,
    address_merkle_tree_pubkeys: &[Pubkey],
    addresses: &[[u8; 32]],
) -> (BatchNonInclusionJsonStruct, Vec<u16>) {
    let mut non_inclusion_proofs = Vec::new();
    let mut address_root_indices = Vec::new();

    let proofs = indexer
        .get_multiple_new_address_proofs(
            address_merkle_tree_pubkeys[0].to_bytes(),
            addresses.to_vec(),
        )
        .await
        .unwrap();
    info!("Proofs: {:?}", proofs);
    // TODO: figure out why new_low_element: None, new_element: None,
    // new_element_next_value: None
    for (i, proof) in proofs.iter().enumerate() {
        non_inclusion_proofs.push(NonInclusionMerkleProofInputs {
            root: BigInt::from_be_bytes(proof.root.as_slice()),
            value: BigInt::from_be_bytes(addresses[i].as_slice()),
            leaf_higher_range_value: proof
                .new_element_next_value
                .as_ref()
                .unwrap()
                .clone()
                .into(),
            next_index: BigInt::from_be_bytes(
                proof.low_address_next_index.to_be_bytes().as_slice(),
            ),
            leaf_lower_range_value: BigInt::from_be_bytes(proof.low_address_value.as_slice()),
            merkle_proof_hashed_indexed_element_leaf: proof
                .low_address_proof
                .iter()
                .map(|x| BigInt::from_be_bytes(x))
                .collect(),
            index_hashed_indexed_element_leaf: BigInt::from_be_bytes(
                proof.low_address_index.to_be_bytes().as_slice(),
            ),
        });
        address_root_indices.push((proof.root_seq % 2400) as u16);
    }

    let non_inclusion_proof_inputs = NonInclusionProofInputs(non_inclusion_proofs.as_slice());
    let batch_non_inclusion_proof_inputs =
        BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(&non_inclusion_proof_inputs);
    (batch_non_inclusion_proof_inputs, address_root_indices)
}

impl<R: RpcConnection> Indexer<R> for PhotonIndexer<R> {
    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts> {
        let env_accounts = EnvAccounts::get_local_test_validator_accounts();
        let mut accounts = Vec::new();

        let state_merkle_tree_accounts = vec![StateMerkleTreeAccounts {
            merkle_tree: env_accounts.merkle_tree_pubkey,
            nullifier_queue: env_accounts.nullifier_queue_pubkey,
            cpi_context: env_accounts.cpi_context_account_pubkey,
        }];
        for pubkey in pubkeys {
            if let Some(account) = state_merkle_tree_accounts
                .iter()
                .find(|x| &x.merkle_tree == pubkey)
            {
                accounts.push(account.clone());
            }
        }
        accounts
    }

    // TODO: remove this (currently required to make E2eTestEnv usable with photon)
    fn get_state_merkle_trees(&self) -> Vec<StateMerkleTreeBundle> {
        let env_accounts = EnvAccounts::get_local_test_validator_accounts();
        let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        ));
        let accounts = StateMerkleTreeAccounts {
            merkle_tree: env_accounts.merkle_tree_pubkey,
            nullifier_queue: env_accounts.nullifier_queue_pubkey,
            cpi_context: env_accounts.cpi_context_account_pubkey,
        };
        let state_merkle_tree_accounts = vec![StateMerkleTreeBundle {
            merkle_tree,
            accounts,
            rollover_fee: FeeConfig::default().state_merkle_tree_rollover as i64,
        }];
        state_merkle_tree_accounts
    }

    // TODO: remove this (currently required to make E2eTestEnv usable with photon)
    fn get_address_merkle_trees(&self) -> Vec<light_test_utils::indexer::AddressMerkleTreeBundle> {
        let mut merkle_tree = Box::new(
            reference::IndexedMerkleTree::<Poseidon, usize>::new(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            )
            .unwrap(),
        );

        merkle_tree.init().unwrap();
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        indexed_array.init().unwrap();
        let address_merkle_tree_accounts = AddressMerkleTreeAccounts {
            merkle_tree: EnvAccounts::get_local_test_validator_accounts()
                .address_merkle_tree_pubkey,
            queue: EnvAccounts::get_local_test_validator_accounts()
                .address_merkle_tree_queue_pubkey,
        };
        vec![AddressMerkleTreeBundle {
            merkle_tree,
            indexed_array,
            accounts: address_merkle_tree_accounts,
            rollover_fee: FeeConfig::default().address_queue_rollover as i64,
        }]
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        _state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> ProofRpcResult {
        if compressed_accounts.is_some()
            && ![1usize, 2usize, 3usize, 4usize, 8usize]
                .contains(&compressed_accounts.unwrap().len())
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
                    let (payload, indices) = process_inclusion_proofs(&self, accounts).await;
                    (indices, Vec::new(), payload.to_string())
                }
                (None, Some(addresses)) => {
                    let (payload, indices) = process_non_inclusion_proofs(
                        &self,
                        address_merkle_tree_pubkeys.unwrap().as_slice(),
                        addresses,
                    )
                    .await;
                    (Vec::<u16>::new(), indices, payload.to_string())
                }
                (Some(accounts), Some(addresses)) => {
                    let (inclusion_payload, inclusion_indices) =
                        process_inclusion_proofs(&self, accounts).await;

                    let (non_inclusion_payload, non_inclusion_indices) =
                        process_non_inclusion_proofs(
                            &self,
                            address_merkle_tree_pubkeys.unwrap().as_slice(),
                            addresses,
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
                // tokio::time::sleep(Duration::from_secs(1)).await;
                // spawn_prover(true, self.proof_types.as_slice()).await;
                retries -= 1;
                info!("remaining retries: {}", retries);
            }
        }
        panic!("Failed to get proof from server");
    }

    // async fn create_proof_for_compressed_accounts(
    //     &mut self,
    //     compressed_accounts: Option<&[[u8; 32]]>,
    //     _state_merkle_tree_pubkeys: Option<&[Pubkey]>,
    //     new_addresses: Option<&[[u8; 32]]>,
    //     address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
    //     _rpc: &mut R,
    // ) -> ProofRpcResult {
    //     if let Some(compressed_accounts) = compressed_accounts {
    //         let mut hashes = Vec::new();
    //         for account in compressed_accounts {
    //             hashes.push(bs58::encode(account).into_string());
    //         }
    //         let proofs = self
    //             .get_multiple_compressed_account_proofs(hashes)
    //             .await
    //             .unwrap();
    //         return ProofRpcResult::CompressedAccountProofs(proofs);
    //     }
    //     if let Some(new_addresses) = new_addresses {
    //         let mut addresses = Vec::new();
    //         for address in new_addresses {
    //             addresses.push(*address);
    //         }
    //         let merkle_tree_pubkey = address_merkle_tree_pubkeys.unwrap()[0];
    //         // TODO: why does get_multiple_new_address_proofs take a pubkey as input and get_multiple_compressed_account_proofs doesn't?
    //         let proofs = self
    //             .get_multiple_new_address_proofs(merkle_tree_pubkey.to_bytes(), addresses)
    //             .await
    //             .unwrap();
    //         return ProofRpcResult;
    //     }
    //     ProofRpcResult {}
    // }

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
        let env_accounts = EnvAccounts::get_local_test_validator_accounts();
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
                                nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey, // TODO: make dynamic. Why don't we get this value from the rpc call?
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
                info!("Error: {:?}", e);
                panic!("get_compressed_accounts_by_owner failed")
            }
        }
    }

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
                                let proof_result_value = x.proof.clone();
                                // proof_result_value.truncate(proof_result_value.len() - 10); // Remove canopy
                                let proof: Vec<[u8; 32]> =
                                    proof_result_value.iter().map(|x| decode_hash(x)).collect();
                                MerkleProof {
                                    hash: x.hash.clone(),
                                    leaf_index: x.leaf_index,
                                    merkle_tree: x.merkle_tree.clone(),
                                    proof,
                                    root: decode_hash(&x.root),
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

        info!("Request: {:?}", request);

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
                    let proof_vec: Vec<[u8; 32]> = photon_proof
                        .proof
                        .iter()
                        .map(|x: &String| decode_hash(x))
                        .collect();
                    proof_vec
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
}
