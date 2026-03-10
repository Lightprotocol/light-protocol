use std::sync::LazyLock;

use light_client::rpc::Rpc;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;
use tracing::warn;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

use crate::{
    errors::ForesterError,
    helius_priority_fee_types::{
        GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest,
        GetPriorityFeeEstimateResponse, RpcRequest,
    },
    Result,
};

const DEFAULT_PRIORITY_FEE_MICROLAMPORTS: u64 = 10_000;

#[derive(Debug, Clone, Copy, Default)]
pub struct PriorityFeeConfig {
    pub compute_unit_price: Option<u64>,
    pub enable_priority_fees: bool,
}

impl PriorityFeeConfig {
    pub async fn resolve<R: Rpc>(&self, rpc: &R, account_keys: Vec<Pubkey>) -> Result<Option<u64>> {
        if !self.enable_priority_fees {
            return Ok(self.compute_unit_price);
        }

        let rpc_url = rpc.get_url();
        let url = reqwest::Url::parse(&rpc_url).map_err(|e| ForesterError::General {
            error: format!(
                "Invalid RPC URL for priority fee resolution: {} ({})",
                rpc_url, e
            ),
        })?;

        match request_priority_fee_estimate(&url, account_keys).await {
            Ok(priority_fee) => Ok(Some(priority_fee)),
            Err(error) => {
                if let Some(priority_fee_error) = error.downcast_ref::<PriorityFeeEstimateError>() {
                    if priority_fee_error.is_unsupported() {
                        let fallback_fee = self.fallback_priority_fee();
                        warn!(
                            rpc_url = %rpc_url,
                            fallback_fee,
                            error = %priority_fee_error,
                            "Priority fee estimation unsupported by RPC; falling back to configured/default CU price"
                        );
                        return Ok(Some(fallback_fee));
                    }
                }

                Err(error)
            }
        }
    }

    fn fallback_priority_fee(&self) -> u64 {
        self.compute_unit_price
            .unwrap_or(DEFAULT_PRIORITY_FEE_MICROLAMPORTS)
    }
}

#[derive(Debug, Error)]
enum PriorityFeeEstimateError {
    #[error("priority fee estimate RPC error {code}: {message}")]
    Rpc { code: i64, message: String },

    #[error("priority fee estimate method unsupported{code_suffix}: {message}")]
    UnsupportedMethod {
        code: Option<i64>,
        code_suffix: String,
        message: String,
    },

    #[error("priority fee estimate not available")]
    MissingEstimate,

    #[error("priority fee estimate invalid: {0}")]
    InvalidEstimate(String),

    #[error("priority fee estimate request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("priority fee estimate response parse failed: {0}")]
    Parse(#[from] serde_json::Error),
}

impl PriorityFeeEstimateError {
    fn unsupported(code: Option<i64>, message: String) -> Self {
        let code_suffix = code
            .map(|value| format!(" (code {value})"))
            .unwrap_or_default();
        Self::UnsupportedMethod {
            code,
            code_suffix,
            message,
        }
    }

    fn is_unsupported(&self) -> bool {
        matches!(self, Self::UnsupportedMethod { .. })
    }
}

#[derive(Debug, Deserialize)]
struct PriorityFeeEstimateRpcResponse {
    result: Option<GetPriorityFeeEstimateResponse>,
    error: Option<PriorityFeeEstimateRpcError>,
}

#[derive(Debug, Deserialize)]
struct PriorityFeeEstimateRpcError {
    code: i64,
    message: String,
}

fn parse_priority_fee_estimate_response(
    response_text: &str,
) -> std::result::Result<u64, PriorityFeeEstimateError> {
    let response: PriorityFeeEstimateRpcResponse = serde_json::from_str(response_text)?;

    if let Some(result) = response.result {
        let priority_fee_estimate = result
            .priority_fee_estimate
            .ok_or(PriorityFeeEstimateError::MissingEstimate)?;

        if !priority_fee_estimate.is_finite()
            || priority_fee_estimate < 0.0
            || priority_fee_estimate > u64::MAX as f64
        {
            return Err(PriorityFeeEstimateError::InvalidEstimate(format!(
                "{priority_fee_estimate}"
            )));
        }

        return Ok(priority_fee_estimate as u64);
    }

    if let Some(error) = response.error {
        if priority_fee_method_is_unsupported(error.code, &error.message) {
            return Err(PriorityFeeEstimateError::unsupported(
                Some(error.code),
                error.message,
            ));
        }

        return Err(PriorityFeeEstimateError::Rpc {
            code: error.code,
            message: error.message,
        });
    }

    Err(PriorityFeeEstimateError::MissingEstimate)
}

fn priority_fee_method_is_unsupported(code: i64, message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    (code == -32601 && message.contains("method not found"))
        || message.contains("unsupported method")
        || (message.contains("method ") && message.contains(" not supported"))
}

/// Request priority fee estimate from Helius RPC endpoint.
pub async fn request_priority_fee_estimate(
    url: &reqwest::Url,
    account_keys: Vec<Pubkey>,
) -> Result<u64> {
    let priority_fee_request = GetPriorityFeeEstimateRequest {
        transaction: None,
        account_keys: Some(
            account_keys
                .iter()
                .map(|pubkey| bs58::encode(pubkey).into_string())
                .collect(),
        ),
        options: Some(GetPriorityFeeEstimateOptions {
            include_all_priority_fee_levels: None,
            recommended: Some(true),
            include_vote: None,
            lookback_slots: None,
            priority_level: None,
            transaction_encoding: None,
        }),
    };

    let rpc_request = RpcRequest::new(
        "getPriorityFeeEstimate".to_string(),
        serde_json::json!({
            "get_priority_fee_estimate_request": priority_fee_request
        }),
    );

    let response = HTTP_CLIENT
        .post(url.clone())
        .header("Content-Type", "application/json")
        .json(&rpc_request)
        .send()
        .await?;

    let response_text = response.text().await?;

    parse_priority_fee_estimate_response(&response_text).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_priority_fee_estimate_response, PriorityFeeConfig, PriorityFeeEstimateError,
        DEFAULT_PRIORITY_FEE_MICROLAMPORTS,
    };

    #[test]
    fn parses_priority_fee_estimate_success_response() {
        let response = r#"{
            "jsonrpc":"2.0",
            "id":"1",
            "result":{"priorityFeeEstimate":12345.0}
        }"#;

        assert_eq!(
            parse_priority_fee_estimate_response(response).unwrap(),
            12_345
        );
    }

    #[test]
    fn detects_unsupported_priority_fee_method_response() {
        let response = r#"{
            "jsonrpc":"2.0",
            "id":"1",
            "error":{"code":-32601,"message":"Method not found"}
        }"#;

        let error = parse_priority_fee_estimate_response(response).unwrap_err();
        assert!(matches!(
            error,
            PriorityFeeEstimateError::UnsupportedMethod {
                code: Some(-32601),
                ..
            }
        ));
    }

    #[test]
    fn preserves_non_unsupported_priority_fee_rpc_errors() {
        let response = r#"{
            "jsonrpc":"2.0",
            "id":"1",
            "error":{"code":-32000,"message":"upstream overloaded"}
        }"#;

        let error = parse_priority_fee_estimate_response(response).unwrap_err();
        assert!(matches!(
            error,
            PriorityFeeEstimateError::Rpc { code: -32000, .. }
        ));
    }

    #[test]
    fn rejects_negative_priority_fee_estimates() {
        let response = r#"{
            "jsonrpc":"2.0",
            "id":"1",
            "result":{"priorityFeeEstimate":-1.0}
        }"#;

        let error = parse_priority_fee_estimate_response(response).unwrap_err();
        assert!(matches!(
            error,
            PriorityFeeEstimateError::InvalidEstimate(_)
        ));
    }

    #[test]
    fn detects_explicitly_unsupported_priority_fee_messages() {
        let response = r#"{
            "jsonrpc":"2.0",
            "id":"1",
            "error":{"code":-32000,"message":"Method getPriorityFeeEstimate not supported by this provider"}
        }"#;

        let error = parse_priority_fee_estimate_response(response).unwrap_err();
        assert!(matches!(
            error,
            PriorityFeeEstimateError::UnsupportedMethod { .. }
        ));
    }

    #[test]
    fn does_not_misclassify_generic_unsupported_errors() {
        let response = r#"{
            "jsonrpc":"2.0",
            "id":"1",
            "error":{"code":-32000,"message":"unsupported transaction version"}
        }"#;

        let error = parse_priority_fee_estimate_response(response).unwrap_err();
        assert!(matches!(
            error,
            PriorityFeeEstimateError::Rpc { code: -32000, .. }
        ));
    }

    #[test]
    fn fallback_fee_prefers_fixed_price_when_present() {
        let config = PriorityFeeConfig {
            compute_unit_price: Some(42_000),
            enable_priority_fees: true,
        };

        assert_eq!(config.fallback_priority_fee(), 42_000);
    }

    #[test]
    fn default_fallback_fee_matches_legacy_default() {
        let config = PriorityFeeConfig {
            compute_unit_price: None,
            enable_priority_fees: true,
        };

        assert_eq!(
            config.fallback_priority_fee(),
            DEFAULT_PRIORITY_FEE_MICROLAMPORTS
        );
    }
}
