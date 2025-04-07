use bytemuck::{Pod, Zeroable};
use light_compressed_account::pubkey::Pubkey;
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
    Clone,
    Copy,
    Pod,
    Zeroable,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
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

#[test]
fn test() {
    let owner = Pubkey::new_unique();
    let program_owner = Some(Pubkey::new_unique());
    let forester = Some(Pubkey::new_unique());
    let access_metadata = AccessMetadata::new(owner, program_owner, forester);
    let rollover_metadata = RolloverMetadata::new(1, 2, Some(95), 100, Some(1000), Some(1));
    let associated_queue = Pubkey::new_unique();
    let mut merkle_tree_metadata = MerkleTreeMetadata::default();
    // 1. Functional - init
    {
        merkle_tree_metadata.init(access_metadata, rollover_metadata, associated_queue);
        assert_eq!(merkle_tree_metadata.access_metadata, access_metadata);
        assert_eq!(merkle_tree_metadata.rollover_metadata, rollover_metadata);
    }
    let next_merkle_tree = Pubkey::new_unique();
    // 2. Failing - rollover with invalid associated queue
    {
        let invalid_associated_queue = Pubkey::new_unique();
        let result = merkle_tree_metadata.rollover(invalid_associated_queue, next_merkle_tree);
        assert_eq!(
            result,
            Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated)
        );
    }
    // 3. Functional - rollover
    {
        merkle_tree_metadata
            .rollover(associated_queue, next_merkle_tree)
            .unwrap();
        assert_eq!(merkle_tree_metadata.next_merkle_tree, next_merkle_tree);
        assert_eq!(merkle_tree_metadata.rollover_metadata.rolledover_slot, 1);
    }
    // 4. Failing - rollover with invalid associated queue
    {
        let result = merkle_tree_metadata.rollover(associated_queue, next_merkle_tree);
        assert_eq!(
            result,
            Err(MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver)
        );
    }
}
