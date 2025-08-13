use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub struct BasicStruct {
    pub field1: u32,
    pub field2: u64,
    pub field3: bool,
}

fn main() {}