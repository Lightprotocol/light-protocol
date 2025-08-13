use light_zero_copy_derive::ZeroCopyMut;

// This should fail because it's missing #[repr(C)]
#[derive(ZeroCopyMut)]
pub struct MissingReprCMut {
    pub field1: u32,
    pub field2: Vec<u8>,
}

fn main() {}