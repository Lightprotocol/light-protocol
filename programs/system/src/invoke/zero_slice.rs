use crate::invoke::processor::CompressedProof;
use anchor_lang::solana_program::msg;
use bytemuck::{bytes_of_mut, checked::from_bytes_mut, Pod, Zeroable};
use std::{marker::PhantomData, mem::size_of};

use crate::sdk::compressed_account::PackedCompressedAccountWithMerkleContext;

use super::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext};
use core::slice;
use std::{
    ops::{Index, IndexMut},
    ptr::{self},
};
pub struct ZeroSlice<'a, T> {
    ptrs: Vec<*mut T>,
    _marker: std::marker::PhantomData<&'a mut T>,
}

impl<'a, T> ZeroSlice<'a, T> {
    // TODO: need to implement a get len method for nested structs
    pub fn get_bytes_len(bytes: &'a mut [u8], start_off_set: &mut usize) -> (usize, usize) {
        msg!("start_off_set{:?}", start_off_set);
        // Extract vector length (assume little-endian encoded u32 for length)
        let (len_bytes, data_bytes) = bytes.split_at_mut(4);
        *start_off_set += 4;
        let vec_len = unsafe { ptr::read_unaligned(len_bytes.as_ptr() as *const u32) as usize };
        msg!("vec_len{:?}", vec_len);
        // Ensure there is enough data for `vec_len` elements
        let required_size = vec_len * std::mem::size_of::<T>();
        *start_off_set += required_size;
        msg!("required_size{:?}", required_size);
        msg!("data_bytes.len(){:?}", data_bytes.len());
        if data_bytes.len() < required_size {
            panic!(
                "Not enough bytes to deserialize: required {}, found {}",
                required_size,
                data_bytes.len()
            );
        }
        return (vec_len, required_size + 4);
    }

    /// Deserialize from unaligned memory.
    pub fn deserialize_unaligned(bytes: &'a mut [u8], vec_len: usize) -> Self {
        // Create mutable pointers for each element
        let mut ptrs = Vec::with_capacity(vec_len);
        let mut start_off_set = 4;
        for i in 0..vec_len {
            let ptr = unsafe { bytes.as_mut_ptr().add(start_off_set) as *mut T };
            start_off_set += std::mem::size_of::<T>();
            ptrs.push(ptr);
        }
        msg!("ptrs{:?}", ptrs);
        Self {
            ptrs,
            _marker: std::marker::PhantomData,
        }
    }
}

// Implement immutable indexing
impl<'a, T: Copy> Index<usize> for ZeroSlice<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.ptrs.len() {
            panic!("Index out of bounds");
        }
        unsafe { &*self.ptrs[index] }
    }
}

// Implement mutable indexing
impl<'a, T: Copy> IndexMut<usize> for ZeroSlice<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.ptrs.len() {
            panic!("Index out of bounds");
        }
        unsafe { &mut *self.ptrs[index] }
    }
}

// Iterator for ZeroSlice
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

impl<'a, T> ZeroSlice<'a, T> {
    pub fn iter(&'a self) -> ZeroSliceIter<'a, T> {
        ZeroSliceIter {
            ptrs: self.ptrs.iter(),
            _marker: PhantomData,
        }
    }
}

// Mutable iterator for ZeroSlice
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

impl<'a, T> ZeroSlice<'a, T> {
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

        let split = num_bytes_option::<CompressedProof>(bytes, &mut start_off_set);
        msg!("proof split{:?}", split);

        let (proof_bytes, bytes) = bytes.split_at_mut(start_off_set);
        let proof: Option<&mut CompressedProof> = option(proof_bytes, &mut 0);
        msg!("proof {:?}", proof);
        let (len, size) = ZeroSlice::<PackedCompressedAccountWithMerkleContext>::get_bytes_len(
            bytes,
            &mut start_off_set,
        );
        msg!("size {:?}", size);
        msg!("bytes len{:?}", bytes.len());
        let (proof_bytes, bytes) = bytes.split_at_mut(size);
        let input_compressed_accounts_with_merkle_context =
            ZeroSlice::deserialize_unaligned(proof_bytes, len);
        let (len, size) = ZeroSlice::<OutputCompressedAccountWithPackedContext>::get_bytes_len(
            bytes,
            &mut start_off_set,
        );
        msg!("bytes len{:?}", bytes.len());
        msg!("size {:?}", size);
        let (proof_bytes, bytes) = bytes.split_at_mut(size);
        let output_compressed_accounts = ZeroSlice::deserialize_unaligned(proof_bytes, len);
        let split: usize = num_bytes_option::<u64>(bytes, &mut start_off_set);
        msg!("bytes len{:?}", bytes.len());
        msg!("size 2 {:?}", split);
        let (proof_bytes, bytes) = bytes.split_at_mut(split);
        let relay_fee = option(proof_bytes, &mut start_off_set);
        let (len, size) =
            ZeroSlice::<NewAddressParamsPacked>::get_bytes_len(bytes, &mut start_off_set);
        msg!("size 3 {:?}", size);
        let (proof_bytes, bytes) = bytes.split_at_mut(size);
        let new_address_params = ZeroSlice::deserialize_unaligned(proof_bytes, len);
        let split: usize = num_bytes_option::<u64>(bytes, &mut start_off_set);
        msg!("split 4 {:?}", split);
        let (proof_bytes, bytes) = bytes.split_at_mut(split);
        let compress_or_decompress_lamports = option(proof_bytes, &mut start_off_set);
        let is_compress = bytes_of_mut::<u8>(&mut bytes[0]);

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

fn option<'a, T: Pod + Zeroable>(
    bytes: &'a mut [u8],
    start_off_set: &mut usize,
) -> Option<&'a mut T> {
    let proof = if bytes[0] == 1 {
        *start_off_set += size_of::<u8>();
        let proof = from_bytes_mut::<T>(&mut bytes[1..]);
        *start_off_set += size_of::<T>();
        Some(proof)
    } else {
        *start_off_set += size_of::<u8>();
        None
    };
    proof
}

fn num_bytes_option<T: Pod + Zeroable>(bytes: &mut [u8], start_off_set: &mut usize) -> usize {
    let len = if bytes[0] == 1 { size_of::<T>() + 1 } else { 1 };
    *start_off_set += len;
    len
}

pub struct InstructionDataInvoke<'a> {
    pub proof: Option<&'a mut CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        ZeroSlice<'a, PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: ZeroSlice<'a, OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<&'a mut u64>,
    pub new_address_params: ZeroSlice<'a, NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<&'a mut u64>,
    pub is_compress: &'a mut [u8],
}

#[cfg(test)]
mod tests {
    use crate::invoke::processor::CompressedProof;

    use super::*;
    use borsh::ser::BorshSerialize;
    #[test]
    fn test_instruction_data_invoke_borsh_serialization() {
        let original = crate::invoke::instruction::InstructionDataInvoke {
            proof: Some(CompressedProof {
                a: [1; 32],
                b: [2; 64],
                c: [3; 32],
            }),
            input_compressed_accounts_with_merkle_context: vec![],
            output_compressed_accounts: vec![],
            relay_fee: None,
            new_address_params: vec![NewAddressParamsPacked::default(); 4],
            compress_or_decompress_lamports: None,
            is_compress: false,
        };

        // Serialize the original object using Borsh
        let mut serialized_data = original.try_to_vec().unwrap();
        msg!("initial len {:?}", serialized_data.len());

        // Deserialize the serialized data using derserialize_borsh
        let deserialized = InstructionDataInvoke::derserialize_borsh(&mut serialized_data);

        // Compare the original and deserialized objects
        assert_eq!(original.proof.unwrap(), *deserialized.proof.unwrap());
        assert_eq!(None, deserialized.relay_fee);
        deserialized.new_address_params.iter().for_each(|x| {
            assert_eq!(NewAddressParamsPacked::default(), *x);
        });
    }
}
