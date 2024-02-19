#[cfg(not(target_os = "solana"))]
use light_poseidon::PoseidonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PoseidonSyscallError {
    #[error("Invalid parameters.")]
    InvalidParameters,
    #[error("Invalid endianness.")]
    InvalidEndianness,
    #[error("Invalid number of inputs. Maximum allowed is 12.")]
    InvalidNumberOfInputs,
    #[error("Input is an empty slice.")]
    EmptyInput,
    #[error(
        "Invalid length of the input. The length matching the modulus of the prime field is 32."
    )]
    InvalidInputLength,
    #[error("Failed to convert bytest into a prime field element.")]
    BytesToPrimeFieldElement,
    #[error("Input is larger than the modulus of the prime field.")]
    InputLargerThanModulus,
    #[error("Failed to convert a vector of bytes into an array.")]
    VecToArray,
    #[error("Failed to convert the number of inputs from u64 to u8.")]
    U64Tou8,
    #[error("Failed to convert bytes to BigInt")]
    BytesToBigInt,
    #[error("Invalid width. Choose a width between 2 and 16 for 1 to 15 inputs.")]
    InvalidWidthCircom,
    #[error("Unexpected error")]
    Unexpected,
}

impl From<u64> for PoseidonSyscallError {
    fn from(error: u64) -> Self {
        match error {
            1 => PoseidonSyscallError::InvalidParameters,
            2 => PoseidonSyscallError::InvalidEndianness,
            3 => PoseidonSyscallError::InvalidNumberOfInputs,
            4 => PoseidonSyscallError::EmptyInput,
            5 => PoseidonSyscallError::InvalidInputLength,
            6 => PoseidonSyscallError::BytesToPrimeFieldElement,
            7 => PoseidonSyscallError::InputLargerThanModulus,
            8 => PoseidonSyscallError::VecToArray,
            9 => PoseidonSyscallError::U64Tou8,
            10 => PoseidonSyscallError::BytesToBigInt,
            11 => PoseidonSyscallError::InvalidWidthCircom,
            _ => PoseidonSyscallError::Unexpected,
        }
    }
}

impl From<PoseidonSyscallError> for u64 {
    fn from(error: PoseidonSyscallError) -> Self {
        match error {
            PoseidonSyscallError::InvalidParameters => 2001,
            PoseidonSyscallError::InvalidEndianness => 2002,
            PoseidonSyscallError::InvalidNumberOfInputs => 2003,
            PoseidonSyscallError::EmptyInput => 2004,
            PoseidonSyscallError::InvalidInputLength => 2005,
            PoseidonSyscallError::BytesToPrimeFieldElement => 2006,
            PoseidonSyscallError::InputLargerThanModulus => 2007,
            PoseidonSyscallError::VecToArray => 2008,
            PoseidonSyscallError::U64Tou8 => 2009,
            PoseidonSyscallError::BytesToBigInt => 2010,
            PoseidonSyscallError::InvalidWidthCircom => 2011,
            PoseidonSyscallError::Unexpected => 2012,
        }
    }
}

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
            HasherError::IntegerOverflow => 1001,
            #[cfg(not(target_os = "solana"))]
            HasherError::Poseidon(_) => 1002,
            #[cfg(target_os = "solana")]
            HasherError::PoseidonSyscall(e) => (u64::from(e)).try_into().unwrap_or(1001),
            HasherError::UnknownSolanaSyscall(e) => e.try_into().unwrap_or(1001),
        }
    }
}

#[cfg(feature = "solana")]
impl From<HasherError> for solana_program::program_error::ProgramError {
    fn from(e: HasherError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}
