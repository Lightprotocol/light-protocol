use std::ops::{Deref, DerefMut};

use super::system_accounts::{get_light_system_account_metas, SystemAccountMetaConfig};

type Inner = light_sdk_types::pack_accounts::PackedAccounts<solana_instruction::AccountMeta>;

/// Packs accounts and creates indices for instruction building (client-side).
///
/// Wraps the generic `PackedAccounts<AccountMeta>` from sdk-types with
/// Solana-specific system account helpers as inherent methods.
#[derive(Debug, Default)]
pub struct PackedAccounts(pub Inner);

impl Deref for PackedAccounts {
    type Target = Inner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PackedAccounts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Inner> for PackedAccounts {
    fn from(inner: Inner) -> Self {
        Self(inner)
    }
}

impl From<PackedAccounts> for Inner {
    fn from(wrapper: PackedAccounts) -> Self {
        wrapper.0
    }
}

impl PackedAccounts {
    /// Creates a new [`PackedAccounts`] with v1 system accounts pre-configured.
    ///
    /// **Use with [`cpi::v1::CpiAccounts`](crate::cpi::v1::CpiAccounts) on the program side.**
    pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> crate::error::Result<Self> {
        let mut accounts = Self::default();
        accounts.add_system_accounts(config)?;
        Ok(accounts)
    }

    /// Adds v1 Light system program accounts to the account list.
    ///
    /// **Use with [`cpi::v1::CpiAccounts`](crate::cpi::v1::CpiAccounts) on the program side.**
    ///
    /// This adds all the accounts required by the Light system program for v1 operations,
    /// including the CPI authority, registered programs, account compression program, and Noop program.
    pub fn add_system_accounts(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()> {
        self.0
            .add_system_accounts_raw(get_light_system_account_metas(config));
        Ok(())
    }

    /// Adds v2 Light system program accounts to the account list.
    ///
    /// **Use with [`cpi::v2::CpiAccounts`](crate::cpi::v2::CpiAccounts) on the program side.**
    ///
    /// This adds all the accounts required by the Light system program for v2 operations.
    /// V2 uses a different account layout optimized for batched state trees.
    #[cfg(feature = "v2")]
    pub fn add_system_accounts_v2(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()> {
        self.0
            .add_system_accounts_raw(super::system_accounts::get_light_system_account_metas_v2(
                config,
            ));
        Ok(())
    }
}
