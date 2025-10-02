#[cfg(all(feature = "v2", feature = "cpi-context"))]
use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;

#[cfg(all(feature = "v2", feature = "cpi-context"))]
use crate::cpi::v2::get_account_metas_from_config_cpi_context;
use crate::{
    cpi::v1::{
        lowlevel::{get_account_metas_from_config, CpiInstructionConfig},
        CpiAccounts,
    },
    AccountInfo, AccountMeta, ProgramError,
};

/// Trait for types that can provide account information for CPI calls
pub trait CpiAccountsTrait<'info> {
    /// Convert to a vector of AccountInfo references
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;

    /// Generate account metas
    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError>;

    /// Get the mode for the instruction (0 for v1, 1 for v2, None if unknown)
    fn get_mode(&self) -> Option<u8>;
}

// Implementation for CpiAccounts
impl<'info> CpiAccountsTrait<'info> for CpiAccounts<'_, 'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.to_account_infos()
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        let config = CpiInstructionConfig::try_from(self).map_err(ProgramError::from)?;
        Ok(get_account_metas_from_config(config))
    }

    fn get_mode(&self) -> Option<u8> {
        Some(0) // v1 mode
    }
}

// Implementation for &[AccountInfo]
impl<'info> CpiAccountsTrait<'info> for &[AccountInfo<'info>] {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.to_vec()
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        // For raw account info slices, create simple account metas
        // preserving the original signer and writable flags
        Ok(self
            .iter()
            .map(|account| AccountMeta {
                pubkey: *account.key,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            })
            .collect())
    }

    fn get_mode(&self) -> Option<u8> {
        None // Unknown mode for raw slices
    }
}

// Implementation for CpiContextWriteAccounts
#[cfg(all(feature = "v2", feature = "cpi-context"))]
impl<'a, 'info> CpiAccountsTrait<'info> for CpiContextWriteAccounts<'a, AccountInfo<'info>> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.fee_payer.clone(),
            self.authority.clone(),
            self.cpi_context.clone(),
        ]
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        // Use the helper function to generate the account metas
        let metas = get_account_metas_from_config_cpi_context(self.clone());
        Ok(metas.to_vec())
    }

    fn get_mode(&self) -> Option<u8> {
        // CPI context write accounts always use v2 mode (1)
        // This type requires both the `v2` and `cpi-context` features
        Some(1)
    }
}
