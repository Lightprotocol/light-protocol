use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

use crate::{access::AccessMetadata, errors::MerkleTreeMetadataError, rollover::RolloverMetadata};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TreeType {
    State = 1,
    Address = 2,
    BatchedState = 3,
    BatchedAddress = 4,
}

#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Default, Clone, Copy, Pod, Zeroable,
)]
pub struct MerkleTreeMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,
    // Queue associated with this Merkle tree.
    pub associated_queue: Pubkey,
    // Next Merkle tree to be used after rollover.
    pub next_merkle_tree: Pubkey,
}

impl MerkleTreeMetadata {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_queue: Pubkey,
    ) {
        self.access_metadata = access_metadata;
        self.rollover_metadata = rollover_metadata;
        self.associated_queue = associated_queue;
    }

    pub fn rollover(
        &mut self,
        old_associated_queue: Pubkey,
        next_merkle_tree: Pubkey,
    ) -> Result<(), MerkleTreeMetadataError> {
        if self.associated_queue != old_associated_queue {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated);
        }

        self.rollover_metadata.rollover()?;
        self.next_merkle_tree = next_merkle_tree;

        Ok(())
    }
}
