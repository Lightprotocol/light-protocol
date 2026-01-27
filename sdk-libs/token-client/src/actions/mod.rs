//! Clean action interfaces for Light Token operations.
//!
//! These actions provide simple, ergonomic interfaces for common Light Token operations.
//!
//! All actions use a params struct pattern with an `execute` method:
//! ```ignore
//! Transfer {
//!     source,
//!     destination,
//!     amount: 1000,
//!     ..Default::default()
//! }.execute(&mut rpc, &payer, &authority).await?;
//! ```

pub mod approve;
pub mod create_ata;
pub mod create_mint;
pub mod mint_to;
pub mod revoke;
pub mod transfer;
pub mod transfer_checked;
pub mod transfer_interface;
pub mod unwrap;
pub mod wrap;

// Re-export all action structs
pub use approve::Approve;
pub use create_ata::CreateAta;
pub use create_mint::{CreateMint, TokenMetadata};
pub use light_token::instruction::{
    derive_associated_token_account, get_associated_token_address,
    get_associated_token_address_and_bump,
};
pub use mint_to::MintTo;
pub use revoke::Revoke;
pub use transfer::Transfer;
pub use transfer_checked::TransferChecked;
pub use transfer_interface::TransferInterface;
pub use unwrap::Unwrap;
pub use wrap::Wrap;
