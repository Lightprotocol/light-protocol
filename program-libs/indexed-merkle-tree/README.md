<!-- cargo-rdme start -->

# light-indexed-merkle-tree

Indexed concurrent Merkle tree. Extends `ConcurrentMerkleTree`
with a sorted linked-list index for non-inclusion proofs.

| Type | Description |
|------|-------------|
| [`IndexedMerkleTree`] | Tree with indexed changelog for ordered inserts |
| [`IndexedMerkleTree26`] | Type alias for height-26 indexed trees |
| [`array`] | Indexed array backing the tree elements |
| [`changelog`] | Indexed changelog entries for concurrent patching |
| [`reference`] | Reference implementation for testing |
| [`zero_copy`] | Zero-copy deserialization from account data |
| [`errors`] | `IndexedMerkleTreeError` variants |

<!-- cargo-rdme end -->
