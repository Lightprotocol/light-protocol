use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum BasicEnum {
    UnitVariant,
    SingleField(u32),
    AnotherField(u64),
}

fn main() {}