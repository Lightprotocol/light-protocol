/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

use crate::models;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetAtaInterfacePost200ResponseResult {
    #[serde(rename = "context")]
    pub context: Box<models::Context>,
    #[serde(rename = "value", skip_serializing_if = "Option::is_none")]
    pub value: Option<Box<models::TokenAccountInterface>>,
}

impl GetAtaInterfacePost200ResponseResult {
    pub fn new(context: models::Context) -> Self {
        Self {
            context: Box::new(context),
            value: None,
        }
    }
}
