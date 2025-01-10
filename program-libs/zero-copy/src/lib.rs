pub mod cyclic_vec;
pub mod errors;
pub mod slice_mut;
pub mod vec;

use std::mem::{align_of, size_of};

pub fn add_padding<LEN, T>(offset: &mut usize) {
    let padding = align_of::<T>().saturating_sub(size_of::<LEN>());
    *offset += padding;
}
