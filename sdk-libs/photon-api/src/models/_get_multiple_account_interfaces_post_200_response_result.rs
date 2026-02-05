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
pub struct GetMultipleAccountInterfacesPost200ResponseResult {
    #[serde(rename = "context")]
    pub context: Box<models::Context>,
    /// List of typed results (Some for found accounts, None for not found)
    #[serde(rename = "value")]
    pub value: Vec<Option<models::InterfaceResult>>,
}

impl GetMultipleAccountInterfacesPost200ResponseResult {
    pub fn new(context: models::Context, value: Vec<Option<models::InterfaceResult>>) -> Self {
        Self {
            context: Box::new(context),
            value,
        }
    }
}
