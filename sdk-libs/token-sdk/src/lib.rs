//! # Light Token SDK
//!
//! The base library to use Light Token Accounts, Light Mints, and compressed token accounts.
//!
//! ## Light Token Accounts
//! - are on Solana devnet.
//! - are Solana accounts.
//! - can hold Light Mint and SPL Mint tokens.
//! - cost 17,288 lamports to create with 24 hours rent.
//! - are rentfree:
//!     - rent exemption is sponsored by the protocol.
//!     - rent is 388 lamports per rent epoch (1.5 hours).
//!     - once the account's lamports balance is insufficient, it is compressed to a compressed token account.
//!     - compressed tokens can be decompressed to a Light Token account.
//!     - configurable lamports per write (eg transfer) keep the Light Token account perpetually funded when used. So you don't have to worry about funding rent.
//!
//! ## Compressed Token Accounts
//! - are on Solana mainnet.
//! - are compressed accounts.
//! - can hold Light Mint and SPL Mint tokens.
//! - cost 5,000 lamports to create.
//! - are well suited for airdrops and reward distribution.
//!
//! ## Light Mints
//! - are on Solana devnet.
//! - are Compressed accounts.
//! - cost 15,000 lamports to create.
//! - support `TokenMetadata`.
//!
//!
//! For full program examples, see the [Light Token Examples](https://github.com/Lightprotocol/examples-light-token).
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
//! | Create Associated Token Account | [`CreateAssociatedTokenAccount`](instruction::CreateAssociatedTokenAccount) | [`CreateAssociatedAccountCpi`](instruction::CreateAssociatedAccountCpi) |
//! | Create Token Account | [`CreateTokenAccount`](instruction::CreateTokenAccount) | [`CreateTokenAccountCpi`](instruction::CreateTokenAccountCpi) |
//! | Transfer | [`Transfer`](instruction::Transfer) | [`TransferCpi`](instruction::TransferCpi) |
//! | Transfer Interface (auto-detect) | [`TransferInterface`](instruction::TransferInterface) | [`TransferInterfaceCpi`](instruction::TransferInterfaceCpi) |
//! | Close Token account | [`CloseAccount`](instruction::CloseAccount) | [`CloseAccountCpi`](instruction::CloseAccountCpi) |
//! | Create Mint | [`CreateMint`](instruction::CreateMint) | [`CreateMintCpi`](instruction::CreateMintCpi) |
//! | MintTo | [`MintTo`](instruction::MintTo) | [`MintToCpi`](instruction::MintToCpi) |
//!
//!
//! # Disclaimer
//! This library is not audited and in a beta state. Use at your own risk and expect breaking changes.

#[cfg(feature = "anchor")]
pub mod anchor;
pub mod compressible;
pub mod constants;
pub mod error;
pub mod instruction;
// pub mod pack;
pub mod spl_interface;
pub mod utils;

// Re-export key constants and functions from constants module
pub use constants::{
    config_pda, cpi_authority, id, LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID,
};
pub use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
pub use light_compressed_token_sdk::compat;
pub use light_token_interface::{
    instructions::extensions::{ExtensionInstructionData, TokenMetadataInstructionData},
    state::AdditionalMetadata,
};
