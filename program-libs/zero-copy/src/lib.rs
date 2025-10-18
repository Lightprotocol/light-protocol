#![no_std]
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod cyclic_vec;
pub mod errors;
#[cfg(feature = "std")]
pub mod num_trait;
pub mod slice;
pub mod slice_mut;
pub mod vec;
use core::mem::{align_of, size_of};
#[cfg(feature = "alloc")]
pub mod traits;
#[cfg(all(feature = "derive", feature = "mut"))]
pub use light_zero_copy_derive::ZeroCopyMut;
#[cfg(feature = "derive")]
pub use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq};
#[cfg(feature = "alloc")]
pub use traits::ZeroCopyNew;
#[cfg(feature = "alloc")]
pub use traits::ZeroCopyStructInner;
#[cfg(feature = "derive")]
pub use zerocopy::{
    little_endian::{self, U16, U32, U64},
    Ref, Unaligned,
};
pub use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[cfg(feature = "std")]
extern crate std;

pub fn add_padding<LEN, T>(offset: &mut usize) {
    let padding = align_of::<T>().saturating_sub(size_of::<LEN>());
    *offset += padding;
}

/// Safely converts u32 to usize with platform overflow detection.
#[inline]
pub fn u32_to_usize(value: u32) -> Result<usize, errors::ZeroCopyError> {
    let result = value as usize;
    if result as u32 != value {
        return Err(errors::ZeroCopyError::PlatformSizeOverflow);
    }
    Ok(result)
}
pub trait ZeroCopyTraits: Copy + KnownLayout + Immutable + FromBytes + IntoBytes {}

impl<T> ZeroCopyTraits for T where T: Copy + KnownLayout + Immutable + FromBytes + IntoBytes {}
