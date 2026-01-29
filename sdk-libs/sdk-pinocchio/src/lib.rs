//! # light-sdk-pinocchio
//!
//! Light Protocol SDK for native Solana programs using pinocchio.
//!
//! | Export | Description |
//! |--------|-------------|
//! | [`derive_light_cpi_signer`] | Derive CPI signer and bump at compile time |
//! | [`LightDiscriminator`] | Discriminator trait; derive macro requires `light-account` feature |
//! | [`LightAccount`] | Compressed account wrapper (requires `light-account` feature) |
//! | [`address`] | Address derivation (v1 and v2) |
//! | [`cpi`] | Light System Program CPI invocation |
//! | [`instruction`] | Instruction types and helpers |

pub mod address;
pub mod cpi;
pub mod error;
pub mod instruction;
// TODO: Add tree_info module with helpers for packing/unpacking address tree info
// Similar to light-sdk's tree_info.rs but adapted for pinocchio (no Anchor dependencies)
// Should include: pack_address_tree_info, unpack_address_tree_info, AddressTreeInfo struct

#[cfg(feature = "light-account")]
pub(crate) use borsh::BorshDeserialize;
pub(crate) use borsh::BorshSerialize;
pub use light_account_checks::discriminator::Discriminator as LightDiscriminator;
pub use light_hasher;
pub use light_macros::{derive_light_cpi_signer, derive_light_cpi_signer_pda};
#[cfg(feature = "light-account")]
pub use light_sdk::LightAccount;
#[cfg(feature = "light-account")]
pub use light_sdk_macros::{LightDiscriminator, LightHasher};
pub use light_sdk_types::{constants, CpiSigner};
