use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MicroLamportPriorityFeeLevels {
    pub min: f64,
    pub low: f64,
    pub medium: f64,
    pub high: f64,
    #[serde(rename = "veryHigh")]
    pub very_high: f64,
    #[serde(rename = "unsafeMax")]
    pub unsafe_max: f64,
}

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
    // #[serde(rename = "priorityFeeLevels")]
    // pub priority_fee_levels: Option<MicroLamportPriorityFeeLevels>,
}

pub struct Timeout {
    pub duration: Duration,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
        }
    }
}

impl From<Timeout> for Duration {
    fn from(val: Timeout) -> Self {
        val.duration
    }
}
