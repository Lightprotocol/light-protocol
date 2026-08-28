use light_concurrent_merkle_tree::event::RawIndexedElement;

/// NET_HEIGHT = HEIGHT -  CANOPY_DEPTH
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedChangelogEntry<I, const NET_HEIGHT: usize>
where
    I: Clone,
{
    /// Element that was a subject to the change.
    pub element: RawIndexedElement<I>,
    /// Merkle proof of that operation.
    pub proof: [[u8; 32]; NET_HEIGHT],
    /// Index of a changelog entry in `ConcurrentMerkleTree` corresponding to
    /// the same operation.
    pub changelog_index: usize,
}

// Pin the deployed v1 address-tree account layout. These types use Rust's
// native layout, which currently reorders their fields.
const _: () = {
    type Element = RawIndexedElement<usize>;
    type Entry = IndexedChangelogEntry<usize, 16>;

    assert!(std::mem::size_of::<Element>() == 80);
    assert!(std::mem::align_of::<Element>() == 8);
    assert!(std::mem::offset_of!(Element, value) == 0);
    assert!(std::mem::offset_of!(Element, next_value) == 32);
    assert!(std::mem::offset_of!(Element, next_index) == 64);
    assert!(std::mem::offset_of!(Element, index) == 72);

    assert!(std::mem::size_of::<Entry>() == 600);
    assert!(std::mem::align_of::<Entry>() == 8);
    assert!(std::mem::offset_of!(Entry, proof) == 0);
    assert!(std::mem::offset_of!(Entry, element) == 512);
    assert!(std::mem::offset_of!(Entry, changelog_index) == 592);
};
