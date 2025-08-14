use core::{convert::Infallible, fmt};
#[cfg(feature = "std")]
use std::error::Error;

#[derive(Debug, PartialEq)]
pub enum ZeroCopyError {
    Full,
    ArraySize(usize, usize),
    IterFromOutOfBounds,
    InsufficientMemoryAllocated(usize, usize),
    UnalignedPointer,
    MemoryNotZeroed,
    InvalidConversion,
    InvalidData(Infallible),
    Size,
    InvalidOptionByte(u8),
    InvalidCapacity,
    LengthGreaterThanCapacity,
    CurrentIndexGreaterThanLength,
    InvalidEnumValue,
    InsufficientCapacity,
    PlatformSizeOverflow,
}

impl fmt::Display for ZeroCopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZeroCopyError::Full => write!(f, "The vector is full, cannot push any new elements"),
            ZeroCopyError::ArraySize(expected, actual) => write!(
                f,
                "Requested array of size {}, but the vector has {} elements",
                expected, actual
            ),
            ZeroCopyError::IterFromOutOfBounds => {
                write!(f, "The requested start index is out of bounds")
            }
            ZeroCopyError::InsufficientMemoryAllocated(allocated, required) => write!(
                f,
                "Memory allocated {}, Memory required {}",
                allocated, required
            ),
            ZeroCopyError::UnalignedPointer => write!(f, "Unaligned pointer"),
            ZeroCopyError::MemoryNotZeroed => write!(f, "Memory not zeroed"),
            ZeroCopyError::InvalidConversion => write!(f, "Invalid conversion"),
            ZeroCopyError::InvalidData(_) => write!(f, "Invalid data"),
            ZeroCopyError::Size => write!(f, "Invalid size"),
            ZeroCopyError::InvalidOptionByte(byte) => write!(
                f,
                "Invalid option byte {} must be 0 (None) or 1 (Some)",
                byte
            ),
            ZeroCopyError::InvalidCapacity => {
                write!(f, "Invalid capacity. Capacity must be greater than 0")
            }
            ZeroCopyError::LengthGreaterThanCapacity => {
                write!(f, "Length is greater than capacity")
            }
            ZeroCopyError::CurrentIndexGreaterThanLength => {
                write!(f, "Current index is greater than length")
            }
            ZeroCopyError::InvalidEnumValue => write!(f, "Invalid enum value"),
            ZeroCopyError::InsufficientCapacity => write!(f, "Insufficient capacity for operation"),
            ZeroCopyError::PlatformSizeOverflow => write!(f, "Value too large for platform usize"),
        }
    }
}

#[cfg(feature = "std")]
impl Error for ZeroCopyError {}

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
            ZeroCopyError::InvalidEnumValue => 15015,
            ZeroCopyError::InsufficientCapacity => 15016,
            ZeroCopyError::PlatformSizeOverflow => 15017,
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
impl From<ZeroCopyError> for solana_program_error::ProgramError {
    fn from(e: ZeroCopyError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
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
