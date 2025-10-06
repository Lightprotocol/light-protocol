// Edge case: Vec of Vec

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct VecOfVec {
    pub matrix: Vec<Vec<u8>>,
    pub rows: Vec<Vec<u32>>,
}

fn main() {
    let original = VecOfVec {
        matrix: vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]],
        rows: vec![vec![10, 20, 30], vec![40, 50, 60]],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = VecOfVec::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = VecOfVec::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // assert byte len
    let config = VecOfVecConfig {
        matrix: vec![vec![(); 3], vec![(); 3], vec![(); 3]],
        rows: vec![vec![(); 3], vec![(); 3]],
    };
    let byte_len = VecOfVec::byte_len(&config).unwrap();
    assert_eq!(serialized.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        VecOfVec::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    *struct_copy_mut.matrix[0][0] = 1;
    *struct_copy_mut.matrix[0][1] = 2;
    *struct_copy_mut.matrix[0][2] = 3;
    *struct_copy_mut.matrix[1][0] = 4;
    *struct_copy_mut.matrix[1][1] = 5;
    *struct_copy_mut.matrix[1][2] = 6;
    *struct_copy_mut.matrix[2][0] = 7;
    *struct_copy_mut.matrix[2][1] = 8;
    *struct_copy_mut.matrix[2][2] = 9;
    *struct_copy_mut.rows[0][0] = 10.into();
    *struct_copy_mut.rows[0][1] = 20.into();
    *struct_copy_mut.rows[0][2] = 30.into();
    *struct_copy_mut.rows[1][0] = 40.into();
    *struct_copy_mut.rows[1][1] = 50.into();
    *struct_copy_mut.rows[1][2] = 60.into();
    assert_eq!(new_bytes, serialized);

    // Note: Cannot use assert_eq! due to Vec fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for VecOfVec");
}
