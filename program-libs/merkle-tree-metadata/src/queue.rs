#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use bytemuck::{Pod, Zeroable};
use light_utils::pubkey::Pubkey;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{access::AccessMetadata, errors::MerkleTreeMetadataError, rollover::RolloverMetadata};

#[repr(C)]
#[derive(
    AnchorDeserialize,
    AnchorSerialize,
    Debug,
    PartialEq,
    Default,
    Pod,
    Zeroable,
    Clone,
    Copy,
    Immutable,
    FromBytes,
    IntoBytes,
    KnownLayout,
)]
pub struct QueueMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,

    // Queue associated with this Merkle tree.
    pub associated_merkle_tree: Pubkey,
    // Next queue to be used after rollover.
    pub next_queue: Pubkey,
    pub queue_type: u64,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum QueueType {
    NullifierQueue = 1,
    AddressQueue = 2,
    BatchedInput = 3,
    BatchedAddress = 4,
    BatchedOutput = 5,
}

impl From<u64> for QueueType {
    fn from(value: u64) -> Self {
        match value {
            1 => QueueType::NullifierQueue,
            2 => QueueType::AddressQueue,
            3 => QueueType::BatchedInput,
            4 => QueueType::BatchedAddress,
            5 => QueueType::BatchedOutput,
            _ => panic!("Invalid queue type"),
        }
    }
}

pub fn check_queue_type(
    queue_type: &u64,
    expected_queue_type: &QueueType,
) -> Result<(), MerkleTreeMetadataError> {
    if *queue_type != (*expected_queue_type) as u64 {
        Err(MerkleTreeMetadataError::InvalidQueueType)
    } else {
        Ok(())
    }
}

impl QueueMetadata {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_merkle_tree: Pubkey,
        queue_type: QueueType,
    ) {
        self.access_metadata = access_metadata;
        self.rollover_metadata = rollover_metadata;
        self.associated_merkle_tree = associated_merkle_tree;
        self.queue_type = queue_type as u64;
    }

    pub fn rollover(
        &mut self,
        old_associated_merkle_tree: Pubkey,
        next_queue: Pubkey,
    ) -> Result<(), MerkleTreeMetadataError> {
        if self.associated_merkle_tree != old_associated_merkle_tree {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated);
        }

        self.rollover_metadata.rollover()?;
        self.next_queue = next_queue;

        Ok(())
    }
}
