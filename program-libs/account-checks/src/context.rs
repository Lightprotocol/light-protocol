use crate::{AccountInfo, ProgramError};

pub trait LightContext<'info>: Sized {
    /// Attributes:
    /// - `#[signer]` - account must be a signer
    /// - `#[account(zero)]` - account must be empty
    /// - `#[account(Option<ProgramId>)]` - checks owner is this program
    /// - `#[unchecked_account]` - account is not checked
    /// - `#[pda_derivation(seeds, Option<ProgramId)- account is derived from seeds
    /// - `#[constraint = statement ]` - custom constraint
    /// - `#[compressed_account(Option<ProgramId>)]` - account is compressed owner is this program by default
    /// - '#[program]` - account is a program
    /// Macro rules for this function:
    /// 1. check that accounts len is sufficient
    ///     1.1. count number of fields marked with account attribute
    ///     1.2. throw if a field is not marked
    /// 2. create a variable for each account
    ///
    /// Notes:
    /// 1. replace instruction_data with optional T to keep instruction data deserialization separate
    #[cfg(feature = "pinocchio")]
    fn from_account_infos<T>(
        accounts: &'info [AccountInfo],
        options_config: Option<T>,
    ) -> Result<(Self, &'info [AccountInfo]), ProgramError>;

    #[cfg(not(feature = "pinocchio"))]
    fn from_account_infos<T>(
        accounts: &'info [AccountInfo<'info>],
        options_config: Option<T>,
    ) -> Result<(Self, &'info [AccountInfo<'info>]), ProgramError>;
}
