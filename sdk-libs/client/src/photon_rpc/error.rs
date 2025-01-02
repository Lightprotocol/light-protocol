use photon_api::{
    apis::{
        default_api::{
            GetCompressedAccountPostError, GetCompressedAccountProofPostError,
            GetLatestCompressionSignaturesPostError, GetMultipleCompressedAccountProofsPostError,
            GetMultipleNewAddressProofsV2PostError, GetTransactionWithCompressionInfoPostError,
        },
        Error as PhotonError,
    },
    models::GetCompressedAccountPost429Response,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PhotonClientError {
    #[error(transparent)]
    GetMultipleCompressedAccountProofsError(
        #[from] PhotonError<GetMultipleCompressedAccountProofsPostError>,
    ),
    #[error(transparent)]
    GetCompressedAccountsByOwnerError(#[from] PhotonError<GetCompressedAccountPost429Response>),
    #[error(transparent)]
    GetMultipleNewAddressProofsError(#[from] PhotonError<GetMultipleNewAddressProofsV2PostError>),
    #[error(transparent)]
    GetCompressedAccountError(#[from] PhotonError<GetCompressedAccountPostError>),
    #[error(transparent)]
    GetCompressedAccountProofError(#[from] PhotonError<GetCompressedAccountProofPostError>),
    #[error(transparent)]
    GetTransactionWithCompressionInfoError(
        #[from] PhotonError<GetTransactionWithCompressionInfoPostError>,
    ),
    #[error(transparent)]
    GetLatestCompressionSignaturesError(
        #[from] PhotonError<GetLatestCompressionSignaturesPostError>,
    ),
    #[error("Decode error: {0}")]
    DecodeError(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}
