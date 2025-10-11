use light_account_checks::AccountInfoTrait;
use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::data::{NewAddressParamsAssignedPacked, NewAddressParamsPacked},
};

use crate::{address::AddressSeed, cpi_accounts::TreeAccounts, AnchorDeserialize, AnchorSerialize};

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

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct PackedAddressTreeInfo {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}

impl PackedAddressTreeInfo {
    pub fn into_new_address_params_packed(self, seed: AddressSeed) -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            address_merkle_tree_account_index: self.address_merkle_tree_pubkey_index,
            address_queue_account_index: self.address_queue_pubkey_index,
            address_merkle_tree_root_index: self.root_index,
            seed: seed.0,
        }
    }

    pub fn into_new_address_params_assigned_packed(
        self,
        seed: AddressSeed,
        assigned_account_index: Option<u8>,
    ) -> NewAddressParamsAssignedPacked {
        NewAddressParamsAssignedPacked {
            address_merkle_tree_account_index: self.address_merkle_tree_pubkey_index,
            address_queue_account_index: self.address_queue_pubkey_index,
            address_merkle_tree_root_index: self.root_index,
            seed: seed.0,
            assigned_account_index: assigned_account_index.unwrap_or_default(),
            assigned_to_account: assigned_account_index.is_some(),
        }
    }

    pub fn get_tree_pubkey<T: AccountInfoTrait + Clone>(
        &self,
        cpi_accounts: &impl TreeAccounts<T>,
    ) -> Result<T::Pubkey, crate::error::LightSdkTypesError> {
        let account =
            cpi_accounts.get_tree_account_info(self.address_merkle_tree_pubkey_index as usize)?;
        Ok(account.pubkey())
    }
}
