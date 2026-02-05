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
pub struct GetAccountInterfacePost200Response {
    #[serde(rename = "error", skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<models::GetBatchAddressUpdateInfoPost200ResponseError>>,
    /// An ID to identify the response.
    #[serde(rename = "id")]
    pub id: String,
    /// The version of the JSON-RPC protocol.
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    #[serde(rename = "result", skip_serializing_if = "Option::is_none")]
    pub result: Option<Box<models::GetAccountInterfacePost200ResponseResult>>,
}

impl GetAccountInterfacePost200Response {
    pub fn new(id: String, jsonrpc: String) -> Self {
        Self {
            error: None,
            id,
            jsonrpc,
            result: None,
        }
    }
}
