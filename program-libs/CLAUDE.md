# Program Libraries

Core Rust libraries used in on-chain programs and sdk-libs. These crates are designed for Solana program environments with careful attention to compute budget and memory constraints.

## Reverse Dependency Hierarchy

For each crate, lists all crates that depend on it (dependents).

| Crate | Depended On By |
|-------|----------------|
| account-checks | batched-merkle-tree, compressible, token-interface |
| aligned-sized | (none) |
| array-map | token-interface |
| batched-merkle-tree | (none) |
| bloom-filter | batched-merkle-tree |
| compressed-account | batched-merkle-tree, compressible, merkle-tree-metadata, token-interface, verifier |
| compressible | token-interface |
| concurrent-merkle-tree | indexed-merkle-tree |
| hash-set | concurrent-merkle-tree, indexed-merkle-tree |
| hasher | batched-merkle-tree, bloom-filter, compressed-account, compressible, concurrent-merkle-tree, hash-set, indexed-array, indexed-merkle-tree, token-interface |
| heap | compressed-account, compressible, token-interface |
| indexed-array | (none) |
| indexed-merkle-tree | (none) |
| macros | batched-merkle-tree, compressed-account, compressible, token-interface |
| merkle-tree-metadata | batched-merkle-tree |
| token-interface | (none) |
| verifier | batched-merkle-tree |
| zero-copy | batched-merkle-tree, compressed-account, compressible, token-interface |
| zero-copy-derive | zero-copy |

## Crate Descriptions

| Crate | Description |
|-------|-------------|
| account-checks | Solana account validation (solana-program + pinocchio) |
| aligned-sized | Macro to get aligned size of Rust structs |
| array-map | Array-based map data structure |
| batched-merkle-tree | Merkle tree updates with ZK proofs |
| bloom-filter | Bloom filter implementation |
| compressed-account | Compressed account types and utilities |
| compressible | Configuration for compressible token accounts |
| concurrent-merkle-tree | Concurrent Merkle tree operations |
| hash-set | Hash set for Solana programs |
| hasher | Poseidon hash implementation |
| heap | Heap data structure for Solana programs |
| indexed-array | Indexed array utilities |
| indexed-merkle-tree | Indexed Merkle tree with address management |
| macros | Procedural macros for Light Protocol |
| merkle-tree-metadata | Metadata types for Merkle trees |
| token-interface | Compressed token types and interfaces |
| verifier | ZKP verification logic in Solana programs |
| zero-copy | Zero-copy serialization for efficient account access |
| zero-copy-derive | Derive macros for zero-copy serialization |

## External Dependencies

Some crates depend on external Light Protocol crates not in program-libs:
- `light-poseidon` - Poseidon hash primitive
- `light-bounded-vec` - Bounded vector implementation
- `light-merkle-tree-reference` - Reference Merkle tree implementation for testing
- `light-program-profiler` - Profiling utilities

## Testing

Unit tests run with `cargo test`:
```bash
cargo test -p light-hasher --all-features
cargo test -p light-compressed-account --all-features
cargo test -p light-batched-merkle-tree
```

Integration tests that require Solana runtime are in `program-tests/`.
