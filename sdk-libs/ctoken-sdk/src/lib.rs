//! # cToken SDK
//!
//! The base library to use cToken Accounts, cMints, and compressed token accounts.
//!
//! ## cToken Accounts
//! - are on Solana devnet.
//! - are Solana accounts.
//! - can hold cMint and spl Mint tokens.
//! - cost 22,000 lamports to create with 24 hours rent.
//! - are compressible:
//!     - rent exemption is sponsored by the protocol.
//!     - rent is 388 lamports per rent epoch (1.5 hours).
//!     - once the account's lamports balance is insufficient, it is compressed to a compressed token account.
//!     - compressed tokens can be decompressed to a cToken account.
//!
//! ## Compressed Token Accounts
//! - are on Solana mainnet.
//! - are compressed accounts.
//! - can hold cMint and spl Mint tokens.
//! - cost 5,000 lamports to create.
//! - are well suited for airdrops and reward distribution.
//!
//! ## cMints:
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
//! | Create Associated cToken Account | [`CreateAssociatedTokenAccount`](ctoken::CreateAssociatedTokenAccount) | [`CreateAssociatedTokenAccountCpi`](ctoken::CreateAssociatedTokenAccountCpi) |
//! | Create cToken Account | [`CreateCTokenAccount`](ctoken::CreateCTokenAccount) | [`CreateCTokenAccountCpi`](ctoken::CreateCTokenAccountCpi) |
//! | TransferInterface | [`TransferCtoken`](ctoken::TransferCtoken) | [`TransferInterface`](ctoken::TransferInterface) |
//! | Close cToken account | [`CloseCTokenAccount`](ctoken::CloseCTokenAccount) | [`CloseCTokenAccountCpi`](ctoken::CloseCTokenAccountCpi) |
//! | Create cMint | [`CreateCMint`](ctoken::CreateCMint) | [`CreateCMintCpi`](ctoken::CreateCMintCpi) |
//! | MintTo cToken account from cMint | [`MintToCToken`](ctoken::MintToCToken) | [`MintToCTokenCpi`](ctoken::MintToCTokenCpi) |
//!
//! Note, TransferInterface supports tokens transfer between ctoken - ctoken, ctoken - spl, spl - ctoken accounts.
//!
//! Disclaimer, this library is not audited and in a beta state. Use at your own risk and expect breaking changes.

pub mod compressed_token;
pub mod compressible;
pub mod ctoken;

pub mod error;
pub mod pack;
pub mod spl_interface;
pub mod utils;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
#[cfg(feature = "compressible")]
pub use compressible::decompress_runtime::{process_decompress_tokens_runtime, CTokenSeedProvider};
pub use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
pub use light_compressed_token_types::*;
pub use pack::compat;
#[cfg(feature = "compressible")]
pub use pack::{Pack, Unpack};
pub use utils::{
    account_meta_from_account_info, is_ctoken_account, AccountInfoToCompress,
    PackedCompressedTokenDataWithContext,
};
