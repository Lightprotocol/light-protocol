// adapted from https://github.com/helius-labs/helius-rust-sdk/blob/dev/src/types/types.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum PriorityLevel {
    Min,
    Low,
    Medium,
    High,
    VeryHigh,
    UnsafeMax,
    Default,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UiTransactionEncoding {
    Binary,
    Base64,
    Base58,
    Json,
    JsonParsed,
}
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct RpcRequest<T> {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    #[serde(rename = "params")]
    pub parameters: T,
}

impl<T> RpcRequest<T> {
    pub fn new(method: String, parameters: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method,
            parameters,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: String,
    pub result: T,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetPriorityFeeEstimateOptions {
    pub priority_level: Option<PriorityLevel>,
    pub include_all_priority_fee_levels: Option<bool>,
    pub transaction_encoding: Option<UiTransactionEncoding>,
    pub lookback_slots: Option<u8>,
    pub recommended: Option<bool>,
    pub include_vote: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GetPriorityFeeEstimateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
    #[serde(rename = "accountKeys", skip_serializing_if = "Option::is_none")]
    pub account_keys: Option<Vec<String>>,
    pub options: Option<GetPriorityFeeEstimateOptions>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GetPriorityFeeEstimateResponse {
    #[serde(rename = "priorityFeeEstimate")]
    pub priority_fee_estimate: Option<f64>,
}
