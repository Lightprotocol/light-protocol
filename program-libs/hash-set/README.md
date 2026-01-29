<!-- cargo-rdme start -->

# light-hash-set

Hash set stored on a Solana account. Uses quadratic probing
with sequence-number-based expiry for element invalidation.

| Type | Description |
|------|-------------|
| [`HashSet`] | Insert, find, contains, and mark elements by sequence number |
| [`HashSetCell`] | Cell storing a 32-byte value and optional sequence number |
| [`HashSetError`] | Full, duplicate, not found, and overflow errors |
| [`HashSetIterator`] | Iterates over occupied cells |
| [`zero_copy`] | Zero-copy deserialization from account data |

<!-- cargo-rdme end -->
