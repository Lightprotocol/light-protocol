<!-- cargo-rdme start -->

# light-array-map

Generic array-backed map with O(n) lookup for small collections.
Built on `tinyvec::ArrayVec`. Tracks insertion order and last updated index.

| Type | Description |
|------|-------------|
| [`ArrayMap`] | Fixed-capacity map keyed by `K` with O(n) scan |
| [`ArrayMapError`] | Capacity exceeded or index out of bounds |
| [`pubkey_eq`] | Compares two `[u8; 32]` keys via 4x `u64` reads |

<!-- cargo-rdme end -->
