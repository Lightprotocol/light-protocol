use std::fmt::Debug;

use async_trait::async_trait;
use light_compressed_account::compressed_account::{
    CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext, MerkleContext,
};
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
use tracing::{debug, error};

use super::MerkleProofWithContext;
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
    }

    pub fn get_rpc(&self) -> &R {
        &self.rpc
    }

    pub fn get_rpc_mut(&mut self) -> &mut R {
        &mut self.rpc
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

    fn extract_result<T>(context: &str, result: Option<T>) -> Result<T, IndexerError> {
        result.ok_or_else(|| IndexerError::missing_result(context, "value not present"))
    }

    fn build_account_params(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<photon_api::models::GetCompressedAccountPostRequestParams, IndexerError> {
        match (address, hash) {
            (None, None) => Err(IndexerError::InvalidParameters(
                "Either address or hash must be provided".to_string(),
            )),
            (Some(_), Some(_)) => Err(IndexerError::InvalidParameters(
                "Only one of address or hash must be provided".to_string(),
            )),
            (address, hash) => Ok(photon_api::models::GetCompressedAccountPostRequestParams {
                address: address.map(|x| Some(x.to_base58())),
                hash: hash.map(|x| Some(x.to_base58())),
            }),
        }
    }

    fn convert_account_data(acc: &Account) -> Result<Option<CompressedAccountData>, IndexerError> {
        match &acc.data {
            Some(data) => {
                let data_hash = Hash::from_base58(&data.data_hash)
                    .map_err(|e| IndexerError::base58_decode_error("data_hash", e))?;

                Ok(Some(CompressedAccountData {
                    discriminator: data.discriminator.to_be_bytes(),
                    data: base64::decode(&data.data)
                        .map_err(|e| IndexerError::base58_decode_error("data", e))?,
                    data_hash,
                }))
            }
            None => Ok(None),
        }
    }

    fn convert_to_compressed_account(
        acc: &Account,
    ) -> Result<CompressedAccountWithMerkleContext, IndexerError> {
        let owner = Pubkey::from(
            Hash::from_base58(&acc.owner)
                .map_err(|e| IndexerError::base58_decode_error("owner", e))?,
        );

        let address = match &acc.address {
            Some(addr) => Some(Hash::from_base58(addr)?),
            None => None,
        };

        let merkle_tree_pubkey = Pubkey::from(
            Hash::from_base58(&acc.tree)
                .map_err(|e| IndexerError::base58_decode_error("tree", e))?,
        );

        let compressed_account = CompressedAccount {
            owner,
            lamports: acc.lamports,
            address,
            data: Self::convert_account_data(acc)?,
        };

        let merkle_context = MerkleContext {
            merkle_tree_pubkey,
            // TODO: add nullifier queue pubkey to photon
            nullifier_queue_pubkey: merkle_tree_pubkey,
            leaf_index: acc.leaf_index,
            prove_by_index: false,
        };

        Ok(CompressedAccountWithMerkleContext {
            compressed_account,
            merkle_context,
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
        &mut self,
        _pubkey: [u8; 32],
        _num_elements: u64,
        _start_offset: Option<u64>,
    ) -> Result<Vec<MerkleProofWithContext>, IndexerError> {
        Err(IndexerError::NotImplemented(
            "get_queue_elements".to_string(),
        ))
    }

    fn get_subtrees(&self, _merkle_tree_pubkey: [u8; 32]) -> Result<Vec<[u8; 32]>, IndexerError> {
        Err(IndexerError::NotImplemented("get_subtrees".to_string()))
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        _compressed_accounts: Option<Vec<[u8; 32]>>,
        _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> Result<ProofRpcResult, IndexerError> {
        Err(IndexerError::NotImplemented(
            "create_proof_for_compressed_accounts".to_string(),
        ))
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

            debug!("API request: {:?}", request);

            let result =
                photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
                    &self.configuration,
                    request,
                )
                .await?;
            debug!("Raw API response: {:?}", result);

            if let Some(error) = &result.error {
                let error_msg = error.message.as_deref().unwrap_or("Unknown error");
                let error_code = error.code.unwrap_or(0);
                tracing::error!("API returned error: {}", error_msg);
                return Err(IndexerError::PhotonError {
                    context: "get_multiple_new_address_proofs".to_string(),
                    message: format!("API Error (code {}): {}", error_code, error_msg),
                });
            }

            let photon_proofs = result
                .result
                .ok_or_else(|| {
                    IndexerError::missing_result(
                        "get_multiple_new_address_proofs",
                        "No result returned from Photon API",
                    )
                })?
                .value;

            photon_proofs
                .iter()
                .map(|x| {
                    let mut proof_vec = x.proof.clone();
                    proof_vec.truncate(proof_vec.len() - 10); // Remove canopy

                    let proof = proof_vec
                        .iter()
                        .map(|x| Hash::from_base58(x))
                        .collect::<Result<Vec<[u8; 32]>, IndexerError>>()
                        .map_err(|e| IndexerError::Base58DecodeError {
                            field: "proof".to_string(),
                            message: e.to_string(),
                        })?;

                    Ok(MerkleProof {
                        hash: x.hash.clone(),
                        leaf_index: x.leaf_index,
                        merkle_tree: x.merkle_tree.clone(),
                        proof,
                        root_seq: x.root_seq,
                    })
                })
                .collect()
        })
        .await
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<CompressedAccountWithMerkleContext>, IndexerError> {
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
            .await?;

            let accounts =
                Self::extract_result("get_compressed_accounts_by_owner", result.result)?.value;
            accounts
                .items
                .iter()
                .map(|acc| Self::convert_to_compressed_account(acc))
                .collect()
        })
        .await
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
            .await?;
            let response = Self::extract_result("get_compressed_account", result.result)?;
            response
                .value
                .ok_or(IndexerError::AccountNotFound)
                .map(|boxed| *boxed)
        })
        .await
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

            let result =
                photon_api::apis::default_api::get_compressed_token_accounts_by_owner_post(
                    &self.configuration,
                    request,
                )
                .await?;

            let response =
                Self::extract_result("get_compressed_token_accounts_by_owner", result.result)?;
            Ok(response.value.into_token_data_vec())
        })
        .await
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
            .await?;

            let response = Self::extract_result("get_compressed_account_balance", result.result)?;
            Ok(response.value)
        })
        .await
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
            .await?;

            let response =
                Self::extract_result("get_compressed_token_account_balance", result.result)?;
            Ok(response.value.amount)
        })
        .await
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
                        addresses: addresses
                            .map(|x| Some(x.iter().map(|x| x.to_base58()).collect())),
                        hashes: hashes.map(|x| Some(x.iter().map(|x| x.to_base58()).collect())),
                    },
                ),
                ..Default::default()
            };

            let result = photon_api::apis::default_api::get_multiple_compressed_accounts_post(
                &self.configuration,
                request,
            )
            .await?;

            let response = Self::extract_result("get_multiple_compressed_accounts", result.result)?;
            Ok(response.value.items)
        })
        .await
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

            let result =
                photon_api::apis::default_api::get_compressed_token_balances_by_owner_post(
                    &self.configuration,
                    request,
                )
                .await?;

            let response =
                Self::extract_result("get_compressed_token_balances_by_owner", result.result)?;
            Ok(*response.value)
        })
        .await
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

            let result =
                photon_api::apis::default_api::get_compression_signatures_for_account_post(
                    &self.configuration,
                    request,
                )
                .await?;

            let response =
                Self::extract_result("get_compression_signatures_for_account", result.result)?;
            Ok(response
                .value
                .items
                .iter()
                .map(|x| x.signature.clone())
                .collect())
        })
        .await
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        debug!("get_multiple_new_address_proofs called with merkle_tree_pubkey: {}, addresses count: {}", bs58::encode(&merkle_tree_pubkey).into_string(), addresses.len());
        self.rate_limited_request(|| async {
            let params: Vec<photon_api::models::address_with_tree::AddressWithTree> = addresses
                .iter()
                .map(|x| photon_api::models::address_with_tree::AddressWithTree {
                    address: bs58::encode(x).into_string(),
                    tree: bs58::encode(&merkle_tree_pubkey).into_string(),
                })
                .collect();

            debug!("Request params: {:?}", params);

            let request = photon_api::models::GetMultipleNewAddressProofsV2PostRequest {
                params,
                ..Default::default()
            };

            let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
                &self.configuration,
                request,
            )
            .await;

            match &result {
                Ok(response) => debug!("Raw API response: {:?}", response),
                Err(e) => error!("API request failed: {:?}", e),
            }

            let result = result?;

            let photon_proofs =
                match Self::extract_result("get_multiple_new_address_proofs", result.result) {
                    Ok(proofs) => {
                        tracing::debug!("Successfully extracted proofs: {:?}", proofs);
                        proofs
                    }
                    Err(e) => {
                        tracing::error!("Failed to extract proofs: {:?}", e);
                        return Err(e);
                    }
                }
                .value;
            let mut proofs = Vec::new();
            for photon_proof in photon_proofs {
                let tree_pubkey = Hash::from_base58(&photon_proof.merkle_tree).map_err(|e| {
                    IndexerError::Base58DecodeError {
                        field: "merkle_tree".to_string(),
                        message: e.to_string(),
                    }
                })?;

                let low_address_value = Hash::from_base58(&photon_proof.lower_range_address)
                    .map_err(|e| IndexerError::Base58DecodeError {
                        field: "lower_range_address".to_string(),
                        message: e.to_string(),
                    })?;

                let next_address_value = Hash::from_base58(&photon_proof.higher_range_address)
                    .map_err(|e| IndexerError::Base58DecodeError {
                        field: "higher_range_address".to_string(),
                        message: e.to_string(),
                    })?;

                let mut proof_vec: Vec<[u8; 32]> = photon_proof
                    .proof
                    .iter()
                    .map(|x| Hash::from_base58(x))
                    .collect::<Result<Vec<[u8; 32]>, IndexerError>>()?;

                proof_vec.truncate(proof_vec.len() - 10); // Remove canopy
                let mut proof_arr = [[0u8; 32]; 16];
                proof_arr.copy_from_slice(&proof_vec);

                let root = Hash::from_base58(&photon_proof.root).map_err(|e| {
                    IndexerError::Base58DecodeError {
                        field: "root".to_string(),
                        message: e.to_string(),
                    }
                })?;

                let proof = NewAddressProofWithContext {
                    merkle_tree: tree_pubkey,
                    low_address_index: photon_proof.low_element_leaf_index as u64,
                    low_address_value,
                    low_address_next_index: photon_proof.next_index as u64,
                    low_address_next_value: next_address_value,
                    low_address_proof: proof_arr,
                    root,
                    root_seq: photon_proof.root_seq,
                    new_low_element: None,
                    new_element: None,
                    new_element_next_value: None,
                };
                proofs.push(proof);
            }

            Ok(proofs)
        })
        .await
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

            let result = photon_api::apis::default_api::get_validity_proof_post(
                &self.configuration,
                request,
            )
            .await?;

            let result = Self::extract_result("get_validity_proof", result.result)?;
            Ok(*result.value)
        })
        .await
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
