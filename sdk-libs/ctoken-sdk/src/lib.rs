//! # Light Token SDK
//!
//! The base library to use Light Token Accounts, Compressed Mints, and compressed token accounts.
//!
//! ## Light Token Accounts
//! - are on Solana devnet.
//! - are Solana accounts.
//! - can hold Compressed Mint and SPL Mint tokens.
//! - cost 17,288 lamports to create with 24 hours rent.
//! - are compressible:
//!     - rent exemption is sponsored by the protocol.
//!     - rent is 388 lamports per rent epoch (1.5 hours).
//!     - once the account's lamports balance is insufficient, it is compressed to a compressed token account.
//!     - compressed tokens can be decompressed to a Light Token account.
//!     - configurable lamports per write (eg transfer) keep the Light Token account perpetually funded when used. So you don't have to worry about funding rent.
//!
//! ## Compressed Token Accounts
//! - are on Solana mainnet.
//! - are compressed accounts.
//! - can hold Compressed Mint and SPL Mint tokens.
//! - cost 5,000 lamports to create.
//! - are well suited for airdrops and reward distribution.
//!
//! ## Compressed Mints:
//! - are on Solana devnet.
//! - are Compressed accounts.
//! - cost 15,000 lamports to create.
//! - support `TokenMetadata`.
//!
//!
//! For full program examples, see the [Program Examples](https://github.com/Lightprotocol/program-examples).
//! For detailed documentation, visit [zkcompression.com](https://www.zkcompression.com/).
//! For rust client development see [`light-client`](https://docs.rs/light-client).
//! For rust program testing see [`light-program-test`](https://docs.rs/light-program-test).
//! For local test validator with light system programs see [Light CLI](https://www.npmjs.com/package/@lightprotocol/zk-compression-cli).
//!
//!
//!
//! ## Features
//!
//! 1. anchor - Derives AnchorSerialize, AnchorDeserialize instead of BorshSerialize, BorshDeserialize.
//! 2. compressible - utility functions for compressible sdk macros.
//!
//! ## Common Operations
//!
//! | Operation | Instruction Builder | CPI Builder |
//! |-----------|----------------|-------------|
//! | Create Associated Token Account | [`CreateAssociatedTokenAccount`](token::CreateAssociatedTokenAccount) | [`CreateAssociatedTokenAccountCpi`](token::CreateAssociatedTokenAccountCpi) |
//! | Create Token Account | [`CreateTokenAccount`](token::CreateTokenAccount) | [`CreateTokenAccountCpi`](token::CreateTokenAccountCpi) |
//! | Transfer Token | [`Transfer`](token::Transfer) | [`TransferCpi`](token::TransferCpi) |
//! | Transfer Token → SPL | [`TransferToSpl`](token::TransferToSpl) | [`TransferToSplCpi`](token::TransferToSplCpi) |
//! | Transfer SPL → Token | [`TransferSplToLightToken`](token::TransferSplToLightToken) | [`TransferSplToLightTokenCpi`](token::TransferSplToLightTokenCpi) |
//! | Transfer (auto-detect) | - | [`TransferInterfaceCpi`](token::TransferInterfaceCpi) |
//! | Decompress to Token account | [`Decompress`](token::Decompress) | - |
//! | Close Token account | [`CloseAccount`](token::CloseAccount) | [`CloseAccountCpi`](token::CloseAccountCpi) |
//! | Create Compressed Mint | [`CreateCMint`](token::CreateCMint) | [`CreateCMintCpi`](token::CreateCMintCpi) |
//! | MintTo Token account from Compressed Mint | [`MintTo`](token::MintTo) | [`MintToCpi`](token::MintToCpi) |
//!
//!
//! # Disclaimer
//! This library is not audited and in a beta state. Use at your own risk and expect breaking changes.

pub mod compressed_token;
pub mod compressible;
pub mod token;

pub mod constants;
pub mod error;
pub mod pack;
pub mod spl_interface;
pub mod utils;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
pub use pack::compat;
