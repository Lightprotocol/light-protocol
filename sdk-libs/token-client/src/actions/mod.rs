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
pub mod load;
pub mod mint_to;
pub mod revoke;
pub mod transfer;
pub mod transfer_checked;
pub mod transfer_interface;
pub mod unwrap;
pub mod wrap;

// Re-export all action structs
pub use approve::{create_approve_instructions, Approve};
pub use create_ata::{create_ata_instructions, CreateAta};
pub use create_mint::{
    create_mint_instructions, CreateMint, CreateMintInstructions, TokenMetadata,
};
pub use light_token::instruction::{
    derive_associated_token_account, get_associated_token_address,
    get_associated_token_address_and_bump,
};
pub use load::{create_load_instructions, Load};
pub use mint_to::{create_mint_to_instructions, MintTo};
pub use revoke::{create_revoke_instructions, Revoke};
pub use transfer::{create_transfer_instructions, Transfer};
pub use transfer_checked::{create_transfer_checked_instructions, TransferChecked};
pub use transfer_interface::{create_transfer_interface_instructions, TransferInterface};
pub use unwrap::{create_unwrap_instructions, Unwrap};
pub use wrap::{create_wrap_instructions, Wrap};
