use light_hasher::{DataHasher, Hasher, Poseidon};
use light_sdk_macros::LightHasher;

#[test]
fn flatten() {
    #[repr(C)]
    #[derive(LightHasher, Debug, PartialEq)]
    pub struct StructInnerF {
        pub a: u8,
        pub b: u16,
    }

    #[repr(C)]
    #[derive(LightHasher, Debug, PartialEq)]
    pub struct StructFlatten {
        #[flatten]
        pub a: StructInnerF,
        pub b: StructInnerF,
    }

    let test = StructFlatten {
        a: StructInnerF { a: 1, b: 2 },
        b: StructInnerF { a: 3, b: 4 },
    };

    let one = {
        let mut array = [0u8; 32];
        array[31] = 1;
        array
    };
    let two = {
        let mut array = [0u8; 32];
        array[31] = 2;
        array
    };
    let three = {
        let mut array = [0u8; 32];
        array[31] = 3;
        array
    };
    let four = {
        let mut array = [0u8; 32];
        array[31] = 4;
        array
    };
    let hash = Poseidon::hashv(&[three.as_slice(), four.as_slice()]).unwrap();
    let manual_slices = [one.as_ref(), two.as_ref(), hash.as_ref()];
    println!("manual_slices {:?}", manual_slices);
    let manual_hash = Poseidon::hashv(&manual_slices).unwrap();
    assert_eq!(test.hash::<Poseidon>().unwrap(), manual_hash);
}

#[test]
fn flatten_twice() {
    #[repr(C)]
    #[derive(LightHasher, Debug, PartialEq)]
    pub struct StructInnerF {
        pub a: u8,
        pub b: u16,
    }

    #[repr(C)]
    #[derive(LightHasher, Debug, PartialEq)]
    pub struct StructFlatten {
        #[flatten]
        pub a: StructInnerF,
        #[flatten]
        pub b: StructInnerF,
    }

    let test = StructFlatten {
        a: StructInnerF { a: 1, b: 2 },
        b: StructInnerF { a: 3, b: 4 },
    };

    let one = {
        let mut array = [0u8; 32];
        array[31] = 1;
        array
    };
    let two = {
        let mut array = [0u8; 32];
        array[31] = 2;
        array
    };
    let three = {
        let mut array = [0u8; 32];
        array[31] = 3;
        array
    };
    let four = {
        let mut array = [0u8; 32];
        array[31] = 4;
        array
    };
    let manual_slices = [one.as_ref(), two.as_ref(), three.as_ref(), four.as_ref()];
    println!("manual_slices {:?}", manual_slices);
    let manual_hash = Poseidon::hashv(&manual_slices).unwrap();
    assert_eq!(test.hash::<Poseidon>().unwrap(), manual_hash);
}
