// Edge case: usize and isize types

use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SizeTypes {
    pub unsigned_size: usize,
    pub signed_size: isize,
    pub vec_usize: Vec<usize>,
    pub opt_isize: Option<isize>,
    pub array_usize: [usize; 8],
}

fn main() {}
