//! Native Solana program decoders using macro-derived implementations.
//!
//! This module contains instruction decoders for native Solana programs
//! that use various discriminator sizes:
//! - 1-byte: SPL Token, Token 2022, Compute Budget, Light Token (CToken)
//! - 4-byte: System Program
//! - 8-byte: Anchor programs (Light Registry, Account Compression, Light System)

// Generic Solana program decoders (always available)
pub mod compute_budget;
pub mod spl_token;
pub mod system;
pub mod token_2022;

pub use compute_budget::ComputeBudgetInstructionDecoder;
pub use spl_token::SplTokenInstructionDecoder;
pub use system::SystemInstructionDecoder;
pub use token_2022::Token2022InstructionDecoder;

// Light Protocol program decoders (requires light-protocol feature)
#[cfg(feature = "light-protocol")]
pub mod account_compression;
#[cfg(feature = "light-protocol")]
pub mod ctoken;
#[cfg(feature = "light-protocol")]
pub mod light_system;
#[cfg(feature = "light-protocol")]
pub mod registry;

#[cfg(feature = "light-protocol")]
pub use account_compression::AccountCompressionInstructionDecoder;
#[cfg(feature = "light-protocol")]
pub use ctoken::CTokenInstructionDecoder;
#[cfg(feature = "light-protocol")]
pub use light_system::LightSystemInstructionDecoder;
#[cfg(feature = "light-protocol")]
pub use registry::RegistryInstructionDecoder;
