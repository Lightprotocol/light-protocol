use std::{fmt::Debug, time::Duration};

use async_trait::async_trait;
use bs58;
use light_merkle_tree_metadata::QueueType;
use photon_api::{
    apis::configuration::{ApiKey, Configuration},
    models::GetCompressedAccountsByOwnerPostRequestParams,
};
use solana_pubkey::Pubkey;
use tracing::{debug, error, warn};

use super::{
    indexer_trait::{IndexerRpcConfig, RetryConfig},
    types::{Account, TokenAccount, TokenBalance},
    BatchAddressUpdateIndexerResponse, MerkleProofWithContext,
};
use crate::indexer::{
    base58::{decode_base58_to_fixed_array, Base58Conversions},
    indexer_trait::{Context, Response, ResponseWithCursor},
    Address, AddressWithTree, Hash, Indexer, IndexerError, MerkleProof, NewAddressProofWithContext,
};

pub struct PhotonIndexer {
    configuration: Configuration,
}

impl PhotonIndexer {
    pub fn default_path() -> String {
        "http://127.0.0.1:8784".to_string()
    }
}

impl PhotonIndexer {
    async fn retry<F, Fut, T>(
        &self,
        config: RetryConfig,
        mut operation: F,
    ) -> Result<T, IndexerError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let max_retries = config.num_retries;
        let mut attempts = 0;
        let mut delay_ms = config.delay_ms;
        let max_delay_ms = config.max_delay_ms;

        loop {
            attempts += 1;

            debug!(
                "Attempt {}/{}: No rate limiter configured",
                attempts, max_retries
            );

            debug!("Attempt {}/{}: Executing operation", attempts, max_retries);
            let result = operation().await;

            match result {
                Ok(value) => {
                    debug!("Attempt {}/{}: Operation succeeded.", attempts, max_retries);
                    return Ok(value);
                }
                Err(e) => {
                    let is_retryable = match &e {
                        IndexerError::ApiError(_) => {
                            warn!("API Error: {}", e);
                            true
                        }
                        IndexerError::PhotonError {
                            context: _,
                            message: _,
                        } => {
                            warn!("Operation failed, checking if retryable...");
                            true
                        }
                        IndexerError::IndexerNotSyncedToSlot => true,
                        IndexerError::Base58DecodeError { .. } => false,
                        IndexerError::AccountNotFound => false,
                        IndexerError::InvalidParameters(_) => false,
                        IndexerError::NotImplemented(_) => false,
                        _ => false,
                    };

                    if is_retryable && attempts < max_retries {
                        warn!(
                            "Attempt {}/{}: Operation failed. Retrying",
                            attempts, max_retries
                        );

                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms = std::cmp::min(delay_ms * 2, max_delay_ms);
                    } else {
                        if is_retryable {
                            error!("Operation failed after max retries.");
                        } else {
                            error!("Operation failed with non-retryable error.");
                        }
                        return Err(e);
                    }
                }
            }
        }
    }
}

impl PhotonIndexer {
    pub fn new(path: String, api_key: Option<String>) -> Self {
        let configuration = Configuration {
            base_path: path,
            api_key: api_key.map(|key| ApiKey {
                prefix: Some("api-key".to_string()),
                key,
            }),
            ..Default::default()
        };

        PhotonIndexer { configuration }
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
                address: address.map(|x| x.to_base58()),
                hash: hash.map(|x| x.to_base58()),
            }),
        }
    }
}

impl Debug for PhotonIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotonIndexer")
            .field("configuration", &self.configuration)
            .finish()
    }
}

#[async_trait]
impl Indexer for PhotonIndexer {
    async fn get_indexer_slot(&self, config: Option<RetryConfig>) -> Result<u64, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config, || async {
            let request = photon_api::models::GetIndexerSlotPostRequest {
                ..Default::default()
            };

            let result =
                photon_api::apis::default_api::get_indexer_slot_post(&self.configuration, request)
                    .await?;

            let result = Self::extract_result("get_indexer_slot", result.result)?;
            Ok(result)
        })
        .await
    }

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<MerkleProof>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let hashes_for_async = hashes.clone();

            let request: photon_api::models::GetMultipleCompressedAccountProofsPostRequest =
                photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
                    params: hashes_for_async
                        .into_iter()
                        .map(|hash| bs58::encode(hash).into_string())
                        .collect(),
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
                    context: "get_multiple_compressed_account_proofs".to_string(),
                    message: format!("API Error (code {}): {}", error_code, error_msg),
                });
            }

            let photon_proofs = result.result.ok_or_else(|| {
                IndexerError::missing_result(
                    "get_multiple_new_address_proofs",
                    "No result returned from Photon API",
                )
            })?;
            if photon_proofs.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let proofs = photon_proofs
                .value
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
                        hash: <[u8; 32] as Base58Conversions>::from_base58(&x.hash)?,
                        leaf_index: x.leaf_index,
                        merkle_tree: Pubkey::from_str_const(x.merkle_tree.as_str()),
                        proof,
                        root_seq: x.root_seq,
                        root: [0u8; 32],
                    })
                })
                .collect::<Result<Vec<MerkleProof>, IndexerError>>()?;

            Ok(Response {
                context: Context {
                    slot: photon_proofs.context.slot,
                },
                value: proofs,
            })
        })
        .await
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<ResponseWithCursor<Vec<Account>, [u8; 32]>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let request = photon_api::models::GetCompressedAccountsByOwnerV2PostRequest {
                    params: Box::from(GetCompressedAccountsByOwnerPostRequestParams {
                        cursor: None,
                        data_slice: None,
                        filters: None,
                        limit: None,
                        owner: owner.to_string(),
                    }),
                    ..Default::default()
                };
                let result =
                    photon_api::apis::default_api::get_compressed_accounts_by_owner_v2_post(
                        &self.configuration,
                        request,
                    )
                    .await?;
                let response = result.result.ok_or(IndexerError::AccountNotFound)?;
                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let accounts: Result<Vec<_>, _> =
                    response.value.items.iter().map(Account::try_from).collect();

                let cursor = response
                    .value
                    .cursor
                    .map(|c| decode_base58_to_fixed_array(&c))
                    .transpose()?
                    .unwrap_or([0u8; 32]);

                Ok(ResponseWithCursor {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: accounts?,
                    cursor,
                })
            }
            #[cfg(not(feature = "v2"))]
            {
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
                let response = result.result.ok_or(IndexerError::AccountNotFound)?;
                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let accounts: Result<Vec<_>, _> =
                    response.value.items.iter().map(Account::try_from).collect();

                let cursor = response
                    .value
                    .cursor
                    .map(|c| decode_base58_to_fixed_array(&c))
                    .transpose()?
                    .unwrap_or([0u8; 32]);

                Ok(ResponseWithCursor {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: accounts?,
                    cursor,
                })
            }
        })
        .await
    }

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Account>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
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
            let api_response = Self::extract_result("get_compressed_account", result.result)?;
            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let account_data = api_response
                .value
                .ok_or(IndexerError::AccountNotFound)
                .map(|boxed| *boxed)?;
            let account = Account::try_from(&account_data)?;

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: account,
            })
        })
        .await
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<ResponseWithCursor<Vec<TokenAccount>, [u8; 32]>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let request = photon_api::models::GetCompressedTokenAccountsByOwnerV2PostRequest {
                    params: Box::from(
                        photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                            cursor: None,
                            limit: None,
                            mint: mint.map(|x| x.to_string()),
                            owner: owner.to_string(),
                        },
                    ),
                    ..Default::default()
                };
                let result =
                    photon_api::apis::default_api::get_compressed_token_accounts_by_owner_v2_post(
                        &self.configuration,
                        request,
                    )
                    .await?;
                let response = result.result.ok_or(IndexerError::AccountNotFound)?;
                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let token_accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(TokenAccount::try_from)
                    .collect();

                let cursor = response
                    .value
                    .cursor
                    .map(|c| decode_base58_to_fixed_array(&c))
                    .transpose()?
                    .unwrap_or([0u8; 32]);

                Ok(ResponseWithCursor {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: token_accounts?,
                    cursor,
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let request = photon_api::models::GetCompressedTokenAccountsByOwnerPostRequest {
                    params: Box::new(
                        photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                            owner: owner.to_string(),
                            mint: mint.map(|x| x.to_string()),
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
                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let token_accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(TokenAccount::try_from)
                    .collect();

                let cursor = response
                    .value
                    .cursor
                    .map(|c| decode_base58_to_fixed_array(&c))
                    .transpose()?
                    .unwrap_or([0u8; 32]);

                Ok(ResponseWithCursor {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: token_accounts?,
                    cursor,
                })
            }
        })
        .await
    }

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
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

            let api_response =
                Self::extract_result("get_compressed_account_balance", result.result)?;
            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: api_response.value,
            })
        })
        .await
    }

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let request = photon_api::models::GetCompressedTokenAccountBalancePostRequest {
                params: Box::new(photon_api::models::GetCompressedAccountPostRequestParams {
                    address: address.map(|x| x.to_base58()),
                    hash: hash.map(|x| x.to_base58()),
                }),
                ..Default::default()
            };

            let result = photon_api::apis::default_api::get_compressed_token_account_balance_post(
                &self.configuration,
                request,
            )
            .await?;

            let api_response =
                Self::extract_result("get_compressed_token_account_balance", result.result)?;
            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: api_response.value.amount,
            })
        })
        .await
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<Account>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let hashes = hashes.clone();
            let addresses = addresses.clone();
            let request = photon_api::models::GetMultipleCompressedAccountsPostRequest {
                params: Box::new(
                    photon_api::models::GetMultipleCompressedAccountsPostRequestParams {
                        addresses: addresses.map(|x| x.iter().map(|x| x.to_base58()).collect()),
                        hashes: hashes.map(|x| x.iter().map(|x| x.to_base58()).collect()),
                    },
                ),
                ..Default::default()
            };

            let result = photon_api::apis::default_api::get_multiple_compressed_accounts_post(
                &self.configuration,
                request,
            )
            .await?;

            let api_response =
                Self::extract_result("get_multiple_compressed_accounts", result.result)?;
            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let accounts = api_response
                .value
                .items
                .iter()
                .map(Account::try_from)
                .collect::<Result<Vec<Account>, IndexerError>>()?;

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: accounts,
            })
        })
        .await
    }

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<ResponseWithCursor<Vec<TokenBalance>, Option<String>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let request = photon_api::models::GetCompressedTokenBalancesByOwnerV2PostRequest {
                    params: Box::new(
                        photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                            owner: owner.to_string(),
                            mint: mint.map(|x| x.to_string()),
                            cursor: None,
                            limit: None,
                        },
                    ),
                    ..Default::default()
                };

                let result =
                    photon_api::apis::default_api::get_compressed_token_balances_by_owner_v2_post(
                        &self.configuration,
                        request,
                    )
                    .await?;

                let api_response =
                    Self::extract_result("get_compressed_token_balances_by_owner", result.result)?;
                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let token_balances: Result<Vec<_>, _> = api_response
                    .value
                    .items
                    .iter()
                    .map(TokenBalance::try_from)
                    .collect();

                Ok(ResponseWithCursor {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: token_balances?,
                    cursor: api_response.value.cursor,
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let request = photon_api::models::GetCompressedTokenBalancesByOwnerPostRequest {
                    params: Box::new(
                        photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                            owner: owner.to_string(),
                            mint: mint.map(|x| x.to_string()),
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

                let api_response =
                    Self::extract_result("get_compressed_token_balances_by_owner", result.result)?;
                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let token_balances: Result<Vec<_>, _> = api_response
                    .value
                    .token_balances
                    .iter()
                    .map(TokenBalance::try_from)
                    .collect();

                Ok(ResponseWithCursor {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: token_balances?,
                    cursor: api_response.value.cursor,
                })
            }
        })
        .await
    }

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<String>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
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

            let api_response =
                Self::extract_result("get_compression_signatures_for_account", result.result)?;
            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let signatures = api_response
                .value
                .items
                .iter()
                .map(|x| x.signature.clone())
                .collect();

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: signatures,
            })
        })
        .await
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<NewAddressProofWithContext>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
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

            match &result {
                Ok(response) => debug!("Raw API response: {:?}", response),
                Err(e) => error!("API request failed: {:?}", e),
            }

            let result = result?;

            let api_response =
                match Self::extract_result("get_multiple_new_address_proofs", result.result) {
                    Ok(proofs) => proofs,
                    Err(e) => {
                        error!("Failed to extract proofs: {:?}", e);
                        return Err(e);
                    }
                };
            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let photon_proofs = api_response.value;
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
                    .map(|x: &String| Hash::from_base58(x))
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
                    merkle_tree: tree_pubkey.into(),
                    low_address_index: photon_proof.low_element_leaf_index,
                    low_address_value,
                    low_address_next_index: photon_proof.next_index,
                    low_address_next_value: next_address_value,
                    low_address_proof: proof_arr.to_vec(),
                    root,
                    root_seq: photon_proof.root_seq,
                    new_low_element: None,
                    new_element: None,
                    new_element_next_value: None,
                };
                proofs.push(proof);
            }

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: proofs,
            })
        })
        .await
    }

    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<super::types::ValidityProofWithContext>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let request = photon_api::models::GetValidityProofV2PostRequest {
                    params: Box::new(photon_api::models::GetValidityProofPostRequestParams {
                        hashes: Some(hashes.iter().map(|x| x.to_base58()).collect()),
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

                let result = photon_api::apis::default_api::get_validity_proof_v2_post(
                    &self.configuration,
                    request,
                )
                .await?;
                let api_response = Self::extract_result("get_validity_proof_v2", result.result)?;
                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let validity_proof =
                    super::types::ValidityProofWithContext::from_api_model_v2(*api_response.value)?;

                Ok(Response {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: validity_proof,
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let request = photon_api::models::GetValidityProofPostRequest {
                    params: Box::new(photon_api::models::GetValidityProofPostRequestParams {
                        hashes: Some(hashes.iter().map(|x| x.to_base58()).collect()),
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

                let api_response = Self::extract_result("get_validity_proof", result.result)?;
                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let validity_proof = super::types::ValidityProofWithContext::from_api_model(
                    *api_response.value,
                    hashes.len(),
                )?;

                Ok(Response {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: validity_proof,
                })
            }
        })
        .await
    }

    async fn get_address_queue_with_proofs(
        &mut self,
        _merkle_tree_pubkey: &Pubkey,
        _zkp_batch_size: u16,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<BatchAddressUpdateIndexerResponse>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!("get_address_queue_with_proofs");
        #[cfg(feature = "v2")]
        {
            let merkle_tree_pubkey = _merkle_tree_pubkey;
            let zkp_batch_size = _zkp_batch_size;
            let config = _config.unwrap_or_default();
            self.retry(config.retry_config, || async {
                let merkle_tree = Hash::from_bytes(merkle_tree_pubkey.to_bytes().as_ref())?;
                let request = photon_api::models::GetBatchAddressUpdateInfoPostRequest {
                    params: Box::new(
                        photon_api::models::GetBatchAddressUpdateInfoPostRequestParams {
                            batch_size: zkp_batch_size,
                            tree: merkle_tree.to_base58(),
                        },
                    ),
                    ..Default::default()
                };

                let result = photon_api::apis::default_api::get_batch_address_update_info_post(
                    &self.configuration,
                    request,
                )
                .await?;

                let api_response =
                    Self::extract_result("get_batch_address_update_info", result.result)?;
                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let addresses = api_response
                    .addresses
                    .iter()
                    .map(|x| crate::indexer::AddressQueueIndex {
                        address: Hash::from_base58(x.address.clone().as_ref()).unwrap(),
                        queue_index: x.queue_index,
                    })
                    .collect();

                let mut proofs: Vec<NewAddressProofWithContext> = vec![];
                for proof in api_response.non_inclusion_proofs {
                    let proof = NewAddressProofWithContext {
                        merkle_tree: *merkle_tree_pubkey,
                        low_address_index: proof.low_element_leaf_index,
                        low_address_value: Hash::from_base58(
                            proof.lower_range_address.clone().as_ref(),
                        )
                        .unwrap(),
                        low_address_next_index: proof.next_index,
                        low_address_next_value: Hash::from_base58(
                            proof.higher_range_address.clone().as_ref(),
                        )
                        .unwrap(),
                        low_address_proof: proof
                            .proof
                            .iter()
                            .map(|x| Hash::from_base58(x.clone().as_ref()).unwrap())
                            .collect(),
                        root: Hash::from_base58(proof.root.clone().as_ref()).unwrap(),
                        root_seq: proof.root_seq,

                        new_low_element: None,
                        new_element: None,
                        new_element_next_value: None,
                    };
                    proofs.push(proof);
                }

                let subtrees = api_response
                    .subtrees
                    .iter()
                    .map(|x| {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(x.as_slice());
                        arr
                    })
                    .collect::<Vec<_>>();

                let result = BatchAddressUpdateIndexerResponse {
                    batch_start_index: api_response.start_index,
                    addresses,
                    non_inclusion_proofs: proofs,
                    subtrees,
                };
                Ok(Response {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: result,
                })
            })
            .await
        }
    }

    async fn get_queue_elements(
        &mut self,
        _pubkey: [u8; 32],
        _queue_type: QueueType,
        _num_elements: u16,
        _start_offset: Option<u64>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<MerkleProofWithContext>>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!("get_queue_elements");
        #[cfg(feature = "v2")]
        {
            let pubkey = _pubkey;
            let queue_type = _queue_type;
            let num_elements = _num_elements;
            let start_offset = _start_offset;
            let config = _config.unwrap_or_default();
            self.retry(config.retry_config, || async {
                let request: photon_api::models::GetQueueElementsPostRequest =
                    photon_api::models::GetQueueElementsPostRequest {
                        params: Box::from(photon_api::models::GetQueueElementsPostRequestParams {
                            tree: bs58::encode(pubkey).into_string(),
                            queue_type: queue_type as u16,
                            num_elements,
                            start_offset,
                        }),
                        ..Default::default()
                    };
                let result = photon_api::apis::default_api::get_queue_elements_post(
                    &self.configuration,
                    request,
                )
                .await;

                let result: Result<Response<Vec<MerkleProofWithContext>>, IndexerError> =
                    match result {
                        Ok(api_response) => match api_response.result {
                            Some(api_result) => {
                                if api_result.context.slot < config.slot {
                                    return Err(IndexerError::IndexerNotSyncedToSlot);
                                }
                                let response = api_result.value;
                                let proofs = response
                                    .iter()
                                    .map(|x| {
                                        let proof = x
                                            .proof
                                            .iter()
                                            .map(|x| Hash::from_base58(x).unwrap())
                                            .collect();
                                        let root = Hash::from_base58(&x.root).unwrap();
                                        let leaf = Hash::from_base58(&x.leaf).unwrap();
                                        let merkle_tree = Hash::from_base58(&x.tree).unwrap();
                                        let tx_hash = x
                                            .tx_hash
                                            .as_ref()
                                            .map(|x| Hash::from_base58(x).unwrap());
                                        let account_hash =
                                            Hash::from_base58(&x.account_hash).unwrap();

                                        MerkleProofWithContext {
                                            proof,
                                            root,
                                            leaf_index: x.leaf_index,
                                            leaf,
                                            merkle_tree,
                                            root_seq: x.root_seq,
                                            tx_hash,
                                            account_hash,
                                        }
                                    })
                                    .collect();

                                Ok(Response {
                                    context: Context {
                                        slot: api_result.context.slot,
                                    },
                                    value: proofs,
                                })
                            }
                            None => {
                                let error = api_response.error.ok_or_else(|| {
                                    IndexerError::PhotonError {
                                        context: "get_queue_elements".to_string(),
                                        message: "No error details provided".to_string(),
                                    }
                                })?;

                                Err(IndexerError::PhotonError {
                                    context: "get_queue_elements".to_string(),
                                    message: error
                                        .message
                                        .unwrap_or_else(|| "Unknown error".to_string()),
                                })
                            }
                        },
                        Err(e) => Err(IndexerError::PhotonError {
                            context: "get_queue_elements".to_string(),
                            message: e.to_string(),
                        }),
                    };

                result
            })
            .await
        }
    }

    async fn get_subtrees(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<[u8; 32]>>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!();
        #[cfg(feature = "v2")]
        {
            todo!();
        }
    }
}
