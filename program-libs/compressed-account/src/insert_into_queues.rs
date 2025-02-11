use std::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

use light_zero_copy::{
    borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySlice, slice_mut::ZeroCopySliceMut,
};
use zerocopy::{
    little_endian::{U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use crate::pubkey::Pubkey;

#[repr(C)]
#[derive(
    KnownLayout, IntoBytes, Immutable, Copy, Clone, FromBytes, PartialEq, Debug, Unaligned,
)]
pub struct InsertNullifierInput {
    pub account_hash: [u8; 32],
    pub leaf_index: U32,
    pub prove_by_index: u8,
    pub tree_index: u8,
    pub queue_index: u8,
}

impl InsertNullifierInput {
    pub fn prove_by_index(&self) -> bool {
        self.prove_by_index == 1
    }
}

#[repr(C)]
#[derive(
    KnownLayout, IntoBytes, Immutable, Copy, Clone, FromBytes, PartialEq, Debug, Unaligned,
)]
pub struct AppendLeavesInput {
    pub account_index: u8,
    pub leaf: [u8; 32],
}
#[repr(C)]
#[derive(
    KnownLayout, IntoBytes, Immutable, Copy, Clone, FromBytes, PartialEq, Debug, Unaligned,
)]
pub struct InsertAddressInput {
    pub address: [u8; 32],
    pub tree_index: u8,
    pub queue_index: u8,
}

#[repr(C)]
#[derive(
    FromBytes, IntoBytes, KnownLayout, Immutable, Copy, Clone, PartialEq, Debug, Unaligned,
)]
pub struct MerkleTreeSequenceNumber {
    pub pubkey: Pubkey,
    /// For output queues the sequence number is the first leaf index.
    pub seq: U64,
}

#[derive(Debug, Clone)]
pub struct InsertIntoQueuesInstructionData<'a> {
    meta: Ref<&'a [u8], InsertIntoQueuesInstructionDataMeta>,
    pub leaves: ZeroCopySlice<'a, u8, AppendLeavesInput, false>,
    pub nullifiers: ZeroCopySlice<'a, u8, InsertNullifierInput, false>,
    pub addresses: ZeroCopySlice<'a, u8, InsertAddressInput, false>,
    pub sequence_numbers: ZeroCopySlice<'a, u8, MerkleTreeSequenceNumber, false>,
    pub output_leaf_indices: ZeroCopySlice<'a, u8, U32, false>,
}

impl InsertIntoQueuesInstructionData<'_> {
    pub fn is_invoked_by_program(&self) -> bool {
        self.meta.is_invoked_by_program == 1
    }
}

impl Deref for InsertIntoQueuesInstructionData<'_> {
    type Target = InsertIntoQueuesInstructionDataMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> Deserialize<'a> for InsertIntoQueuesInstructionData<'a> {
    type Output = Self;
    fn zero_copy_at(bytes: &'a [u8]) -> std::result::Result<(Self, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], InsertIntoQueuesInstructionDataMeta>::from_prefix(bytes)?;

        let (leaves, bytes) = ZeroCopySlice::<u8, AppendLeavesInput, false>::from_bytes_at(bytes)?;

        let (nullifiers, bytes) =
            ZeroCopySlice::<u8, InsertNullifierInput, false>::from_bytes_at(bytes)?;

        let (addresses, bytes) =
            ZeroCopySlice::<u8, InsertAddressInput, false>::from_bytes_at(bytes)?;
        let (sequence_numbers, bytes) =
            ZeroCopySlice::<u8, MerkleTreeSequenceNumber, false>::from_bytes_at(bytes)?;

        let output_leaf_indices =
            ZeroCopySlice::<u8, zerocopy::little_endian::U32, false>::from_bytes(bytes)?;
        Ok((
            InsertIntoQueuesInstructionData {
                meta,
                leaves,
                nullifiers,
                addresses,
                sequence_numbers,
                output_leaf_indices,
            },
            bytes,
        ))
    }
}

#[repr(C)]
#[derive(
    FromBytes, IntoBytes, KnownLayout, Immutable, Copy, Clone, PartialEq, Debug, Unaligned,
)]
pub struct InsertIntoQueuesInstructionDataMeta {
    is_invoked_by_program: u8,
    pub bump: u8,
    pub num_queues: u8,
    pub num_output_queues: u8,
    pub start_output_appends: u8,
    pub num_address_queues: u8,
    pub tx_hash: [u8; 32],
}

#[derive(Debug)]
pub struct InsertIntoQueuesInstructionDataMut<'a> {
    meta: Ref<&'a mut [u8], InsertIntoQueuesInstructionDataMeta>,
    pub leaves: ZeroCopySliceMut<'a, u8, AppendLeavesInput, false>,
    pub nullifiers: ZeroCopySliceMut<'a, u8, InsertNullifierInput, false>,
    pub addresses: ZeroCopySliceMut<'a, u8, InsertAddressInput, false>,
    pub sequence_numbers: ZeroCopySliceMut<'a, u8, MerkleTreeSequenceNumber, false>,
    pub output_leaf_indices: ZeroCopySliceMut<'a, u8, U32, false>,
}

impl<'a> InsertIntoQueuesInstructionDataMut<'a> {
    pub fn is_invoked_by_program(&self) -> bool {
        self.meta.is_invoked_by_program == 1
    }

    pub fn set_invoked_by_program(&mut self, value: bool) {
        self.meta.is_invoked_by_program = value as u8;
    }

    pub fn required_size_for_capacity(
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
        num_output_trees: u8,
    ) -> usize {
        size_of::<InsertIntoQueuesInstructionDataMeta>()
            + ZeroCopySliceMut::<u8, AppendLeavesInput, false>::required_size_for_capacity(
                leaves_capacity,
            )
            + ZeroCopySliceMut::<u8, InsertNullifierInput, false>::required_size_for_capacity(
                nullifiers_capacity,
            )
            + ZeroCopySliceMut::<u8, InsertAddressInput, false>::required_size_for_capacity(
                addresses_capacity,
            )
            + ZeroCopySliceMut::<u8, MerkleTreeSequenceNumber, false>::required_size_for_capacity(
                num_output_trees,
            )
            + ZeroCopySliceMut::<u8, U32, false>::required_size_for_capacity(leaves_capacity)
    }

    pub fn new(
        bytes: &'a mut [u8],
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
        num_output_trees: u8,
    ) -> std::result::Result<Self, ZeroCopyError> {
        let (meta, bytes) =
            Ref::<&mut [u8], InsertIntoQueuesInstructionDataMeta>::from_prefix(bytes)?;
        let (leaves, bytes) =
            ZeroCopySliceMut::<u8, AppendLeavesInput, false>::new_at(leaves_capacity, bytes)?;
        let (nullifiers, bytes) = ZeroCopySliceMut::<u8, InsertNullifierInput, false>::new_at(
            nullifiers_capacity,
            bytes,
        )?;
        let (addresses, bytes) =
            ZeroCopySliceMut::<u8, InsertAddressInput, false>::new_at(addresses_capacity, bytes)?;
        let (sequence_numbers, bytes) =
            ZeroCopySliceMut::<u8, MerkleTreeSequenceNumber, false>::new_at(
                num_output_trees,
                bytes,
            )?;
        let output_leaf_indices = ZeroCopySliceMut::<u8, U32, false>::new(leaves_capacity, bytes)?;
        Ok(InsertIntoQueuesInstructionDataMut {
            meta,
            leaves,
            nullifiers,
            addresses,
            sequence_numbers,
            output_leaf_indices,
        })
    }
}

impl Deref for InsertIntoQueuesInstructionDataMut<'_> {
    type Target = InsertIntoQueuesInstructionDataMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl DerefMut for InsertIntoQueuesInstructionDataMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.meta
    }
}
