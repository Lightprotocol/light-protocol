use core::mem::size_of;
use std::vec::Vec;

use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, KnownLayout, Ref,
};

/// Trait for calculating the byte length of a type when serialized.
/// This is needed for variable-sized types like Vec and Option where the size depends on the actual data.
pub trait ByteLen {
    fn byte_len(&self) -> usize;
}

// Implement ByteLen for primitive types
impl ByteLen for u8 {
    fn byte_len(&self) -> usize {
        size_of::<u8>()
    }
}

impl ByteLen for u16 {
    fn byte_len(&self) -> usize {
        size_of::<u16>()
    }
}

impl ByteLen for u32 {
    fn byte_len(&self) -> usize {
        size_of::<u32>()
    }
}

impl ByteLen for u64 {
    fn byte_len(&self) -> usize {
        size_of::<u64>()
    }
}

impl ByteLen for bool {
    fn byte_len(&self) -> usize {
        size_of::<u8>() // booleans are serialized as u8
    }
}

// Implement ByteLen for zerocopy little-endian types
impl ByteLen for U16 {
    fn byte_len(&self) -> usize {
        size_of::<U16>()
    }
}

impl ByteLen for U32 {
    fn byte_len(&self) -> usize {
        size_of::<U32>()
    }
}

impl ByteLen for U64 {
    fn byte_len(&self) -> usize {
        size_of::<U64>()
    }
}

// Implement ByteLen for fixed-size array types
impl<T: KnownLayout + Immutable + FromBytes, const N: usize> ByteLen for [T; N] {
    fn byte_len(&self) -> usize {
        size_of::<Self>()
    }
}

// Implement ByteLen for Ref types
impl<T: KnownLayout + Immutable + FromBytes> ByteLen for Ref<&[u8], T> {
    fn byte_len(&self) -> usize {
        size_of::<T>()
    }
}

impl<T: KnownLayout + Immutable + FromBytes> ByteLen for Ref<&mut [u8], T> {
    fn byte_len(&self) -> usize {
        size_of::<T>()
    }
}

// Implement ByteLen for Vec<T>
impl<T: ByteLen> ByteLen for Vec<T> {
    fn byte_len(&self) -> usize {
        // 4 bytes for length prefix + sum of byte_len for each element
        4 + self.iter().map(|t| t.byte_len()).sum::<usize>()
    }
}

// Implement ByteLen for Option<T>
impl<T: ByteLen> ByteLen for Option<T> {
    fn byte_len(&self) -> usize {
        if let Some(value) = self.as_ref() {
            // 1 byte for discriminator + value's byte_len
            1 + value.byte_len()
        } else {
            // Just 1 byte for None discriminator
            1
        }
    }
}

// Implement ByteLen for solana_pubkey::Pubkey
#[cfg(feature = "solana")]
impl ByteLen for solana_pubkey::Pubkey {
    fn byte_len(&self) -> usize {
        32 // Pubkey is always 32 bytes
    }
}