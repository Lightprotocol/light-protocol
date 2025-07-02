use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{borsh_mut::DeserializeMut, ZeroCopyMut};

#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopyMut)]
pub struct TestStruct {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
}

fn main() {
    let test_struct = TestStruct {
        a: 42,
        b: 1337,
        vec: vec![1, 2, 3, 4, 5],
    };
    
    let mut bytes = test_struct.try_to_vec().unwrap();
    println!("Serialized bytes length: {}", bytes.len());
    println!("Calculated byte_len: {}", test_struct.byte_len());
    
    // Test mutable zero-copy
    let (mut zero_copy_mut, remaining) = TestStruct::zero_copy_at_mut(&mut bytes).unwrap();
    println!("Mutable a value: {}", zero_copy_mut.a);
    println!("Mutable b value: {}", zero_copy_mut.b.into());
    println!("Remaining bytes: {:?}", remaining);
    
    // Modify the values
    zero_copy_mut.a = 100;
    zero_copy_mut.b = 200u16.into();
    
    // Verify the changes
    let deserialized = TestStruct::try_from_slice(&bytes).unwrap();
    println!("Modified struct: {:?}", deserialized);
}