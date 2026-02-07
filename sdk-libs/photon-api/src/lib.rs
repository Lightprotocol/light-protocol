//! Photon API client generated from OpenAPI spec using progenitor.
//!
//! This crate provides a Rust client for the Photon indexer API.
//! Types are generated at build time by progenitor; HTTP calls use reqwest directly.

#![allow(unused_imports, clippy::all, dead_code)]
#![allow(mismatched_lifetime_syntaxes)]

// Include the generated code from build.rs (provides the `types` module).
include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub mod apis {
    use super::*;

    /// Configuration for the Photon API client.
    #[derive(Clone)]
    pub struct Configuration {
        pub base_path: String,
        pub api_key: Option<String>,
        pub client: reqwest::Client,
    }

    impl std::fmt::Debug for Configuration {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Configuration")
                .field("base_path", &self.base_path)
                .finish()
        }
    }

    impl Default for Configuration {
        fn default() -> Self {
            Self {
                base_path: "https://devnet.helius-rpc.com".to_string(),
                api_key: None,
                client: reqwest::Client::new(),
            }
        }
    }

    impl Configuration {
        /// Create a new configuration from a URL string.
        ///
        /// If the URL contains an `api-key` query parameter, it is extracted
        /// and appended to every request as `?api-key=KEY`.
        ///
        /// ```ignore
        /// // Without API key
        /// let config = Configuration::new("https://rpc.example.com".to_string());
        ///
        /// // With API key
        /// let config = Configuration::new("https://rpc.example.com?api-key=YOUR_KEY".to_string());
        /// ```
        pub fn new(url: String) -> Self {
            let (base_path, api_key) = Self::parse_url(&url);
            Self {
                base_path,
                api_key,
                client: reqwest::Client::new(),
            }
        }

        fn build_url(&self, endpoint: &str) -> String {
            match &self.api_key {
                Some(key) => format!("{}/{}?api-key={}", self.base_path, endpoint, key),
                None => format!("{}/{}", self.base_path, endpoint),
            }
        }

        pub(crate) fn parse_url(url: &str) -> (String, Option<String>) {
            if let Some(query_start) = url.find('?') {
                let base = &url[..query_start];
                let query = &url[query_start + 1..];
                for param in query.split('&') {
                    if let Some(value) = param.strip_prefix("api-key=") {
                        return (base.to_string(), Some(value.to_string()));
                    }
                }
                (url.to_string(), None)
            } else {
                (url.to_string(), None)
            }
        }
    }

    pub mod configuration {
        pub use super::Configuration;
    }

    /// Error type for API calls.
    #[derive(Debug)]
    pub enum Error<T> {
        Reqwest(reqwest::Error),
        ResponseError {
            status: u16,
            body: String,
        },
        #[doc(hidden)]
        _Phantom(std::marker::PhantomData<T>),
    }

    impl<T> std::fmt::Display for Error<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Reqwest(e) => write!(f, "HTTP error: {}", e),
                Error::ResponseError { status, body } => {
                    write!(f, "Error response (status {}): {}", status, body)
                }
                Error::_Phantom(_) => unreachable!(),
            }
        }
    }

    /// Default API module providing function-style API calls.
    pub mod default_api {
        use super::*;

        // ----------------------------------------------------------------
        // Body construction helper functions
        // ----------------------------------------------------------------

        pub fn make_get_compressed_account_body(
            params: types::PostGetCompressedAccountBodyParams,
        ) -> types::PostGetCompressedAccountBody {
            types::PostGetCompressedAccountBody {
                id: types::PostGetCompressedAccountBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedAccountBodyJsonrpc::X20,
                method: types::PostGetCompressedAccountBodyMethod::GetCompressedAccount,
                params,
            }
        }

        pub fn make_get_compressed_account_balance_body(
            params: types::PostGetCompressedAccountBalanceBodyParams,
        ) -> types::PostGetCompressedAccountBalanceBody {
            types::PostGetCompressedAccountBalanceBody {
                id: types::PostGetCompressedAccountBalanceBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedAccountBalanceBodyJsonrpc::X20,
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
                jsonrpc: types::PostGetCompressedAccountsByOwnerV2BodyJsonrpc::X20,
                method: types::PostGetCompressedAccountsByOwnerV2BodyMethod::GetCompressedAccountsByOwnerV2,
                params,
            }
        }

        pub fn make_get_compressed_balance_by_owner_body(
            params: types::PostGetCompressedBalanceByOwnerBodyParams,
        ) -> types::PostGetCompressedBalanceByOwnerBody {
            types::PostGetCompressedBalanceByOwnerBody {
                id: types::PostGetCompressedBalanceByOwnerBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedBalanceByOwnerBodyJsonrpc::X20,
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
                jsonrpc: types::PostGetCompressedMintTokenHoldersBodyJsonrpc::X20,
                method: types::PostGetCompressedMintTokenHoldersBodyMethod::GetCompressedMintTokenHolders,
                params,
            }
        }

        pub fn make_get_compressed_token_account_balance_body(
            params: types::PostGetCompressedTokenAccountBalanceBodyParams,
        ) -> types::PostGetCompressedTokenAccountBalanceBody {
            types::PostGetCompressedTokenAccountBalanceBody {
                id: types::PostGetCompressedTokenAccountBalanceBodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenAccountBalanceBodyJsonrpc::X20,
                method: types::PostGetCompressedTokenAccountBalanceBodyMethod::GetCompressedTokenAccountBalance,
                params,
            }
        }

        pub fn make_get_validity_proof_v2_body(
            params: types::PostGetValidityProofV2BodyParams,
        ) -> types::PostGetValidityProofV2Body {
            types::PostGetValidityProofV2Body {
                id: types::PostGetValidityProofV2BodyId::TestAccount,
                jsonrpc: types::PostGetValidityProofV2BodyJsonrpc::X20,
                method: types::PostGetValidityProofV2BodyMethod::GetValidityProofV2,
                params,
            }
        }

        pub fn make_get_multiple_new_address_proofs_v2_body(
            params: Vec<types::AddressWithTree>,
        ) -> types::PostGetMultipleNewAddressProofsV2Body {
            types::PostGetMultipleNewAddressProofsV2Body {
                id: types::PostGetMultipleNewAddressProofsV2BodyId::TestAccount,
                jsonrpc: types::PostGetMultipleNewAddressProofsV2BodyJsonrpc::X20,
                method: types::PostGetMultipleNewAddressProofsV2BodyMethod::GetMultipleNewAddressProofsV2,
                params,
            }
        }

        pub fn make_get_compressed_token_accounts_by_delegate_v2_body(
            params: types::PostGetCompressedTokenAccountsByDelegateV2BodyParams,
        ) -> types::PostGetCompressedTokenAccountsByDelegateV2Body {
            types::PostGetCompressedTokenAccountsByDelegateV2Body {
                id: types::PostGetCompressedTokenAccountsByDelegateV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenAccountsByDelegateV2BodyJsonrpc::X20,
                method: types::PostGetCompressedTokenAccountsByDelegateV2BodyMethod::GetCompressedTokenAccountsByDelegateV2,
                params,
            }
        }

        pub fn make_get_compressed_token_accounts_by_owner_v2_body(
            params: types::PostGetCompressedTokenAccountsByOwnerV2BodyParams,
        ) -> types::PostGetCompressedTokenAccountsByOwnerV2Body {
            types::PostGetCompressedTokenAccountsByOwnerV2Body {
                id: types::PostGetCompressedTokenAccountsByOwnerV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenAccountsByOwnerV2BodyJsonrpc::X20,
                method: types::PostGetCompressedTokenAccountsByOwnerV2BodyMethod::GetCompressedTokenAccountsByOwnerV2,
                params,
            }
        }

        pub fn make_get_compressed_token_balances_by_owner_v2_body(
            params: types::PostGetCompressedTokenBalancesByOwnerV2BodyParams,
        ) -> types::PostGetCompressedTokenBalancesByOwnerV2Body {
            types::PostGetCompressedTokenBalancesByOwnerV2Body {
                id: types::PostGetCompressedTokenBalancesByOwnerV2BodyId::TestAccount,
                jsonrpc: types::PostGetCompressedTokenBalancesByOwnerV2BodyJsonrpc::X20,
                method: types::PostGetCompressedTokenBalancesByOwnerV2BodyMethod::GetCompressedTokenBalancesByOwnerV2,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_account_body(
            params: types::PostGetCompressionSignaturesForAccountBodyParams,
        ) -> types::PostGetCompressionSignaturesForAccountBody {
            types::PostGetCompressionSignaturesForAccountBody {
                id: types::PostGetCompressionSignaturesForAccountBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForAccountBodyJsonrpc::X20,
                method: types::PostGetCompressionSignaturesForAccountBodyMethod::GetCompressionSignaturesForAccount,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_address_body(
            params: types::PostGetCompressionSignaturesForAddressBodyParams,
        ) -> types::PostGetCompressionSignaturesForAddressBody {
            types::PostGetCompressionSignaturesForAddressBody {
                id: types::PostGetCompressionSignaturesForAddressBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForAddressBodyJsonrpc::X20,
                method: types::PostGetCompressionSignaturesForAddressBodyMethod::GetCompressionSignaturesForAddress,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_owner_body(
            params: types::PostGetCompressionSignaturesForOwnerBodyParams,
        ) -> types::PostGetCompressionSignaturesForOwnerBody {
            types::PostGetCompressionSignaturesForOwnerBody {
                id: types::PostGetCompressionSignaturesForOwnerBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForOwnerBodyJsonrpc::X20,
                method: types::PostGetCompressionSignaturesForOwnerBodyMethod::GetCompressionSignaturesForOwner,
                params,
            }
        }

        pub fn make_get_compression_signatures_for_token_owner_body(
            params: types::PostGetCompressionSignaturesForTokenOwnerBodyParams,
        ) -> types::PostGetCompressionSignaturesForTokenOwnerBody {
            types::PostGetCompressionSignaturesForTokenOwnerBody {
                id: types::PostGetCompressionSignaturesForTokenOwnerBodyId::TestAccount,
                jsonrpc: types::PostGetCompressionSignaturesForTokenOwnerBodyJsonrpc::X20,
                method: types::PostGetCompressionSignaturesForTokenOwnerBodyMethod::GetCompressionSignaturesForTokenOwner,
                params,
            }
        }

        pub fn make_get_indexer_health_body() -> types::PostGetIndexerHealthBody {
            types::PostGetIndexerHealthBody {
                id: types::PostGetIndexerHealthBodyId::TestAccount,
                jsonrpc: types::PostGetIndexerHealthBodyJsonrpc::X20,
                method: types::PostGetIndexerHealthBodyMethod::GetIndexerHealth,
            }
        }

        pub fn make_get_indexer_slot_body() -> types::PostGetIndexerSlotBody {
            types::PostGetIndexerSlotBody {
                id: types::PostGetIndexerSlotBodyId::TestAccount,
                jsonrpc: types::PostGetIndexerSlotBodyJsonrpc::X20,
                method: types::PostGetIndexerSlotBodyMethod::GetIndexerSlot,
            }
        }

        pub fn make_get_multiple_compressed_account_proofs_body(
            params: Vec<types::Hash>,
        ) -> types::PostGetMultipleCompressedAccountProofsBody {
            types::PostGetMultipleCompressedAccountProofsBody {
                id: types::PostGetMultipleCompressedAccountProofsBodyId::TestAccount,
                jsonrpc: types::PostGetMultipleCompressedAccountProofsBodyJsonrpc::X20,
                method: types::PostGetMultipleCompressedAccountProofsBodyMethod::GetMultipleCompressedAccountProofs,
                params,
            }
        }

        pub fn make_get_multiple_compressed_accounts_body(
            params: types::PostGetMultipleCompressedAccountsBodyParams,
        ) -> types::PostGetMultipleCompressedAccountsBody {
            types::PostGetMultipleCompressedAccountsBody {
                id: types::PostGetMultipleCompressedAccountsBodyId::TestAccount,
                jsonrpc: types::PostGetMultipleCompressedAccountsBodyJsonrpc::X20,
                method: types::PostGetMultipleCompressedAccountsBodyMethod::GetMultipleCompressedAccounts,
                params,
            }
        }

        pub fn make_get_validity_proof_body(
            params: types::PostGetValidityProofBodyParams,
        ) -> types::PostGetValidityProofBody {
            types::PostGetValidityProofBody {
                id: types::PostGetValidityProofBodyId::TestAccount,
                jsonrpc: types::PostGetValidityProofBodyJsonrpc::X20,
                method: types::PostGetValidityProofBodyMethod::GetValidityProof,
                params,
            }
        }

        pub fn make_get_queue_elements_body(
            params: types::PostGetQueueElementsBodyParams,
        ) -> types::PostGetQueueElementsBody {
            types::PostGetQueueElementsBody {
                id: types::PostGetQueueElementsBodyId::TestAccount,
                jsonrpc: types::PostGetQueueElementsBodyJsonrpc::X20,
                method: types::PostGetQueueElementsBodyMethod::GetQueueElements,
                params,
            }
        }

        pub fn make_get_queue_info_body(
            params: types::PostGetQueueInfoBodyParams,
        ) -> types::PostGetQueueInfoBody {
            types::PostGetQueueInfoBody {
                id: types::PostGetQueueInfoBodyId::TestAccount,
                jsonrpc: types::PostGetQueueInfoBodyJsonrpc::X20,
                method: types::PostGetQueueInfoBodyMethod::GetQueueInfo,
                params,
            }
        }

        pub fn make_get_account_interface_body(
            params: types::PostGetAccountInterfaceBodyParams,
        ) -> types::PostGetAccountInterfaceBody {
            types::PostGetAccountInterfaceBody {
                id: types::PostGetAccountInterfaceBodyId::TestAccount,
                jsonrpc: types::PostGetAccountInterfaceBodyJsonrpc::X20,
                method: types::PostGetAccountInterfaceBodyMethod::GetAccountInterface,
                params,
            }
        }

        pub fn make_get_token_account_interface_body(
            params: types::PostGetTokenAccountInterfaceBodyParams,
        ) -> types::PostGetTokenAccountInterfaceBody {
            types::PostGetTokenAccountInterfaceBody {
                id: types::PostGetTokenAccountInterfaceBodyId::TestAccount,
                jsonrpc: types::PostGetTokenAccountInterfaceBodyJsonrpc::X20,
                method: types::PostGetTokenAccountInterfaceBodyMethod::GetTokenAccountInterface,
                params,
            }
        }

        pub fn make_get_ata_interface_body(
            params: types::PostGetAtaInterfaceBodyParams,
        ) -> types::PostGetAtaInterfaceBody {
            types::PostGetAtaInterfaceBody {
                id: types::PostGetAtaInterfaceBodyId::TestAccount,
                jsonrpc: types::PostGetAtaInterfaceBodyJsonrpc::X20,
                method: types::PostGetAtaInterfaceBodyMethod::GetAtaInterface,
                params,
            }
        }

        pub fn make_get_multiple_account_interfaces_body(
            params: types::PostGetMultipleAccountInterfacesBodyParams,
        ) -> types::PostGetMultipleAccountInterfacesBody {
            types::PostGetMultipleAccountInterfacesBody {
                id: types::PostGetMultipleAccountInterfacesBodyId::TestAccount,
                jsonrpc: types::PostGetMultipleAccountInterfacesBodyJsonrpc::X20,
                method:
                    types::PostGetMultipleAccountInterfacesBodyMethod::GetMultipleAccountInterfaces,
                params,
            }
        }

        // ----------------------------------------------------------------
        // API call functions — direct reqwest, progenitor types for serde
        // ----------------------------------------------------------------

        macro_rules! api_call {
            ($fn_name:ident, $endpoint:expr, $body_type:ty, $response_type:ty) => {
                pub async fn $fn_name(
                    configuration: &Configuration,
                    body: $body_type,
                ) -> Result<$response_type, Error<$response_type>> {
                    let url = configuration.build_url($endpoint);
                    let response = configuration
                        .client
                        .post(&url)
                        .header(reqwest::header::ACCEPT, "application/json")
                        .json(&body)
                        .send()
                        .await
                        .map_err(Error::Reqwest)?;

                    let status = response.status().as_u16();
                    if status == 200 {
                        response
                            .json::<$response_type>()
                            .await
                            .map_err(Error::Reqwest)
                    } else {
                        let body = response.text().await.unwrap_or_default();
                        Err(Error::ResponseError { status, body })
                    }
                }
            };
        }

        api_call!(
            get_compressed_account_post,
            "getCompressedAccount",
            types::PostGetCompressedAccountBody,
            types::PostGetCompressedAccountResponse
        );
        api_call!(
            get_compressed_account_balance_post,
            "getCompressedAccountBalance",
            types::PostGetCompressedAccountBalanceBody,
            types::PostGetCompressedAccountBalanceResponse
        );
        api_call!(
            get_compressed_accounts_by_owner_post,
            "getCompressedAccountsByOwner",
            types::PostGetCompressedAccountsByOwnerBody,
            types::PostGetCompressedAccountsByOwnerResponse
        );
        api_call!(
            get_compressed_accounts_by_owner_v2_post,
            "getCompressedAccountsByOwnerV2",
            types::PostGetCompressedAccountsByOwnerV2Body,
            types::PostGetCompressedAccountsByOwnerV2Response
        );
        api_call!(
            get_compressed_balance_by_owner_post,
            "getCompressedBalanceByOwner",
            types::PostGetCompressedBalanceByOwnerBody,
            types::PostGetCompressedBalanceByOwnerResponse
        );
        api_call!(
            get_compressed_mint_token_holders_post,
            "getCompressedMintTokenHolders",
            types::PostGetCompressedMintTokenHoldersBody,
            types::PostGetCompressedMintTokenHoldersResponse
        );
        api_call!(
            get_compressed_token_account_balance_post,
            "getCompressedTokenAccountBalance",
            types::PostGetCompressedTokenAccountBalanceBody,
            types::PostGetCompressedTokenAccountBalanceResponse
        );
        api_call!(
            get_compressed_token_accounts_by_delegate_post,
            "getCompressedTokenAccountsByDelegate",
            types::PostGetCompressedTokenAccountsByDelegateBody,
            types::PostGetCompressedTokenAccountsByDelegateResponse
        );
        api_call!(
            get_compressed_token_accounts_by_delegate_v2_post,
            "getCompressedTokenAccountsByDelegateV2",
            types::PostGetCompressedTokenAccountsByDelegateV2Body,
            types::PostGetCompressedTokenAccountsByDelegateV2Response
        );
        api_call!(
            get_compressed_token_accounts_by_owner_post,
            "getCompressedTokenAccountsByOwner",
            types::PostGetCompressedTokenAccountsByOwnerBody,
            types::PostGetCompressedTokenAccountsByOwnerResponse
        );
        api_call!(
            get_compressed_token_accounts_by_owner_v2_post,
            "getCompressedTokenAccountsByOwnerV2",
            types::PostGetCompressedTokenAccountsByOwnerV2Body,
            types::PostGetCompressedTokenAccountsByOwnerV2Response
        );
        api_call!(
            get_compressed_token_balances_by_owner_post,
            "getCompressedTokenBalancesByOwner",
            types::PostGetCompressedTokenBalancesByOwnerBody,
            types::PostGetCompressedTokenBalancesByOwnerResponse
        );
        api_call!(
            get_compressed_token_balances_by_owner_v2_post,
            "getCompressedTokenBalancesByOwnerV2",
            types::PostGetCompressedTokenBalancesByOwnerV2Body,
            types::PostGetCompressedTokenBalancesByOwnerV2Response
        );
        api_call!(
            get_compression_signatures_for_account_post,
            "getCompressionSignaturesForAccount",
            types::PostGetCompressionSignaturesForAccountBody,
            types::PostGetCompressionSignaturesForAccountResponse
        );
        api_call!(
            get_compression_signatures_for_address_post,
            "getCompressionSignaturesForAddress",
            types::PostGetCompressionSignaturesForAddressBody,
            types::PostGetCompressionSignaturesForAddressResponse
        );
        api_call!(
            get_compression_signatures_for_owner_post,
            "getCompressionSignaturesForOwner",
            types::PostGetCompressionSignaturesForOwnerBody,
            types::PostGetCompressionSignaturesForOwnerResponse
        );
        api_call!(
            get_compression_signatures_for_token_owner_post,
            "getCompressionSignaturesForTokenOwner",
            types::PostGetCompressionSignaturesForTokenOwnerBody,
            types::PostGetCompressionSignaturesForTokenOwnerResponse
        );
        api_call!(
            get_indexer_health_post,
            "getIndexerHealth",
            types::PostGetIndexerHealthBody,
            types::PostGetIndexerHealthResponse
        );
        api_call!(
            get_indexer_slot_post,
            "getIndexerSlot",
            types::PostGetIndexerSlotBody,
            types::PostGetIndexerSlotResponse
        );
        api_call!(
            get_multiple_compressed_account_proofs_post,
            "getMultipleCompressedAccountProofs",
            types::PostGetMultipleCompressedAccountProofsBody,
            types::PostGetMultipleCompressedAccountProofsResponse
        );
        api_call!(
            get_multiple_compressed_accounts_post,
            "getMultipleCompressedAccounts",
            types::PostGetMultipleCompressedAccountsBody,
            types::PostGetMultipleCompressedAccountsResponse
        );
        api_call!(
            get_multiple_new_address_proofs_v2_post,
            "getMultipleNewAddressProofsV2",
            types::PostGetMultipleNewAddressProofsV2Body,
            types::PostGetMultipleNewAddressProofsV2Response
        );
        api_call!(
            get_validity_proof_post,
            "getValidityProof",
            types::PostGetValidityProofBody,
            types::PostGetValidityProofResponse
        );
        api_call!(
            get_validity_proof_v2_post,
            "getValidityProofV2",
            types::PostGetValidityProofV2Body,
            types::PostGetValidityProofV2Response
        );
        api_call!(
            get_queue_elements_post,
            "getQueueElements",
            types::PostGetQueueElementsBody,
            types::PostGetQueueElementsResponse
        );
        api_call!(
            get_queue_info_post,
            "getQueueInfo",
            types::PostGetQueueInfoBody,
            types::PostGetQueueInfoResponse
        );
        api_call!(
            get_account_interface_post,
            "getAccountInterface",
            types::PostGetAccountInterfaceBody,
            types::PostGetAccountInterfaceResponse
        );
        api_call!(
            get_token_account_interface_post,
            "getTokenAccountInterface",
            types::PostGetTokenAccountInterfaceBody,
            types::PostGetTokenAccountInterfaceResponse
        );
        api_call!(
            get_ata_interface_post,
            "getAtaInterface",
            types::PostGetAtaInterfaceBody,
            types::PostGetAtaInterfaceResponse
        );
        api_call!(
            get_multiple_account_interfaces_post,
            "getMultipleAccountInterfaces",
            types::PostGetMultipleAccountInterfacesBody,
            types::PostGetMultipleAccountInterfacesResponse
        );
    }
}

#[cfg(test)]
mod tests {
    use super::apis::{configuration::Configuration, default_api};

    #[test]
    fn test_parse_url_with_api_key() {
        let (base, key) = Configuration::parse_url("https://rpc.example.com?api-key=MY_KEY");
        assert_eq!(base, "https://rpc.example.com");
        assert_eq!(key, Some("MY_KEY".to_string()));
    }

    #[test]
    fn test_parse_url_without_api_key() {
        let (base, key) = Configuration::parse_url("https://rpc.example.com");
        assert_eq!(base, "https://rpc.example.com");
        assert_eq!(key, None);
    }

    #[test]
    fn test_parse_url_with_other_query_params() {
        let (base, key) =
            Configuration::parse_url("https://rpc.example.com?other=value&api-key=KEY123");
        assert_eq!(base, "https://rpc.example.com");
        assert_eq!(key, Some("KEY123".to_string()));
    }

    #[test]
    fn test_new_with_api_key_in_url() {
        let config = Configuration::new("https://rpc.example.com?api-key=SECRET".to_string());
        assert_eq!(config.base_path, "https://rpc.example.com");
        assert_eq!(config.api_key, Some("SECRET".to_string()));
    }

    #[test]
    fn test_make_get_compressed_account_body() {
        let params = super::types::PostGetCompressedAccountBodyParams {
            address: Some(super::types::SerializablePubkey(
                "11111111111111111111111111111111".to_string(),
            )),
            hash: None,
        };
        let body = default_api::make_get_compressed_account_body(params);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "getCompressedAccount");
        assert_eq!(json["id"], "test-account");
        assert_eq!(
            json["params"]["address"],
            "11111111111111111111111111111111"
        );
    }

    #[test]
    fn test_make_get_indexer_health_body() {
        let body = default_api::make_get_indexer_health_body();
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "getIndexerHealth");
    }

    #[test]
    fn test_make_get_indexer_slot_body() {
        let body = default_api::make_get_indexer_slot_body();
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "getIndexerSlot");
    }

    #[test]
    fn test_make_get_validity_proof_body() {
        let params = super::types::PostGetValidityProofBodyParams {
            hashes: vec![super::types::Hash("abc123".to_string())],
            new_addresses_with_trees: vec![],
        };
        let body = default_api::make_get_validity_proof_body(params);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "getValidityProof");
        assert_eq!(json["params"]["hashes"][0], "abc123");
    }

    #[tokio::test]
    async fn test_api_call_sends_correct_request() {
        use wiremock::{
            matchers::{header, method, path, query_param},
            Mock, MockServer, ResponseTemplate,
        };

        let mock_server = MockServer::start().await;

        let response_json = serde_json::json!({
            "jsonrpc": "2.0",
            "result": "ok",
            "id": "test-account"
        });

        Mock::given(method("POST"))
            .and(path("/getIndexerHealth"))
            .and(query_param("api-key", "TEST_KEY"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
            .mount(&mock_server)
            .await;

        let config = Configuration::new(format!("{}?api-key=TEST_KEY", mock_server.uri()));

        let body = default_api::make_get_indexer_health_body();
        let result = default_api::get_indexer_health_post(&config, body).await;

        result.expect("API call with api-key should succeed");
    }

    #[tokio::test]
    async fn test_api_call_without_api_key() {
        use wiremock::{
            matchers::{header, method, path},
            Mock, MockServer, ResponseTemplate,
        };

        let mock_server = MockServer::start().await;

        let response_json = serde_json::json!({
            "jsonrpc": "2.0",
            "result": "ok",
            "id": "test-account"
        });

        Mock::given(method("POST"))
            .and(path("/getIndexerHealth"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
            .mount(&mock_server)
            .await;

        let config = Configuration::new(mock_server.uri());

        let body = default_api::make_get_indexer_health_body();
        let result = default_api::get_indexer_health_post(&config, body).await;

        result.expect("API call without api-key should succeed");
    }

    #[tokio::test]
    async fn test_api_call_error_response() {
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/getIndexerHealth"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let config = Configuration::new(mock_server.uri());

        let body = default_api::make_get_indexer_health_body();
        let result = default_api::get_indexer_health_post(&config, body).await;

        match result {
            Err(super::apis::Error::ResponseError { status, body }) => {
                assert_eq!(status, 500);
                assert_eq!(body, "Internal Server Error");
            }
            other => panic!("Expected ResponseError, got {:?}", other),
        }
    }
}
