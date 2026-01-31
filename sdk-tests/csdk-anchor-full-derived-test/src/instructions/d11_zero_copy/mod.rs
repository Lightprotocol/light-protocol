//! D11: Zero-copy (AccountLoader) instruction account structs.
//!
//! Tests `#[light_account(init, zero_copy)]` with various combinations:
//! - Zero-copy + Token Vault
//! - Zero-copy + ATA
//! - Multiple zero-copy PDAs
//! - Mixed zero-copy and Borsh accounts
//! - Zero-copy with ctx.accounts.* seeds
//! - Zero-copy with params-only seeds
//! - Zero-copy + Vault + MintTo

pub mod mixed_zc_borsh;
pub mod multiple_zc;
pub mod with_ata;
pub mod with_ctx_seeds;
pub mod with_mint_to;
pub mod with_params_seeds;
pub mod with_vault;

pub use mixed_zc_borsh::*;
pub use multiple_zc::*;
pub use with_ata::*;
pub use with_ctx_seeds::*;
pub use with_mint_to::*;
pub use with_params_seeds::*;
pub use with_vault::*;
