use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::Hasher;

#[allow(clippy::too_many_arguments)]
pub fn assert_merkle_tree_initialized<H, const HEIGHT: usize>(
    merkle_tree: &ConcurrentMerkleTree<H, HEIGHT>,
    height: usize,
    changelog_capacity: usize,
    roots_capacity: usize,
    canopy_depth: usize,
    expected_changelog_length: usize,
    expected_roots_length: usize,
    expected_next_index: usize,
    expected_rightmost_leaf: &[u8; 32],
) where
    H: Hasher,
{
    assert_eq!(merkle_tree.height, height);
    assert_eq!(merkle_tree.changelog_capacity, changelog_capacity);
    assert_eq!(merkle_tree.changelog_length, expected_changelog_length);
    assert_eq!(
        merkle_tree.current_changelog_index,
        expected_changelog_length.saturating_sub(1)
    );
    assert_eq!(merkle_tree.roots_capacity, roots_capacity);
    assert_eq!(merkle_tree.roots_length, expected_roots_length);
    assert_eq!(
        merkle_tree.current_root_index,
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(merkle_tree.canopy_depth, canopy_depth);
    assert_eq!(merkle_tree.next_index, expected_next_index);
    assert_eq!(
        merkle_tree.sequence_number,
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(&merkle_tree.rightmost_leaf, expected_rightmost_leaf);
}
