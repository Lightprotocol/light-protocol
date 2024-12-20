use bytemuck::{Pod, Zeroable};
use light_batched_merkle_tree::zero_copy::{bytes_to_struct_unchecked, ZeroCopyError};
use light_hasher::Discriminator;

/// Tests:
/// 1. functional init
/// 2. failing init again
/// 3. functional deserialize
/// 4. failing deserialize invalid data
/// 5. failing deserialize invalid discriminator
#[test]
fn test_bytes_to_struct() {
    #[repr(C)]
    #[derive(Debug, PartialEq, Copy, Clone, Pod, Zeroable)]
    pub struct MyStruct {
        pub data: u64,
    }
    impl Discriminator for MyStruct {
        const DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    }
    let mut bytes = vec![0; 8 + std::mem::size_of::<MyStruct>()];
    let mut empty_bytes = vec![0; 8 + std::mem::size_of::<MyStruct>()];

    // Test 1 functional init.
    let inited_struct = bytes_to_struct_unchecked::<MyStruct>(&mut bytes).unwrap();
    unsafe {
        (*inited_struct).data = 1;
    }
    assert_eq!(bytes[0..8], MyStruct::DISCRIMINATOR);
    assert_eq!(bytes[8..].to_vec(), vec![1, 0, 0, 0, 0, 0, 0, 0]);
    // Test 2 failing init again.
    assert_eq!(
        bytes_to_struct_unchecked::<MyStruct>(&mut bytes).unwrap_err(),
        ZeroCopyError::InvalidDiscriminator.into()
    );

    // Test 3 functional deserialize.
    let inited_struct = unsafe { *bytes_to_struct_unchecked::<MyStruct>(&mut bytes).unwrap() };
    assert_eq!(inited_struct, MyStruct { data: 1 });
    // Test 4 failing deserialize invalid data.
    assert_eq!(
        bytes_to_struct_unchecked::<MyStruct>(&mut empty_bytes).unwrap_err(),
        ZeroCopyError::InvalidDiscriminator.into()
    );
    // Test 5 failing deserialize invalid discriminator.
    bytes[0] = 0;
    assert_eq!(
        bytes_to_struct_unchecked::<MyStruct>(&mut bytes).unwrap_err(),
        ZeroCopyError::InvalidDiscriminator.into()
    );
}
