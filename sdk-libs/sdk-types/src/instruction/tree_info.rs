use light_account_checks::AccountInfoTrait;
use light_compressed_account::instruction_data::data::NewAddressParamsPacked;

use crate::{AnchorDeserialize, AnchorSerialize, CpiAccounts};

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct PackedStateTreeInfo {
    pub root_index: u16,
    pub prove_by_index: bool,
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
}

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct PackedAddressTreeInfo {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}

impl PackedAddressTreeInfo {
    pub fn into_new_address_params_packed(self, seed: [u8; 32]) -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            address_merkle_tree_account_index: self.address_merkle_tree_pubkey_index,
            address_queue_account_index: self.address_queue_pubkey_index,
            address_merkle_tree_root_index: self.root_index,
            seed,
        }
    }

    pub fn get_tree_pubkey<T: AccountInfoTrait>(
        &self,
        cpi_accounts: &CpiAccounts<'_, T>,
    ) -> Result<T::Pubkey, crate::error::LightSdkTypesError> {
        let account =
            cpi_accounts.get_tree_account_info(self.address_merkle_tree_pubkey_index as usize)?;
        Ok(account.pubkey())
    }
}
