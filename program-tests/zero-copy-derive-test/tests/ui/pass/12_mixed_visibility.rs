// Edge case: Mixed field visibility

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<u8> fields
#[repr(C)]
pub struct MixedVisibility {
    pub public_field: u32,
    pub(crate) crate_field: u64,
    private_field: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MixedVisibility {
        public_field: 100,
        crate_field: 200,
        private_field: vec![1, 2, 3],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = MixedVisibility::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = MixedVisibility::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = MixedVisibilityConfig { private_field: 3 };
    let byte_len = MixedVisibility::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        MixedVisibility::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.public_field = 100.into();
    struct_copy_mut.crate_field = 200.into();
    struct_copy_mut.private_field[0] = 1;
    struct_copy_mut.private_field[1] = 2;
    struct_copy_mut.private_field[2] = 3;
    assert_eq!(new_bytes, bytes);
}
