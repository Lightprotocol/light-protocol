use light_zero_copy_derive::ZeroCopy;

// This should fail because it's missing #[repr(C)]
#[derive(ZeroCopy)]
pub struct MissingReprC {
    pub field1: u32,
    pub field2: u64,
}

fn main() {}