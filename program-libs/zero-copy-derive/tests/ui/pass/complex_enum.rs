use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum ComplexEnum {
    UnitVariant,
    U64Field(u64), 
    BoolField(bool),
    U32Field(u32),
}

fn main() {}