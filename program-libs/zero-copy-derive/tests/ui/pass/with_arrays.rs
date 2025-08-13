use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub struct WithArrays {
    pub buffer: [u8; 32],
    pub data: [u8; 16],
    pub small_array: [u8; 4],
}

fn main() {}