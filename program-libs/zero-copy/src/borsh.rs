use core::{
    mem::size_of,
    ops::{Deref, DerefMut},
};
use std::vec::Vec;

use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, KnownLayout, Ref,
};

use crate::errors::ZeroCopyError;

pub trait Deserialize
where
    Self: Sized,
{
    // TODO: rename to ZeroCopy, can be used as <StructName as Deserialize>::ZeroCopy
    type Output<'a>;

    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError>;
}

// Implement Deserialize for fixed-size array types
impl<T: KnownLayout + Immutable + FromBytes, const N: usize> Deserialize for [T; N] {
    type Output<'a> = Ref<&'a [u8], [T; N]>;

    #[inline]
    fn zero_copy_at<'a>(
        bytes: &'a [u8],
    ) -> Result<(Ref<&'a [u8], [T; N]>, &'a [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&'a [u8], [T; N]>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    type Output<'a> = Option<T::Output<'a>>;
    #[inline]
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (option_byte, bytes) = bytes.split_at(1);
        Ok(match option_byte[0] {
            0u8 => (None, bytes),
            1u8 => {
                let (value, bytes) = T::zero_copy_at(bytes)?;
                (Some(value), bytes)
            }
            _ => return Err(ZeroCopyError::InvalidOptionByte(option_byte[0])),
        })
    }
}

impl Deserialize for u8 {
    type Output<'a> = Self;

    /// Not a zero copy but cheaper.
    /// A u8 should not be deserialized on it's own but as part of a struct.
    #[inline]
    fn zero_copy_at<'a>(bytes: &[u8]) -> Result<(u8, &[u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (bytes, remaining_bytes) = bytes.split_at(size_of::<u8>());
        Ok((bytes[0], remaining_bytes))
    }
}

// Implementation for specific zerocopy little-endian types
impl<T: KnownLayout + Immutable + FromBytes> Deserialize for Ref<&[u8], T> {
    type Output<'a> = Ref<&'a [u8], T>;

    #[inline]
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&[u8], T>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    type Output<'a> = Vec<T::Output<'a>>;
    #[inline]
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        // TODO: add check that remaining data is enough to read num_slices
        // This prevents agains invalid data allocating a lot of heap memory
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

macro_rules! impl_deserialize_for_primitive {
    ($($t:ty),*) => {
        $(
            impl Deserialize for $t {
                type Output<'a> = Ref<&'a [u8], $t>;

                #[inline]
                fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
                    Self::Output::zero_copy_at(bytes)
                }
            }
        )*
    };
}

impl_deserialize_for_primitive!(u16, u32, u64, i16, i32, i64);

pub fn borsh_vec_u8_as_slice(bytes: &[u8]) -> Result<(&[u8], &[u8]), ZeroCopyError> {
    let (num_slices, bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
    let num_slices = u32::from(*num_slices) as usize;
    Ok(bytes.split_at(num_slices))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VecU8<T>(Vec<T>);
impl<T> VecU8<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<T> Deref for VecU8<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VecU8<u8> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Deserialize> Deserialize for VecU8<T> {
    type Output<'a> = Vec<T::Output<'a>>;

    #[inline]
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&[u8], u8>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

pub trait ZeroCopyStructInner {
    type ZeroCopyInner;
}

impl ZeroCopyStructInner for u64 {
    type ZeroCopyInner = U64;
}
impl ZeroCopyStructInner for u32 {
    type ZeroCopyInner = U32;
}
impl ZeroCopyStructInner for u16 {
    type ZeroCopyInner = U16;
}
impl ZeroCopyStructInner for u8 {
    type ZeroCopyInner = u8;
}

impl<T: ZeroCopyStructInner + Copy> ZeroCopyStructInner for Vec<T> {
    type ZeroCopyInner = Vec<T::ZeroCopyInner>;
}

impl<T: ZeroCopyStructInner + Copy> ZeroCopyStructInner for Option<T> {
    type ZeroCopyInner = Option<T::ZeroCopyInner>;
}

// Add ZeroCopyStructInner for array types
impl<const N: usize> ZeroCopyStructInner for [u8; N] {
    type ZeroCopyInner = Ref<&'static [u8], [u8; N]>;
}

#[test]
fn test_vecu8() {
    use std::vec;
    let bytes = vec![8, 1u8, 2, 3, 4, 5, 6, 7, 8];
    let (vec, remaining_bytes) = VecU8::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(vec, vec![1u8, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(remaining_bytes, &[]);
}

#[test]
fn test_deserialize_ref() {
    let bytes = [1, 0, 0, 0]; // Little-endian representation of 1
    let (ref_data, remaining) = Ref::<&[u8], U32>::zero_copy_at(&bytes).unwrap();
    assert_eq!(u32::from(*ref_data), 1);
    assert_eq!(remaining, &[]);
    let res = Ref::<&[u8], U32>::zero_copy_at(&[]);
    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_option_some() {
    let bytes = [1, 2]; // 1 indicates Some, followed by the value 2
    let (option_value, remaining) = Option::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(option_value, Some(2));
    assert_eq!(remaining, &[]);
    let res = Option::<u8>::zero_copy_at(&[]);
    assert_eq!(res, Err(ZeroCopyError::ArraySize(1, 0)));
    let bytes = [2, 0]; // 2 indicates invalid option byte
    let res = Option::<u8>::zero_copy_at(&bytes);
    assert_eq!(res, Err(ZeroCopyError::InvalidOptionByte(2)));
}

#[test]
fn test_deserialize_option_none() {
    let bytes = [0]; // 0 indicates None
    let (option_value, remaining) = Option::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(option_value, None);
    assert_eq!(remaining, &[]);
}

#[test]
fn test_deserialize_u8() {
    let bytes = [0xFF]; // Value 255
    let (value, remaining) = u8::zero_copy_at(&bytes).unwrap();
    assert_eq!(value, 255);
    assert_eq!(remaining, &[]);
    let res = u8::zero_copy_at(&[]);
    assert_eq!(res, Err(ZeroCopyError::ArraySize(1, 0)));
}

#[test]
fn test_deserialize_u16() {
    let bytes = 2323u16.to_le_bytes();
    let (value, remaining) = u16::zero_copy_at(bytes.as_slice()).unwrap();
    assert_eq!(*value, 2323u16);
    assert_eq!(remaining, &[]);
    let res = u16::zero_copy_at(&[0u8]);
    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_vec() {
    let bytes = [2, 0, 0, 0, 1, 2]; // Length 2, followed by values 1 and 2
    let (vec, remaining) = Vec::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(vec, std::vec![1, 2]);
    assert_eq!(remaining, &[]);
}

#[test]
fn test_vecu8_deref() {
    let data = std::vec![1, 2, 3];
    let vec_u8 = VecU8(data.clone());
    assert_eq!(&*vec_u8, &data);

    let mut vec = VecU8::new();
    vec.push(1u8);
    assert_eq!(*vec, std::vec![1u8]);
}

#[test]
fn test_deserialize_vecu8() {
    let bytes = [3, 4, 5, 6]; // Length 3, followed by values 4, 5, 6
    let (vec, remaining) = VecU8::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(vec, std::vec![4, 5, 6]);
    assert_eq!(remaining, &[]);
}

#[cfg(test)]
pub mod test {
    use std::vec;

    use borsh::{BorshDeserialize, BorshSerialize};
    use zerocopy::{
        little_endian::{U16, U64},
        IntoBytes, Ref, Unaligned,
    };

    use super::*;
    use crate::{borsh_mut::DeserializeMut, slice::ZeroCopySliceBorsh};

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

    impl Deserialize for Struct1 {
        type Output<'a> = ZStruct1<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
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

    impl Deserialize for Struct2 {
        type Output<'a> = ZStruct2<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&[u8], ZStruct2Meta>::from_prefix(bytes)?;
            let (vec, bytes) = <Vec<u8> as Deserialize>::zero_copy_at(bytes)?;
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

    impl Deserialize for Struct3 {
        type Output<'a> = ZStruct3<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&[u8], ZStruct3Meta>::from_prefix(bytes)?;
            let (vec, bytes) = ZeroCopySliceBorsh::zero_copy_at(bytes)?;
            let (c, bytes) = Ref::<&[u8], U64>::from_prefix(bytes)?;
            Ok((Self::Output { meta, vec, c }, bytes))
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

    impl Deserialize for Struct4 {
        type Output<'a> = ZStruct4<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&[u8], ZStruct4Meta>::from_prefix(bytes)?;
            let (vec, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
            let (c, bytes) =
                Ref::<&[u8], <u64 as ZeroCopyStructInner>::ZeroCopyInner>::from_prefix(bytes)?;
            let (vec_2, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
            Ok((
                Self::Output {
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

    impl Deserialize for Struct5 {
        type Output<'a> = ZStruct5<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
            let (a, bytes) = Vec::<ZeroCopySliceBorsh::<<u8 as ZeroCopyStructInner>::ZeroCopyInner>>::zero_copy_at(bytes)?;
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
        pub a: Vec<<Struct2 as Deserialize>::Output<'a>>,
    }

    impl Deserialize for Struct6 {
        type Output<'a> = ZStruct6<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
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

    impl Deserialize for Struct7 {
        type Output<'a> = ZStruct7<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&[u8], ZStruct7Meta>::from_prefix(bytes)?;
            let (option, bytes) = <Option<u8> as Deserialize>::zero_copy_at(bytes)?;
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
        pub b: <Struct2 as Deserialize>::Output<'a>,
    }

    impl Deserialize for NestedStruct {
        type Output<'a> = ZNestedStruct<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
            let (a, bytes) = <u8 as ZeroCopyStructInner>::ZeroCopyInner::zero_copy_at(bytes)?;
            let (b, bytes) = <Struct2 as Deserialize>::zero_copy_at(bytes)?;
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
        pub a: Vec<<NestedStruct as Deserialize>::Output<'a>>,
    }

    impl Deserialize for Struct8 {
        type Output<'a> = ZStruct8<'a>;

        fn zero_copy_at<'a>(
            bytes: &'a [u8],
        ) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
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
}
