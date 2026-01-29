<!-- cargo-rdme start -->

# light-hasher

Trait for generic hash function usage on Solana.

| Type | Description |
|------|-------------|
| [`Hasher`] | Trait with `hash`, `hashv`, and `zero_bytes` |
| [`Poseidon`] | Poseidon hash over BN254 |
| [`Keccak`] | Keccak-256 hash |
| [`Sha256`] | SHA-256 hash |
| [`DataHasher`] | Trait to hash structured data |
| [`HasherError`] | Error type for hash operations |
| [`hash_chain`] | Sequential hash chaining |
| [`hash_to_field_size`] | Truncate hash output to BN254 field size |
| [`zero_bytes`] | Precomputed zero-leaf hashes per hasher |

<!-- cargo-rdme end -->
