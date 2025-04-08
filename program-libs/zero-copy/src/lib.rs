#![no_std]

pub mod cyclic_vec;
pub mod errors;
pub mod slice;
pub mod slice_mut;
pub mod vec;
use core::mem::{align_of, size_of};
#[cfg(feature = "std")]
pub mod borsh;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[cfg(feature = "std")]
extern crate std;

pub fn add_padding<LEN, T>(offset: &mut usize) {
    let padding = align_of::<T>().saturating_sub(size_of::<LEN>());
    *offset += padding;
}
pub trait ZeroCopyTraits: Copy + KnownLayout + Immutable + FromBytes + IntoBytes {}

impl<T> ZeroCopyTraits for T where T: Copy + KnownLayout + Immutable + FromBytes + IntoBytes {}

#[cfg(not(feature = "solana"))]
use pinocchio::program_error::ProgramError;
#[cfg(feature = "solana")]
use solana_program::program_error::ProgramError;
