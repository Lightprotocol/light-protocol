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
pub struct GetQueueElementsPost200ResponseResult {
    #[serde(rename = "context")]
    pub context: Box<models::Context>,
    #[serde(rename = "value")]
    pub value: Vec<models::MerkleProofWithContextV2>,
}

impl GetQueueElementsPost200ResponseResult {
    pub fn new(
        context: models::Context,
        value: Vec<models::MerkleProofWithContextV2>,
    ) -> GetQueueElementsPost200ResponseResult {
        GetQueueElementsPost200ResponseResult {
            context: Box::new(context),
            value,
        }
    }
}
