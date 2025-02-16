#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use bytemuck::{Pod, Zeroable};
use light_compressed_account::pubkey::Pubkey;
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

pub const NULLIFIER_QUEUE_TYPE: u64 = 1;
pub const ADDRESS_QUEUE_TYPE: u64 = 2;
pub const BATCHED_INPUT_QUEUE_TYPE: u64 = 3;
pub const BATCHED_ADDRESS_QUEUE_TYPE: u64 = 4;
pub const BATCHED_OUTPUT_QUEUE_TYPE: u64 = 5;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access;

    /// Helper function to create a default `QueueMetadata` struct for testing.
    fn create_queue_metadata(
        associated_merkle_tree: Pubkey,
        queue_type: QueueType,
    ) -> QueueMetadata {
        QueueMetadata {
            access_metadata: AccessMetadata {
                owner: Pubkey::new_unique(),
                program_owner: Pubkey::new_unique(),
                forester: Pubkey::new_unique(),
            },
            rollover_metadata: RolloverMetadata {
                index: 0,
                rollover_fee: 1000,
                rollover_threshold: 95,
                network_fee: 10,
                rolledover_slot: u64::MAX,
                close_threshold: 200,
                additional_bytes: 0,
            },
            associated_merkle_tree,
            next_queue: Pubkey::default(),
            queue_type: queue_type as u64,
        }
    }

    #[test]
    fn test_check_queue_type_valid() {
        let valid_queue_type = QueueType::NullifierQueue;
        assert!(check_queue_type(&(valid_queue_type as u64), &valid_queue_type).is_ok());
    }

    #[test]
    fn test_check_queue_type_invalid() {
        let queue_type = QueueType::NullifierQueue;
        let expected_queue_type = QueueType::AddressQueue;
        assert!(matches!(
            check_queue_type(&(queue_type as u64), &expected_queue_type),
            Err(MerkleTreeMetadataError::InvalidQueueType)
        ));
    }

    #[test]
    fn test_init_method() {
        let associated_merkle_tree = Pubkey::new_unique();
        let queue_type = QueueType::BatchedInput;
        let access_metadata = access::AccessMetadata {
            owner: Pubkey::new_unique(),
            program_owner: Pubkey::new_unique(),
            forester: Pubkey::new_unique(),
        };
        let rollover_metadata = RolloverMetadata {
            index: 1,
            rollover_fee: 1000,
            rollover_threshold: 95,
            network_fee: 10,
            rolledover_slot: u64::MAX,
            close_threshold: 200,
            additional_bytes: 1,
        };
        let mut queue_metadata = QueueMetadata::default();
        queue_metadata.init(
            access_metadata,
            rollover_metadata,
            associated_merkle_tree,
            queue_type,
        );
        assert_eq!(queue_metadata.access_metadata, access_metadata);
        assert_eq!(queue_metadata.rollover_metadata, rollover_metadata);
        assert_eq!(
            queue_metadata.associated_merkle_tree,
            associated_merkle_tree
        );
        assert_eq!(queue_metadata.queue_type, queue_type as u64);
    }

    #[test]
    fn test_rollover_method_valid() {
        let associated_merkle_tree = Pubkey::new_unique();
        let next_queue = Pubkey::new_unique();
        let mut queue_metadata =
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierQueue);

        // Update the next queue as part of the method.
        assert!(queue_metadata
            .rollover(associated_merkle_tree, next_queue)
            .is_ok());
        assert_eq!(queue_metadata.next_queue, next_queue);
    }

    #[test]
    fn test_rollover_method_invalid_merkle_tree() {
        let associated_merkle_tree = Pubkey::new_unique();
        let wrong_tree = Pubkey::new_unique();
        let next_queue = Pubkey::new_unique();
        let mut queue_metadata =
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierQueue);

        // Should fail because `wrong_tree` does not match the associated merkle tree.
        assert!(matches!(
            queue_metadata.rollover(wrong_tree, next_queue),
            Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated)
        ));
    }

    #[test]
    fn test_rollover_method_not_configured() {
        let associated_merkle_tree = Pubkey::new_unique();
        let mut queue_metadata =
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierQueue);

        // Simulate a case where rollover threshold is not configured.
        queue_metadata.rollover_metadata.rollover_threshold = u64::MAX;
        assert!(matches!(
            queue_metadata.rollover_metadata.rollover(),
            Err(MerkleTreeMetadataError::RolloverNotConfigured)
        ));
    }

    #[test]
    fn test_rollover_method_already_rolled_over() {
        let associated_merkle_tree = Pubkey::new_unique();
        let mut queue_metadata =
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierQueue);

        // Simulate a case where it is already rolled over.
        queue_metadata.rollover_metadata.rolledover_slot = 10;
        assert!(matches!(
            queue_metadata.rollover_metadata.rollover(),
            Err(MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver)
        ));
    }

    #[test]
    fn test_queue_type_from() {
        assert_eq!(QueueType::NullifierQueue, QueueType::from(1));
        assert_eq!(QueueType::AddressQueue, QueueType::from(2));
        assert_eq!(QueueType::BatchedInput, QueueType::from(3));
        assert_eq!(QueueType::BatchedAddress, QueueType::from(4));
        assert_eq!(QueueType::BatchedOutput, QueueType::from(5));
    }

    #[should_panic = "Invalid queue type"]
    #[test]
    fn test_queue_type_from_invalid() {
        let _ = QueueType::from(0);
    }
}
