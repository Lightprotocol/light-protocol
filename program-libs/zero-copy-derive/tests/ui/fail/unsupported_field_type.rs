use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
pub struct UnsupportedFieldType {
    pub valid_field: u32,
    pub invalid_field: String, // String is not supported
}

fn main() {}