#![no_std]

pub mod cyclic_vec;
pub mod errors;
#[cfg(feature = "std")]
pub mod num_trait;
pub mod slice;
pub mod slice_mut;
pub mod vec;
use core::mem::{align_of, size_of};
#[cfg(feature = "std")]
pub mod borsh;
#[cfg(feature = "std")]
pub mod borsh_mut;
#[cfg(feature = "derive")]
pub use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq};
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
pub trait ZeroCopyTraits: Copy + KnownLayout + Immutable + FromBytes + IntoBytes {}

impl<T> ZeroCopyTraits for T where T: Copy + KnownLayout + Immutable + FromBytes + IntoBytes {}
