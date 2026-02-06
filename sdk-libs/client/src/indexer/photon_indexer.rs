use std::{fmt::Debug, time::Duration};

use async_trait::async_trait;
use bs58;
use light_sdk_types::constants::STATE_MERKLE_TREE_CANOPY_DEPTH;
use photon_api::apis::configuration::Configuration;
use solana_pubkey::Pubkey;
use tracing::{error, trace, warn};

use super::types::{
    AccountInterface, CompressedAccount, CompressedTokenAccount, OwnerBalance,
    SignatureWithMetadata, TokenAccountInterface, TokenBalance,
};
use crate::indexer::{
    base58::Base58Conversions,
    config::RetryConfig,
    response::{Context, Items, ItemsWithCursor, Response},
    Address, AddressWithTree, GetCompressedAccountsByOwnerConfig,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions, Hash, Indexer, IndexerError,
    IndexerRpcConfig, MerkleProof, NewAddressProofWithContext, PaginatedOptions,
};

// Tests are in program-tests/client-test/tests/light-client.rs
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

            trace!(
                "Attempt {}/{}: No rate limiter configured",
                attempts,
                max_retries
            );

            trace!("Attempt {}/{}: Executing operation", attempts, max_retries);
            let result = operation().await;

            match result {
                Ok(value) => {
                    trace!("Attempt {}/{}: Operation succeeded.", attempts, max_retries);
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
        let configuration = Configuration::new_with_api_key(path, api_key);
        PhotonIndexer { configuration }
    }

    pub fn new_with_config(configuration: Configuration) -> Self {
        PhotonIndexer { configuration }
    }

    fn extract_result<T>(context: &str, result: Option<T>) -> Result<T, IndexerError> {
        result.ok_or_else(|| IndexerError::missing_result(context, "value not present"))
    }

    fn check_api_error<E: std::fmt::Debug>(
        context: &str,
        error: Option<E>,
    ) -> Result<(), IndexerError> {
        if let Some(error) = error {
            return Err(IndexerError::ApiError(format!(
                "API error in {}: {:?}",
                context, error
            )));
        }
        Ok(())
    }

    fn extract_result_with_error_check<T, E: std::fmt::Debug>(
        context: &str,
        error: Option<E>,
        result: Option<T>,
    ) -> Result<T, IndexerError> {
        Self::check_api_error(context, error)?;
        Self::extract_result(context, result)
    }

    fn build_account_params(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<photon_api::models::PostGetCompressedAccountBodyParams, IndexerError> {
        match (address, hash) {
            (None, None) => Err(IndexerError::InvalidParameters(
                "Either address or hash must be provided".to_string(),
            )),
            (Some(_), Some(_)) => Err(IndexerError::InvalidParameters(
                "Only one of address or hash must be provided".to_string(),
            )),
            (address, hash) => Ok(photon_api::models::PostGetCompressedAccountBodyParams {
                address: address.map(|x| photon_api::models::SerializablePubkey(x.to_base58())),
                hash: hash.map(|x| photon_api::models::Hash(x.to_base58())),
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
    async fn get_compressed_account(
        &self,
        address: Address,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<CompressedAccount>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = self.build_account_params(Some(address), None)?;
            let request = photon_api::apis::default_api::make_get_compressed_account_body(params);

            let result = photon_api::apis::default_api::get_compressed_account_post(
                &self.configuration,
                request,
            )
            .await?;

            let api_response = result.result.ok_or_else(|| {
                IndexerError::ApiError(
                    result
                        .error
                        .map(|e| format!("{:?}", e))
                        .unwrap_or_else(|| "Unknown error".to_string()),
                )
            })?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let account = match api_response.value {
                Some(ref acc) => Some(CompressedAccount::try_from(acc)?),
                None => None,
            };

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: account,
            })
        })
        .await
    }

    async fn get_compressed_account_by_hash(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<CompressedAccount>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = self.build_account_params(None, Some(hash))?;
            let request = photon_api::apis::default_api::make_get_compressed_account_body(params);

            let result = photon_api::apis::default_api::get_compressed_account_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_compressed_account_by_hash", result.error)?;
            let api_response =
                Self::extract_result("get_compressed_account_by_hash", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let account = match api_response.value {
                Some(ref acc) => Some(CompressedAccount::try_from(acc)?),
                None => None,
            };

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: account,
            })
        })
        .await
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedAccountsByOwnerConfig>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedAccount>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let params = photon_api::models::PostGetCompressedAccountsByOwnerV2BodyParams {
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Hash),
                    data_slice: options.as_ref().and_then(|o| {
                        o.data_slice.as_ref().map(|ds| {
                            photon_api::models::DataSlice {
                                length: ds.length as u64,
                                offset: ds.offset as u64,
                            }
                        })
                    }),
                    filters: options.as_ref().and_then(|o| o.filters_to_photon()).unwrap_or_default(),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                    owner: photon_api::models::SerializablePubkey(owner.to_string()),
                };
                let request = photon_api::apis::default_api::make_get_compressed_accounts_by_owner_v2_body(params);
                let result =
                    photon_api::apis::default_api::get_compressed_accounts_by_owner_v2_post(
                        &self.configuration,
                        request,
                    )
                    .await?;

                Self::check_api_error("get_compressed_accounts_by_owner_v2", result.error)?;
                let response = Self::extract_result("get_compressed_accounts_by_owner_v2", result.result)?;

                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(CompressedAccount::try_from)
                    .collect();

                let cursor = response.value.cursor.map(|h| h.0);

                Ok(Response {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: accounts?,
                        cursor,
                    },
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let params = photon_api::models::PostGetCompressedAccountsByOwnerBodyParams {
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Hash),
                    data_slice: options.as_ref().and_then(|o| {
                        o.data_slice.as_ref().map(|ds| {
                            photon_api::models::DataSlice {
                                length: ds.length as u64,
                                offset: ds.offset as u64,
                            }
                        })
                    }),
                    filters: options.as_ref().and_then(|o| o.filters_to_photon()).unwrap_or_default(),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                    owner: photon_api::models::SerializablePubkey(owner.to_string()),
                };
                let request = photon_api::models::PostGetCompressedAccountsByOwnerBody {
                    id: photon_api::models::PostGetCompressedAccountsByOwnerBodyId::TestAccount,
                    jsonrpc: photon_api::models::PostGetCompressedAccountsByOwnerBodyJsonrpc::_20,
                    method: photon_api::models::PostGetCompressedAccountsByOwnerBodyMethod::GetCompressedAccountsByOwner,
                    params,
                };
                let result = photon_api::apis::default_api::get_compressed_accounts_by_owner_post(
                    &self.configuration,
                    request,
                )
                .await?;

                Self::check_api_error("get_compressed_accounts_by_owner", result.error)?;
                let response = Self::extract_result("get_compressed_accounts_by_owner", result.result)?;

                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(CompressedAccount::try_from)
                    .collect();

                let cursor = response.value.cursor.map(|h| h.0);

                Ok(Response {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: accounts?,
                        cursor,
                    },
                })
            }
        })
        .await
    }

    async fn get_compressed_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressedAccountBalanceBodyParams {
                address: address.map(|x| photon_api::models::SerializablePubkey(x.to_base58())),
                hash: hash.map(|x| photon_api::models::Hash(x.to_base58())),
            };
            let request =
                photon_api::apis::default_api::make_get_compressed_account_balance_body(params);

            let result = photon_api::apis::default_api::get_compressed_account_balance_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_compressed_account_balance", result.error)?;
            let api_response =
                Self::extract_result("get_compressed_account_balance", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: api_response.value.0,
            })
        })
        .await
    }

    async fn get_compressed_balance_by_owner(
        &self,
        owner: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressedBalanceByOwnerBodyParams {
                owner: photon_api::models::SerializablePubkey(owner.to_string()),
            };
            let request =
                photon_api::apis::default_api::make_get_compressed_balance_by_owner_body(params);

            let result = photon_api::apis::default_api::get_compressed_balance_by_owner_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_compressed_balance_by_owner", result.error)?;
            let api_response =
                Self::extract_result("get_compressed_balance_by_owner", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: api_response.value.0,
            })
        })
        .await
    }

    async fn get_compressed_mint_token_holders(
        &self,
        mint: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<OwnerBalance>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressedMintTokenHoldersBodyParams {
                mint: photon_api::models::SerializablePubkey(mint.to_string()),
                cursor: options
                    .as_ref()
                    .and_then(|o| o.cursor.clone())
                    .map(photon_api::models::Base58String),
                limit: options
                    .as_ref()
                    .and_then(|o| o.limit)
                    .map(|l| photon_api::models::Limit(l as u64)),
            };
            let request =
                photon_api::apis::default_api::make_get_compressed_mint_token_holders_body(params);

            let result = photon_api::apis::default_api::get_compressed_mint_token_holders_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_compressed_mint_token_holders", result.error)?;
            let api_response =
                Self::extract_result("get_compressed_mint_token_holders", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let owner_balances: Result<Vec<_>, _> = api_response
                .value
                .items
                .iter()
                .map(OwnerBalance::try_from)
                .collect();

            let cursor = api_response.value.cursor.map(|c| c.0);

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: ItemsWithCursor {
                    items: owner_balances?,
                    cursor,
                },
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
            let params = photon_api::models::PostGetCompressedTokenAccountBalanceBodyParams {
                address: address.map(|x| photon_api::models::SerializablePubkey(x.to_base58())),
                hash: hash.map(|x| photon_api::models::Hash(x.to_base58())),
            };
            let request =
                photon_api::apis::default_api::make_get_compressed_token_account_balance_body(
                    params,
                );

            let result = photon_api::apis::default_api::get_compressed_token_account_balance_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_compressed_token_account_balance", result.error)?;
            let api_response =
                Self::extract_result("get_compressed_token_account_balance", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: api_response.value.amount.0,
            })
        })
        .await
    }

    async fn get_compressed_token_accounts_by_delegate(
        &self,
        delegate: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedTokenAccount>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let params = photon_api::models::PostGetCompressedTokenAccountsByDelegateV2BodyParams {
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Base58String),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                    mint: options.as_ref().and_then(|o| o.mint.as_ref()).map(|x| photon_api::models::SerializablePubkey(x.to_string())),
                    delegate: photon_api::models::SerializablePubkey(delegate.to_string()),
                };
                let request = photon_api::apis::default_api::make_get_compressed_token_accounts_by_delegate_v2_body(params);

                let result = photon_api::apis::default_api::get_compressed_token_accounts_by_delegate_v2_post(
                    &self.configuration,
                    request,
                )
                .await?;

                Self::check_api_error("get_compressed_token_accounts_by_delegate_v2", result.error)?;
                let response = Self::extract_result("get_compressed_token_accounts_by_delegate_v2", result.result)?;

                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let token_accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(CompressedTokenAccount::try_from)
                    .collect();

                let cursor = response.value.cursor.map(|h| h.0);

                Ok(Response {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: token_accounts?,
                        cursor,
                    },
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let params = photon_api::models::PostGetCompressedTokenAccountsByDelegateBodyParams {
                    delegate: photon_api::models::SerializablePubkey(delegate.to_string()),
                    mint: options.as_ref().and_then(|o| o.mint.as_ref()).map(|x| photon_api::models::SerializablePubkey(x.to_string())),
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Base58String),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                };
                let request = photon_api::models::PostGetCompressedTokenAccountsByDelegateBody {
                    id: photon_api::models::PostGetCompressedTokenAccountsByDelegateBodyId::TestAccount,
                    jsonrpc: photon_api::models::PostGetCompressedTokenAccountsByDelegateBodyJsonrpc::_20,
                    method: photon_api::models::PostGetCompressedTokenAccountsByDelegateBodyMethod::GetCompressedTokenAccountsByDelegate,
                    params,
                };

                let result = photon_api::apis::default_api::get_compressed_token_accounts_by_delegate_post(
                    &self.configuration,
                    request,
                )
                .await?;

                Self::check_api_error("get_compressed_token_accounts_by_delegate", result.error)?;
                let response = Self::extract_result("get_compressed_token_accounts_by_delegate", result.result)?;

                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let token_accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(CompressedTokenAccount::try_from)
                    .collect();

                let cursor = response.value.cursor.map(|h| h.0);

                Ok(Response {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: token_accounts?,
                        cursor,
                    },
                })
            }
        })
        .await
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedTokenAccount>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let params = photon_api::models::PostGetCompressedTokenAccountsByOwnerV2BodyParams {
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Base58String),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                    mint: options
                        .as_ref()
                        .and_then(|o| o.mint.as_ref())
                        .map(|x| photon_api::models::SerializablePubkey(x.to_string())),
                    owner: photon_api::models::SerializablePubkey(owner.to_string()),
                };
                let request = photon_api::apis::default_api::make_get_compressed_token_accounts_by_owner_v2_body(params);
                let result =
                    photon_api::apis::default_api::get_compressed_token_accounts_by_owner_v2_post(
                        &self.configuration,
                        request,
                    )
                    .await?;

                Self::check_api_error("get_compressed_token_accounts_by_owner_v2", result.error)?;
                let response = Self::extract_result("get_compressed_token_accounts_by_owner_v2", result.result)?;

                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let token_accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(CompressedTokenAccount::try_from)
                    .collect();

                let cursor = response.value.cursor.map(|h| h.0);

                Ok(Response {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: token_accounts?,
                        cursor,
                    },
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let params = photon_api::models::PostGetCompressedTokenAccountsByOwnerBodyParams {
                    owner: photon_api::models::SerializablePubkey(owner.to_string()),
                    mint: options
                        .as_ref()
                        .and_then(|o| o.mint.as_ref())
                        .map(|x| photon_api::models::SerializablePubkey(x.to_string())),
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Base58String),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                };
                let request = photon_api::models::PostGetCompressedTokenAccountsByOwnerBody {
                    id: photon_api::models::PostGetCompressedTokenAccountsByOwnerBodyId::TestAccount,
                    jsonrpc: photon_api::models::PostGetCompressedTokenAccountsByOwnerBodyJsonrpc::_20,
                    method: photon_api::models::PostGetCompressedTokenAccountsByOwnerBodyMethod::GetCompressedTokenAccountsByOwner,
                    params,
                };

                let result =
                    photon_api::apis::default_api::get_compressed_token_accounts_by_owner_post(
                        &self.configuration,
                        request,
                    )
                    .await?;

                Self::check_api_error("get_compressed_token_accounts_by_owner", result.error)?;
                let response = Self::extract_result("get_compressed_token_accounts_by_owner", result.result)?;

                if response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let token_accounts: Result<Vec<_>, _> = response
                    .value
                    .items
                    .iter()
                    .map(CompressedTokenAccount::try_from)
                    .collect();

                let cursor = response.value.cursor.map(|h| h.0);

                Ok(Response {
                    context: Context {
                        slot: response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: token_accounts?,
                        cursor,
                    },
                })
            }
        })
        .await
    }

    async fn get_compressed_token_balances_by_owner_v2(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenBalance>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            #[cfg(feature = "v2")]
            {
                let params = photon_api::models::PostGetCompressedTokenBalancesByOwnerV2BodyParams {
                    owner: photon_api::models::SerializablePubkey(owner.to_string()),
                    mint: options
                        .as_ref()
                        .and_then(|o| o.mint.as_ref())
                        .map(|x| photon_api::models::SerializablePubkey(x.to_string())),
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Base58String),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                };
                let request = photon_api::apis::default_api::make_get_compressed_token_balances_by_owner_v2_body(params);

                let result =
                    photon_api::apis::default_api::get_compressed_token_balances_by_owner_v2_post(
                        &self.configuration,
                        request,
                    )
                    .await?;

                Self::check_api_error("get_compressed_token_balances_by_owner_v2", result.error)?;
                let api_response = Self::extract_result("get_compressed_token_balances_by_owner_v2", result.result)?;

                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let token_balances: Result<Vec<_>, _> = api_response
                    .value
                    .items
                    .iter()
                    .map(TokenBalance::try_from)
                    .collect();

                Ok(Response {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: token_balances?,
                        cursor: api_response.value.cursor.map(|c| c.0),
                    },
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let params = photon_api::models::PostGetCompressedTokenBalancesByOwnerBodyParams {
                    owner: photon_api::models::SerializablePubkey(owner.to_string()),
                    mint: options
                        .as_ref()
                        .and_then(|o| o.mint.as_ref())
                        .map(|x| photon_api::models::SerializablePubkey(x.to_string())),
                    cursor: options.as_ref().and_then(|o| o.cursor.clone()).map(photon_api::models::Base58String),
                    limit: options.as_ref().and_then(|o| o.limit).map(|l| photon_api::models::Limit(l as u64)),
                };
                let request = photon_api::models::PostGetCompressedTokenBalancesByOwnerBody {
                    id: photon_api::models::PostGetCompressedTokenBalancesByOwnerBodyId::TestAccount,
                    jsonrpc: photon_api::models::PostGetCompressedTokenBalancesByOwnerBodyJsonrpc::_20,
                    method: photon_api::models::PostGetCompressedTokenBalancesByOwnerBodyMethod::GetCompressedTokenBalancesByOwner,
                    params,
                };

                let result =
                    photon_api::apis::default_api::get_compressed_token_balances_by_owner_post(
                        &self.configuration,
                        request,
                    )
                    .await?;

                Self::check_api_error("get_compressed_token_balances_by_owner", result.error)?;
                let api_response = Self::extract_result("get_compressed_token_balances_by_owner", result.result)?;

                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }

                let token_balances: Result<Vec<_>, _> = api_response
                    .value
                    .token_balances
                    .iter()
                    .map(TokenBalance::try_from)
                    .collect();

                Ok(Response {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: ItemsWithCursor {
                        items: token_balances?,
                        cursor: api_response.value.cursor.map(|c| c.0),
                    },
                })
            }
        })
        .await
    }

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<SignatureWithMetadata>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressionSignaturesForAccountBodyParams {
                hash: photon_api::models::Hash(hash.to_base58()),
            };
            let request =
                photon_api::apis::default_api::make_get_compression_signatures_for_account_body(
                    params,
                );

            let result =
                photon_api::apis::default_api::get_compression_signatures_for_account_post(
                    &self.configuration,
                    request,
                )
                .await?;

            Self::check_api_error("get_compression_signatures_for_account", result.error)?;
            let api_response =
                Self::extract_result("get_compression_signatures_for_account", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let signatures: Vec<SignatureWithMetadata> = api_response
                .value
                .items
                .iter()
                .map(SignatureWithMetadata::from)
                .collect();

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: Items { items: signatures },
            })
        })
        .await
    }

    async fn get_compression_signatures_for_address(
        &self,
        address: &[u8; 32],
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressionSignaturesForAddressBodyParams {
                address: photon_api::models::SerializablePubkey(address.to_base58()),
                cursor: options.as_ref().and_then(|o| o.cursor.clone()),
                limit: options
                    .as_ref()
                    .and_then(|o| o.limit)
                    .map(|l| photon_api::models::Limit(l as u64)),
            };
            let request =
                photon_api::apis::default_api::make_get_compression_signatures_for_address_body(
                    params,
                );

            let result =
                photon_api::apis::default_api::get_compression_signatures_for_address_post(
                    &self.configuration,
                    request,
                )
                .await?;

            Self::check_api_error("get_compression_signatures_for_address", result.error)?;
            let api_response =
                Self::extract_result("get_compression_signatures_for_address", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let signatures: Vec<SignatureWithMetadata> = api_response
                .value
                .items
                .iter()
                .map(SignatureWithMetadata::from)
                .collect();

            let cursor = api_response.value.cursor;

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: ItemsWithCursor {
                    items: signatures,
                    cursor,
                },
            })
        })
        .await
    }

    async fn get_compression_signatures_for_owner(
        &self,
        owner: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressionSignaturesForOwnerBodyParams {
                owner: photon_api::models::SerializablePubkey(owner.to_string()),
                cursor: options.as_ref().and_then(|o| o.cursor.clone()),
                limit: options
                    .as_ref()
                    .and_then(|o| o.limit)
                    .map(|l| photon_api::models::Limit(l as u64)),
            };
            let request =
                photon_api::apis::default_api::make_get_compression_signatures_for_owner_body(
                    params,
                );

            let result = photon_api::apis::default_api::get_compression_signatures_for_owner_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_compression_signatures_for_owner", result.error)?;
            let api_response =
                Self::extract_result("get_compression_signatures_for_owner", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let signatures: Vec<SignatureWithMetadata> = api_response
                .value
                .items
                .iter()
                .map(SignatureWithMetadata::from)
                .collect();

            let cursor = api_response.value.cursor;

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: ItemsWithCursor {
                    items: signatures,
                    cursor,
                },
            })
        })
        .await
    }

    async fn get_compression_signatures_for_token_owner(
        &self,
        owner: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetCompressionSignaturesForTokenOwnerBodyParams {
                owner: photon_api::models::SerializablePubkey(owner.to_string()),
                cursor: options.as_ref().and_then(|o| o.cursor.clone()),
                limit: options
                    .as_ref()
                    .and_then(|o| o.limit)
                    .map(|l| photon_api::models::Limit(l as u64)),
            };
            let request =
                photon_api::apis::default_api::make_get_compression_signatures_for_token_owner_body(
                    params,
                );

            let result =
                photon_api::apis::default_api::get_compression_signatures_for_token_owner_post(
                    &self.configuration,
                    request,
                )
                .await?;

            Self::check_api_error("get_compression_signatures_for_token_owner", result.error)?;
            let api_response =
                Self::extract_result("get_compression_signatures_for_token_owner", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let signatures: Vec<SignatureWithMetadata> = api_response
                .value
                .items
                .iter()
                .map(SignatureWithMetadata::from)
                .collect();

            let cursor = api_response.value.cursor;

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: ItemsWithCursor {
                    items: signatures,
                    cursor,
                },
            })
        })
        .await
    }

    async fn get_indexer_health(&self, config: Option<RetryConfig>) -> Result<bool, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config, || async {
            let request = photon_api::apis::default_api::make_get_indexer_health_body();

            let result = photon_api::apis::default_api::get_indexer_health_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_indexer_health", result.error)?;
            // result.result is not Optional for this endpoint
            let _health = result.result;

            Ok(true)
        })
        .await
    }

    async fn get_indexer_slot(&self, config: Option<RetryConfig>) -> Result<u64, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config, || async {
            let request = photon_api::apis::default_api::make_get_indexer_slot_body();

            let result =
                photon_api::apis::default_api::get_indexer_slot_post(&self.configuration, request)
                    .await?;

            Self::check_api_error("get_indexer_slot", result.error)?;
            // result.result is u64 directly for this endpoint
            Ok(result.result)
        })
        .await
    }

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<MerkleProof>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let hashes_for_async = hashes.clone();

            let params: Vec<photon_api::models::Hash> = hashes_for_async
                .into_iter()
                .map(|hash| photon_api::models::Hash(bs58::encode(hash).into_string()))
                .collect();
            let request =
                photon_api::apis::default_api::make_get_multiple_compressed_account_proofs_body(
                    params,
                );

            let result =
                photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
                    &self.configuration,
                    request,
                )
                .await?;

            Self::check_api_error("get_multiple_compressed_account_proofs", result.error)?;
            let photon_proofs =
                Self::extract_result("get_multiple_compressed_account_proofs", result.result)?;

            if photon_proofs.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let proofs = photon_proofs
                .value
                .iter()
                .map(|x| {
                    let mut proof_vec = x.proof.clone();
                    if proof_vec.len() < STATE_MERKLE_TREE_CANOPY_DEPTH {
                        return Err(IndexerError::InvalidParameters(format!(
                            "Merkle proof length ({}) is less than canopy depth ({})",
                            proof_vec.len(),
                            STATE_MERKLE_TREE_CANOPY_DEPTH,
                        )));
                    }
                    proof_vec.truncate(proof_vec.len() - STATE_MERKLE_TREE_CANOPY_DEPTH);

                    let proof = proof_vec
                        .iter()
                        .map(|s| Hash::from_base58(s))
                        .collect::<Result<Vec<[u8; 32]>, IndexerError>>()
                        .map_err(|e| IndexerError::Base58DecodeError {
                            field: "proof".to_string(),
                            message: e.to_string(),
                        })?;

                    Ok(MerkleProof {
                        hash: <[u8; 32] as Base58Conversions>::from_base58(&x.hash)?,
                        leaf_index: x.leaf_index as u64,
                        merkle_tree: Pubkey::from_str_const(x.merkle_tree.0.as_str()),
                        proof,
                        root_seq: x.root_seq,
                        root: <[u8; 32] as Base58Conversions>::from_base58(&x.root)?,
                    })
                })
                .collect::<Result<Vec<MerkleProof>, IndexerError>>()?;

            Ok(Response {
                context: Context {
                    slot: photon_proofs.context.slot,
                },
                value: Items { items: proofs },
            })
        })
        .await
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<Option<CompressedAccount>>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let hashes = hashes.clone();
            let addresses = addresses.clone();
            let params = photon_api::models::PostGetMultipleCompressedAccountsBodyParams {
                addresses: addresses.map(|x| {
                    x.iter()
                        .map(|a| photon_api::models::SerializablePubkey(a.to_base58()))
                        .collect()
                }),
                hashes: hashes.map(|x| {
                    x.iter()
                        .map(|h| photon_api::models::Hash(h.to_base58()))
                        .collect()
                }),
            };
            let request =
                photon_api::apis::default_api::make_get_multiple_compressed_accounts_body(params);

            let result = photon_api::apis::default_api::get_multiple_compressed_accounts_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_multiple_compressed_accounts", result.error)?;
            let api_response =
                Self::extract_result("get_multiple_compressed_accounts", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }
            let accounts = api_response
                .value
                .items
                .iter()
                .map(|account_opt| match account_opt {
                    Some(account) => CompressedAccount::try_from(account).map(Some),
                    None => Ok(None),
                })
                .collect::<Result<Vec<Option<CompressedAccount>>, IndexerError>>()?;

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: Items { items: accounts },
            })
        })
        .await
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<NewAddressProofWithContext>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params: Vec<photon_api::models::AddressWithTree> = addresses
                .iter()
                .map(|x| photon_api::models::AddressWithTree {
                    address: photon_api::models::SerializablePubkey(bs58::encode(x).into_string()),
                    tree: photon_api::models::SerializablePubkey(
                        bs58::encode(&merkle_tree_pubkey).into_string(),
                    ),
                })
                .collect();

            let request =
                photon_api::apis::default_api::make_get_multiple_new_address_proofs_v2_body(params);

            let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_multiple_new_address_proofs", result.error)?;
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
                let tree_pubkey = Hash::from_base58(&photon_proof.merkle_tree.0).map_err(|e| {
                    IndexerError::Base58DecodeError {
                        field: "merkle_tree".to_string(),
                        message: e.to_string(),
                    }
                })?;

                let low_address_value = Hash::from_base58(&photon_proof.lower_range_address.0)
                    .map_err(|e| IndexerError::Base58DecodeError {
                        field: "lower_range_address".to_string(),
                        message: e.to_string(),
                    })?;

                let next_address_value = Hash::from_base58(&photon_proof.higher_range_address.0)
                    .map_err(|e| IndexerError::Base58DecodeError {
                        field: "higher_range_address".to_string(),
                        message: e.to_string(),
                    })?;

                let mut proof_vec: Vec<[u8; 32]> = photon_proof
                    .proof
                    .iter()
                    .map(|x| Hash::from_base58(x))
                    .collect::<Result<Vec<[u8; 32]>, IndexerError>>()?;

                const ADDRESS_TREE_CANOPY_DEPTH: usize = 10;
                if proof_vec.len() < ADDRESS_TREE_CANOPY_DEPTH {
                    return Err(IndexerError::InvalidParameters(format!(
                        "Address proof length ({}) is less than canopy depth ({})",
                        proof_vec.len(),
                        ADDRESS_TREE_CANOPY_DEPTH,
                    )));
                }
                proof_vec.truncate(proof_vec.len() - ADDRESS_TREE_CANOPY_DEPTH);
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
                    low_address_index: photon_proof.low_element_leaf_index as u64,
                    low_address_value,
                    low_address_next_index: photon_proof.next_index as u64,
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
                value: Items { items: proofs },
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
                let params = photon_api::models::PostGetValidityProofV2BodyParams {
                    hashes: hashes
                        .iter()
                        .map(|x| photon_api::models::Hash(x.to_base58()))
                        .collect(),
                    new_addresses_with_trees: new_addresses_with_trees
                        .iter()
                        .map(|x| photon_api::models::AddressWithTree {
                            address: photon_api::models::SerializablePubkey(x.address.to_base58()),
                            tree: photon_api::models::SerializablePubkey(x.tree.to_string()),
                        })
                        .collect(),
                };
                let request =
                    photon_api::apis::default_api::make_get_validity_proof_v2_body(params);

                let result = photon_api::apis::default_api::get_validity_proof_v2_post(
                    &self.configuration,
                    request,
                )
                .await?;

                Self::check_api_error("get_validity_proof_v2", result.error)?;
                let api_response = Self::extract_result("get_validity_proof_v2", result.result)?;

                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let validity_proof =
                    super::types::ValidityProofWithContext::from_api_model_v2(api_response.value)?;

                Ok(Response {
                    context: Context {
                        slot: api_response.context.slot,
                    },
                    value: validity_proof,
                })
            }
            #[cfg(not(feature = "v2"))]
            {
                let params = photon_api::models::PostGetValidityProofBodyParams {
                    hashes: hashes
                        .iter()
                        .map(|x| photon_api::models::Hash(x.to_base58()))
                        .collect(),
                    new_addresses_with_trees: new_addresses_with_trees
                        .iter()
                        .map(|x| photon_api::models::AddressWithTree {
                            address: photon_api::models::SerializablePubkey(x.address.to_base58()),
                            tree: photon_api::models::SerializablePubkey(x.tree.to_string()),
                        })
                        .collect(),
                };
                let request = photon_api::apis::default_api::make_get_validity_proof_body(params);

                let result = photon_api::apis::default_api::get_validity_proof_post(
                    &self.configuration,
                    request,
                )
                .await?;

                Self::check_api_error("get_validity_proof", result.error)?;
                let api_response = Self::extract_result("get_validity_proof", result.result)?;

                if api_response.context.slot < config.slot {
                    return Err(IndexerError::IndexerNotSyncedToSlot);
                }
                let validity_proof = super::types::ValidityProofWithContext::from_api_model(
                    api_response.value,
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

    async fn get_queue_info(
        &self,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<super::QueueInfoResult>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetQueueInfoBodyParams { trees: None };
            let request = photon_api::apis::default_api::make_get_queue_info_body(params);

            let result =
                photon_api::apis::default_api::get_queue_info_post(&self.configuration, request)
                    .await?;

            let api_response = Self::extract_result_with_error_check(
                "get_queue_info",
                result.error,
                result.result,
            )?;

            if api_response.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let queues = api_response
                .queues
                .iter()
                .map(|q| -> Result<_, IndexerError> {
                    let tree_bytes = super::base58::decode_base58_to_fixed_array(&q.tree)?;
                    let queue_bytes = super::base58::decode_base58_to_fixed_array(&q.queue)?;

                    Ok(super::QueueInfo {
                        tree: Pubkey::new_from_array(tree_bytes),
                        queue: Pubkey::new_from_array(queue_bytes),
                        queue_type: q.queue_type,
                        queue_size: q.queue_size,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Response {
                context: Context {
                    slot: api_response.slot,
                },
                value: super::QueueInfoResult {
                    queues,
                    slot: api_response.slot,
                },
            })
        })
        .await
    }

    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        options: super::QueueElementsV2Options,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<super::QueueElementsResult>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let tree_hash =
                photon_api::models::Hash(bs58::encode(&merkle_tree_pubkey).into_string());

            // Build queue request objects
            let output_queue = if options.output_queue_limit.is_some()
                || options.output_queue_start_index.is_some()
            {
                Some(photon_api::models::QueueRequest {
                    limit: options.output_queue_limit.unwrap_or(100),
                    start_index: options.output_queue_start_index,
                    zkp_batch_size: options.output_queue_zkp_batch_size,
                })
            } else {
                None
            };

            let input_queue = if options.input_queue_limit.is_some()
                || options.input_queue_start_index.is_some()
            {
                Some(photon_api::models::QueueRequest {
                    limit: options.input_queue_limit.unwrap_or(100),
                    start_index: options.input_queue_start_index,
                    zkp_batch_size: options.input_queue_zkp_batch_size,
                })
            } else {
                None
            };

            let address_queue = if options.address_queue_limit.is_some()
                || options.address_queue_start_index.is_some()
            {
                Some(photon_api::models::QueueRequest {
                    limit: options.address_queue_limit.unwrap_or(100),
                    start_index: options.address_queue_start_index,
                    zkp_batch_size: options.address_queue_zkp_batch_size,
                })
            } else {
                None
            };

            let params = photon_api::models::PostGetQueueElementsBodyParams {
                tree: tree_hash,
                output_queue,
                input_queue,
                address_queue,
            };
            let request = photon_api::apis::default_api::make_get_queue_elements_body(params);

            let result = photon_api::apis::default_api::get_queue_elements_post(
                &self.configuration,
                request,
            )
            .await?;

            Self::check_api_error("get_queue_elements", result.error)?;
            let api_response = Self::extract_result("get_queue_elements", result.result)?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            // Convert API StateQueueData to local StateQueueData
            let state_queue = if let Some(sq) = api_response.state_queue {
                let output_queue = if let Some(oq) = sq.output_queue {
                    Some(super::OutputQueueData {
                        leaf_indices: oq.leaf_indices.clone(),
                        account_hashes: oq
                            .account_hashes
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                        old_leaves: oq
                            .leaves
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                        first_queue_index: oq.first_queue_index,
                        next_index: oq.next_index,
                        leaves_hash_chains: oq
                            .leaves_hash_chains
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                    })
                } else {
                    None
                };

                let input_queue = if let Some(iq) = sq.input_queue {
                    Some(super::InputQueueData {
                        leaf_indices: iq.leaf_indices.clone(),
                        account_hashes: iq
                            .account_hashes
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                        current_leaves: iq
                            .leaves
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                        tx_hashes: iq
                            .tx_hashes
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                        nullifiers: iq
                            .nullifiers
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                        first_queue_index: iq.first_queue_index,
                        leaves_hash_chains: iq
                            .leaves_hash_chains
                            .iter()
                            .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                            .collect::<Result<Vec<_>, _>>()?,
                    })
                } else {
                    None
                };

                Some(super::StateQueueData {
                    nodes: sq.nodes.iter().map(|n| n.index).collect(),
                    node_hashes: sq
                        .nodes
                        .iter()
                        .map(|n| super::base58::decode_base58_to_fixed_array(&n.hash.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    initial_root: super::base58::decode_base58_to_fixed_array(&sq.initial_root.0)?,
                    root_seq: sq.root_seq,
                    output_queue,
                    input_queue,
                })
            } else {
                None
            };

            // Convert API AddressQueueData to local AddressQueueData
            let address_queue = if let Some(aq) = api_response.address_queue {
                Some(super::AddressQueueData {
                    addresses: aq
                        .addresses
                        .iter()
                        .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    low_element_values: aq
                        .low_element_values
                        .iter()
                        .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    low_element_next_values: aq
                        .low_element_next_values
                        .iter()
                        .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    low_element_indices: aq.low_element_indices.clone(),
                    low_element_next_indices: aq.low_element_next_indices.clone(),
                    nodes: aq.nodes.iter().map(|n| n.index).collect(),
                    node_hashes: aq
                        .nodes
                        .iter()
                        .map(|n| super::base58::decode_base58_to_fixed_array(&n.hash.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    initial_root: super::base58::decode_base58_to_fixed_array(&aq.initial_root.0)?,
                    leaves_hash_chains: aq
                        .leaves_hash_chains
                        .iter()
                        .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    subtrees: aq
                        .subtrees
                        .iter()
                        .map(|h| super::base58::decode_base58_to_fixed_array(&h.0))
                        .collect::<Result<Vec<_>, _>>()?,
                    start_index: aq.start_index,
                    root_seq: aq.root_seq,
                })
            } else {
                None
            };

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: super::QueueElementsResult {
                    state_queue,
                    address_queue,
                },
            })
        })
        .await
    }

    async fn get_subtrees(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<[u8; 32]>>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!();
        #[cfg(feature = "v2")]
        {
            todo!();
        }
    }
}

// ============ Interface Methods ============
// These methods use the Interface endpoints that race hot (on-chain) and cold (compressed) lookups

impl PhotonIndexer {
    /// Get account data from either on-chain or compressed sources.
    /// Races both lookups and returns the result with the higher slot.
    pub async fn get_account_interface(
        &self,
        address: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<AccountInterface>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetAccountInterfaceBodyParams {
                address: photon_api::models::SerializablePubkey(address.to_string()),
            };
            let request = photon_api::apis::default_api::make_get_account_interface_body(params);

            let result = photon_api::apis::default_api::get_account_interface_post(
                &self.configuration,
                request,
            )
            .await?;

            let api_response = Self::extract_result_with_error_check(
                "get_account_interface",
                result.error,
                result.result,
            )?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let account = match api_response.value {
                Some(ref ai) => Some(AccountInterface::try_from(ai)?),
                None => None,
            };

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: account,
            })
        })
        .await
    }

    /// Get token account data from either on-chain or compressed sources.
    /// Races both lookups and returns the result with the higher slot.
    pub async fn get_token_account_interface(
        &self,
        address: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<TokenAccountInterface>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetTokenAccountInterfaceBodyParams {
                address: photon_api::models::SerializablePubkey(address.to_string()),
            };
            let request =
                photon_api::apis::default_api::make_get_token_account_interface_body(params);

            let result = photon_api::apis::default_api::get_token_account_interface_post(
                &self.configuration,
                request,
            )
            .await?;

            let api_response = Self::extract_result_with_error_check(
                "get_token_account_interface",
                result.error,
                result.result,
            )?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let account = match api_response.value {
                Some(ref tai) => Some(TokenAccountInterface::try_from(tai)?),
                None => None,
            };

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: account,
            })
        })
        .await
    }

    /// Get Associated Token Account data from either on-chain or compressed sources.
    /// Derives the Light Protocol ATA address from owner+mint, then races hot/cold lookups.
    pub async fn get_associated_token_account_interface(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<TokenAccountInterface>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetAtaInterfaceBodyParams {
                owner: photon_api::models::SerializablePubkey(owner.to_string()),
                mint: photon_api::models::SerializablePubkey(mint.to_string()),
            };
            let request = photon_api::apis::default_api::make_get_ata_interface_body(params);

            let result =
                photon_api::apis::default_api::get_ata_interface_post(&self.configuration, request)
                    .await?;

            let api_response = Self::extract_result_with_error_check(
                "get_associated_token_account_interface",
                result.error,
                result.result,
            )?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let account = match api_response.value {
                Some(ref tai) => Some(TokenAccountInterface::try_from(tai)?),
                None => None,
            };

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: account,
            })
        })
        .await
    }

    /// Get multiple account interfaces in a batch.
    /// Returns a vector where each element corresponds to an input address.
    pub async fn get_multiple_account_interfaces(
        &self,
        addresses: Vec<&Pubkey>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<Option<AccountInterface>>>, IndexerError> {
        let config = config.unwrap_or_default();
        self.retry(config.retry_config, || async {
            let params = photon_api::models::PostGetMultipleAccountInterfacesBodyParams {
                addresses: addresses
                    .iter()
                    .map(|addr| photon_api::models::SerializablePubkey(addr.to_string()))
                    .collect(),
            };
            let request =
                photon_api::apis::default_api::make_get_multiple_account_interfaces_body(params);

            let result = photon_api::apis::default_api::get_multiple_account_interfaces_post(
                &self.configuration,
                request,
            )
            .await?;

            let api_response = Self::extract_result_with_error_check(
                "get_multiple_account_interfaces",
                result.error,
                result.result,
            )?;

            if api_response.context.slot < config.slot {
                return Err(IndexerError::IndexerNotSyncedToSlot);
            }

            let accounts: Result<Vec<Option<AccountInterface>>, IndexerError> = api_response
                .value
                .into_iter()
                .map(|maybe_acc| {
                    maybe_acc
                        .map(|ai| AccountInterface::try_from(&ai))
                        .transpose()
                })
                .collect();

            Ok(Response {
                context: Context {
                    slot: api_response.context.slot,
                },
                value: accounts?,
            })
        })
        .await
    }
}
