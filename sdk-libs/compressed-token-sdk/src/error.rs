use thiserror::Error;

/// Error type for the Light Compressed Token SDK
#[derive(Debug, Error)]
pub enum CTokenSdkError {
    #[error("Insufficient balance")]
    InsufficientBalance,
}
