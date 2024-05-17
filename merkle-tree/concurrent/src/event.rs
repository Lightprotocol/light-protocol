use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Changelogs {
    pub changelogs: Vec<ChangelogEvent>,
}

/// Event containing the Merkle path of the given
/// [`StateMerkleTree`](light_merkle_tree_program::state::StateMerkleTree)
/// change. Indexers can use this type of events to re-build a non-sparse
/// version of state Merkle tree.
#[derive(BorshDeserialize, BorshSerialize, Debug)]
#[repr(C)]
pub enum ChangelogEvent {
    V1(ChangelogEventV1),
    V2(ChangelogEventV2),
}

/// Node of the Merkle path with an index representing the position in a
/// non-sparse Merkle tree.
#[derive(BorshDeserialize, BorshSerialize, Debug, Eq, PartialEq)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

/// Version 1 of the [`ChangelogEvent`](light_merkle_tree_program::state::ChangelogEvent).
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct ChangelogEventV1 {
    /// Public key of the tree.
    pub id: [u8; 32],
    // Merkle paths.
    pub paths: Vec<Vec<PathNode>>,
    /// Number of successful operations on the on-chain tree.
    pub seq: u64,
    /// Changelog event index.
    pub index: u32,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct ChangelogEventV2 {
    /// Public key of the tree.
    pub id: [u8; 32],
    pub leaves: Vec<UpdatedLeaf>,
    /// Number of successful operations on the on-chain tree.
    /// seq corresponds to leaves[0].
    /// seq + 1 corresponds to leaves[1].
    pub seq: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct UpdatedLeaf {
    pub leaf: [u8; 32],
    /// Leaf index.
    pub leaf_index: u64,
}
