/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.45.0
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::models;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountWithOptionalTokenData {
    #[serde(rename = "account")]
    pub account: Box<models::Account>,
    #[serde(rename = "optionalTokenData", skip_serializing_if = "Option::is_none")]
    pub optional_token_data: Option<Box<models::TokenData>>,
}

impl AccountWithOptionalTokenData {
    pub fn new(account: models::Account) -> AccountWithOptionalTokenData {
        AccountWithOptionalTokenData {
            account: Box::new(account),
            optional_token_data: None,
        }
    }
}