// Edge case: Mixed field visibility
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};
use borsh::{BorshSerialize, BorshDeserialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

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
}
