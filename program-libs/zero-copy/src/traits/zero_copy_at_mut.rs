#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::mem::size_of;

use zerocopy::{
    little_endian::{I16, I32, I64, U16, U32, U64},
    FromBytes, Immutable, KnownLayout, Ref,
};

use crate::errors::ZeroCopyError;

pub trait ZeroCopyAtMut<'a>
where
    Self: Sized,
{
    // TODO: rename to ZeroCopy, can be used as <StructName as ZeroCopyAtMut>::ZeroCopy
    type ZeroCopyAtMut;
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError>;
}

// Implement ZeroCopyAtMut for fixed-size array types
impl<'a, T: KnownLayout + Immutable + FromBytes, const N: usize> ZeroCopyAtMut<'a> for [T; N] {
    type ZeroCopyAtMut = Ref<&'a mut [u8], [T; N]>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&'a mut [u8], [T; N]>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

impl<'a, T: ZeroCopyAtMut<'a>> ZeroCopyAtMut<'a> for Option<T> {
    type ZeroCopyAtMut = Option<T::ZeroCopyAtMut>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (option_byte, bytes) = bytes.split_at_mut(1);
        Ok(match option_byte[0] {
            0u8 => (None, bytes),
            1u8 => {
                let (value, bytes) = T::zero_copy_at_mut(bytes)?;
                (Some(value), bytes)
            }
            _ => return Err(ZeroCopyError::InvalidOptionByte(option_byte[0])),
        })
    }
}

impl<'a> ZeroCopyAtMut<'a> for u8 {
    type ZeroCopyAtMut = Ref<&'a mut [u8], u8>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        Ref::<&'a mut [u8], u8>::from_prefix(bytes).map_err(ZeroCopyError::from)
    }
}

impl<'a> ZeroCopyAtMut<'a> for bool {
    type ZeroCopyAtMut = Ref<&'a mut [u8], u8>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        Ref::<&'a mut [u8], u8>::from_prefix(bytes).map_err(ZeroCopyError::from)
    }
}

// Implementation for specific zerocopy little-endian types
impl<'a, T: KnownLayout + Immutable + FromBytes> ZeroCopyAtMut<'a> for Ref<&'a mut [u8], T> {
    type ZeroCopyAtMut = Ref<&'a mut [u8], T>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&mut [u8], T>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: ZeroCopyAtMut<'a>> ZeroCopyAtMut<'a> for Vec<T> {
    type ZeroCopyAtMut = Vec<T::ZeroCopyAtMut>;
    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&mut [u8], U32>::from_prefix(bytes)?;
        let num_slices = crate::u32_to_usize(u32::from(*num_slices))?;
        // Prevent heap exhaustion attacks by checking if num_slices is reasonable
        // Each element needs at least 1 byte when serialized
        if bytes.len() < num_slices {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                num_slices,
            ));
        }
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at_mut(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

macro_rules! impl_deserialize_for_primitive {
    ($(($native:ty, $zerocopy:ty)),*) => {
        $(
            impl<'a> ZeroCopyAtMut<'a> for $native {
                type ZeroCopyAtMut = Ref<&'a mut [u8], $zerocopy>;

                #[inline]
                fn zero_copy_at_mut(bytes: &'a mut [u8]) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
                    Ref::<&'a mut [u8], $zerocopy>::from_prefix(bytes).map_err(ZeroCopyError::from)
                }
            }
        )*
    };
}

impl_deserialize_for_primitive!(
    (u16, U16),
    (u32, U32),
    (u64, U64),
    (i16, I16),
    (i32, I32),
    (i64, I64),
    (U16, U16),
    (U32, U32),
    (U64, U64),
    (I16, I16),
    (I32, I32),
    (I64, I64)
);

pub fn borsh_vec_u8_as_slice_mut(
    bytes: &mut [u8],
) -> Result<(&mut [u8], &mut [u8]), ZeroCopyError> {
    let (num_slices, bytes) = Ref::<&mut [u8], U32>::from_prefix(bytes)?;
    let num_slices = crate::u32_to_usize(u32::from(*num_slices))?;
    if num_slices > bytes.len() {
        return Err(ZeroCopyError::ArraySize(num_slices, bytes.len()));
    }
    Ok(bytes.split_at_mut(num_slices))
}

pub trait ZeroCopyStructInnerMut {
    type ZeroCopyInnerMut;
}

impl ZeroCopyStructInnerMut for u64 {
    type ZeroCopyInnerMut = U64;
}
impl ZeroCopyStructInnerMut for u32 {
    type ZeroCopyInnerMut = U32;
}
impl ZeroCopyStructInnerMut for u16 {
    type ZeroCopyInnerMut = U16;
}
impl ZeroCopyStructInnerMut for u8 {
    type ZeroCopyInnerMut = u8;
}
impl ZeroCopyStructInnerMut for U64 {
    type ZeroCopyInnerMut = U64;
}
impl ZeroCopyStructInnerMut for U32 {
    type ZeroCopyInnerMut = U32;
}
impl ZeroCopyStructInnerMut for U16 {
    type ZeroCopyInnerMut = U16;
}

#[cfg(feature = "alloc")]
impl<T: ZeroCopyStructInnerMut + Copy> ZeroCopyStructInnerMut for Vec<T> {
    type ZeroCopyInnerMut = Vec<T::ZeroCopyInnerMut>;
}

impl<T: ZeroCopyStructInnerMut + Copy> ZeroCopyStructInnerMut for Option<T> {
    type ZeroCopyInnerMut = Option<T::ZeroCopyInnerMut>;
}

impl<const N: usize> ZeroCopyStructInnerMut for [u8; N] {
    type ZeroCopyInnerMut = Ref<&'static mut [u8], [u8; N]>;
}

#[test]
fn test_deserialize_mut_ref() {
    let mut bytes = [1, 0, 0, 0]; // Little-endian representation of 1
    let (ref_data, remaining) = Ref::<&mut [u8], U32>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(u32::from(*ref_data), 1);
    assert_eq!(remaining, &mut []);
    let res = Ref::<&mut [u8], U32>::zero_copy_at_mut(&mut []);
    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_mut_option_some() {
    let mut bytes = [1, 2]; // 1 indicates Some, followed by the value 2
    let (option_value, remaining) = Option::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(option_value.map(|x| *x), Some(2));
    assert_eq!(remaining, &mut []);
    let res = Option::<u8>::zero_copy_at_mut(&mut []);
    assert_eq!(res, Err(ZeroCopyError::ArraySize(1, 0)));
    let mut bytes = [2, 0]; // 2 indicates invalid option byte
    let res = Option::<u8>::zero_copy_at_mut(&mut bytes);
    assert_eq!(res, Err(ZeroCopyError::InvalidOptionByte(2)));
}

#[test]
fn test_deserialize_mut_option_none() {
    let mut bytes = [0]; // 0 indicates None
    let (option_value, remaining) = Option::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(option_value, None);
    assert_eq!(remaining, &mut []);
}

#[test]
fn test_deserialize_mut_u8() {
    let mut bytes = [0xFF]; // Value 255
    let (value, remaining) = u8::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(*value, 255);
    assert_eq!(remaining, &mut []);
    let res = u8::zero_copy_at_mut(&mut []);
    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_mut_u16() {
    let mut bytes = 2323u16.to_le_bytes();
    let (value, remaining) = u16::zero_copy_at_mut(bytes.as_mut_slice()).unwrap();
    assert_eq!(*value, 2323u16);
    assert_eq!(remaining, &mut []);
    let mut value = [0u8];
    let res = u16::zero_copy_at_mut(&mut value);

    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_mut_vec() {
    let mut bytes = [2, 0, 0, 0, 1, 2]; // Length 2, followed by values 1 and 2
    let (vec, remaining) = Vec::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(
        vec.iter().map(|x| **x).collect::<Vec<u8>>(),
        std::vec![1u8, 2]
    );
    assert_eq!(remaining, &mut []);
}

#[cfg(test)]
pub mod test {
    use std::{
        ops::{Deref, DerefMut},
        vec,
    };

    use borsh::{BorshDeserialize, BorshSerialize};
    use zerocopy::{
        little_endian::{U16, U64},
        IntoBytes, Ref, Unaligned,
    };

    use super::*;
    use crate::slice_mut::ZeroCopySliceMutBorsh;

    // Rules:
    // 1. create ZStruct for the struct
    //      1.1. the first fields are extracted into a meta struct until we reach a Vec, Option or type that does not implement Copy, and we implement deref for the meta struct
    //      1.2. represent vectors to ZeroCopySlice & don't include these into the meta struct
    //      1.3. replace u16 with U16, u32 with U32, etc
    //      1.4. every field after the first vector is directly included in the ZStruct and deserialized 1 by 1
    //      1.5. If a vector contains a nested vector (does not implement Copy) it  must implement ZeroCopyAtMut
    //      1.6. Elements in an Option must implement ZeroCopyAtMut
    //      1.7. a type that does not implement Copy must implement ZeroCopyAtMut, and is deserialized 1 by 1

    // Derive Macro needs to derive:
    //  1. ZeroCopyStructInnerMut
    // 2. ZeroCopyAtMut
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

    #[repr(C)]
    #[derive(Debug, PartialEq, KnownLayout, Immutable, Unaligned, FromBytes, IntoBytes)]
    pub struct ZStruct1Meta {
        pub a: u8,
        pub b: U16,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    pub struct ZStruct1<'a> {
        pub meta: Ref<&'a mut [u8], ZStruct1Meta>,
    }
    impl<'a> Deref for ZStruct1<'a> {
        type Target = Ref<&'a mut [u8], ZStruct1Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl DerefMut for ZStruct1<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.meta
        }
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct1 {
        type ZeroCopyAtMut = ZStruct1<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct1Meta>::from_prefix(bytes)?;
            Ok((ZStruct1 { meta }, bytes))
        }
    }

    #[test]
    fn test_struct_1() {
        let ref_struct = Struct1 { a: 1, b: 2 };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (mut struct1, remaining) = Struct1::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(struct1.a, 1u8);
        assert_eq!(struct1.b, 2u16);
        assert_eq!(remaining, &mut []);
        struct1.meta.a = 2;
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
        meta: Ref<&'a mut [u8], ZStruct2Meta>,
        pub vec: &'a mut [u8],
    }

    impl PartialEq<Struct2> for ZStruct2<'_> {
        fn eq(&self, other: &Struct2) -> bool {
            let meta: &ZStruct2Meta = &self.meta;
            if meta.a != other.a || other.b != meta.b.into() {
                return false;
            }
            self.vec == other.vec.as_slice()
        }
    }

    impl<'a> Deref for ZStruct2<'a> {
        type Target = Ref<&'a mut [u8], ZStruct2Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct2 {
        type ZeroCopyAtMut = ZStruct2<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct2Meta>::from_prefix(bytes)?;
            let (len, bytes) = bytes.split_at_mut(4);
            let len = U32::from_bytes(
                len.try_into()
                    .map_err(|_| ZeroCopyError::ArraySize(4, len.len()))?,
            );
            let vec_len = crate::u32_to_usize(u32::from(len))?;
            let (vec, bytes) = bytes.split_at_mut(vec_len);
            Ok((ZStruct2 { meta, vec }, bytes))
        }
    }

    #[test]
    fn test_struct_2() {
        let ref_struct = Struct2 {
            a: 1,
            b: 2,
            vec: vec![1u8; 32],
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (struct2, remaining) = Struct2::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(struct2.a, 1u8);
        assert_eq!(struct2.b, 2u16);
        assert_eq!(struct2.vec.to_vec(), vec![1u8; 32]);
        assert_eq!(remaining, &mut []);
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
        meta: Ref<&'a mut [u8], ZStruct3Meta>,
        pub vec: ZeroCopySliceMutBorsh<'a, u8>,
        pub c: Ref<&'a mut [u8], U64>,
    }

    impl<'a> Deref for ZStruct3<'a> {
        type Target = Ref<&'a mut [u8], ZStruct3Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct3 {
        type ZeroCopyAtMut = ZStruct3<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct3Meta>::from_prefix(bytes)?;
            let (vec, bytes) = ZeroCopySliceMutBorsh::zero_copy_at_mut(bytes)?;
            let (c, bytes) = Ref::<&mut [u8], U64>::from_prefix(bytes)?;
            Ok((Self::ZeroCopyAtMut { meta, vec, c }, bytes))
        }
    }

    #[test]
    fn test_struct_3() {
        let ref_struct = Struct3 {
            a: 1,
            b: 2,
            vec: vec![1u8; 32],
            c: 3,
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct3::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(zero_copy.a, 1u8);
        assert_eq!(zero_copy.b, 2u16);
        assert_eq!(zero_copy.vec.to_vec(), vec![1u8; 32]);
        assert_eq!(u64::from(*zero_copy.c), 3);
        assert_eq!(remaining, &mut []);
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, Clone)]
    pub struct Struct4Nested {
        a: u8,
        b: u16,
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct4Nested {
        type ZeroCopyAtMut = ZStruct4Nested;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (bytes, remaining_bytes) = Ref::<&mut [u8], ZStruct4Nested>::from_prefix(bytes)?;
            Ok((*bytes, remaining_bytes))
        }
    }

    #[repr(C)]
    #[derive(
        Debug, PartialEq, Copy, Clone, KnownLayout, Immutable, IntoBytes, Unaligned, FromBytes,
    )]
    pub struct ZStruct4Nested {
        pub a: u8,
        pub b: U16,
    }

    impl ZeroCopyStructInnerMut for Struct4Nested {
        type ZeroCopyInnerMut = ZStruct4Nested;
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
        pub a: <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut,
        pub b: <u16 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut,
    }

    #[derive(Debug, PartialEq)]
    pub struct ZStruct4<'a> {
        meta: Ref<&'a mut [u8], ZStruct4Meta>,
        pub vec: ZeroCopySliceMutBorsh<'a, <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>,
        pub c: Ref<&'a mut [u8], <u64 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>,
        pub vec_2:
            ZeroCopySliceMutBorsh<'a, <Struct4Nested as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>,
    }

    impl<'a> Deref for ZStruct4<'a> {
        type Target = Ref<&'a mut [u8], ZStruct4Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct4 {
        type ZeroCopyAtMut = ZStruct4<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct4Meta>::from_prefix(bytes)?;
            let (vec, bytes) = ZeroCopySliceMutBorsh::from_bytes_at(bytes)?;
            let (c, bytes) =
                Ref::<&mut [u8], <u64 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>::from_prefix(
                    bytes,
                )?;
            let (vec_2, bytes) = ZeroCopySliceMutBorsh::from_bytes_at(bytes)?;
            Ok((
                Self::ZeroCopyAtMut {
                    meta,
                    vec,
                    c,
                    vec_2,
                },
                bytes,
            ))
        }
    }

    /// TODO:
    /// - add SIZE const generic ZeroCopyAtMut trait
    /// - add new with data function
    impl Struct4 {
        // pub fn byte_len(&self) -> usize {
        //     size_of::<u8>()
        //         + size_of::<u16>()
        //         + size_of::<u8>() * self.vec.len()
        //         + size_of::<u64>()
        //         + size_of::<Struct4Nested>() * self.vec_2.len()
        // }
    }

    #[test]
    fn test_struct_4() {
        let ref_struct = Struct4 {
            a: 1,
            b: 2,
            vec: vec![1u8; 32],
            c: 3,
            vec_2: vec![Struct4Nested { a: 1, b: 2 }; 32],
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct4::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(zero_copy.a, 1u8);
        assert_eq!(zero_copy.b, 2u16);
        assert_eq!(zero_copy.vec.to_vec(), vec![1u8; 32]);
        assert_eq!(u64::from(*zero_copy.c), 3);
        assert_eq!(
            zero_copy.vec_2.to_vec(),
            vec![ZStruct4Nested { a: 1, b: 2.into() }; 32]
        );
        assert_eq!(remaining, &mut []);
    }

    #[repr(C)]
    #[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
    pub struct Struct5 {
        pub a: Vec<Vec<u8>>,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    pub struct ZStruct5<'a> {
        pub a: Vec<ZeroCopySliceMutBorsh<'a, <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>>,
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct5 {
        type ZeroCopyAtMut = ZStruct5<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (a, bytes) = Vec::<
                ZeroCopySliceMutBorsh<<u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>,
            >::zero_copy_at_mut(bytes)?;
            Ok((ZStruct5 { a }, bytes))
        }
    }

    #[test]
    fn test_struct_5() {
        let ref_struct = Struct5 {
            a: vec![vec![1u8; 32]; 32],
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct5::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(
            zero_copy.a.iter().map(|x| x.to_vec()).collect::<Vec<_>>(),
            vec![vec![1u8; 32]; 32]
        );
        assert_eq!(remaining, &mut []);
    }

    // If a struct inside a vector contains a vector it must implement ZeroCopyAtMut.
    #[repr(C)]
    #[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
    pub struct Struct6 {
        pub a: Vec<Struct2>,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    pub struct ZStruct6<'a> {
        pub a: Vec<<Struct2 as ZeroCopyAtMut<'a>>::ZeroCopyAtMut>,
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct6 {
        type ZeroCopyAtMut = ZStruct6<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (a, bytes) = Vec::<Struct2>::zero_copy_at_mut(bytes)?;
            Ok((ZStruct6 { a }, bytes))
        }
    }

    #[test]
    fn test_struct_6() {
        let ref_struct = Struct6 {
            a: vec![
                Struct2 {
                    a: 1,
                    b: 2,
                    vec: vec![1u8; 32],
                };
                32
            ],
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct6::zero_copy_at_mut(&mut bytes).unwrap();
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
        assert_eq!(remaining, &mut []);
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
        meta: Ref<&'a mut [u8], ZStruct7Meta>,
        pub option: Option<<u8 as ZeroCopyAtMut<'a>>::ZeroCopyAtMut>,
    }

    impl PartialEq<Struct7> for ZStruct7<'_> {
        fn eq(&self, other: &Struct7) -> bool {
            let meta: &ZStruct7Meta = &self.meta;
            if meta.a != other.a || other.b != meta.b.into() {
                return false;
            }
            self.option.as_ref().map(|x| **x) == other.option
        }
    }

    impl<'a> Deref for ZStruct7<'a> {
        type Target = Ref<&'a mut [u8], ZStruct7Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct7 {
        type ZeroCopyAtMut = ZStruct7<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct7Meta>::from_prefix(bytes)?;
            let (option, bytes) = <Option<u8> as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((ZStruct7 { meta, option }, bytes))
        }
    }

    #[test]
    fn test_struct_7() {
        let ref_struct = Struct7 {
            a: 1,
            b: 2,
            option: Some(3),
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct7::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(zero_copy.a, 1u8);
        assert_eq!(zero_copy.b, 2u16);
        assert_eq!(zero_copy.option.map(|x| *x), Some(3));
        assert_eq!(remaining, &mut []);

        let ref_struct = Struct7 {
            a: 1,
            b: 2,
            option: None,
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct7::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(zero_copy.a, 1u8);
        assert_eq!(zero_copy.b, 2u16);
        assert_eq!(zero_copy.option, None);
        assert_eq!(remaining, &mut []);
    }

    // If a struct inside a vector contains a vector it must implement ZeroCopyAtMut.
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
        pub a: <u8 as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
        pub b: <Struct2 as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    }

    impl<'a> ZeroCopyAtMut<'a> for NestedStruct {
        type ZeroCopyAtMut = ZNestedStruct<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (a, bytes) =
                <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut::zero_copy_at_mut(bytes)?;
            let (b, bytes) = <Struct2 as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((ZNestedStruct { a, b }, bytes))
        }
    }

    impl PartialEq<NestedStruct> for ZNestedStruct<'_> {
        fn eq(&self, other: &NestedStruct) -> bool {
            *self.a == other.a && self.b == other.b
        }
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    pub struct ZStruct8<'a> {
        pub a: Vec<<NestedStruct as ZeroCopyAtMut<'a>>::ZeroCopyAtMut>,
    }

    impl<'a> ZeroCopyAtMut<'a> for Struct8 {
        type ZeroCopyAtMut = ZStruct8<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
            let (a, bytes) = Vec::<NestedStruct>::zero_copy_at_mut(bytes)?;
            Ok((ZStruct8 { a }, bytes))
        }
    }

    #[test]
    fn test_struct_8() {
        let ref_struct = Struct8 {
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
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();

        let (zero_copy, remaining) = Struct8::zero_copy_at_mut(&mut bytes).unwrap();
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
        assert_eq!(remaining, &mut []);
    }
}
