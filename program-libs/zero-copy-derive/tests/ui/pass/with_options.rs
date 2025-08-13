use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub struct WithOptions {
    pub maybe_value: Option<u64>,
    pub maybe_flag: Option<bool>,
    pub maybe_small: Option<u16>,
}

fn main() {}