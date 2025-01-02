use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ZeroCopyError {
    #[error("The vector is full, cannot push any new elements")]
    Full,
    #[error("Requested array of size {0}, but the vector has {1} elements")]
    ArraySize(usize, usize),
    #[error("The requested start index is out of bounds.")]
    IterFromOutOfBounds,
    #[error("Memory allocated {0}, Memory required {0}")]
    InsufficientMemoryAllocated(usize, usize),
    #[error("Invalid Account size.")]
    InvalidAccountSize,
    #[error("Unaligned pointer.")]
    UnalignedPointer,
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for u32 {
    fn from(e: ZeroCopyError) -> u32 {
        match e {
            ZeroCopyError::Full => 15001,
            ZeroCopyError::ArraySize(_, _) => 15002,
            ZeroCopyError::IterFromOutOfBounds => 15003,
            ZeroCopyError::InsufficientMemoryAllocated(_, _) => 15004,
            ZeroCopyError::InvalidAccountSize => 15005,
            ZeroCopyError::UnalignedPointer => 15006,
        }
    }
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for solana_program::program_error::ProgramError {
    fn from(e: ZeroCopyError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}
