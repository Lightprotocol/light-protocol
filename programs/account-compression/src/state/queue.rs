use anchor_lang::prelude::*;

use crate::{errors::AccountCompressionErrorCode, AccessMetadata, RolloverMetadata};

#[account(zero_copy)]
#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq)]
pub struct QueueMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,

    // Queue associated with this Merkle tree.
    pub associated_merkle_tree: Pubkey,
    // Next queue to be used after rollover.
    pub next_queue: Pubkey,
}

impl QueueMetadata {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_merkle_tree: Pubkey,
    ) {
        self.access_metadata = access_metadata;
        self.rollover_metadata = rollover_metadata;
        self.associated_merkle_tree = associated_merkle_tree;
    }

    pub fn rollover(
        &mut self,
        old_associated_merkle_tree: Pubkey,
        next_queue: Pubkey,
    ) -> Result<()> {
        if self.associated_merkle_tree != old_associated_merkle_tree {
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }

        self.rollover_metadata.rollover()?;
        self.next_queue = next_queue;

        Ok(())
    }
}
