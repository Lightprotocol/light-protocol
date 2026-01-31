use light_sdk_interface::instruction::PackedAccounts;

use super::system_accounts::{get_light_system_account_metas, SystemAccountMetaConfig};

/// Extension trait adding Light system account helpers to [`PackedAccounts`].
///
/// These methods depend on [`SystemAccountMetaConfig`] and `find_cpi_signer_macro!`
/// which are SDK-specific (use CPI signer derivation).
pub trait PackedAccountsExt {
    /// Creates a new [`PackedAccounts`] with v1 system accounts pre-configured.
    ///
    /// **Use with [`cpi::v1::CpiAccounts`](crate::cpi::v1::CpiAccounts) on the program side.**
    fn new_with_system_accounts(config: SystemAccountMetaConfig) -> crate::error::Result<Self>
    where
        Self: Sized;

    /// Adds v1 Light system program accounts to the account list.
    ///
    /// **Use with [`cpi::v1::CpiAccounts`](crate::cpi::v1::CpiAccounts) on the program side.**
    ///
    /// This adds all the accounts required by the Light system program for v1 operations,
    /// including the CPI authority, registered programs, account compression program, and Noop program.
    fn add_system_accounts(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()>;

    /// Adds v2 Light system program accounts to the account list.
    ///
    /// **Use with [`cpi::v2::CpiAccounts`](crate::cpi::v2::CpiAccounts) on the program side.**
    ///
    /// This adds all the accounts required by the Light system program for v2 operations.
    /// V2 uses a different account layout optimized for batched state trees.
    #[cfg(feature = "v2")]
    fn add_system_accounts_v2(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()>;
}

impl PackedAccountsExt for PackedAccounts {
    fn new_with_system_accounts(config: SystemAccountMetaConfig) -> crate::error::Result<Self> {
        let mut accounts = PackedAccounts::default();
        accounts.add_system_accounts(config)?;
        Ok(accounts)
    }

    fn add_system_accounts(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()> {
        self.add_system_accounts_raw(get_light_system_account_metas(config));
        Ok(())
    }

    #[cfg(feature = "v2")]
    fn add_system_accounts_v2(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()> {
        self.add_system_accounts_raw(
            super::system_accounts::get_light_system_account_metas_v2(config),
        );
        Ok(())
    }
}
