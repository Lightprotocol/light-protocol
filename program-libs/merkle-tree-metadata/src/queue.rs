use bytemuck::{Pod, Zeroable};
use light_compressed_account::{pubkey::Pubkey, QueueType};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{
    access::AccessMetadata, errors::MerkleTreeMetadataError, rollover::RolloverMetadata,
    AnchorDeserialize, AnchorSerialize,
};

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
        let valid_queue_type = QueueType::NullifierV1;
        assert!(check_queue_type(&(valid_queue_type as u64), &valid_queue_type).is_ok());
    }

    #[test]
    fn test_check_queue_type_invalid() {
        let queue_type = QueueType::NullifierV1;
        let expected_queue_type = QueueType::AddressV1;
        assert!(matches!(
            check_queue_type(&(queue_type as u64), &expected_queue_type),
            Err(MerkleTreeMetadataError::InvalidQueueType)
        ));
    }

    #[test]
    fn test_init_method() {
        let associated_merkle_tree = Pubkey::new_unique();
        let queue_type = QueueType::InputStateV2;
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
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierV1);

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
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierV1);

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
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierV1);

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
            create_queue_metadata(associated_merkle_tree, QueueType::NullifierV1);

        // Simulate a case where it is already rolled over.
        queue_metadata.rollover_metadata.rolledover_slot = 10;
        assert!(matches!(
            queue_metadata.rollover_metadata.rollover(),
            Err(MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver)
        ));
    }

    #[test]
    fn test_queue_type_from() {
        assert_eq!(QueueType::NullifierV1, QueueType::from(1));
        assert_eq!(QueueType::AddressV1, QueueType::from(2));
        assert_eq!(QueueType::InputStateV2, QueueType::from(3));
        assert_eq!(QueueType::AddressV2, QueueType::from(4));
        assert_eq!(QueueType::OutputStateV2, QueueType::from(5));
    }

    #[should_panic = "Invalid queue type"]
    #[test]
    fn test_queue_type_from_invalid() {
        let _ = QueueType::from(0);
    }
}
