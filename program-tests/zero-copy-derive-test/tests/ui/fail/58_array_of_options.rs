// Edge case: Array containing Options

use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ArrayOfOptions {
    pub maybe_values: [Option<u32>; 10],
    pub maybe_flags: [Option<bool>; 8],
    pub maybe_nums: [Option<u64>; 4],
}

fn main() {}
