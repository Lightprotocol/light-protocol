//! Photon API client generated from OpenAPI spec using progenitor.
//!
//! This crate provides a Rust client for the Photon indexer API.

#![allow(unused_imports, clippy::all, dead_code)]
#![allow(mismatched_lifetime_syntaxes)]

// Include the generated code from build.rs
include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

/// Re-export commonly used types at the crate root for backward compatibility
pub mod models {
    pub use super::types::*;
}

/// Backward-compatible APIs module
pub mod apis {
    use super::*;

    /// Configuration for the Photon API client
    #[derive(Debug, Clone)]
    pub struct Configuration {
        pub base_path: String,
        pub api_key: Option<ApiKey>,
        pub client: reqwest::Client,
        pub user_agent: Option<String>,
        pub basic_auth: Option<(String, Option<String>)>,
        pub oauth_access_token: Option<String>,
        pub bearer_access_token: Option<String>,
    }

    impl Default for Configuration {
        fn default() -> Self {
            Self {
                base_path: "https://devnet.helius-rpc.com".to_string(),
                api_key: None,
                client: reqwest::Client::new(),
                user_agent: Some("progenitor/0.9".to_string()),
                basic_auth: None,
                oauth_access_token: None,
                bearer_access_token: None,
            }
        }
    }

    impl Configuration {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn new_with_api(url: String, api_key: Option<String>) -> Self {
            Self {
                base_path: url,
                api_key: api_key.map(|key| ApiKey {
                    prefix: Some("api-key".to_string()),
                    key,
                }),
                ..Default::default()
            }
        }

        /// Create a progenitor Client from this configuration
        pub fn to_client(&self) -> Client {
            Client::new_with_client(&self.base_path, self.client.clone())
        }
    }

    #[derive(Debug, Clone)]
    pub struct ApiKey {
        pub prefix: Option<String>,
        pub key: String,
    }

    pub type BasicAuth = (String, Option<String>);

    pub mod configuration {
        pub use super::{ApiKey, BasicAuth, Configuration};
    }

    /// Error type for API calls
    #[derive(Debug)]
    pub enum Error<T> {
        Reqwest(reqwest::Error),
        Serde(serde_json::Error),
        Io(std::io::Error),
        ResponseError(ResponseContent<T>),
    }

    impl<T> std::fmt::Display for Error<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Reqwest(e) => write!(f, "Request error: {}", e),
                Error::Serde(e) => write!(f, "Serialization error: {}", e),
                Error::Io(e) => write!(f, "IO error: {}", e),
                Error::ResponseError(e) => write!(f, "Response error: {}", e.status),
            }
        }
    }

    impl<T: std::fmt::Debug> std::error::Error for Error<T> {}

    impl<T> From<reqwest::Error> for Error<T> {
        fn from(e: reqwest::Error) -> Self {
            Error::Reqwest(e)
        }
    }

    impl<T> From<serde_json::Error> for Error<T> {
        fn from(e: serde_json::Error) -> Self {
            Error::Serde(e)
        }
    }

    impl<T> From<std::io::Error> for Error<T> {
        fn from(e: std::io::Error) -> Self {
            Error::Io(e)
        }
    }

    #[derive(Debug, Clone)]
    pub struct ResponseContent<T> {
        pub status: reqwest::StatusCode,
        pub content: String,
        pub entity: Option<T>,
    }

    /// Default API module providing backward-compatible function-style API calls
    pub mod default_api {
        use super::*;

        // Body construction helper functions
        pub fn make_get_compressed_account_body(
            params: types::PostGetCompressedAccountBodyParams,
        ) -> types::PostGetCompressedAccountBody {
            types::PostGetCompressedAccountBody {
                id: types::PostGetCompressedAccountBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedAccountBodyJsonrpc::_20,
                method: types::PostGetCompressedAccountBodyMethod::GetCompressedAccount,
                params,
            }
        }

        pub fn make_get_compressed_account_balance_body(
            params: types::PostGetCompressedAccountBalanceBodyParams,
        ) -> types::PostGetCompressedAccountBalanceBody {
            types::PostGetCompressedAccountBalanceBody {
                id: types::PostGetCompressedAccountBalanceBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedAccountBalanceBodyJsonrpc::_20,
                method:
                    types::PostGetCompressedAccountBalanceBodyMethod::GetCompressedAccountBalance,
                params,
            }
        }

        pub fn make_get_compressed_accounts_by_owner_v2_body(
            params: types::PostGetCompressedAccountsByOwnerV2BodyParams,
        ) -> types::PostGetCompressedAccountsByOwnerV2Body {
            types::PostGetCompressedAccountsByOwnerV2Body {
                id: types::PostGetCompressedAccountsByOwnerV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedAccountsByOwnerV2BodyJsonrpc::_20,
                method: types::PostGetCompressedAccountsByOwnerV2BodyMethod::GetCompressedAccountsByOwnerV2,
                params,
            }
        }

        pub fn make_get_compressed_balance_by_owner_body(
            params: types::PostGetCompressedBalanceByOwnerBodyParams,
        ) -> types::PostGetCompressedBalanceByOwnerBody {
            types::PostGetCompressedBalanceByOwnerBody {
                id: types::PostGetCompressedBalanceByOwnerBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedBalanceByOwnerBodyJsonrpc::_20,
                method:
                    types::PostGetCompressedBalanceByOwnerBodyMethod::GetCompressedBalanceByOwner,
                params,
            }
        }

        pub fn make_get_compressed_mint_token_holders_body(
            params: types::PostGetCompressedMintTokenHoldersBodyParams,
        ) -> types::PostGetCompressedMintTokenHoldersBody {
            types::PostGetCompressedMintTokenHoldersBody {
                id: types::PostGetCompressedMintTokenHoldersBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedMintTokenHoldersBodyJsonrpc::_20,
                method: types::PostGetCompressedMintTokenHoldersBodyMethod::GetCompressedMintTokenHolders,
                params,
            }
        }

        pub fn make_get_compressed_token_account_balance_body(
            params: types::PostGetCompressedTokenAccountBalanceBodyParams,
        ) -> types::PostGetCompressedTokenAccountBalanceBody {
            types::PostGetCompressedTokenAccountBalanceBody {
                id: types::PostGetCompressedTokenAccountBalanceBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenAccountBalanceBodyJsonrpc::_20,
                method: types::PostGetCompressedTokenAccountBalanceBodyMethod::GetCompressedTokenAccountBalance,
                params,
            }
        }

        pub fn make_get_validity_proof_v2_body(
            params: types::PostGetValidityProofV2BodyParams,
        ) -> types::PostGetValidityProofV2Body {
            types::PostGetValidityProofV2Body {
                id: types::PostGetValidityProofV2BodyId::TestAccount,
                jsonrpc: types::PostGetValidityProofV2BodyJsonrpc::_20,
                method: types::PostGetValidityProofV2BodyMethod::GetValidityProofV2,
                params,
            }
        }

        pub fn make_get_multiple_new_address_proofs_v2_body(
            params: Vec<types::AddressWithTree>,
        ) -> types::PostGetMultipleNewAddressProofsV2Body {
            types::PostGetMultipleNewAddressProofsV2Body {
                id: types::PostGetMultipleNewAddressProofsV2BodyId::TestAccount,
                jsonrpc: types::PostGetMultipleNewAddressProofsV2BodyJsonrpc::_20,
                method: types::PostGetMultipleNewAddressProofsV2BodyMethod::GetMultipleNewAddressProofsV2,
                params,
            }
        }

        pub fn make_get_compressed_token_accounts_by_delegate_v2_body(
            params: types::PostGetCompressedTokenAccountsByDelegateV2BodyParams,
        ) -> types::PostGetCompressedTokenAccountsByDelegateV2Body {
            types::PostGetCompressedTokenAccountsByDelegateV2Body {
                id: types::PostGetCompressedTokenAccountsByDelegateV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenAccountsByDelegateV2BodyJsonrpc::_20,
                method: types::PostGetCompressedTokenAccountsByDelegateV2BodyMethod::GetCompressedTokenAccountsByDelegateV2,
                params,
            }
        }

        pub fn make_get_compressed_token_accounts_by_owner_v2_body(
            params: types::PostGetCompressedTokenAccountsByOwnerV2BodyParams,
        ) -> types::PostGetCompressedTokenAccountsByOwnerV2Body {
            types::PostGetCompressedTokenAccountsByOwnerV2Body {
                id: types::PostGetCompressedTokenAccountsByOwnerV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenAccountsByOwnerV2BodyJsonrpc::_20,
                method: types::PostGetCompressedTokenAccountsByOwnerV2BodyMethod::GetCompressedTokenAccountsByOwnerV2,
                params,
            }
        }

        pub fn make_get_compressed_token_balances_by_owner_v2_body(
            params: types::PostGetCompressedTokenBalancesByOwnerV2BodyParams,
        ) -> types::PostGetCompressedTokenBalancesByOwnerV2Body {
            types::PostGetCompressedTokenBalancesByOwnerV2Body {
                id: types::PostGetCompressedTokenBalancesByOwnerV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenBalancesByOwnerV2BodyJsonrpc::_20,
                method: types::PostGetCompressedTokenBalancesByOwnerV2BodyMethod::GetCompressedTokenBalancesByOwnerV2,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_account_body(
            params: types::PostGetCompressionSignaturesForAccountBodyParams,
        ) -> types::PostGetCompressionSignaturesForAccountBody {
            types::PostGetCompressionSignaturesForAccountBody {
                id: types::PostGetCompressionSignaturesForAccountBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForAccountBodyJsonrpc::_20,
                method: types::PostGetCompressionSignaturesForAccountBodyMethod::GetCompressionSignaturesForAccount,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_address_body(
            params: types::PostGetCompressionSignaturesForAddressBodyParams,
        ) -> types::PostGetCompressionSignaturesForAddressBody {
            types::PostGetCompressionSignaturesForAddressBody {
                id: types::PostGetCompressionSignaturesForAddressBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForAddressBodyJsonrpc::_20,
                method: types::PostGetCompressionSignaturesForAddressBodyMethod::GetCompressionSignaturesForAddress,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_owner_body(
            params: types::PostGetCompressionSignaturesForOwnerBodyParams,
        ) -> types::PostGetCompressionSignaturesForOwnerBody {
            types::PostGetCompressionSignaturesForOwnerBody {
                id: types::PostGetCompressionSignaturesForOwnerBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForOwnerBodyJsonrpc::_20,
                method: types::PostGetCompressionSignaturesForOwnerBodyMethod::GetCompressionSignaturesForOwner,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_token_owner_body(
            params: types::PostGetCompressionSignaturesForTokenOwnerBodyParams,
        ) -> types::PostGetCompressionSignaturesForTokenOwnerBody {
            types::PostGetCompressionSignaturesForTokenOwnerBody {
                id: types::PostGetCompressionSignaturesForTokenOwnerBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForTokenOwnerBodyJsonrpc::_20,
                method: types::PostGetCompressionSignaturesForTokenOwnerBodyMethod::GetCompressionSignaturesForTokenOwner,
                params,
            }
        }

        pub fn make_get_indexer_health_body() -> types::PostGetIndexerHealthBody {
            types::PostGetIndexerHealthBody {
                id: types::PostGetIndexerHealthBodyId::TestAccount,
                jsonrpc: types::PostGetIndexerHealthBodyJsonrpc::_20,
                method: types::PostGetIndexerHealthBodyMethod::GetIndexerHealth,
            }
        }

        pub fn make_get_indexer_slot_body() -> types::PostGetIndexerSlotBody {
            types::PostGetIndexerSlotBody {
                id: types::PostGetIndexerSlotBodyId::TestAccount,
                jsonrpc: types::PostGetIndexerSlotBodyJsonrpc::_20,
                method: types::PostGetIndexerSlotBodyMethod::GetIndexerSlot,
            }
        }

        pub fn make_get_multiple_compressed_account_proofs_body(
            params: Vec<types::Hash>,
        ) -> types::PostGetMultipleCompressedAccountProofsBody {
            types::PostGetMultipleCompressedAccountProofsBody {
                id: types::PostGetMultipleCompressedAccountProofsBodyId::TestAccount,
                jsonrpc: types::PostGetMultipleCompressedAccountProofsBodyJsonrpc::_20,
                method: types::PostGetMultipleCompressedAccountProofsBodyMethod::GetMultipleCompressedAccountProofs,
                params,
            }
        }

        pub fn make_get_multiple_compressed_accounts_body(
            params: types::PostGetMultipleCompressedAccountsBodyParams,
        ) -> types::PostGetMultipleCompressedAccountsBody {
            types::PostGetMultipleCompressedAccountsBody {
                id: types::PostGetMultipleCompressedAccountsBodyId::TestAccount,
                jsonrpc: types::PostGetMultipleCompressedAccountsBodyJsonrpc::_20,
                method: types::PostGetMultipleCompressedAccountsBodyMethod::GetMultipleCompressedAccounts,
                params,
            }
        }

        pub fn make_get_validity_proof_body(
            params: types::PostGetValidityProofBodyParams,
        ) -> types::PostGetValidityProofBody {
            types::PostGetValidityProofBody {
                id: types::PostGetValidityProofBodyId::TestAccount,
                jsonrpc: types::PostGetValidityProofBodyJsonrpc::_20,
                method: types::PostGetValidityProofBodyMethod::GetValidityProof,
                params,
            }
        }

        pub fn make_get_queue_elements_body(
            params: types::PostGetQueueElementsBodyParams,
        ) -> types::PostGetQueueElementsBody {
            types::PostGetQueueElementsBody {
                id: types::PostGetQueueElementsBodyId::TestAccount,
                jsonrpc: types::PostGetQueueElementsBodyJsonrpc::_20,
                method: types::PostGetQueueElementsBodyMethod::GetQueueElements,
                params,
            }
        }

        pub fn make_get_queue_info_body(
            params: types::PostGetQueueInfoBodyParams,
        ) -> types::PostGetQueueInfoBody {
            types::PostGetQueueInfoBody {
                id: types::PostGetQueueInfoBodyId::TestAccount,
                jsonrpc: types::PostGetQueueInfoBodyJsonrpc::_20,
                method: types::PostGetQueueInfoBodyMethod::GetQueueInfo,
                params,
            }
        }

        pub fn make_get_account_interface_body(
            params: types::PostGetAccountInterfaceBodyParams,
        ) -> types::PostGetAccountInterfaceBody {
            types::PostGetAccountInterfaceBody {
                id: types::PostGetAccountInterfaceBodyId::TestAccount,
                jsonrpc: types::PostGetAccountInterfaceBodyJsonrpc::_20,
                method: types::PostGetAccountInterfaceBodyMethod::GetAccountInterface,
                params,
            }
        }

        pub fn make_get_token_account_interface_body(
            params: types::PostGetTokenAccountInterfaceBodyParams,
        ) -> types::PostGetTokenAccountInterfaceBody {
            types::PostGetTokenAccountInterfaceBody {
                id: types::PostGetTokenAccountInterfaceBodyId::TestAccount,
                jsonrpc: types::PostGetTokenAccountInterfaceBodyJsonrpc::_20,
                method: types::PostGetTokenAccountInterfaceBodyMethod::GetTokenAccountInterface,
                params,
            }
        }

        pub fn make_get_ata_interface_body(
            params: types::PostGetAtaInterfaceBodyParams,
        ) -> types::PostGetAtaInterfaceBody {
            types::PostGetAtaInterfaceBody {
                id: types::PostGetAtaInterfaceBodyId::TestAccount,
                jsonrpc: types::PostGetAtaInterfaceBodyJsonrpc::_20,
                method: types::PostGetAtaInterfaceBodyMethod::GetAtaInterface,
                params,
            }
        }

        pub fn make_get_multiple_account_interfaces_body(
            params: types::PostGetMultipleAccountInterfacesBodyParams,
        ) -> types::PostGetMultipleAccountInterfacesBody {
            types::PostGetMultipleAccountInterfacesBody {
                id: types::PostGetMultipleAccountInterfacesBodyId::TestAccount,
                jsonrpc: types::PostGetMultipleAccountInterfacesBodyJsonrpc::_20,
                method:
                    types::PostGetMultipleAccountInterfacesBodyMethod::GetMultipleAccountInterfaces,
                params,
            }
        }

        // Macro to reduce boilerplate for API calls
        macro_rules! api_call {
            ($fn_name:ident, $client_method:ident, $body_type:ty, $response_type:ty) => {
                pub async fn $fn_name(
                    configuration: &Configuration,
                    body: $body_type,
                ) -> Result<$response_type, Error<$response_type>> {
                    let client = configuration.to_client();
                    let response =
                        client
                            .$client_method()
                            .body(body)
                            .send()
                            .await
                            .map_err(|e| match e {
                                progenitor_client::Error::InvalidRequest(msg) => {
                                    Error::Serde(serde_json::Error::io(std::io::Error::other(msg)))
                                }
                                progenitor_client::Error::CommunicationError(e) => {
                                    Error::Reqwest(e)
                                }
                                progenitor_client::Error::ErrorResponse(rv) => {
                                    Error::ResponseError(ResponseContent {
                                        status: rv.status(),
                                        content: format!("{:?}", rv.into_inner()),
                                        entity: None,
                                    })
                                }
                                progenitor_client::Error::InvalidResponsePayload(_, e) => {
                                    Error::Serde(serde_json::Error::io(std::io::Error::other(
                                        e.to_string(),
                                    )))
                                }
                                progenitor_client::Error::UnexpectedResponse(resp) => {
                                    Error::ResponseError(ResponseContent {
                                        status: resp.status(),
                                        content: "Unexpected response".to_string(),
                                        entity: None,
                                    })
                                }
                                progenitor_client::Error::ResponseBodyError(e) => Error::Reqwest(e),
                                progenitor_client::Error::InvalidUpgrade(e) => Error::Reqwest(e),
                                progenitor_client::Error::PreHookError(msg) => {
                                    Error::Serde(serde_json::Error::io(std::io::Error::other(msg)))
                                }
                                progenitor_client::Error::PostHookError(msg) => {
                                    Error::Serde(serde_json::Error::io(std::io::Error::other(msg)))
                                }
                            })?;
                    Ok(response.into_inner())
                }
            };
        }

        api_call!(
            get_compressed_account_post,
            post_get_compressed_account,
            types::PostGetCompressedAccountBody,
            types::PostGetCompressedAccountResponse
        );

        api_call!(
            get_compressed_account_balance_post,
            post_get_compressed_account_balance,
            types::PostGetCompressedAccountBalanceBody,
            types::PostGetCompressedAccountBalanceResponse
        );

        api_call!(
            get_compressed_accounts_by_owner_post,
            post_get_compressed_accounts_by_owner,
            types::PostGetCompressedAccountsByOwnerBody,
            types::PostGetCompressedAccountsByOwnerResponse
        );

        api_call!(
            get_compressed_accounts_by_owner_v2_post,
            post_get_compressed_accounts_by_owner_v2,
            types::PostGetCompressedAccountsByOwnerV2Body,
            types::PostGetCompressedAccountsByOwnerV2Response
        );

        api_call!(
            get_compressed_balance_by_owner_post,
            post_get_compressed_balance_by_owner,
            types::PostGetCompressedBalanceByOwnerBody,
            types::PostGetCompressedBalanceByOwnerResponse
        );

        api_call!(
            get_compressed_mint_token_holders_post,
            post_get_compressed_mint_token_holders,
            types::PostGetCompressedMintTokenHoldersBody,
            types::PostGetCompressedMintTokenHoldersResponse
        );

        api_call!(
            get_compressed_token_account_balance_post,
            post_get_compressed_token_account_balance,
            types::PostGetCompressedTokenAccountBalanceBody,
            types::PostGetCompressedTokenAccountBalanceResponse
        );

        api_call!(
            get_compressed_token_accounts_by_delegate_post,
            post_get_compressed_token_accounts_by_delegate,
            types::PostGetCompressedTokenAccountsByDelegateBody,
            types::PostGetCompressedTokenAccountsByDelegateResponse
        );

        api_call!(
            get_compressed_token_accounts_by_delegate_v2_post,
            post_get_compressed_token_accounts_by_delegate_v2,
            types::PostGetCompressedTokenAccountsByDelegateV2Body,
            types::PostGetCompressedTokenAccountsByDelegateV2Response
        );

        api_call!(
            get_compressed_token_accounts_by_owner_post,
            post_get_compressed_token_accounts_by_owner,
            types::PostGetCompressedTokenAccountsByOwnerBody,
            types::PostGetCompressedTokenAccountsByOwnerResponse
        );

        api_call!(
            get_compressed_token_accounts_by_owner_v2_post,
            post_get_compressed_token_accounts_by_owner_v2,
            types::PostGetCompressedTokenAccountsByOwnerV2Body,
            types::PostGetCompressedTokenAccountsByOwnerV2Response
        );

        api_call!(
            get_compressed_token_balances_by_owner_post,
            post_get_compressed_token_balances_by_owner,
            types::PostGetCompressedTokenBalancesByOwnerBody,
            types::PostGetCompressedTokenBalancesByOwnerResponse
        );

        api_call!(
            get_compressed_token_balances_by_owner_v2_post,
            post_get_compressed_token_balances_by_owner_v2,
            types::PostGetCompressedTokenBalancesByOwnerV2Body,
            types::PostGetCompressedTokenBalancesByOwnerV2Response
        );

        api_call!(
            get_compression_signatures_for_account_post,
            post_get_compression_signatures_for_account,
            types::PostGetCompressionSignaturesForAccountBody,
            types::PostGetCompressionSignaturesForAccountResponse
        );

        api_call!(
            get_compression_signatures_for_address_post,
            post_get_compression_signatures_for_address,
            types::PostGetCompressionSignaturesForAddressBody,
            types::PostGetCompressionSignaturesForAddressResponse
        );

        api_call!(
            get_compression_signatures_for_owner_post,
            post_get_compression_signatures_for_owner,
            types::PostGetCompressionSignaturesForOwnerBody,
            types::PostGetCompressionSignaturesForOwnerResponse
        );

        api_call!(
            get_compression_signatures_for_token_owner_post,
            post_get_compression_signatures_for_token_owner,
            types::PostGetCompressionSignaturesForTokenOwnerBody,
            types::PostGetCompressionSignaturesForTokenOwnerResponse
        );

        api_call!(
            get_indexer_health_post,
            post_get_indexer_health,
            types::PostGetIndexerHealthBody,
            types::PostGetIndexerHealthResponse
        );

        api_call!(
            get_indexer_slot_post,
            post_get_indexer_slot,
            types::PostGetIndexerSlotBody,
            types::PostGetIndexerSlotResponse
        );

        api_call!(
            get_multiple_compressed_account_proofs_post,
            post_get_multiple_compressed_account_proofs,
            types::PostGetMultipleCompressedAccountProofsBody,
            types::PostGetMultipleCompressedAccountProofsResponse
        );

        api_call!(
            get_multiple_compressed_accounts_post,
            post_get_multiple_compressed_accounts,
            types::PostGetMultipleCompressedAccountsBody,
            types::PostGetMultipleCompressedAccountsResponse
        );

        api_call!(
            get_multiple_new_address_proofs_v2_post,
            post_get_multiple_new_address_proofs_v2,
            types::PostGetMultipleNewAddressProofsV2Body,
            types::PostGetMultipleNewAddressProofsV2Response
        );

        api_call!(
            get_validity_proof_post,
            post_get_validity_proof,
            types::PostGetValidityProofBody,
            types::PostGetValidityProofResponse
        );

        api_call!(
            get_validity_proof_v2_post,
            post_get_validity_proof_v2,
            types::PostGetValidityProofV2Body,
            types::PostGetValidityProofV2Response
        );

        api_call!(
            get_queue_elements_post,
            post_get_queue_elements,
            types::PostGetQueueElementsBody,
            types::PostGetQueueElementsResponse
        );

        api_call!(
            get_queue_info_post,
            post_get_queue_info,
            types::PostGetQueueInfoBody,
            types::PostGetQueueInfoResponse
        );

        api_call!(
            get_account_interface_post,
            post_get_account_interface,
            types::PostGetAccountInterfaceBody,
            types::PostGetAccountInterfaceResponse
        );

        api_call!(
            get_token_account_interface_post,
            post_get_token_account_interface,
            types::PostGetTokenAccountInterfaceBody,
            types::PostGetTokenAccountInterfaceResponse
        );

        api_call!(
            get_ata_interface_post,
            post_get_ata_interface,
            types::PostGetAtaInterfaceBody,
            types::PostGetAtaInterfaceResponse
        );

        api_call!(
            get_multiple_account_interfaces_post,
            post_get_multiple_account_interfaces,
            types::PostGetMultipleAccountInterfacesBody,
            types::PostGetMultipleAccountInterfacesResponse
        );
    }
}
