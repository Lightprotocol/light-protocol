<!-- cargo-rdme start -->

# light-compressed-account

Compressed account struct and utility types for Light Protocol.

| Type | Description |
|------|-------------|
| [`CompressedAccountError`] | Error codes 12001–12025 for account operations |
| [`QueueType`] | Nullifier, address, and state queue variants |
| [`TreeType`] | State and address tree version variants |
| [`CpiSigner`] | Program ID, CPI signer pubkey, and bump |
| [`address`] | Address derivation and seed structs |
| [`compressed_account`] | Core compressed account struct |
| [`constants`] | Program IDs and account discriminators as byte arrays |
| [`discriminators`] | Instruction discriminators for `invoke`, `invoke_cpi`, and queue operations |
| [`instruction_data`] | Instruction data types and proof structs |
| [`nullifier`] | Nullifier computation |
| [`pubkey`] | `Pubkey` struct (re-exported at root) and `AsPubkey` conversion trait |
| [`tx_hash`] | Transaction hash computation |

<!-- cargo-rdme end -->
