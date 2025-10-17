#[cfg(feature = "poseidon")]
use light_poseidon::PoseidonError;
use thiserror::Error;

use crate::poseidon::PoseidonSyscallError;

#[derive(Debug, Error, PartialEq)]
pub enum HasherError {
    #[error("Integer overflow, value too large")]
    IntegerOverflow,
    #[cfg(feature = "poseidon")]
    #[error("Poseidon hasher error: {0}")]
    Poseidon(#[from] PoseidonError),
    #[error("Poseidon syscall error: {0}")]
    PoseidonSyscall(#[from] PoseidonSyscallError),
    #[error("Unknown Solana syscall error: {0}")]
    UnknownSolanaSyscall(u64),
    #[error("Allowed input length {0} provided {1}")]
    InvalidInputLength(usize, usize),
    #[error("Invalid number of fields")]
    InvalidNumFields,
    #[error("Empty input")]
    EmptyInput,
    #[error("Borsh serialization failed.")]
    BorshError,
    #[error(
        "Option hash to field size returned [0u8;32], a collision with None for an Option type."
    )]
    OptionHashToFieldSizeZero,
    #[error("Poseidon feature is not enabled. Without feature poseidon only syscalls are accessible in target os solana")]
    PoseidonFeatureNotEnabled,
    #[error("SHA256 feature is not enabled. Enable the sha256 feature to use SHA256 hashing in non-Solana targets")]
    Sha256FeatureNotEnabled,
    #[error("Keccak feature is not enabled. Enable the keccak feature to use Keccak hashing in non-Solana targets")]
    KeccakFeatureNotEnabled,
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand. `num_derive::ToPrimitive`
// doesn't support data-carrying enums.
impl From<HasherError> for u32 {
    fn from(e: HasherError) -> u32 {
        match e {
            HasherError::IntegerOverflow => 7001,
            #[cfg(feature = "poseidon")]
            HasherError::Poseidon(_) => 7002,
            HasherError::PoseidonSyscall(e) => (u64::from(e)).try_into().unwrap_or(7003),
            HasherError::UnknownSolanaSyscall(e) => e.try_into().unwrap_or(7004),
            HasherError::InvalidInputLength(_, _) => 7005,
            HasherError::InvalidNumFields => 7006,
            HasherError::EmptyInput => 7007,
            HasherError::BorshError => 7008,
            HasherError::OptionHashToFieldSizeZero => 7009,
            HasherError::PoseidonFeatureNotEnabled => 7010,
            HasherError::Sha256FeatureNotEnabled => 7011,
            HasherError::KeccakFeatureNotEnabled => 7012,
        }
    }
}

#[cfg(feature = "solana")]
impl From<HasherError> for solana_program_error::ProgramError {
    fn from(e: HasherError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<HasherError> for pinocchio::program_error::ProgramError {
    fn from(e: HasherError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}
