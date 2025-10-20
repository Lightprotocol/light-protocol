use borsh::maybestd::io::Error as BorshError;
use light_compressed_account::CompressedAccountError;
use light_zero_copy::errors::ZeroCopyError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseIndexerEventError {
    #[error("Deserialize light system program instruction error")]
    DeserializeSystemInstructionError,
    #[error("Deserialize account compression program instruction error")]
    DeserializeAccountLightSystemCpiInputsError,
    #[error("Instruction data too small {0} expected {1}")]
    InstructionDataTooSmall(usize, usize),
    #[error("Zero copy error {0}")]
    ZeroCopyError(#[from] ZeroCopyError),
    #[error("Borsh error {0}")]
    BorshError(#[from] BorshError),
    #[error("Compressed account error {0}")]
    CompressedAccountError(#[from] CompressedAccountError),
    #[error("Hasher error {0}")]
    HasherError(#[from] light_hasher::HasherError),
}
