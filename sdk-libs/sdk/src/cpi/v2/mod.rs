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
#[cfg(feature = "cpi-context")]
mod accounts_cpi_context;
mod invoke;

pub use accounts::CpiAccounts;
#[cfg(feature = "cpi-context")]
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
/// - [`with_light_account()`](crate::cpi::LightCpiInstruction::with_light_account) - Add a compressed account (handles output hashing, and type conversion to instruction data).
/// - [`with_new_addresses()`](crate::cpi::v2::LightSystemProgramCpi::with_new_addresses) - Create new compressed account addresses.
/// - [`with_read_only_addresses()`](crate::cpi::v2::LightSystemProgramCpi::with_read_only_addresses) - Validate that addresses don't exist without creating them.
/// - [`with_read_only_accounts()`](crate::cpi::v2::LightSystemProgramCpi::with_read_only_accounts) - Validate that compressed account state exists without updating it.
/// - [`compress_lamports()`](crate::cpi::v2::LightSystemProgramCpi::compress_lamports) - Compress SOL into compressed accounts.
/// - [`decompress_lamports()`](crate::cpi::v2::LightSystemProgramCpi::decompress_lamports) - Decompress SOL from compressed accounts.
///
/// **Note**: An instruction can either compress **or** decompress lamports, not both.
/// ## Advanced Methods
///
/// For fine-grained control, use these low-level methods instead of [`with_light_account()`](crate::cpi::LightCpiInstruction::with_light_account):
///
/// - [`with_account_infos()`](crate::cpi::v2::LightSystemProgramCpi::with_account_infos) - Manually specify CompressedAccountInfos.
///
/// # Examples
///
/// ## Create a compressed account with an address
/// ```rust,no_run
/// # use light_sdk::cpi::{v2::LightSystemProgramCpi, InvokeLightSystemProgram, LightCpiInstruction, CpiSigner};
/// # use light_sdk::instruction::ValidityProof;
/// # use light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked;
/// # use light_sdk::{LightAccount, LightDiscriminator};
/// # use borsh::{BorshSerialize, BorshDeserialize};
/// # use solana_pubkey::Pubkey;
/// # use solana_program_error::ProgramError;
/// #
/// # const LIGHT_CPI_SIGNER: CpiSigner = CpiSigner {
/// #     program_id: [0; 32],
/// #     cpi_signer: [0; 32],
/// #     bump: 255,
/// # };
/// #
/// # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
/// # pub struct MyAccount {
/// #     pub value: u64,
/// # }
/// #
/// # fn example() -> Result<(), ProgramError> {
/// # let proof = ValidityProof::default();
/// # let new_address_params = NewAddressParamsAssignedPacked::default();
/// # let program_id = Pubkey::new_unique();
/// # let account = LightAccount::<MyAccount>::new_init(&program_id, None, 0);
/// # let key = Pubkey::new_unique();
/// # let owner = Pubkey::default();
/// # let mut lamports = 0u64;
/// # let mut data = [];
/// # let fee_payer = &solana_account_info::AccountInfo::new(
/// #     &key,
/// #     true,
/// #     true,
/// #     &mut lamports,
/// #     &mut data,
/// #     &owner,
/// #     false,
/// #     0,
/// # );
/// # let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(fee_payer, &[], LIGHT_CPI_SIGNER);
/// LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
///     .with_new_addresses(&[new_address_params])
///     .with_light_account(account)?
///     .invoke(cpi_accounts)?;
/// # Ok(())
/// # }
/// ```
/// ## Update a compressed account
/// ```rust,no_run
/// # use light_sdk::cpi::{v2::LightSystemProgramCpi, InvokeLightSystemProgram, LightCpiInstruction, CpiSigner};
/// # use light_sdk::instruction::ValidityProof;
/// # use light_sdk::{LightAccount, LightDiscriminator};
/// # use light_sdk::instruction::account_meta::CompressedAccountMeta;
/// # use borsh::{BorshSerialize, BorshDeserialize};
/// # use solana_pubkey::Pubkey;
/// # use solana_program_error::ProgramError;
/// #
/// # const LIGHT_CPI_SIGNER: CpiSigner = CpiSigner {
/// #     program_id: [0; 32],
/// #     cpi_signer: [0; 32],
/// #     bump: 255,
/// # };
/// #
/// # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
/// # pub struct MyAccount {
/// #     pub value: u64,
/// # }
/// #
/// # fn example() -> Result<(), ProgramError> {
/// # let proof = ValidityProof::default();
/// # let program_id = Pubkey::new_unique();
/// # let account_meta = CompressedAccountMeta::default();
/// # let account_data = MyAccount::default();
/// # let account = LightAccount::<MyAccount>::new_mut(&program_id, &account_meta, account_data)?;
/// # let key = Pubkey::new_unique();
/// # let owner = Pubkey::default();
/// # let mut lamports = 0u64;
/// # let mut data = [];
/// # let fee_payer = &solana_account_info::AccountInfo::new(
/// #     &key,
/// #     true,
/// #     true,
/// #     &mut lamports,
/// #     &mut data,
/// #     &owner,
/// #     false,
/// #     0,
/// # );
/// # let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(fee_payer, &[], LIGHT_CPI_SIGNER);
/// LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
///     .with_light_account(account)?
///     .invoke(cpi_accounts)?;
/// # Ok(())
/// # }
/// ```
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
