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
pub struct GetCompressedAccountBalancePost200ResponseResult {
    #[serde(rename = "context")]
    pub context: Box<models::Context>,
    #[serde(rename = "value")]
    pub value: u64,
}

impl GetCompressedAccountBalancePost200ResponseResult {
    pub fn new(
        context: models::Context,
        value: u64,
    ) -> GetCompressedAccountBalancePost200ResponseResult {
        GetCompressedAccountBalancePost200ResponseResult {
            context: Box::new(context),
            value,
        }
    }
}
