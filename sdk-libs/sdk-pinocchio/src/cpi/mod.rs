mod account;
mod instruction;
pub mod invoke;

pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;

pub use account::*;
pub use instruction::*;
pub use invoke::InvokeLightSystemProgram;
/// Derives cpi signer and bump to invoke the light system program at compile time.
pub use light_macros::derive_light_cpi_signer;
/// Contains program id, derived cpi signer, and bump.
pub use light_sdk_types::{cpi_accounts::CpiAccountsConfig, CpiSigner};
