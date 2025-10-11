// Edge case: Very large struct (25+ fields)
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<u8> fields
#[repr(C)]
pub struct LargeStruct {
    pub field_001: u32,
    pub field_002: u32,
    pub field_003: u32,
    pub field_004: u32,
    pub field_005: u32,
    pub field_006: u32,
    pub field_007: u32,
    pub field_008: u32,
    pub field_009: u32,
    pub field_010: u32,
    pub field_011: u32,
    pub field_012: u32,
    pub field_013: u32,
    pub field_014: u32,
    pub field_015: u32,
    pub field_016: u32,
    pub field_017: u32,
    pub field_018: u32,
    pub field_019: u32,
    pub field_020: u32,
    pub field_021: Vec<u8>,
    pub field_022: Option<u64>,
    pub field_023: u32,
    pub field_024: u32,
    pub field_025: u32,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = LargeStruct {
        field_001: 1,
        field_002: 2,
        field_003: 3,
        field_004: 4,
        field_005: 5,
        field_006: 6,
        field_007: 7,
        field_008: 8,
        field_009: 9,
        field_010: 10,
        field_011: 11,
        field_012: 12,
        field_013: 13,
        field_014: 14,
        field_015: 15,
        field_016: 16,
        field_017: 17,
        field_018: 18,
        field_019: 19,
        field_020: 20,
        field_021: vec![1, 2, 3],
        field_022: Some(22),
        field_023: 23,
        field_024: 24,
        field_025: 25,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = LargeStruct::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = LargeStruct::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = LargeStructConfig {
        field_021: 3,
        field_022: true,
    };
    let byte_len = LargeStruct::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        LargeStruct::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.field_001 = 1.into();
    struct_copy_mut.field_002 = 2.into();
    struct_copy_mut.field_003 = 3.into();
    struct_copy_mut.field_004 = 4.into();
    struct_copy_mut.field_005 = 5.into();
    struct_copy_mut.field_006 = 6.into();
    struct_copy_mut.field_007 = 7.into();
    struct_copy_mut.field_008 = 8.into();
    struct_copy_mut.field_009 = 9.into();
    struct_copy_mut.field_010 = 10.into();
    struct_copy_mut.field_011 = 11.into();
    struct_copy_mut.field_012 = 12.into();
    struct_copy_mut.field_013 = 13.into();
    struct_copy_mut.field_014 = 14.into();
    struct_copy_mut.field_015 = 15.into();
    struct_copy_mut.field_016 = 16.into();
    struct_copy_mut.field_017 = 17.into();
    struct_copy_mut.field_018 = 18.into();
    struct_copy_mut.field_019 = 19.into();
    struct_copy_mut.field_020 = 20.into();
    struct_copy_mut.field_021[0] = 1;
    struct_copy_mut.field_021[1] = 2;
    struct_copy_mut.field_021[2] = 3;
    if let Some(ref mut val) = struct_copy_mut.field_022 {
        **val = 22.into();
    }
    *struct_copy_mut.field_023 = 23u32.into();
    *struct_copy_mut.field_024 = 24u32.into();
    *struct_copy_mut.field_025 = 25u32.into();
    assert_eq!(new_bytes, bytes);
}
