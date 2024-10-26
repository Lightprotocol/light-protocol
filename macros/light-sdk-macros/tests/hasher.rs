use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use light_hasher::{bytes::AsByteVec, DataHasher, Poseidon};
use light_sdk_macros::LightHasher;

#[derive(LightHasher)]
pub struct MyAccount {
    pub a: bool,
    pub b: u64,
    #[nested]
    pub c: MyNestedStruct,
    #[truncate]
    pub d: [u8; 32],
    #[skip]
    pub e: MyNestedNonHashableStruct,
    pub f: Option<usize>,
}

#[derive(LightHasher)]
pub struct MyNestedStruct {
    pub a: i32,
    pub b: u32,
    #[truncate]
    pub c: String,
}

pub struct MyNestedNonHashableStruct {
    pub a: PhantomData<()>,
    pub b: Rc<RefCell<usize>>,
}

#[test]
fn test_light_hasher() {
    let my_account = MyAccount {
        a: true,
        b: u64::MAX,
        c: MyNestedStruct {
            a: i32::MIN,
            b: u32::MAX,
            c: "wao".to_string(),
        },
        d: [u8::MAX; 32],
        e: MyNestedNonHashableStruct {
            a: PhantomData,
            b: Rc::new(RefCell::new(usize::MAX)),
        },
        f: None,
    };
    assert_eq!(
        my_account.hash::<Poseidon>().unwrap(),
        [
            44, 62, 31, 169, 73, 125, 135, 126, 176, 7, 127, 96, 183, 224, 156, 140, 105, 77, 225,
            230, 174, 196, 38, 92, 0, 44, 19, 25, 255, 109, 6, 168
        ]
    );

    let my_account = MyAccount {
        a: true,
        b: u64::MAX,
        c: MyNestedStruct {
            a: i32::MIN,
            b: u32::MAX,
            c: "wao".to_string(),
        },
        d: [u8::MAX; 32],
        e: MyNestedNonHashableStruct {
            a: PhantomData,
            b: Rc::new(RefCell::new(usize::MAX)),
        },
        f: Some(0),
    };
    assert_eq!(
        my_account.hash::<Poseidon>().unwrap(),
        [
            32, 205, 141, 227, 236, 5, 28, 219, 24, 164, 215, 79, 151, 131, 162, 82, 224, 101, 171,
            201, 4, 181, 26, 146, 6, 1, 95, 107, 239, 19, 233, 80
        ]
    );
}

#[test]
fn test_nested_struct_hashing() {
    let nested_struct = MyNestedStruct {
        a: i32::MIN,
        b: u32::MAX,
        c: "wao".to_string(),
    };

    // Manual implementation of AsByteVec for comparison
    let manual_bytes: Vec<Vec<u8>> = vec![
        nested_struct.a.to_le_bytes().to_vec(),
        nested_struct.b.to_le_bytes().to_vec(),
        light_utils::hash_to_bn254_field_size_be(nested_struct.c.as_bytes()).unwrap().0.to_vec(),
    ];

    // Compare manual implementation with macro-generated one
    assert_eq!(nested_struct.as_byte_vec(), manual_bytes);

    // Test hashing
    let hash_result = nested_struct.hash::<Poseidon>().unwrap();
    assert_eq!(hash_result.len(), 32);
}
