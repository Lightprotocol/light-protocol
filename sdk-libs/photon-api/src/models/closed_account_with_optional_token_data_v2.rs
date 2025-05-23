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

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClosedAccountWithOptionalTokenDataV2 {
    #[serde(rename = "account")]
    pub account: Box<models::ClosedAccountV2>,
    #[serde(rename = "optionalTokenData", skip_serializing_if = "Option::is_none")]
    pub optional_token_data: Option<Box<models::TokenData>>,
}

impl ClosedAccountWithOptionalTokenDataV2 {
    pub fn new(account: models::ClosedAccountV2) -> ClosedAccountWithOptionalTokenDataV2 {
        ClosedAccountWithOptionalTokenDataV2 {
            account: Box::new(account),
            optional_token_data: None,
        }
    }
}
