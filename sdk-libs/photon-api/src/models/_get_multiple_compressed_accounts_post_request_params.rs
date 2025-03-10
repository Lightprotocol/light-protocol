/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::models;

/// GetMultipleCompressedAccountsPostRequestParams : Request for compressed account data
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetMultipleCompressedAccountsPostRequestParams {
    #[serde(rename = "addresses", default, skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<String>>,
    #[serde(rename = "hashes", default, skip_serializing_if = "Option::is_none")]
    pub hashes: Option<Vec<String>>,
}

impl GetMultipleCompressedAccountsPostRequestParams {
    /// Request for compressed account data
    pub fn new() -> GetMultipleCompressedAccountsPostRequestParams {
        GetMultipleCompressedAccountsPostRequestParams {
            addresses: None,
            hashes: None,
        }
    }
}
