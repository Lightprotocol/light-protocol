use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
pub union UnsupportedUnion {
    pub a: u32,
    pub b: i32,
}

fn main() {}