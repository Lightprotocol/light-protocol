#[cfg(not(target_os = "solana"))]
use light_poseidon::PoseidonError;
#[cfg(target_os = "solana")]
use solana_program::poseidon::PoseidonSyscallError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HasherError {
    #[error("Integer overflow, value too large")]
    IntegerOverflow,
    #[cfg(not(target_os = "solana"))]
    #[error("Poseidon hasher error: {0}")]
    Poseidon(#[from] PoseidonError),
    #[cfg(target_os = "solana")]
    #[error("Poseidon syscall error: {0}")]
    PoseidonSyscall(#[from] PoseidonSyscallError),
    #[error("Unknown Solana syscall error: {0}")]
    UnknownSolanaSyscall(u64),
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand. `num_derive::ToPrimitive`
// doesn't support data-carrying enums.
#[cfg(feature = "solana")]
impl From<HasherError> for u32 {
    fn from(e: HasherError) -> u32 {
        match e {
            HasherError::IntegerOverflow => 7001,
            #[cfg(not(target_os = "solana"))]
            HasherError::Poseidon(_) => 7002,
            #[cfg(target_os = "solana")]
            HasherError::PoseidonSyscall(e) => (u64::from(e)).try_into().unwrap_or(7003),
            HasherError::UnknownSolanaSyscall(e) => e.try_into().unwrap_or(7004),
        }
    }
}

#[cfg(feature = "solana")]
impl From<HasherError> for solana_program::program_error::ProgramError {
    fn from(e: HasherError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}
