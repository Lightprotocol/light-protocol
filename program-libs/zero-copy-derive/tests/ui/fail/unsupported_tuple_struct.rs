use light_zero_copy_derive::ZeroCopy;

// This should fail because ZeroCopy doesn't support tuple structs  
#[derive(ZeroCopy)]
pub struct UnsupportedTupleStruct(u32, u64, String);

fn main() {}