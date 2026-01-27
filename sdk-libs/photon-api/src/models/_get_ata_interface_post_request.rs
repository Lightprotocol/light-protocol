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
pub struct GetAtaInterfacePostRequest {
    /// An ID to identify the request.
    #[serde(rename = "id")]
    pub id: String,
    /// The version of the JSON-RPC protocol.
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    /// The name of the method to invoke.
    #[serde(rename = "method")]
    pub method: String,
    #[serde(rename = "params")]
    pub params: Box<models::GetAtaInterfacePostRequestParams>,
}

impl GetAtaInterfacePostRequest {
    pub fn new(params: models::GetAtaInterfacePostRequestParams) -> Self {
        Self {
            id: "test-id".to_string(),
            jsonrpc: "2.0".to_string(),
            method: "getAtaInterface".to_string(),
            params: Box::new(params),
        }
    }
}
