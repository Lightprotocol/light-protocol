#![cfg(all(feature = "std", feature = "derive"))]

use std::{ops::Deref, vec};

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{
    errors::ZeroCopyError,
    slice::ZeroCopySliceBorsh,
    traits::{ZeroCopyAt, ZeroCopyStructInner},
};
use zerocopy::{
    little_endian::{U16, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

// Rules:
// 1. create ZStruct for the struct
//      1.1. the first fields are extracted into a meta struct until we reach a Vec, Option or type that does not implement Copy, and we implement deref for the meta struct
//      1.2. represent vectors to ZeroCopySlice & don't include these into the meta struct
//      1.3. replace u16 with U16, u32 with U32, etc
//      1.4. every field after the first vector is directly included in the ZStruct and deserialized 1 by 1
//      1.5. If a vector contains a nested vector (does not implement Copy) it  must implement Deserialize
//      1.6. Elements in an Option must implement Deserialize
//      1.7. a type that does not implement Copy must implement Deserialize, and is deserialized 1 by 1

// Derive Macro needs to derive:
//  1. ZeroCopyStructInner
// 2. Deserialize
// 3. PartialEq<Struct> for ZStruct<'_>
//
// For every struct1 - struct7 create struct_derived1 - struct_derived7 and replicate the tests for the new structs.

// Tests for manually implemented structures (without derive macro)

#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Struct1 {
    pub a: u8,
    pub b: u16,
}

// pub fn data_hash_struct_1(a: u8, b: u16) -> [u8; 32] {

// }

#[repr(C)]
#[derive(Debug, PartialEq, KnownLayout, Immutable, Unaligned, FromBytes)]
pub struct ZStruct1Meta {
    pub a: u8,
    pub b: U16,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZStruct1<'a> {
    meta: Ref<&'a [u8], ZStruct1Meta>,
}
impl<'a> Deref for ZStruct1<'a> {
    type Target = Ref<&'a [u8], ZStruct1Meta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for Struct1 {
    type ZeroCopyAt = ZStruct1<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZStruct1Meta>::from_prefix(bytes)?;
        Ok((ZStruct1 { meta }, bytes))
    }
}

#[test]
fn test_struct_1() {
    let bytes = Struct1 { a: 1, b: 2 }.try_to_vec().unwrap();
    let (struct1, remaining) = Struct1::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct1.a, 1u8);
    assert_eq!(struct1.b, 2u16);
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub struct Struct2 {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
}

#[repr(C)]
#[derive(Debug, PartialEq, KnownLayout, Immutable, Unaligned, FromBytes)]
pub struct ZStruct2Meta {
    pub a: u8,
    pub b: U16,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZStruct2<'a> {
    meta: Ref<&'a [u8], ZStruct2Meta>,
    pub vec: <Vec<u8> as ZeroCopyStructInner>::ZeroCopyInner,
}

impl PartialEq<Struct2> for ZStruct2<'_> {
    fn eq(&self, other: &Struct2) -> bool {
        let meta: &ZStruct2Meta = &self.meta;
        if meta.a != other.a || other.b != meta.b.into() {
            return false;
        }
        self.vec.as_slice() == other.vec.as_slice()
    }
}

impl<'a> Deref for ZStruct2<'a> {
    type Target = Ref<&'a [u8], ZStruct2Meta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for Struct2 {
    type ZeroCopyAt = ZStruct2<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZStruct2Meta>::from_prefix(bytes)?;
        let (vec, bytes) = <Vec<u8> as ZeroCopyAt>::zero_copy_at(bytes)?;
        Ok((ZStruct2 { meta, vec }, bytes))
    }
}

#[test]
fn test_struct_2() {
    let bytes = Struct2 {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
    }
    .try_to_vec()
    .unwrap();
    let (struct2, remaining) = Struct2::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct2.a, 1u8);
    assert_eq!(struct2.b, 2u16);
    assert_eq!(struct2.vec.to_vec(), vec![1u8; 32]);
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Struct3 {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
    pub c: u64,
}

#[repr(C)]
#[derive(Debug, PartialEq, KnownLayout, Immutable, Unaligned, FromBytes)]
pub struct ZStruct3Meta {
    pub a: u8,
    pub b: U16,
}

#[derive(Debug, PartialEq)]
pub struct ZStruct3<'a> {
    meta: Ref<&'a [u8], ZStruct3Meta>,
    pub vec: ZeroCopySliceBorsh<'a, u8>,
    pub c: Ref<&'a [u8], U64>,
}

impl<'a> Deref for ZStruct3<'a> {
    type Target = Ref<&'a [u8], ZStruct3Meta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for Struct3 {
    type ZeroCopyAt = ZStruct3<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZStruct3Meta>::from_prefix(bytes)?;
        let (vec, bytes) = ZeroCopySliceBorsh::zero_copy_at(bytes)?;
        let (c, bytes) = Ref::<&[u8], U64>::from_prefix(bytes)?;
        Ok((ZStruct3 { meta, vec, c }, bytes))
    }
}

#[test]
fn test_struct_3() {
    let bytes = Struct3 {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
        c: 3,
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct3::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.vec.to_vec(), vec![1u8; 32]);
    assert_eq!(u64::from(*zero_copy.c), 3);
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, Clone)]
pub struct Struct4Nested {
    a: u8,
    b: u16,
}

#[repr(C)]
#[derive(
    Debug, PartialEq, Copy, Clone, KnownLayout, Immutable, IntoBytes, Unaligned, FromBytes,
)]
pub struct ZStruct4Nested {
    pub a: u8,
    pub b: U16,
}

impl ZeroCopyStructInner for Struct4Nested {
    type ZeroCopyInner = ZStruct4Nested;
}

#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Struct4 {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
    pub c: u64,
    pub vec_2: Vec<Struct4Nested>,
}

#[repr(C)]
#[derive(Debug, PartialEq, KnownLayout, Immutable, Unaligned, IntoBytes, FromBytes)]
pub struct ZStruct4Meta {
    pub a: <u8 as ZeroCopyStructInner>::ZeroCopyInner,
    pub b: <u16 as ZeroCopyStructInner>::ZeroCopyInner,
}

#[derive(Debug, PartialEq)]
pub struct ZStruct4<'a> {
    meta: Ref<&'a [u8], ZStruct4Meta>,
    pub vec: ZeroCopySliceBorsh<'a, <u8 as ZeroCopyStructInner>::ZeroCopyInner>,
    pub c: Ref<&'a [u8], <u64 as ZeroCopyStructInner>::ZeroCopyInner>,
    pub vec_2: ZeroCopySliceBorsh<'a, <Struct4Nested as ZeroCopyStructInner>::ZeroCopyInner>,
}

impl<'a> Deref for ZStruct4<'a> {
    type Target = Ref<&'a [u8], ZStruct4Meta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for Struct4 {
    type ZeroCopyAt = ZStruct4<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZStruct4Meta>::from_prefix(bytes)?;
        let (vec, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (c, bytes) =
            Ref::<&[u8], <u64 as ZeroCopyStructInner>::ZeroCopyInner>::from_prefix(bytes)?;
        let (vec_2, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        Ok((
            ZStruct4 {
                meta,
                vec,
                c,
                vec_2,
            },
            bytes,
        ))
    }
}

#[test]
fn test_struct_4() {
    let bytes = Struct4 {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
        c: 3,
        vec_2: vec![Struct4Nested { a: 1, b: 2 }; 32],
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct4::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.vec.to_vec(), vec![1u8; 32]);
    assert_eq!(u64::from(*zero_copy.c), 3);
    assert_eq!(
        zero_copy.vec_2.to_vec(),
        vec![ZStruct4Nested { a: 1, b: 2.into() }; 32]
    );
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Struct5 {
    pub a: Vec<Vec<u8>>,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZStruct5<'a> {
    pub a: Vec<ZeroCopySliceBorsh<'a, <u8 as ZeroCopyStructInner>::ZeroCopyInner>>,
}

impl<'a> ZeroCopyAt<'a> for Struct5 {
    type ZeroCopyAt = ZStruct5<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (a, bytes) =
            Vec::<ZeroCopySliceBorsh<<u8 as ZeroCopyStructInner>::ZeroCopyInner>>::zero_copy_at(
                bytes,
            )?;
        Ok((ZStruct5 { a }, bytes))
    }
}

#[test]
fn test_struct_5() {
    let bytes = Struct5 {
        a: vec![vec![1u8; 32]; 32],
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct5::zero_copy_at(&bytes).unwrap();
    assert_eq!(
        zero_copy.a.iter().map(|x| x.to_vec()).collect::<Vec<_>>(),
        vec![vec![1u8; 32]; 32]
    );
    assert_eq!(remaining, &[]);
}

// If a struct inside a vector contains a vector it must implement Deserialize.
#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Struct6 {
    pub a: Vec<Struct2>,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZStruct6<'a> {
    pub a: Vec<<Struct2 as ZeroCopyAt<'a>>::ZeroCopyAt>,
}

impl<'a> ZeroCopyAt<'a> for Struct6 {
    type ZeroCopyAt = ZStruct6<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (a, bytes) = Vec::<Struct2>::zero_copy_at(bytes)?;
        Ok((ZStruct6 { a }, bytes))
    }
}

#[test]
fn test_struct_6() {
    let bytes = Struct6 {
        a: vec![
            Struct2 {
                a: 1,
                b: 2,
                vec: vec![1u8; 32],
            };
            32
        ],
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct6::zero_copy_at(&bytes).unwrap();
    assert_eq!(
        zero_copy.a.iter().collect::<Vec<_>>(),
        vec![
            &Struct2 {
                a: 1,
                b: 2,
                vec: vec![1u8; 32],
            };
            32
        ]
    );
    assert_eq!(remaining, &[]);
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub struct Struct7 {
    pub a: u8,
    pub b: u16,
    pub option: Option<u8>,
}

#[repr(C)]
#[derive(Debug, PartialEq, KnownLayout, Immutable, Unaligned, FromBytes)]
pub struct ZStruct7Meta {
    pub a: u8,
    pub b: U16,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZStruct7<'a> {
    meta: Ref<&'a [u8], ZStruct7Meta>,
    pub option: <Option<u8> as ZeroCopyStructInner>::ZeroCopyInner,
}

impl PartialEq<Struct7> for ZStruct7<'_> {
    fn eq(&self, other: &Struct7) -> bool {
        let meta: &ZStruct7Meta = &self.meta;
        if meta.a != other.a || other.b != meta.b.into() {
            return false;
        }
        self.option == other.option
    }
}

impl<'a> Deref for ZStruct7<'a> {
    type Target = Ref<&'a [u8], ZStruct7Meta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for Struct7 {
    type ZeroCopyAt = ZStruct7<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZStruct7Meta>::from_prefix(bytes)?;
        let (option, bytes) = <Option<u8> as ZeroCopyAt>::zero_copy_at(bytes)?;
        Ok((ZStruct7 { meta, option }, bytes))
    }
}

#[test]
fn test_struct_7() {
    let bytes = Struct7 {
        a: 1,
        b: 2,
        option: Some(3),
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct7::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.option, Some(3));
    assert_eq!(remaining, &[]);

    let bytes = Struct7 {
        a: 1,
        b: 2,
        option: None,
    }
    .try_to_vec()
    .unwrap();
    let (zero_copy, remaining) = Struct7::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1u8);
    assert_eq!(zero_copy.b, 2u16);
    assert_eq!(zero_copy.option, None);
    assert_eq!(remaining, &[]);
}

// If a struct inside a vector contains a vector it must implement Deserialize.
#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Struct8 {
    pub a: Vec<NestedStruct>,
}

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct NestedStruct {
    pub a: u8,
    pub b: Struct2,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZNestedStruct<'a> {
    pub a: <u8 as ZeroCopyStructInner>::ZeroCopyInner,
    pub b: <Struct2 as ZeroCopyAt<'a>>::ZeroCopyAt,
}

impl<'a> ZeroCopyAt<'a> for NestedStruct {
    type ZeroCopyAt = ZNestedStruct<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (a, bytes) = <u8 as ZeroCopyStructInner>::ZeroCopyInner::zero_copy_at(bytes)?;
        let (b, bytes) = <Struct2 as ZeroCopyAt>::zero_copy_at(bytes)?;
        Ok((ZNestedStruct { a, b }, bytes))
    }
}

impl PartialEq<NestedStruct> for ZNestedStruct<'_> {
    fn eq(&self, other: &NestedStruct) -> bool {
        self.a == other.a && self.b == other.b
    }
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZStruct8<'a> {
    pub a: Vec<<NestedStruct as ZeroCopyAt<'a>>::ZeroCopyAt>,
}

impl<'a> ZeroCopyAt<'a> for Struct8 {
    type ZeroCopyAt = ZStruct8<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(<Self as ZeroCopyAt<'a>>::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (a, bytes) = Vec::<NestedStruct>::zero_copy_at(bytes)?;
        Ok((ZStruct8 { a }, bytes))
    }
}

#[test]
fn test_struct_8() {
    let bytes = Struct8 {
        a: vec![
            NestedStruct {
                a: 1,
                b: Struct2 {
                    a: 1,
                    b: 2,
                    vec: vec![1u8; 32],
                },
            };
            32
        ],
    }
    .try_to_vec()
    .unwrap();

    let (zero_copy, remaining) = Struct8::zero_copy_at(&bytes).unwrap();
    assert_eq!(
        zero_copy.a.iter().collect::<Vec<_>>(),
        vec![
            &NestedStruct {
                a: 1,
                b: Struct2 {
                    a: 1,
                    b: 2,
                    vec: vec![1u8; 32],
                },
            };
            32
        ]
    );
    assert_eq!(remaining, &[]);
}
