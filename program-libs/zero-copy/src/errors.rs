use core::convert::Infallible;

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
    #[error("Unaligned pointer.")]
    UnalignedPointer,
    #[error("Memory not zeroed.")]
    MemoryNotZeroed,
    #[error("InvalidConversion.")]
    InvalidConversion,
    #[error("Invalid data {0}.")]
    InvalidData(Infallible),
    #[error("Invalid size.")]
    Size,
    #[error("Invalid option byte {0} must be 0 (None) or 1 (Some).")]
    InvalidOptionByte(u8),
    #[error("Invalid capacity. Capacity must be greater than 0.")]
    InvalidCapacity,
    #[error("Length is greater than capacity.")]
    LengthGreaterThanCapacity,
    #[error("Current index is greater than length.")]
    CurrentIndexGreaterThanLength,
}

impl From<ZeroCopyError> for u32 {
    fn from(e: ZeroCopyError) -> u32 {
        match e {
            ZeroCopyError::Full => 15001,
            ZeroCopyError::ArraySize(_, _) => 15002,
            ZeroCopyError::IterFromOutOfBounds => 15003,
            ZeroCopyError::InsufficientMemoryAllocated(_, _) => 15004,
            ZeroCopyError::UnalignedPointer => 15006,
            ZeroCopyError::MemoryNotZeroed => 15007,
            ZeroCopyError::InvalidConversion => 15008,
            ZeroCopyError::InvalidData(_) => 15009,
            ZeroCopyError::Size => 15010,
            ZeroCopyError::InvalidOptionByte(_) => 15011,
            ZeroCopyError::InvalidCapacity => 15012,
            ZeroCopyError::LengthGreaterThanCapacity => 15013,
            ZeroCopyError::CurrentIndexGreaterThanLength => 15014,
        }
    }
}

#[cfg(feature = "pinocchio")]
impl From<ZeroCopyError> for pinocchio::program_error::ProgramError {
    fn from(e: ZeroCopyError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for solana_program::program_error::ProgramError {
    fn from(e: ZeroCopyError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

impl<Src, Dst: ?Sized>
    From<
        zerocopy::ConvertError<
            zerocopy::AlignmentError<Src, Dst>,
            zerocopy::SizeError<Src, Dst>,
            core::convert::Infallible,
        >,
    > for ZeroCopyError
{
    fn from(
        err: zerocopy::ConvertError<
            zerocopy::AlignmentError<Src, Dst>,
            zerocopy::SizeError<Src, Dst>,
            core::convert::Infallible,
        >,
    ) -> Self {
        match err {
            zerocopy::ConvertError::Alignment(_) => ZeroCopyError::UnalignedPointer,
            zerocopy::ConvertError::Size(_) => ZeroCopyError::Size,
            zerocopy::ConvertError::Validity(i) => ZeroCopyError::InvalidData(i),
        }
    }
}
