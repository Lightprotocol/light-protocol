use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::event::RawIndexedElement;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedChangelogEntry<I>
where
    I: Clone,
{
    /// Element that was a subject to the change.
    pub element: RawIndexedElement<I>,
    /// Merkle proof of that operation.
    pub proof: BoundedVec<[u8; 32]>,
    /// Index of a changelog entry in `ConcurrentMerkleTree` corresponding to
    /// the same operation.
    pub changelog_index: usize,
}
