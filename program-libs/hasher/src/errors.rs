use light_poseidon::PoseidonError;
use thiserror::Error;

use crate::poseidon::PoseidonSyscallError;

#[derive(Debug, Error, PartialEq)]
pub enum HasherError {
    #[error("Integer overflow, value too large")]
    IntegerOverflow,
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
        "Option hash to field size returned [0u8;32] a collision with None for an Option type."
    )]
    OptionHashToFieldSizeZero,
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand. `num_derive::ToPrimitive`
// doesn't support data-carrying enums.
impl From<HasherError> for u32 {
    fn from(e: HasherError) -> u32 {
        match e {
            HasherError::IntegerOverflow => 7001,
            HasherError::Poseidon(_) => 7002,
            HasherError::PoseidonSyscall(e) => (u64::from(e)).try_into().unwrap_or(7003),
            HasherError::UnknownSolanaSyscall(e) => e.try_into().unwrap_or(7004),
            HasherError::InvalidInputLength(_, _) => 7005,
            HasherError::InvalidNumFields => 7006,
            HasherError::EmptyInput => 7007,
            HasherError::BorshError => 7008,
            HasherError::OptionHashToFieldSizeZero => 7009,
        }
    }
}

#[cfg(any(feature = "solana", feature = "pinocchio"))]
impl From<HasherError> for crate::ProgramError {
    fn from(e: HasherError) -> Self {
        crate::ProgramError::Custom(e.into())
    }
}
