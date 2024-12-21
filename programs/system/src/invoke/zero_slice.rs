use crate::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccount, PackedMerkleContext},
};
use anchor_lang::solana_program::msg;
use anchor_lang::solana_program::pubkey::Pubkey;
use bytemuck::{bytes_of_mut, checked::from_bytes_mut, Pod, Zeroable};
use core::fmt;
use std::{marker::PhantomData, mem::size_of};

use crate::sdk::compressed_account::PackedCompressedAccountWithMerkleContext;

use super::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext};
use std::{
    ops::{Index, IndexMut},
    ptr::{self},
};

pub trait HasRef<'a> {
    type Ref;

    fn as_ref(&'a mut self) -> Self::Ref;
}

#[derive(Debug)]
pub struct ZeroSliceMut<'a, T> {
    ptrs: Vec<*mut T>,
    _marker: std::marker::PhantomData<&'a mut T>,
}

pub struct ZeroSlice<'a, T> {
    ptrs: Vec<*const T>,
    _marker: std::marker::PhantomData<&'a T>,
}

pub trait Length {
    fn size(bytes: &mut [u8]) -> usize;
}

macro_rules! impl_length_for_integer_type {
    ($int_ty:ty) => {
        impl Length for $int_ty {
            fn size(_bytes: &mut [u8]) -> usize {
                let len = std::mem::size_of::<$int_ty>();
                len
            }
        }
    };
}

impl_length_for_integer_type!(u8);
impl_length_for_integer_type!(u16);
impl_length_for_integer_type!(u32);
impl_length_for_integer_type!(u64);
impl_length_for_integer_type!(usize);
impl_length_for_integer_type!([u8; 8]);
impl_length_for_integer_type!([u8; 32]);
impl_length_for_integer_type!([u8; 64]);
impl_length_for_integer_type!(Pubkey);

pub trait ReadUnalignedMut<'a> {
    fn read_unaligned_mut(bytes: &'a mut [u8]) -> &'a mut Self;
}
pub trait ReadUnalignedOptionMut<'a, T> {
    fn read_option_unaligned_mut(bytes: &'a mut [u8]) -> Option<&mut T>;
}
trait ReadUnaligned<'a> {
    fn read_unaligned(bytes: &'a [u8]) -> &'a Self;
}

macro_rules! impl_read_unaligned_mut {
    ($int_ty:ty) => {
        impl<'a> ReadUnalignedMut<'a> for $int_ty {
            fn read_unaligned_mut(bytes: &'a mut [u8]) -> &'a mut Self {
                assert!(
                    bytes.len() >= std::mem::size_of::<Self>(),
                    "Slice too small"
                );
                unsafe { &mut *(bytes.as_mut_ptr() as *mut $int_ty) }
            }
        }
    };
}

macro_rules! impl_read_unaligned {
    ($int_ty:ty) => {
        impl<'a> ReadUnaligned<'a> for $int_ty {
            fn read_unaligned(bytes: &'a [u8]) -> &'a Self {
                // assert!(
                //     bytes.len() >= std::mem::size_of::<Self>(),
                //     "Slice too small"
                // );
                // assert!(
                //     bytes.as_ptr().align_offset(std::mem::align_of::<Self>()) == 0,
                //     "Memory not aligned"
                // );
                unsafe { &*(bytes.as_ptr() as *const $int_ty) }
            }
        }
    };
}
impl_read_unaligned!(u8);
impl_read_unaligned!(u16);
impl_read_unaligned!(u32);
impl_read_unaligned!(u64);
impl_read_unaligned!(usize);
impl_read_unaligned!([u8; 8]);
impl_read_unaligned!([u8; 32]);
impl_read_unaligned!([u8; 64]);
impl_read_unaligned!(CompressedProof);

impl_read_unaligned_mut!(u8);
impl_read_unaligned_mut!(u16);
impl_read_unaligned_mut!(u32);
impl_read_unaligned_mut!(u64);
impl_read_unaligned_mut!(usize);
impl_read_unaligned_mut!([u8; 8]);
impl_read_unaligned_mut!([u8; 32]);
impl_read_unaligned_mut!([u8; 64]);
impl_read_unaligned_mut!(CompressedProof);
// impl_length_for_struct_type!(CompressedProof);

// fn num_bytes_option<T: Pod + Zeroable>(bytes: &mut [u8], start_off_set: &mut usize) -> usize {}
impl<T> Length for Option<T>
where
    T: Length,
{
    fn size(bytes: &mut [u8]) -> usize {
        let flag = bytes[0];
        let start_off_set = 1; // Move past the flag
        println!("flag {:?}", flag);
        println!("start_off_set option {:?}", start_off_set);
        if flag == 1 {
            let (_len_bytes, bytes) = bytes.split_at_mut(start_off_set);
            let len = T::size(bytes); // 1 for the flag
            println!("len {:?}", len);
            // *start_off_set += len;
            len + 1
        } else {
            1
        }
    }
}

impl Length for bool {
    fn size(_bytes: &mut [u8]) -> usize {
        1
    }
}
impl<'a> ReadUnalignedMut<'a> for bool {
    fn read_unaligned_mut(bytes: &mut [u8]) -> &mut Self {
        unsafe { &mut *(bytes.as_mut_ptr() as *mut bool) }
    }
}

impl<'a, T> ReadUnalignedOptionMut<'a, T> for Option<T>
where
    T: Pod + Zeroable + Length + ReadUnalignedMut<'a> + fmt::Debug + Clone,
{
    fn read_option_unaligned_mut(bytes: &'a mut [u8]) -> Option<&mut T> {
        assert!(
            bytes.len() > 0,
            "Bytes slice is too small to determine Option discriminant"
        );

        if bytes[0] == 1 {
            Some(&mut *T::read_unaligned_mut(&mut bytes[1..]))
        } else {
            None
        }
    }
}

impl<'a, T> Length for ZeroSliceMut<'a, T>
where
    T: Length,
{
    // TODO: need to implement a get len method for nested structs
    fn size(bytes: &mut [u8]) -> usize {
        let start_off_set = 4;

        // Extract vector length (assume little-endian encoded u32 for length)
        let (len_bytes, bytes) = bytes.split_at_mut(start_off_set);
        let vec_len = unsafe { ptr::read_unaligned(len_bytes.as_ptr() as *const u32) as usize };
        if vec_len == 0 {
            return start_off_set;
        }
        // Ensure there is enough data for `vec_len` elements
        let required_size = vec_len * T::size(bytes);
        // *start_off_set += required_size;
        if bytes.len() < required_size {
            panic!(
                "Not enough bytes to deserialize: required {}, found {}",
                required_size,
                bytes.len()
            );
        }
        return required_size + start_off_set;
    }
}

impl<T> Length for Vec<T>
where
    T: Length,
{
    fn size(bytes: &mut [u8]) -> usize {
        let start_off_set = 4;
        // Extract vector length (assume little-endian encoded u32 for length)
        let (len_bytes, bytes) = bytes.split_at_mut(start_off_set);

        let vec_len = unsafe { ptr::read_unaligned(len_bytes.as_ptr() as *const u32) as usize };

        // Ensure there is enough data for `vec_len` elements
        let required_size = vec_len * T::size(bytes);
        // *start_off_set += required_size;

        if bytes.len() < required_size {
            panic!(
                "Not enough bytes to deserialize: required {}, found {}",
                required_size,
                bytes.len()
            );
        }
        return required_size;
    }
}

impl<'a, T> ZeroSliceMut<'a, T>
where
    T: Length + fmt::Debug + Clone,
{
    /// Deserialize from unaligned memory.
    pub fn deserialize_unaligned(bytes: &'a mut [u8]) -> Self {
        let (len_bytes, bytes) = bytes.split_at_mut(4);
        let len = *u32::read_unaligned(len_bytes) as usize;
        // Create mutable pointers for each element
        let mut ptrs = Vec::with_capacity(len);
        let mut start_off_set = 0;
        for _ in 0..len {
            let length = T::size(bytes);
            // TODO: replace with read_unaligned_mut, deserialize_unaligned, or read_option_unaligned_mut
            let ptr = bytes[start_off_set..start_off_set + length].as_mut_ptr() as *mut T;
            start_off_set += length;
            ptrs.push(ptr);
        }
        Self {
            ptrs,
            _marker: std::marker::PhantomData,
        }
    }
}

// Implement immutable indexing
impl<'a, T: Copy> Index<usize> for ZeroSliceMut<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.ptrs.len() {
            panic!("Index out of bounds");
        }
        unsafe { &*self.ptrs[index] }
    }
}

// Implement mutable indexing
impl<'a, T: Copy> IndexMut<usize> for ZeroSliceMut<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.ptrs.len() {
            panic!("Index out of bounds");
        }
        unsafe { &mut *self.ptrs[index] }
    }
}

// Iterator for ZeroSliceMut
pub struct ZeroSliceIter<'a, T> {
    ptrs: std::slice::Iter<'a, *mut T>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for ZeroSliceIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.ptrs.next().map(|&ptr| unsafe { &*ptr })
    }
}

impl<'a, T> ZeroSliceMut<'a, T> {
    pub fn iter(&'a self) -> ZeroSliceIter<'a, T> {
        ZeroSliceIter {
            ptrs: self.ptrs.iter(),
            _marker: PhantomData,
        }
    }
}
impl<'a, T> ZeroSliceMut<'a, T> {
    pub fn len(&self) -> usize {
        self.ptrs.len()
    }
}
// Mutable iterator for ZeroSliceMut
pub struct ZeroSliceIterMut<'a, T> {
    ptrs: std::slice::IterMut<'a, *mut T>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for ZeroSliceIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.ptrs.next().map(|&mut ptr| unsafe { &mut *ptr })
    }
}

impl<'a, T> ZeroSliceMut<'a, T> {
    pub fn iter_mut(&'a mut self) -> ZeroSliceIterMut<'a, T> {
        ZeroSliceIterMut {
            ptrs: self.ptrs.iter_mut(),
            _marker: PhantomData,
        }
    }
}

impl<'a> InstructionDataInvoke<'a> {
    pub fn derserialize_borsh(bytes: &'a mut [u8]) -> Self {
        let mut start_off_set = 0;
        println!("bytes 0..4 {:?}", bytes.to_vec());
        println!("off_set start {}", start_off_set);
        let split = Option::<CompressedProof>::size(bytes);
        start_off_set += split;
        println!("off_set 0 {:?}", start_off_set); // 129 Some(proof)
        println!("len {:?}", split);
        let (proof_bytes, bytes) = bytes.split_at_mut(split);
        println!("len bytes {:?}", bytes.len());
        let proof = Option::<CompressedProof>::read_option_unaligned_mut(proof_bytes);
        println!("bytes 0..4 {:?}", bytes.to_vec());
        println!("off_set 1 {:?}", start_off_set); // 129 ok
        let len = ZeroSliceMut::<PackedCompressedAccountWithMerkleContext>::size(bytes);
        start_off_set += len;
        println!("off_set 2 {:?}", start_off_set); // 133 empty vec
        println!("len {:?}", len);
        let (proof_bytes, bytes) = bytes.split_at_mut(len);
        println!("len bytes {:?}", bytes.len());
        println!("bytes 0..4 {:?}", bytes.to_vec());

        let input_compressed_accounts_with_merkle_context =
            ZeroSliceMut::deserialize_unaligned(proof_bytes);

        let len = ZeroSliceMut::<OutputCompressedAccountWithPackedContext>::size(bytes);
        start_off_set += len;
        println!("off_set 3 {:?}", start_off_set); // 137 empty vec
        println!("len {:?}", len);
        let (proof_bytes, bytes) = bytes.split_at_mut(len);
        println!("len bytes {:?}", bytes.len());

        let output_compressed_accounts = ZeroSliceMut::deserialize_unaligned(proof_bytes);
        println!("bytes 0..4 {:?}", bytes.to_vec());

        let len: usize = Option::<u64>::size(bytes);
        start_off_set += len;
        println!("off_set 4 {:?}", start_off_set); // 138 None
        println!("relay_fee len {:?}", len);
        let (proof_bytes, bytes) = bytes.split_at_mut(len);
        println!("len bytes {:?}", bytes.len());

        let relay_fee = Option::read_option_unaligned_mut(proof_bytes);

        let len = ZeroSliceMut::<NewAddressParamsPacked>::size(bytes);
        println!("bytes 0..4 {:?}", bytes.to_vec());
        println!("bytes 134..138 {:?}", bytes[134..138].to_vec());
        println!("bytes 138..142 {:?}", bytes[138..142].to_vec());
        start_off_set += len;
        println!("off_set 5 {:?}", start_off_set); // 142
        println!("address len {:?}", len);

        let (proof_bytes, bytes) = bytes.split_at_mut(len);
        let new_address_params = ZeroSliceMut::deserialize_unaligned(proof_bytes);

        let split: usize = Option::<u64>::size(bytes);
        start_off_set += split;
        println!("off_set 6 {:?}", start_off_set);
        println!("compress_or_decompress_lamports len {:?}", split);

        let (proof_bytes, bytes) = bytes.split_at_mut(split);
        let compress_or_decompress_lamports = Option::<u64>::read_option_unaligned_mut(proof_bytes);

        let len = bool::size(bytes);
        let (proof_bytes, bytes) = bytes.split_at_mut(len);
        let is_compress = bool::read_unaligned_mut(proof_bytes);

        let res = Self {
            proof,
            input_compressed_accounts_with_merkle_context,
            output_compressed_accounts,
            relay_fee,
            new_address_params,
            compress_or_decompress_lamports,
            is_compress,
        };
        return res;
    }
}

/// #[derive(MutableRef)]:
/// - Options become Option<&'a mut T>
/// - Vecs become ZeroSliceMut<'a, T>
/// - Other types become &'a mut T
/// - struct is #[repr(C)]
///
/// #[derive(MutableRef)]
// pub struct TestStruct {
//     pub a: u64,
//     pub b: u64,
// }

// The macro generates:

// pub struct TestStructRef<'a> {
//     pub a: &'a mut u64,
//     pub b: &'a mut u64,
// }

// impl<'a> HasRef<'a> for TestStruct {
//     type Ref = TestStructRef<'a>;

//     fn as_ref(&'a mut self) -> Self::Ref {
//         TestStructRef {
//             a: &mut self.a,
//             b: &mut self.b,
//         }
//     }
// }

#[repr(C)]
#[derive(Debug)]
pub struct InstructionDataInvoke<'a> {
    pub proof: Option<&'a mut CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        ZeroSliceMut<'a, PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: ZeroSliceMut<'a, OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<&'a mut u64>,
    pub new_address_params: ZeroSliceMut<'a, NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<&'a mut u64>,
    pub is_compress: &'a mut bool,
}

use borsh::BorshSerialize;
use light_macros::DeriveLength;
#[derive(DeriveLength, BorshSerialize)]
pub struct MyStruct {
    pub a: u32,
    pub b: Option<u64>,
    pub c: u16,
}

#[cfg(test)]
mod tests {
    use crate::invoke::processor::CompressedProof;

    use super::*;

    #[test]
    fn test_borsh_zero_copy() {
        let original = crate::invoke::instruction::InstructionDataInvoke {
            proof: Some(CompressedProof {
                a: [1; 32],
                b: [2; 64],
                c: [3; 32],
            }),
            input_compressed_accounts_with_merkle_context: vec![],
            output_compressed_accounts: vec![],
            relay_fee: Some(1),
            new_address_params: vec![
                NewAddressParamsPacked {
                    seed: [1; 32],
                    address_merkle_tree_account_index: 2,
                    address_queue_account_index: 3,
                    address_merkle_tree_root_index: 4,
                };
                4
            ],
            compress_or_decompress_lamports: Some(1),
            is_compress: false,
        };

        // Serialize the original object using Borsh
        let mut serialized_data = original.try_to_vec().unwrap();
        println!("serialized_data {:?}", serialized_data);
        println!("initial len {:?}", serialized_data.len());
        let mut first_addresses = serialized_data[138 + 4..142 + 36].to_vec();
        println!("serialized_data seeed {:?}", first_addresses);
        // let man_des = light_batched_merkle_tree::zero_copy::bytes_to_struct_unchecked::<
        //     NewAddressParamsPacked,
        // >(&mut first_addresses)
        // .unwrap();
        // println!("man_des {:?}", man_des);
        println!("serialized_data u8 {:?}", serialized_data[142 + 32]);
        println!("serialized_data u8 {:?}", serialized_data[142 + 33]);
        println!(
            "serialized_data u16 {:?}",
            u16::from_le_bytes(
                serialized_data[142 + 34..142 + 36]
                    .to_vec()
                    .try_into()
                    .unwrap()
            )
        );

        // Deserialize the serialized data using derserialize_borsh
        let mut deserialized = InstructionDataInvoke::derserialize_borsh(&mut serialized_data);
        println!("deserialized {:?}", deserialized);
        // Compare the original and deserialized objects
        assert_eq!(original.proof.unwrap(), *deserialized.proof.unwrap());
        assert!(deserialized.relay_fee.is_some());
        assert_eq!(
            original.new_address_params.len(),
            deserialized.new_address_params.len()
        );
        deserialized
            .new_address_params
            .iter()
            .zip(original.new_address_params.iter())
            .enumerate()
            .for_each(|(i, (x, y))| {
                println!("i{:?}", i);
                assert_eq!(*x, *y);
            });
        assert_eq!(
            original.compress_or_decompress_lamports.is_none(),
            deserialized.compress_or_decompress_lamports.is_none()
        );
        assert_eq!(original.is_compress, *deserialized.is_compress);
        assert_eq!(
            original.input_compressed_accounts_with_merkle_context.len(),
            deserialized
                .input_compressed_accounts_with_merkle_context
                .len()
        );
        *deserialized.is_compress = false;
        let mut deserialized = InstructionDataInvoke::derserialize_borsh(&mut serialized_data);
        assert_eq!(*deserialized.is_compress, false);
    }
    // fn deserialize_option<'a, T: Pod + Zeroable>(
    //     -    bytes: &'a mut [u8],
    //     -    start_off_set: &mut usize,
    //     -) -> Option<&'a mut T> {
    //     -    let proof = if bytes[0] == 1 {
    //     -        *start_off_set += size_of::<u8>();
    //     -        let proof = from_bytes_mut::<T>(&mut bytes[1..]);
    //     -        *start_off_set += size_of::<T>();
    //     -        Some(proof)
    //     -    } else {
    //     -        *start_off_set += size_of::<u8>();
    //     -        None
    //     -    };
    //     -    proof
    //     -}
    #[test]
    fn test_mystruct_length() {
        let mut bytes = vec![0; 100]; // Example byte slice for testing
        let mut offset = 0;
        let expected_length = std::mem::size_of::<u32>() // size of `a`
            + 1 //+ std::mem::size_of::<u64>() // size of `b` (Option<u64>: 1-byte flag + u64 size)
            + std::mem::size_of::<u16>(); // size of `c`

        let actual_length = MyStruct::size(&mut bytes);

        assert_eq!(actual_length, expected_length);

        let mut bytes = Vec::new(); // Example byte slice for testing
        let mut offset = 0;
        let expected_length = std::mem::size_of::<u32>() // size of `a`
            + 1 + std::mem::size_of::<u64>() // size of `b` (Option<u64>: 1-byte flag + u64 size)
            + std::mem::size_of::<u16>(); // size of `c`
        let test = MyStruct {
            a: 1,
            b: Some(2),
            c: 3,
        };
        test.serialize(&mut bytes).unwrap();
        println!("bytes {:?}", bytes);

        let actual_length = MyStruct::size(&mut bytes);

        assert_eq!(actual_length, expected_length);
    }
}
