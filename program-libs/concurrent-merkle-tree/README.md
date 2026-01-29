<!-- cargo-rdme start -->

# light-concurrent-merkle-tree

Concurrent Merkle tree that accepts multiple leaf updates
without invalidating other in-flight proofs.

| Type | Description |
|------|-------------|
| [`ConcurrentMerkleTree`] | Append, update, and validate leaves concurrently |
| [`ConcurrentMerkleTree26`] | Type alias for height-26 trees |
| [`changelog`] | Changelog entries for concurrent proof patching |
| [`errors`] | `ConcurrentMerkleTreeError` variants |
| [`hash`] | `compute_parent_node` and `compute_root` |
| [`zero_copy`] | Zero-copy deserialization from account data |
| [`event`] | Changelog event for indexers |

<!-- cargo-rdme end -->
