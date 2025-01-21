#![no_std]

pub mod cyclic_vec;
pub mod errors;
pub mod slice_mut;
pub mod vec;
use core::mem::{align_of, size_of};

#[cfg(feature = "std")]
extern crate std;

pub fn add_padding<LEN, T>(offset: &mut usize) {
    let padding = align_of::<T>().saturating_sub(size_of::<LEN>());
    *offset += padding;
}
