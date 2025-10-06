// Edge case: Option containing Vec

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for complex Option<Vec> fields
#[repr(C)]
pub struct OptionVec {
    pub maybe_data: Option<Vec<u8>>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = OptionVec {
        maybe_data: Some(vec![1, 2, 3, 4, 5]),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = OptionVec::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option<Vec>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = OptionVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = OptionVecConfig {
        maybe_data: (true, vec![(); 5]),
    };
    let byte_len = OptionVec::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        OptionVec::new_zero_copy(&mut new_bytes, config).unwrap();
    // set Option<Vec> field values
    if let Some(ref mut data) = struct_copy_mut.maybe_data {
        *data[0] = 1;
        *data[1] = 2;
        *data[2] = 3;
        *data[3] = 4;
        *data[4] = 5;
    }
    assert_eq!(new_bytes, bytes);
}
