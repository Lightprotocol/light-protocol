use light_account_checks::AccountInfoTrait;
use light_compressed_account::compressed_account::PackedMerkleContext;
// Re-export from light-compressed-account
pub use light_compressed_account::instruction_data::data::PackedAddressTreeInfo;

use crate::{cpi_accounts::TreeAccounts, AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct PackedStateTreeInfo {
    pub root_index: u16,
    pub prove_by_index: bool,
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
}

impl From<PackedStateTreeInfo> for PackedMerkleContext {
    fn from(value: PackedStateTreeInfo) -> Self {
        PackedMerkleContext {
            prove_by_index: value.prove_by_index,
            merkle_tree_pubkey_index: value.merkle_tree_pubkey_index,
            queue_pubkey_index: value.queue_pubkey_index,
            leaf_index: value.leaf_index,
        }
    }
}

/// Extension trait for PackedAddressTreeInfo SDK-specific methods.
/// Since PackedAddressTreeInfo is defined in light-compressed-account,
/// we use an extension trait to add methods that depend on SDK types.
pub trait PackedAddressTreeInfoExt {
    fn get_tree_pubkey<T: AccountInfoTrait + Clone>(
        &self,
        cpi_accounts: &impl TreeAccounts<T>,
    ) -> Result<T::Pubkey, crate::error::LightSdkTypesError>;
}

impl PackedAddressTreeInfoExt for PackedAddressTreeInfo {
    fn get_tree_pubkey<T: AccountInfoTrait + Clone>(
        &self,
        cpi_accounts: &impl TreeAccounts<T>,
    ) -> Result<T::Pubkey, crate::error::LightSdkTypesError> {
        let account =
            cpi_accounts.get_tree_account_info(self.address_merkle_tree_pubkey_index as usize)?;
        Ok(account.pubkey())
    }
}
