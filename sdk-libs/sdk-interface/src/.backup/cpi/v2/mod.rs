//! V2 CPI for Light system program - optimized for compressed PDAs.
//!
//! # Main Types
//!
//! - [`LightSystemProgramCpi`] - CPI instruction data builder
//! - [`CpiAccounts`] - CPI accounts struct
//!
//!
//! # Advanced Usage
//!
//! For maximum flexible light system program CPIs, see the [`lowlevel`] module or use `light-compressed-account` directly.

mod accounts;
mod accounts_cpi_context;
mod invoke;

pub use accounts::CpiAccounts;
pub use accounts_cpi_context::*;
/// Light system program CPI instruction data builder.
///
/// Use this builder to construct instructions for compressed account operations:
/// creating, updating, closing accounts, and compressing/decompressing SOL.
///
/// # Builder Methods
///
/// ## Common Methods
///
/// - [`with_new_addresses()`](crate::cpi::v2::LightSystemProgramCpi::with_new_addresses) - Create new compressed account addresses.
/// - [`with_read_only_addresses()`](crate::cpi::v2::LightSystemProgramCpi::with_read_only_addresses) - Validate that addresses don't exist without creating them.
/// - [`with_read_only_accounts()`](crate::cpi::v2::LightSystemProgramCpi::with_read_only_accounts) - Validate that compressed account state exists without updating it.
/// - [`compress_lamports()`](crate::cpi::v2::LightSystemProgramCpi::compress_lamports) - Compress SOL into compressed accounts.
/// - [`decompress_lamports()`](crate::cpi::v2::LightSystemProgramCpi::decompress_lamports) - Decompress SOL from compressed accounts.
///
/// **Note**: An instruction can either compress **or** decompress lamports, not both.
/// ## Advanced Methods
///
/// For fine-grained control:
///
/// - [`with_account_infos()`](crate::cpi::v2::LightSystemProgramCpi::with_account_infos) - Manually specify CompressedAccountInfos.
pub use light_compressed_account::instruction_data::with_account_info::InstructionDataInvokeCpiWithAccountInfo as LightSystemProgramCpi;

/// Low-level types and functions for flexible Light system program CPIs.
///
/// # Main Types
///
/// For most use cases, you only need:
/// - [`LightSystemProgramCpi`] - Main CPI interface
/// - [`CpiAccounts`] - Account management
///
/// The remaining types in this module are exported for low-level operations and internal use.
pub mod lowlevel {

    /// CPI context for batched compressed account operations.
    pub use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
    /// Account information for compressed accounts in CPI operations.
    pub use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
    /// Input account information for compressed accounts.
    pub use light_compressed_account::instruction_data::with_account_info::InAccountInfo;
    /// Output account information for compressed accounts.
    pub use light_compressed_account::instruction_data::with_account_info::OutAccountInfo;
    /// Input compressed account for read-only operations.
    pub use light_compressed_account::instruction_data::with_readonly::InAccount;
    /// V2 CPI instruction data for read-only compressed account operations.
    ///
    /// Provides more flexibility for complex operations such as changing the compressed account owner.
    /// Most users should use [`crate::cpi::v2::LightSystemProgramCpi`] instead.
    pub use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;

    pub use crate::cpi::v2::accounts::to_account_metas;
}
