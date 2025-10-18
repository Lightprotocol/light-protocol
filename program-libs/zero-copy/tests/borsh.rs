#![cfg(all(feature = "std", feature = "derive", feature = "mut"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopy, ZeroCopyEq, ZeroCopyMut,
};

#[repr(C)]
#[derive(Debug, PartialEq, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshDeserialize, BorshSerialize)]
pub struct Struct1Derived {
    pub a: u8,
    pub b: u16,
}

#[test]
fn test_struct_1_derived() {
    let ref_struct = Struct1Derived { a: 1, b: 2 };
    let mut bytes = ref_struct.try_to_vec().unwrap();

    {
        let (struct1, remaining) = Struct1Derived::zero_copy_at(&bytes).unwrap();
        assert_eq!(struct1.a, 1u8);
        assert_eq!(struct1.b, 2u16);
        assert_eq!(struct1, ref_struct);
        assert_eq!(remaining, &[]);
    }
    {
        let (mut struct1, _) = Struct1Derived::zero_copy_at_mut(&mut bytes).unwrap();
        struct1.a = 2;
        struct1.b = 3.into();
    }
    let borsh = Struct1Derived::deserialize(&mut &bytes[..]).unwrap();
    let (struct_1, _) = Struct1Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_1.a, 2); // Modified value from mutable operations
    assert_eq!(struct_1.b, 3); // Modified value from mutable operations
    assert_eq!(struct_1, borsh);
}

// Struct2 equivalent: Manual implementation that should match Struct2
#[repr(C)]
#[derive(
    Debug, PartialEq, Clone, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ZeroCopyEq,
)]
pub struct Struct2Derived {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
}

#[test]
fn test_struct_2_derived() {
    let ref_struct = Struct2Derived {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct2, remaining) = Struct2Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct2.a, 1u8);
    assert_eq!(struct2.b, 2u16);
    assert_eq!(struct2.vec.to_vec(), vec![1u8; 32]);
    assert_eq!(remaining, &[]);
    assert_eq!(struct2, ref_struct);
}

// Struct3 equivalent: fields should match Struct3
#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ZeroCopyEq)]
pub struct Struct3Derived {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
    pub c: u64,
}

#[test]
fn test_struct_3_derived() {
    let ref_struct = Struct3Derived {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
        c: 3,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = Struct3Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.vec.to_vec(), vec![1u8; 32]);
    assert_eq!(u64::from(*zero_copy.c), 3);
    assert_eq!(zero_copy, ref_struct);

    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(
    Debug, PartialEq, BorshSerialize, BorshDeserialize, Clone, ZeroCopy, ZeroCopyMut, ZeroCopyEq,
)]
pub struct Struct4NestedDerived {
    a: u8,
    b: u16,
}

#[repr(C)]
#[derive(
    Debug, PartialEq, BorshSerialize, BorshDeserialize, Clone, ZeroCopy, ZeroCopyMut, ZeroCopyEq,
)]
pub struct Struct4Derived {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
    pub c: u64,
    pub vec_2: Vec<Struct4NestedDerived>,
}

#[test]
fn test_struct_4_derived() {
    let ref_struct = Struct4Derived {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
        c: 3,
        vec_2: vec![Struct4NestedDerived { a: 1, b: 2 }; 32],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = Struct4Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.vec.to_vec(), vec![1u8; 32]);
    assert_eq!(u64::from(*zero_copy.c), 3);
    // Check vec_2 length is correct
    assert_eq!(zero_copy.vec_2.len(), 32);
    assert_eq!(zero_copy, ref_struct);
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ZeroCopyEq,
)]
pub struct Struct5Derived {
    pub a: Vec<Vec<u8>>,
}

#[test]
fn test_struct_5_derived() {
    let ref_struct = Struct5Derived {
        a: vec![vec![1u8; 32]; 32],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = Struct5Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(
        zero_copy.a.iter().map(|x| x.to_vec()).collect::<Vec<_>>(),
        vec![vec![1u8; 32]; 32]
    );
    assert_eq!(zero_copy, ref_struct);
    assert_eq!(remaining, &[]);
}

// If a struct inside a vector contains a vector it must implement Deserialize.
#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ZeroCopyEq)]
pub struct Struct6Derived {
    pub a: Vec<Struct2Derived>,
}

#[test]
fn test_struct_6_derived() {
    let ref_struct = Struct6Derived {
        a: vec![
            Struct2Derived {
                a: 1,
                b: 2,
                vec: vec![1u8; 32],
            };
            32
        ],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = Struct6Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(
        zero_copy.a.iter().collect::<Vec<_>>(),
        vec![
            &Struct2Derived {
                a: 1,
                b: 2,
                vec: vec![1u8; 32],
            };
            32
        ]
    );
    assert_eq!(zero_copy, ref_struct);
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct Struct7Derived {
    pub a: u8,
    pub b: u16,
    pub option: Option<u8>,
}

#[test]
fn test_struct_7_derived() {
    let ref_struct = Struct7Derived {
        a: 1,
        b: 2,
        option: Some(3),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = Struct7Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.option, Some(3));
    assert_eq!(remaining, &[]);

    let bytes = Struct7Derived {
        a: 1,
        b: 2,
        option: None,
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct7Derived::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.option, None);
    assert_eq!(remaining, &[]);
}

// If a struct inside a vector contains a vector it must implement Deserialize.
#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ZeroCopyEq)]
pub struct Struct8Derived {
    pub a: Vec<NestedStructDerived>,
}

#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ZeroCopyEq,
)]
pub struct NestedStructDerived {
    pub a: u8,
    pub b: Struct2Derived,
}

#[test]
fn test_struct_8_derived() {
    let ref_struct = Struct8Derived {
        a: vec![
            NestedStructDerived {
                a: 1,
                b: Struct2Derived {
                    a: 1,
                    b: 2,
                    vec: vec![1u8; 32],
                },
            };
            32
        ],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = Struct8Derived::zero_copy_at(&bytes).unwrap();
    // Check length of vec matches
    assert_eq!(zero_copy.a.len(), 32);
    assert_eq!(zero_copy, ref_struct);

    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct ArrayStruct {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

#[test]
fn test_array_struct() {
    let array_struct = ArrayStruct {
        a: [1u8; 32],
        b: [2u8; 64],
        c: [3u8; 32],
    };
    let bytes = array_struct.try_to_vec().unwrap();

    let (zero_copy, remaining) = ArrayStruct::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, [1u8; 32]);
    assert_eq!(zero_copy.b, [2u8; 64]);
    assert_eq!(zero_copy.c, [3u8; 32]);
    assert_eq!(zero_copy, array_struct);
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(
    Debug,
    PartialEq,
    Default,
    Clone,
    BorshSerialize,
    BorshDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[test]
fn test_compressed_account_data() {
    let compressed_account_data = CompressedAccountData {
        discriminator: [1u8; 8],
        data: vec![2u8; 32],
        data_hash: [3u8; 32],
    };
    let bytes = compressed_account_data.try_to_vec().unwrap();

    let (zero_copy, remaining) = CompressedAccountData::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.discriminator, [1u8; 8]);
    // assert_eq!(zero_copy.data, compressed_account_data.data.as_slice());
    assert_eq!(*zero_copy.data_hash, [3u8; 32]);
    assert_eq!(zero_copy, compressed_account_data);
    assert_eq!(remaining, &[]);
}
