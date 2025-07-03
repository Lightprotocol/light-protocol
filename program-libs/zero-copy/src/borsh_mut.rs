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

pub trait DeserializeMut<'a>
where
    Self: Sized,
{
    // TODO: rename to ZeroCopy, can be used as <StructName as DeserializeMut>::ZeroCopy
    type Output;
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError>;
}

// Implement DeserializeMut for fixed-size array types
impl<'a, T: KnownLayout + Immutable + FromBytes, const N: usize> DeserializeMut<'a> for [T; N] {
    type Output = Ref<&'a mut [u8], [T; N]>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&'a mut [u8], [T; N]>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

impl<'a, T: DeserializeMut<'a>> DeserializeMut<'a> for Option<T> {
    type Output = Option<T::Output>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
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

impl<'a> DeserializeMut<'a> for u8 {
    type Output = Self;

    /// Not a zero copy but cheaper.
    /// A u8 should not be deserialized on it's own but as part of a struct.
    #[inline]
    fn zero_copy_at_mut(bytes: &'a mut [u8]) -> Result<(u8, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (bytes, remaining_bytes) = bytes.split_at_mut(size_of::<u8>());
        Ok((bytes[0], remaining_bytes))
    }
}

// Implementation for specific zerocopy little-endian types
impl<'a, T: KnownLayout + Immutable + FromBytes> DeserializeMut<'a> for Ref<&'a mut [u8], T> {
    type Output = Ref<&'a mut [u8], T>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&mut [u8], T>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }

}

impl<'a, T: DeserializeMut<'a>> DeserializeMut<'a> for Vec<T> {
    type Output = Vec<T::Output>;
    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&mut [u8], U32>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        // TODO: add check that remaining data is enough to read num_slices
        // This prevents agains invalid data allocating a lot of heap memory
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
    ($($t:ty),*) => {
        $(
            impl<'a> DeserializeMut<'a> for $t {
                type Output = Ref<&'a mut [u8], $t>;

                #[inline]
                fn zero_copy_at_mut(bytes: &'a mut [u8]) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
                    Self::Output::zero_copy_at_mut(bytes)
                }
            }
        )*
    };
}

impl_deserialize_for_primitive!(u16, u32, u64, i16, i32, i64);

// Add DeserializeMut for zerocopy little-endian types
impl<'a> DeserializeMut<'a> for zerocopy::little_endian::U16 {
    type Output = Ref<&'a mut [u8], zerocopy::little_endian::U16>;

    #[inline]
    fn zero_copy_at_mut(bytes: &'a mut [u8]) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(Ref::<&mut [u8], zerocopy::little_endian::U16>::from_prefix(bytes)?)
    }
}

impl<'a> DeserializeMut<'a> for zerocopy::little_endian::U32 {
    type Output = Ref<&'a mut [u8], zerocopy::little_endian::U32>;

    #[inline]
    fn zero_copy_at_mut(bytes: &'a mut [u8]) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(Ref::<&mut [u8], zerocopy::little_endian::U32>::from_prefix(bytes)?)
    }
}

impl<'a> DeserializeMut<'a> for zerocopy::little_endian::U64 {
    type Output = Ref<&'a mut [u8], zerocopy::little_endian::U64>;

    #[inline]
    fn zero_copy_at_mut(bytes: &'a mut [u8]) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(Ref::<&mut [u8], zerocopy::little_endian::U64>::from_prefix(bytes)?)
    }
}

pub fn borsh_vec_u8_as_slice_mut(
    bytes: &mut [u8],
) -> Result<(&mut [u8], &mut [u8]), ZeroCopyError> {
    let (num_slices, bytes) = Ref::<&mut [u8], U32>::from_prefix(bytes)?;
    let num_slices = u32::from(*num_slices) as usize;
    Ok(bytes.split_at_mut(num_slices))
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

impl<'a, T: DeserializeMut<'a>> DeserializeMut<'a> for VecU8<T> {
    type Output = Vec<T::Output>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&mut [u8], u8>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at_mut(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

// Implement ByteLen for VecU8<T>
impl<T> crate::ByteLen for VecU8<T> {
    fn byte_len(&self) -> usize {
        // Vec length + each element length
        1 + core::mem::size_of::<T>()
    }
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
fn test_vecu8() {
    use std::vec;
    let mut bytes = vec![8, 1u8, 2, 3, 4, 5, 6, 7, 8];
    let (vec, remaining_bytes) = VecU8::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(vec, vec![1u8, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(remaining_bytes, &mut []);
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
    assert_eq!(option_value, Some(2));
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
    assert_eq!(value, 255);
    assert_eq!(remaining, &mut []);
    let res = u8::zero_copy_at_mut(&mut []);
    assert_eq!(res, Err(ZeroCopyError::ArraySize(1, 0)));
}

#[test]
fn test_deserialize_mut_u16() {
    let mut bytes = 2323u16.to_le_bytes();
    let (value, remaining) = u16::zero_copy_at_mut(bytes.as_mut_slice()).unwrap();
    assert_eq!(*value, 2323u16);
    assert_eq!(remaining, &mut []);
    let mut value = [0u8];
    let res = u16::zero_copy_at_mut(&mut value);
    // TODO: investigate why error is not Size as in borsh.rs test.
    assert_eq!(res, Err(ZeroCopyError::UnalignedPointer));
}

#[test]
fn test_deserialize_mut_vec() {
    let mut bytes = [2, 0, 0, 0, 1, 2]; // Length 2, followed by values 1 and 2
    let (vec, remaining) = Vec::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(vec, std::vec![1, 2]);
    assert_eq!(remaining, &mut []);
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
fn test_deserialize_mut_vecu8() {
    let mut bytes = [3, 4, 5, 6]; // Length 3, followed by values 4, 5, 6
    let (vec, remaining) = VecU8::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(vec, std::vec![4, 5, 6]);
    assert_eq!(remaining, &mut []);
}

#[cfg(test)]
pub mod test {
    use std::vec;

    use borsh::{BorshDeserialize, BorshSerialize};
    use crate::ByteLen;
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
    //      1.5. If a vector contains a nested vector (does not implement Copy) it  must implement DeserializeMut
    //      1.6. Elements in an Option must implement DeserializeMut
    //      1.7. a type that does not implement Copy must implement DeserializeMut, and is deserialized 1 by 1

    // Derive Macro needs to derive:
    //  1. ZeroCopyStructInnerMut
    // 2. DeserializeMut
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

    impl crate::ByteLen for Struct1 {
        fn byte_len(&self) -> usize {
            self.a.byte_len() + self.b.byte_len()
        }
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

    impl<'a> DerefMut for ZStruct1<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.meta
        }
    }

    impl<'a> DeserializeMut<'a> for Struct1 {
        type Output = ZStruct1<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct1Meta>::from_prefix(bytes)?;
            Ok((ZStruct1 { meta }, bytes))
        }
    }

    #[test]
    fn test_struct_1() {
        let ref_struct = Struct1 { a: 1, b: 2 };
        let mut bytes = ref_struct.try_to_vec().unwrap();
        assert_eq!(ref_struct.byte_len(), bytes.len());
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

    impl crate::ByteLen for Struct2 {
        fn byte_len(&self) -> usize {
            self.a.byte_len() + self.b.byte_len() + self.vec.byte_len()
        }
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
        pub vec: <Vec<u8> as ZeroCopyStructInnerMut>::ZeroCopyInnerMut,
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
        type Target = Ref<&'a mut [u8], ZStruct2Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl<'a> DeserializeMut<'a> for Struct2 {
        type Output = ZStruct2<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct2Meta>::from_prefix(bytes)?;
            let (vec, bytes) = <Vec<u8> as DeserializeMut<'a>>::zero_copy_at_mut(bytes)?;
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
        assert_eq!(ref_struct.byte_len(), bytes.len());
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

    impl crate::ByteLen for Struct3 {
        fn byte_len(&self) -> usize {
            self.a.byte_len() + self.b.byte_len() + self.vec.byte_len() + self.c.byte_len()
        }
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

    impl<'a> DeserializeMut<'a> for Struct3 {
        type Output = ZStruct3<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct3Meta>::from_prefix(bytes)?;
            let (vec, bytes) = ZeroCopySliceMutBorsh::zero_copy_at_mut(bytes)?;
            let (c, bytes) = Ref::<&mut [u8], U64>::from_prefix(bytes)?;
            Ok((Self::Output { meta, vec, c }, bytes))
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
        assert_eq!(ref_struct.byte_len(), bytes.len());
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

    impl crate::ByteLen for Struct4Nested {
        fn byte_len(&self) -> usize {
            core::mem::size_of::<u8>() + core::mem::size_of::<u16>()
        }
    }

    impl<'a> DeserializeMut<'a> for Struct4Nested {
        type Output = ZStruct4Nested;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
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

    impl crate::ByteLen for Struct4 {
        fn byte_len(&self) -> usize {
            self.a.byte_len()
                + self.b.byte_len()
                + self.vec.byte_len()
                + self.c.byte_len()
                + 4 // Vec header size for vec_2
                + self.vec_2.iter().map(|n| n.byte_len()).sum::<usize>()
        }
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

    impl<'a> DeserializeMut<'a> for Struct4 {
        type Output = ZStruct4<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct4Meta>::from_prefix(bytes)?;
            let (vec, bytes) = ZeroCopySliceMutBorsh::from_bytes_at(bytes)?;
            let (c, bytes) =
                Ref::<&mut [u8], <u64 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>::from_prefix(
                    bytes,
                )?;
            let (vec_2, bytes) = ZeroCopySliceMutBorsh::from_bytes_at(bytes)?;
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

    /// TODO:
    /// - add SIZE const generic DeserializeMut trait
    /// - add new with data function
    impl Struct4 {
        // pub fn byte_len(&self) -> usize {
        //     size_of::<u8>()
        //         + size_of::<u16>()
        //         + size_of::<u8>() * self.vec.len()
        //         + size_of::<u64>()
        //         + size_of::<Struct4Nested>() * self.vec_2.len()
        // }

        pub fn new_with_data<'a>(
            bytes: &'a mut [u8],
            data: &Struct4,
        ) -> (ZStruct4<'a>, &'a mut [u8]) {
            let (mut zero_copy, bytes) =
                <Struct4 as DeserializeMut>::zero_copy_at_mut(bytes).unwrap();
            zero_copy.meta.a = data.a;
            zero_copy.meta.b = data.b.into();
            zero_copy
                .vec
                .iter_mut()
                .zip(data.vec.iter())
                .for_each(|(x, y)| *x = *y);
            (zero_copy, bytes)
        }
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
        assert_eq!(ref_struct.byte_len(), bytes.len());
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

    impl crate::ByteLen for Struct5 {
        fn byte_len(&self) -> usize {
            self.a.byte_len()
        }
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    pub struct ZStruct5<'a> {
        pub a: Vec<ZeroCopySliceMutBorsh<'a, <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut>>,
    }

    impl<'a> DeserializeMut<'a> for Struct5 {
        type Output = ZStruct5<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
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
        assert_eq!(ref_struct.byte_len(), bytes.len());

        let (zero_copy, remaining) = Struct5::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(
            zero_copy.a.iter().map(|x| x.to_vec()).collect::<Vec<_>>(),
            vec![vec![1u8; 32]; 32]
        );
        assert_eq!(remaining, &mut []);
    }

    // If a struct inside a vector contains a vector it must implement DeserializeMut.
    #[repr(C)]
    #[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
    pub struct Struct6 {
        pub a: Vec<Struct2>,
    }

    impl crate::ByteLen for Struct6 {
        fn byte_len(&self) -> usize {
            self.a.byte_len()
        }
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    pub struct ZStruct6<'a> {
        pub a: Vec<<Struct2 as DeserializeMut<'a>>::Output>,
    }

    impl<'a> DeserializeMut<'a> for Struct6 {
        type Output = ZStruct6<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
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
        assert_eq!(ref_struct.byte_len(), bytes.len());
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
        pub option: <Option<u8> as ZeroCopyStructInnerMut>::ZeroCopyInnerMut,
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
        type Target = Ref<&'a mut [u8], ZStruct7Meta>;

        fn deref(&self) -> &Self::Target {
            &self.meta
        }
    }

    impl<'a> DeserializeMut<'a> for Struct7 {
        type Output = ZStruct7<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (meta, bytes) = Ref::<&mut [u8], ZStruct7Meta>::from_prefix(bytes)?;
            let (option, bytes) = <Option<u8> as DeserializeMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((ZStruct7 { meta, option }, bytes))
        }
    }

    impl crate::ByteLen for Struct7 {
        fn byte_len(&self) -> usize {
            self.a.byte_len() + self.b.byte_len() + self.option.byte_len()
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
        assert_eq!(ref_struct.byte_len(), bytes.len());
        let (zero_copy, remaining) = Struct7::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(zero_copy.a, 1u8);
        assert_eq!(zero_copy.b, 2u16);
        assert_eq!(zero_copy.option, Some(3));
        assert_eq!(remaining, &mut []);

        let ref_struct = Struct7 {
            a: 1,
            b: 2,
            option: None,
        };
        let mut bytes = ref_struct.try_to_vec().unwrap();
        assert_eq!(ref_struct.byte_len(), bytes.len());
        let (zero_copy, remaining) = Struct7::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(zero_copy.a, 1u8);
        assert_eq!(zero_copy.b, 2u16);
        assert_eq!(zero_copy.option, None);
        assert_eq!(remaining, &mut []);
    }

    // If a struct inside a vector contains a vector it must implement DeserializeMut.
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
        pub a: <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut,
        pub b: <Struct2 as DeserializeMut<'a>>::Output,
    }

    impl<'a> DeserializeMut<'a> for NestedStruct {
        type Output = ZNestedStruct<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (a, bytes) =
                <u8 as ZeroCopyStructInnerMut>::ZeroCopyInnerMut::zero_copy_at_mut(bytes)?;
            let (b, bytes) = <Struct2 as DeserializeMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((ZNestedStruct { a, b }, bytes))
        }
    }

    impl crate::ByteLen for NestedStruct {
        fn byte_len(&self) -> usize {
            self.a.byte_len() + self.b.byte_len()
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
        pub a: Vec<<NestedStruct as DeserializeMut<'a>>::Output>,
    }

    impl<'a> DeserializeMut<'a> for Struct8 {
        type Output = ZStruct8<'a>;

        fn zero_copy_at_mut(
            bytes: &'a mut [u8],
        ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
            let (a, bytes) = Vec::<NestedStruct>::zero_copy_at_mut(bytes)?;
            Ok((ZStruct8 { a }, bytes))
        }
    }

    impl crate::ByteLen for Struct8 {
        fn byte_len(&self) -> usize {
            4 // Vec header size
            + self.a.iter().map(|n| n.byte_len()).sum::<usize>()
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
        assert_eq!(ref_struct.byte_len(), bytes.len());

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
