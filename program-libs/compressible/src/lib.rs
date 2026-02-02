//! # light-compressible
//!
//! Compressible account lifecycle for accounts with sponsored rent-exemption.
//! The program pays the rent exemption for the account. Transaction fee payers
//! bump a virtual rent balance when writing to the account, which keeps the
//! account "hot". "Cold" accounts virtual rent balance below threshold
//! (eg 24h without write bump) get auto-compressed. The cold account's state
//! is cryptographically preserved on the Solana ledger. Users can load a
//! cold account into hot state in-flight when using the account again.

//!
//! | Type | Description |
//! |------|-------------|
//! | [`CompressionInfo`](compression_info::CompressionInfo) | Rent state, authorities, and compression config per account |
//! | [`CompressibleConfig`](config::CompressibleConfig) | Program-level config: rent sponsor, authorities, address space |
//! | [`RentConfig`](rent::RentConfig) | Rent function parameters for compression eligibility |
//! | [`compression_info`] | `is_compressible`, `claim`, and top-up logic |
//! | [`registry_instructions`] | Instructions for the compression registry |
//! | [`rent`] | Epoch-based rent calculation and claim amounts |

pub mod compression_info;
pub mod config;
pub mod error;
pub mod registry_instructions;
pub mod rent;

/// Decompressed PDA discriminator - marks a compressed account as a decompressed PDA placeholder.
/// When a CMint or other PDA is decompressed to a Solana account, the compressed account
/// stores this discriminator and the PDA pubkey (hashed) to preserve the address.
pub const DECOMPRESSED_PDA_DISCRIMINATOR: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 0];

#[cfg(feature = "anchor")]
pub(crate) use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub(crate) use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
