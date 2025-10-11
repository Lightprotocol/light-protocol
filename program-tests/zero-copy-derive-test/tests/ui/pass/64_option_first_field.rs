// Edge case: Struct starting with Option (affects meta boundary)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct OptionFirstField {
    pub optional: Option<u32>, // Meta stops here
    pub value: u64,
    pub more: u32,
    pub data: Vec<u8>,
}

fn main() {
    let original = OptionFirstField {
        optional: Some(42),
        value: 123456789,
        more: 987654321,
        data: vec![1, 2, 3, 4, 5],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();
    let _deserialized: OptionFirstField = OptionFirstField::try_from_slice(&serialized).unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = OptionFirstField::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = OptionFirstField::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // assert byte len
    let config = OptionFirstFieldConfig {
        optional: true,
        data: 5,
    };
    let byte_len = OptionFirstField::byte_len(&config).unwrap();
    assert_eq!(serialized.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        OptionFirstField::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    if let Some(ref mut val) = struct_copy_mut.optional {
        **val = 42.into();
    }
    *struct_copy_mut.value = 123456789.into();
    *struct_copy_mut.more = 987654321.into();
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    struct_copy_mut.data[3] = 4;
    struct_copy_mut.data[4] = 5;
    assert_eq!(new_bytes, serialized);

    // Note: Cannot use assert_eq! due to Vec fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for OptionFirstField");
}
