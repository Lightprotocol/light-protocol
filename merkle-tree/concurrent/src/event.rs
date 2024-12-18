use borsh::{BorshDeserialize, BorshSerialize};
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MerkleTreeEvents {
    pub events: Vec<MerkleTreeEvent>,
}

/// Event containing the Merkle path of the given
/// [`StateMerkleTree`](light_merkle_tree_program::state::StateMerkleTree)
/// change. Indexers can use this type of events to re-build a non-sparse
/// version of state Merkle tree.
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
#[repr(C)]
pub enum MerkleTreeEvent {
    V1(ChangelogEvent),
    V2(NullifierEvent),
    V3(IndexedMerkleTreeEvent),
}

/// Node of the Merkle path with an index representing the position in a
/// non-sparse Merkle tree.
#[derive(BorshDeserialize, BorshSerialize, Debug, Eq, PartialEq)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

/// Version 1 of the [`ChangelogEvent`](light_merkle_tree_program::state::ChangelogEvent).
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct ChangelogEvent {
    /// Public key of the tree.
    pub id: [u8; 32],
    // Merkle paths.
    pub paths: Vec<Vec<PathNode>>,
    /// Number of successful operations on the on-chain tree.
    pub seq: u64,
    /// Changelog event index.
    pub index: u32,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct NullifierEvent {
    /// Public key of the tree.
    pub id: [u8; 32],
    /// Indices of leaves that were nullified.
    /// Nullified means updated with [0u8;32].
    pub nullified_leaves_indices: Vec<u64>,
    /// Number of successful operations on the on-chain tree.
    /// seq corresponds to leaves[0].
    /// seq + 1 corresponds to leaves[1].
    pub seq: u64,
}

#[derive(Debug, Default, Clone, Copy, BorshSerialize, BorshDeserialize, Eq, PartialEq)]
pub struct RawIndexedElement<I>
where
    I: Clone,
{
    pub value: [u8; 32],
    pub next_index: I,
    pub next_value: [u8; 32],
    pub index: I,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq)]
pub struct IndexedMerkleTreeUpdate<I>
where
    I: Clone,
{
    pub new_low_element: RawIndexedElement<I>,
    /// Leaf hash in new_low_element.index.
    pub new_low_element_hash: [u8; 32],
    pub new_high_element: RawIndexedElement<I>,
    /// Leaf hash in new_high_element.index,
    /// is equivalent with next_index.
    pub new_high_element_hash: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct IndexedMerkleTreeEvent {
    /// Public key of the tree.
    pub id: [u8; 32],
    pub updates: Vec<IndexedMerkleTreeUpdate<usize>>,
    /// Number of successful operations on the on-chain tree.
    /// seq corresponds to leaves[0].
    /// seq + 1 corresponds to leaves[1].
    pub seq: u64,
}
