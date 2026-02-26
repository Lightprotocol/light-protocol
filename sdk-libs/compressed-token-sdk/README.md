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
[light-token](../token-sdk): Solana account that holds token balances of light-mints, SPL or Token 22 mints for most token use cases (launchpads, DeFi, payments). Mint and token accounts with sponsored rent-exemption.
Compressed token: Compressed account storing token data. Rent-free, for storage and distribution. Prefer Light Token for other purposes. Used by Light Token under the hood for rent-free storage of inactive Light Tokens. Supported by Phantom and Backpack.

## Features

- `v1` - Enable v1 compressed token support
- `anchor` - Enable Anchor framework integration

For full examples, see the [Compressed Token Examples](https://github.com/Lightprotocol/examples-zk-compression).

## Operations reference

| Operation | GitHub example |
|-----------|----------------|
| Create mint | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/create-mint.ts) |
| Mint to | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/mint-to.ts) |
| Transfer | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/transfer.ts) |
| Approve | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/approve.ts) |
| Revoke | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/revoke.ts) |
| Compress | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/compress.ts) |
| Compress SPL account | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/compress-spl-account.ts) |
| Decompress | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/decompress.ts) |
| Merge token accounts | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/merge-token-accounts.ts) |
| Create token pool | [example](https://github.com/Lightprotocol/examples-zk-compression/blob/main/compressed-token-cookbook/actions/create-token-pool.ts) |

### Toolkit guides

| Topic | GitHub example |
|-------|----------------|
| Airdrop | [example](https://github.com/Lightprotocol/examples-zk-compression/tree/main/example-token-distribution) |
| Privy integration | [example](https://github.com/Lightprotocol/examples-zk-compression/tree/main/privy) |

## Modules

- [`compressed_token`] - Core compressed token types and instruction builders
- [`error`] - Error types for compressed token operations
- [`utils`] - Utility functions and default account configurations
- [`constants`] - Program IDs and other constants
- [`spl_interface`] - SPL interface PDA derivation utilities

<!-- cargo-rdme end -->
