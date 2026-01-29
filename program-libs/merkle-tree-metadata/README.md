<!-- cargo-rdme start -->

# light-merkle-tree-metadata

Metadata structs for concurrent, indexed, and batched Merkle trees.

| Module | Description |
|--------|-------------|
| [`merkle_tree`] | Tree metadata: height, next index, owner, delegate |
| [`queue`] | Queue metadata: type, capacity, sequence numbers |
| [`access`] | Owner and delegate access control checks |
| [`fee`] | Fee parameters for tree and queue operations |
| [`rollover`] | Rollover threshold and status tracking |
| [`events`] | Changelog events emitted on tree updates |
| [`errors`] | `MerkleTreeMetadataError` variants |

<!-- cargo-rdme end -->
