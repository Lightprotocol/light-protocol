<!-- cargo-rdme start -->

# Light Compressed Token SDK

Low-level SDK for compressed token operations on Light Protocol.

This crate provides the core building blocks for working with compressed token accounts,
including instruction builders for transfers, mints, and compress/decompress operations.

## Compressed Token Accounts
- are on Solana mainnet.
- are compressed accounts.
- can hold Light Mint and SPL Mint tokens.
- cost 5,000 lamports to create.
- are well suited for airdrops and reward distribution.

## Difference to Light-Token:
light-token: Solana account that holds token balances of light-mints, SPL or Token 22 mints.
Compressed token: Compressed account storing token data. Rent-free, for storage and distribution.

## Features

- `v1` - Enable v1 compressed token support
- `anchor` - Enable Anchor framework integration

For full examples, see the [Compressed Token Examples](https://github.com/Lightprotocol/examples-zk-compression).

## Operations reference

| Operation | Docs guide | GitHub example |
|-----------|-----------|----------------|
| Create mint | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/create-mint.ts) |
| Mint to | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/mint-to.ts) |
| Transfer | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/transfer.ts) |
| Approve | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/approve.ts) |
| Revoke | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/revoke.ts) |
| Compress | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/compress.ts) |
| Compress SPL account | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/compress-spl-account.ts) |
| Decompress | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/decompress.ts) |
| Merge token accounts | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/merge-token-accounts.ts) |
| Create token pool | [create-compressed-token-accounts](https://www.zkcompression.com/compressed-tokens/guides/create-compressed-token-accounts) | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/create-token-pool.ts) |

### Toolkit guides

| Topic | Docs guide | GitHub example |
|-------|-----------|----------------|
| Airdrop | [airdrop](https://www.zkcompression.com/compressed-tokens/advanced-guides/airdrop) | [example](https://github.com/Lightprotocol/examples-zk-compression/tree/main/example-token-distribution) |
| Privy integration | [privy](https://www.zkcompression.com/compressed-tokens/for-privy) | [example](https://github.com/Lightprotocol/examples-zk-compression/tree/main/privy) |

## Modules

- [`compressed_token`] - Core compressed token types and instruction builders
- [`error`] - Error types for compressed token operations
- [`utils`] - Utility functions and default account configurations
- [`constants`] - Program IDs and other constants
- [`spl_interface`] - SPL interface PDA derivation utilities

<!-- cargo-rdme end -->
