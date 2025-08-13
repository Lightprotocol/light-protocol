use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub struct WithVectors {
    pub data: Vec<u8>,
    pub numbers: Vec<u32>,
    pub flags: Vec<bool>,
}

fn main() {}