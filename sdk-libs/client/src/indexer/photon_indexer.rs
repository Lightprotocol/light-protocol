use std::fmt::Debug;

use async_trait::async_trait;
use light_sdk::{proof::ProofRpcResult, token::TokenDataWithMerkleContext};
use photon_api::{
    apis::configuration::{ApiKey, Configuration},
    models::{
        Account, CompressedProofWithContext, GetCompressedAccountsByOwnerPostRequestParams,
        TokenBalanceList,
    },
};
use solana_program::pubkey::Pubkey;
use solana_sdk::bs58;

use crate::{
    indexer::{
        Address, AddressMerkleTreeBundle, AddressWithTree, Base58Conversions,
        FromPhotonTokenAccountList, Hash, Indexer, IndexerError, LeafIndexInfo, MerkleProof,
        NewAddressProofWithContext, ProofOfLeaf,
    },
    rate_limiter::{RateLimiter, UseRateLimiter},
    rpc::RpcConnection,
};

pub struct PhotonIndexer<R: RpcConnection> {
    configuration: Configuration,
    #[allow(dead_code)]
    rpc: R,
    rate_limiter: Option<RateLimiter>,
}

impl<R: RpcConnection> UseRateLimiter for PhotonIndexer<R> {
    fn set_rate_limiter(&mut self, rate_limiter: RateLimiter) {
        self.rate_limiter = Some(rate_limiter);
    }

    fn rate_limiter(&self) -> Option<&RateLimiter> {
        self.rate_limiter.as_ref()
    }
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

        PhotonIndexer {
            configuration,
            rpc,
            rate_limiter: None,
    }
    
    async fn rate_limited_request<F, Fut, T>(&self, operation: F) -> Result<T, IndexerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire_with_wait().await;
        }
        operation().await
    }

    fn build_account_params(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<photon_api::models::GetCompressedAccountPostRequestParams, IndexerError> {
        if address.is_none() && hash.is_none() {
            return Err(IndexerError::Custom(
                "Either address or hash must be provided".to_string(),
            ));
        }

        if address.is_some() && hash.is_some() {
            return Err(IndexerError::Custom(
                "Only one of address or hash must be provided".to_string(),
            ));
        }

        Ok(photon_api::models::GetCompressedAccountPostRequestParams {
            address: address.map(|x| Some(x.to_base58())),
            hash: hash.map(|x| Some(x.to_base58())),
        })
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
        self.rate_limited_request(|| async {
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
                                let proof: Vec<[u8; 32]> = proof_result_value
                                    .iter()
                                    .map(|x| Hash::from_base58(x).unwrap())
                                    .collect();
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
    ) -> Result<Vec<Hash>, IndexerError> {
        self.rate_limited_request(|| async {
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

        Ok(hashes
            .iter()
            .map(|x| Hash::from_base58(x).unwrap())
            .collect())
    }

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<Account, IndexerError> {
        self.rate_limited_request(|| async {
        let params = self.build_account_params(address, hash)?;
        let request = photon_api::models::GetCompressedAccountPostRequest {
            params: Box::new(params),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_account_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => {
                if let Some(acc) = result.value {
                    Ok(*acc)
                } else {
                    Err(IndexerError::Custom("Missing account".to_string()))
                }
            }
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError> {
        self.rate_limited_request(|| async {
        let request = photon_api::models::GetCompressedTokenAccountsByOwnerPostRequest {
            params: Box::new(
                photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                    owner: owner.to_string(),
                    mint: mint.map(|x| Some(x.to_string())),
                    cursor: None,
                    limit: None,
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_token_accounts_by_owner_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(result.value.into_token_data_vec()),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError> {
        self.rate_limited_request(|| async {
        let params = self.build_account_params(address, hash)?;
        let request = photon_api::models::GetCompressedAccountBalancePostRequest {
            params: Box::new(params),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_account_balance_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(result.value),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError> {
        self.rate_limited_request(|| async {
        let request = photon_api::models::GetCompressedTokenAccountBalancePostRequest {
            params: Box::new(photon_api::models::GetCompressedAccountPostRequestParams {
                address: address.map(|x| Some(x.to_base58())),
                hash: hash.map(|x| Some(x.to_base58())),
            }),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_token_account_balance_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(result.value.amount),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
    ) -> Result<Vec<Account>, IndexerError> {
        self.rate_limited_request(|| async {
        let request = photon_api::models::GetMultipleCompressedAccountsPostRequest {
            params: Box::new(
                photon_api::models::GetMultipleCompressedAccountsPostRequestParams {
                    addresses: addresses.map(|x| Some(x.iter().map(|x| x.to_base58()).collect())),
                    hashes: hashes.map(|x| Some(x.iter().map(|x| x.to_base58()).collect())),
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_compressed_accounts_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(result.value.items),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<TokenBalanceList, IndexerError> {
        self.rate_limited_request(|| async {
        let request = photon_api::models::GetCompressedTokenBalancesByOwnerPostRequest {
            params: Box::new(
                photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                    owner: owner.to_string(),
                    mint: mint.map(|x| Some(x.to_string())),
                    cursor: None,
                    limit: None,
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_token_balances_by_owner_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(*result.value),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
    ) -> Result<Vec<String>, IndexerError> {
        self.rate_limited_request(|| async {
        let request = photon_api::models::GetCompressionSignaturesForAccountPostRequest {
            params: Box::new(
                photon_api::models::GetCompressedAccountProofPostRequestParams {
                    hash: hash.to_base58(),
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compression_signatures_for_account_post(
            &self.configuration,
            request,
        )
        .await
        .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(result
                .value
                .items
                .iter()
                .map(|x| x.signature.clone())
                .collect()),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        self.rate_limited_request(|| async {
        
        let params: Vec<photon_api::models::address_with_tree::AddressWithTree> = addresses
            .iter()
            .map(|x| photon_api::models::address_with_tree::AddressWithTree {
                address: bs58::encode(x).into_string(),
                tree: bs58::encode(&merkle_tree_pubkey).into_string(),
            })
            .collect();

        let request = photon_api::models::GetMultipleNewAddressProofsV2PostRequest {
            params,
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
            &self.configuration,
            request,
        )
        .await;

        if result.is_err() {
            return Err(IndexerError::Custom(result.err().unwrap().to_string()));
        }

        let photon_proofs = result.unwrap().result.unwrap().value;
        // net height 16 =  height(26) - canopy(10)
        let mut proofs: Vec<NewAddressProofWithContext<16>> = Vec::new();
        for photon_proof in photon_proofs {
            let tree_pubkey = Hash::from_base58(&photon_proof.merkle_tree)?;
            let low_address_value = Hash::from_base58(&photon_proof.lower_range_address)?;
            let next_address_value = Hash::from_base58(&photon_proof.higher_range_address)?;
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
                        .map(|x: &String| Hash::from_base58(x))
                        .collect::<Result<Vec<[u8; 32]>, IndexerError>>()?;
                    proof_vec.truncate(proof_vec.len() - 10); // Remove canopy
                    let mut proof_arr = [[0u8; 32]; 16];
                    proof_arr.copy_from_slice(&proof_vec);
                    proof_arr
                },
                root: Hash::from_base58(&photon_proof.root)?,
                root_seq: photon_proof.root_seq,
                new_low_element: None,
                new_element: None,
                new_element_next_value: None,
            };
            proofs.push(proof);
        }

        Ok(proofs)
        }).await
    }

    async fn get_multiple_new_address_proofs_h40(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        _addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError> {
        unimplemented!()
    }

    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<CompressedProofWithContext, IndexerError> {
        self.rate_limited_request(|| async {
        let request = photon_api::models::GetValidityProofPostRequest {
            params: Box::new(photon_api::models::GetValidityProofPostRequestParams {
                hashes: Some(hashes.iter().map(|x| x.to_base58()).collect()),
                new_addresses: None,
                new_addresses_with_trees: Some(
                    new_addresses_with_trees
                        .iter()
                        .map(|x| photon_api::models::AddressWithTree {
                            address: x.address.to_base58(),
                            tree: x.tree.to_string(),
                        })
                        .collect(),
                ),
            }),
            ..Default::default()
        };

        let result =
            photon_api::apis::default_api::get_validity_proof_post(&self.configuration, request)
                .await
                .map_err(|e| IndexerError::Custom(e.to_string()))?;

        match result.result {
            Some(result) => Ok(*result.value),
            None => Err(IndexerError::Custom("Missing result".to_string())),
        }
        }).await
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
    ) -> Vec<LeafIndexInfo> {
        todo!()
    }

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        todo!()
    }
}
