use core::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

use light_zero_copy::{
    errors::ZeroCopyError, slice::ZeroCopySlice, slice_mut::ZeroCopySliceMut, traits::ZeroCopyAt,
};
use zerocopy::{
    little_endian::{U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use crate::{
    discriminators::DISCRIMINATOR_INSERT_INTO_QUEUES, pubkey::Pubkey, InstructionDiscriminator,
    TreeType,
};

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
    pub tree_pubkey: Pubkey,
    pub queue_pubkey: Pubkey,
    pub tree_type: U64,
    /// For output queues the sequence number is the first leaf index.
    pub seq: U64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InsertIntoQueuesInstructionData<'a> {
    meta: Ref<&'a [u8], InsertIntoQueuesInstructionDataMeta>,
    pub leaves: ZeroCopySlice<'a, u8, AppendLeavesInput, false>,
    pub nullifiers: ZeroCopySlice<'a, u8, InsertNullifierInput, false>,
    pub addresses: ZeroCopySlice<'a, u8, InsertAddressInput, false>,
    pub output_sequence_numbers: ZeroCopySlice<'a, u8, MerkleTreeSequenceNumber, false>,
    pub input_sequence_numbers: ZeroCopySlice<'a, u8, MerkleTreeSequenceNumber, false>,
    pub address_sequence_numbers: ZeroCopySlice<'a, u8, MerkleTreeSequenceNumber, false>,
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

impl InstructionDiscriminator for InsertIntoQueuesInstructionData<'_> {
    fn discriminator(&self) -> &'static [u8] {
        &DISCRIMINATOR_INSERT_INTO_QUEUES
    }
}

impl<'a> ZeroCopyAt<'a> for InsertIntoQueuesInstructionData<'a> {
    type ZeroCopyAt = Self;
    fn zero_copy_at(bytes: &'a [u8]) -> core::result::Result<(Self, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], InsertIntoQueuesInstructionDataMeta>::from_prefix(bytes)?;

        let (leaves, bytes) = ZeroCopySlice::<u8, AppendLeavesInput, false>::from_bytes_at(bytes)?;

        let (nullifiers, bytes) =
            ZeroCopySlice::<u8, InsertNullifierInput, false>::from_bytes_at(bytes)?;

        let (addresses, bytes) =
            ZeroCopySlice::<u8, InsertAddressInput, false>::from_bytes_at(bytes)?;
        let (output_sequence_numbers, bytes) =
            ZeroCopySlice::<u8, MerkleTreeSequenceNumber, false>::from_bytes_at(bytes)?;
        let (input_sequence_numbers, bytes) =
            ZeroCopySlice::<u8, MerkleTreeSequenceNumber, false>::from_bytes_at(bytes)?;
        let (address_sequence_numbers, bytes) =
            ZeroCopySlice::<u8, MerkleTreeSequenceNumber, false>::from_bytes_at(bytes)?;
        let (output_leaf_indices, bytes) =
            ZeroCopySlice::<u8, zerocopy::little_endian::U32, false>::from_bytes_at(bytes)?;
        Ok((
            InsertIntoQueuesInstructionData {
                meta,
                leaves,
                nullifiers,
                addresses,
                output_sequence_numbers,
                input_sequence_numbers,
                address_sequence_numbers,
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
    pub output_sequence_numbers: ZeroCopySliceMut<'a, u8, MerkleTreeSequenceNumber, false>,
    pub input_sequence_numbers: ZeroCopySliceMut<'a, u8, MerkleTreeSequenceNumber, false>,
    pub address_sequence_numbers: ZeroCopySliceMut<'a, u8, MerkleTreeSequenceNumber, false>,
    pub output_leaf_indices: ZeroCopySliceMut<'a, u8, U32, false>,
}

impl<'a> InsertIntoQueuesInstructionDataMut<'a> {
    pub fn is_invoked_by_program(&self) -> bool {
        self.meta.is_invoked_by_program == 1
    }

    pub fn set_invoked_by_program(&mut self, value: bool) {
        self.meta.is_invoked_by_program = value as u8;
    }

    pub fn insert_input_sequence_number(
        &mut self,
        index: &mut usize,
        tree_pubkey: &Pubkey,
        queue_pubkey: &Pubkey,
        tree_type: u64,
        seq: u64,
    ) {
        Self::insert_sequence_number(
            &mut self.input_sequence_numbers,
            index,
            tree_pubkey,
            Some(queue_pubkey),
            tree_type,
            seq,
        );
    }

    pub fn insert_address_sequence_number(
        &mut self,
        index: &mut usize,
        tree_pubkey: &Pubkey,
        seq: u64,
    ) {
        Self::insert_sequence_number(
            &mut self.address_sequence_numbers,
            index,
            tree_pubkey,
            None,
            TreeType::AddressV2 as u64,
            seq,
        );
    }

    fn insert_sequence_number(
        sequence_numbers: &mut ZeroCopySliceMut<'a, u8, MerkleTreeSequenceNumber, false>,
        index: &mut usize,
        tree_pubkey: &Pubkey,
        queue_pubkey: Option<&Pubkey>,
        tree_type: u64,
        seq: u64,
    ) {
        let pos = sequence_numbers
            .iter()
            .position(|x| x.tree_pubkey == *tree_pubkey);
        if pos.is_none() {
            sequence_numbers[*index].tree_pubkey = *tree_pubkey;
            if let Some(queue_pubkey) = queue_pubkey {
                sequence_numbers[*index].queue_pubkey = *queue_pubkey;
            }
            sequence_numbers[*index].tree_type = tree_type.into();
            sequence_numbers[*index].seq = seq.into();
            *index += 1;
        }
    }

    pub fn required_size_for_capacity(
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
        num_output_trees: u8,
        num_input_trees: u8,
        num_address_trees: u8,
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
            + ZeroCopySliceMut::<u8, MerkleTreeSequenceNumber, false>::required_size_for_capacity(
                num_input_trees,
            )
            + ZeroCopySliceMut::<u8, MerkleTreeSequenceNumber, false>::required_size_for_capacity(
                num_address_trees,
            )
            + ZeroCopySliceMut::<u8, U32, false>::required_size_for_capacity(leaves_capacity)
    }

    pub fn new_at(
        bytes: &'a mut [u8],
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
        num_output_trees: u8,
        num_input_trees: u8,
        num_address_trees: u8,
    ) -> core::result::Result<(Self, &'a mut [u8]), ZeroCopyError> {
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
        let (output_sequence_numbers, bytes) = ZeroCopySliceMut::<
            u8,
            MerkleTreeSequenceNumber,
            false,
        >::new_at(num_output_trees, bytes)?;
        let (input_sequence_numbers, bytes) = ZeroCopySliceMut::<
            u8,
            MerkleTreeSequenceNumber,
            false,
        >::new_at(num_input_trees, bytes)?;
        let (address_sequence_numbers, bytes) = ZeroCopySliceMut::<
            u8,
            MerkleTreeSequenceNumber,
            false,
        >::new_at(num_address_trees, bytes)?;
        let (output_leaf_indices, bytes) =
            ZeroCopySliceMut::<u8, U32, false>::new_at(leaves_capacity, bytes)?;
        Ok((
            InsertIntoQueuesInstructionDataMut {
                meta,
                leaves,
                nullifiers,
                addresses,
                output_sequence_numbers,
                input_sequence_numbers,
                address_sequence_numbers,
                output_leaf_indices,
            },
            bytes,
        ))
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

#[cfg(all(test, feature = "std"))]
mod test {

    use super::*;
    #[test]
    fn test_ix_data() {
        {
            let leaves_capacity: u8 = 20;
            let nullifiers_capacity: u8 = 20;
            let addresses_capacity: u8 = 0;
            let num_output_trees: u8 = 10;
            let num_input_trees: u8 = 10;
            let num_address_trees: u8 = 0;
            let size = InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
                leaves_capacity,
                nullifiers_capacity,
                addresses_capacity,
                num_output_trees,
                num_input_trees,
                num_address_trees,
            );
            println!("size update 20 pdas {}", size);
            assert_eq!(size, 3165, "size update 20 pdas");
        }
        {
            let leaves_capacity: u8 = 20;
            let nullifiers_capacity: u8 = 0;
            let addresses_capacity: u8 = 20;
            let num_output_trees: u8 = 10;
            let num_input_trees: u8 = 0;
            let num_address_trees: u8 = 1;
            let size = InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
                leaves_capacity,
                nullifiers_capacity,
                addresses_capacity,
                num_output_trees,
                num_input_trees,
                num_address_trees,
            );
            println!("size create 20 pdas {}", size);
            assert_eq!(size, 2345, "size create 20 pdas");
        }
        {
            let leaves_capacity: u8 = 30;
            let nullifiers_capacity: u8 = 0;
            let addresses_capacity: u8 = 0;
            let num_output_trees: u8 = 10;
            let num_input_trees: u8 = 0;
            let num_address_trees: u8 = 0;
            let size = InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
                leaves_capacity,
                nullifiers_capacity,
                addresses_capacity,
                num_output_trees,
                num_input_trees,
                num_address_trees,
            );
            println!("size create 30 ctokens {}", size);
            assert_eq!(size, 1955, "size create 30 ctokens");
        }
    }

    #[test]
    fn test_rnd_insert_into_queues_ix_data() {
        use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
        let seed = thread_rng().gen();
        println!("seed {}", seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let num_iters = 1000;

        for _ in 0..num_iters {
            let leaves_capacity: u8 = rng.gen();
            let nullifiers_capacity: u8 = rng.gen();
            let addresses_capacity: u8 = rng.gen();
            let num_output_trees: u8 = rng.gen();
            let num_input_trees: u8 = rng.gen();
            let num_address_trees: u8 = rng.gen();
            let size = InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
                leaves_capacity,
                nullifiers_capacity,
                addresses_capacity,
                num_output_trees,
                num_input_trees,
                num_address_trees,
            );
            let mut bytes = vec![0u8; size];
            let (mut new_data, _) = InsertIntoQueuesInstructionDataMut::new_at(
                &mut bytes,
                leaves_capacity,
                nullifiers_capacity,
                addresses_capacity,
                num_output_trees,
                num_input_trees,
                num_address_trees,
            )
            .unwrap();
            *new_data.meta = InsertIntoQueuesInstructionDataMeta {
                is_invoked_by_program: rng.gen(),
                bump: rng.gen(),
                num_queues: rng.gen(),
                num_output_queues: rng.gen(),
                start_output_appends: rng.gen(),
                num_address_queues: rng.gen(),
                tx_hash: rng.gen(),
            };
            for i in 0..leaves_capacity {
                new_data.leaves[i as usize] = AppendLeavesInput {
                    account_index: rng.gen(),
                    leaf: rng.gen(),
                };
            }
            for i in 0..nullifiers_capacity {
                new_data.nullifiers[i as usize] = InsertNullifierInput {
                    account_hash: rng.gen(),
                    leaf_index: rng.gen::<u32>().into(),
                    prove_by_index: rng.gen(),
                    tree_index: rng.gen(),
                    queue_index: rng.gen(),
                };
            }
            for i in 0..addresses_capacity {
                new_data.addresses[i as usize] = InsertAddressInput {
                    address: rng.gen(),
                    tree_index: rng.gen(),
                    queue_index: rng.gen(),
                };
            }
            let nullifiers = new_data.nullifiers.to_vec();
            let leaves = new_data.leaves.to_vec();
            let addresses = new_data.addresses.to_vec();
            let meta = *new_data.meta;
            let zero_copy = InsertIntoQueuesInstructionData::zero_copy_at(&bytes)
                .unwrap()
                .0;
            assert_eq!(meta, *zero_copy.meta);
            assert_eq!(leaves.as_slice(), zero_copy.leaves.as_slice());
            assert_eq!(nullifiers.as_slice(), zero_copy.nullifiers.as_slice());
            assert_eq!(addresses.as_slice(), zero_copy.addresses.as_slice());
        }
    }
}
