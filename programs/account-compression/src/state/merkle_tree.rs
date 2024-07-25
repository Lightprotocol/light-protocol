use anchor_lang::prelude::*;

use crate::{errors::AccountCompressionErrorCode, AccessMetadata, RolloverMetadata};

#[account(zero_copy)]
#[derive(AnchorDeserialize, Debug, PartialEq, Default)]
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
    ) -> Result<()> {
        if self.associated_queue != old_associated_queue {
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }

        self.rollover_metadata.rollover()?;
        self.next_merkle_tree = next_merkle_tree;

        Ok(())
    }
}
