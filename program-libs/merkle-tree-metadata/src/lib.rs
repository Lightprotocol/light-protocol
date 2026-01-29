//! # light-merkle-tree-metadata
//!
//! Metadata structs for concurrent, indexed, and batched Merkle trees.
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`merkle_tree`] | Tree metadata: height, next index, owner, delegate |
//! | [`queue`] | Queue metadata: type, capacity, sequence numbers |
//! | [`access`] | Owner and delegate access control checks |
//! | [`fee`] | Fee parameters for tree and queue operations |
//! | [`rollover`] | Rollover threshold and status tracking |
//! | [`events`] | Changelog events emitted on tree updates |
//! | [`errors`] | `MerkleTreeMetadataError` variants |

pub mod access;
pub mod errors;
pub mod events;
pub mod fee;
pub mod merkle_tree;
pub mod queue;
pub mod rollover;
pub mod utils;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub(crate) use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_compressed_account::{
    QueueType, TreeType, ADDRESS_MERKLE_TREE_TYPE_V1, ADDRESS_MERKLE_TREE_TYPE_V2,
    ADDRESS_QUEUE_TYPE_V1, ADDRESS_QUEUE_TYPE_V2, INPUT_STATE_QUEUE_TYPE_V2,
    NULLIFIER_QUEUE_TYPE_V1, OUTPUT_STATE_QUEUE_TYPE_V2, STATE_MERKLE_TREE_TYPE_V1,
    STATE_MERKLE_TREE_TYPE_V2,
};
