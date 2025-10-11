use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ComplexMutable {
    pub primitives: u64,
    pub vector_data: Vec<u32>,
    pub optional_value: Option<u16>,
    pub fixed_buffer: [u8; 16],
    pub flag: bool,
}

fn main() {}