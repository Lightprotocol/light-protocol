use crate::utils::decode_hash;
use account_compression::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use forester_utils::indexer::{
    AddressMerkleTreeAccounts, AddressMerkleTreeBundle, Indexer, IndexerError, MerkleProof,
    NewAddressProofWithContext, ProofRpcResult, StateMerkleTreeAccounts, StateMerkleTreeBundle,
    TokenDataWithContext,
};
use light_client::rpc::RpcConnection;
use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_system_program::sdk::event::PublicTransactionEvent;
use photon_api::apis::configuration::{ApiKey, Configuration};
use photon_api::models::{AddressWithTree, GetCompressedAccountsByOwnerPostRequestParams};
use solana_sdk::bs58;
use solana_sdk::signature::Keypair;
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

#[async_trait]
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

        debug!("Response: {:?}", result);

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
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError> {
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

    async fn account_nullified(&self, _merkle_tree_pubkey: Pubkey, _account_hash: &str) {
        unimplemented!()
    }

    async fn address_tree_updated(
        &self,
        _merkle_tree_pubkey: Pubkey,
        _context: &NewAddressProofWithContext,
    ) {
        unimplemented!()
    }

    async fn get_state_merkle_tree_accounts(
        &self,
        _pubkeys: &[Pubkey],
    ) -> Vec<StateMerkleTreeAccounts> {
        unimplemented!()
    }

    async fn add_event_and_compressed_accounts(
        &self,
        _event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithContext>,
    ) {
        unimplemented!()
    }

    async fn get_state_merkle_trees(&self) -> Vec<StateMerkleTreeBundle> {
        unimplemented!()
    }

    async fn get_address_merkle_trees(&self) -> Vec<AddressMerkleTreeBundle> {
        unimplemented!()
    }

    async fn get_token_compressed_accounts(&self) -> Vec<TokenDataWithContext> {
        unimplemented!()
    }

    fn get_payer(&self) -> &Keypair {
        unimplemented!()
    }

    fn get_group_pda(&self) -> &Pubkey {
        unimplemented!()
    }

    async fn create_proof_for_compressed_accounts(
        &self,
        _compressed_accounts: Option<&[[u8; 32]]>,
        _state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &R,
    ) -> ProofRpcResult {
        unimplemented!()
    }

    async fn add_address_merkle_tree_accounts(
        &self,
        _merkle_tree_keypair: &Keypair,
        _queue_keypair: &Keypair,
        _owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        unimplemented!()
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        _owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        unimplemented!()
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        _owner: &Pubkey,
    ) -> Vec<TokenDataWithContext> {
        unimplemented!()
    }

    async fn add_state_bundle(&self, _state_bundle: StateMerkleTreeBundle) {
        unimplemented!()
    }

    async fn add_address_bundle(&self, _address_bundle: AddressMerkleTreeBundle) {
        unimplemented!()
    }

    async fn clear_state_trees(&self) {
        unimplemented!()
    }
}
