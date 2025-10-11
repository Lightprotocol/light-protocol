use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

// Define data structures needed
#[derive(Clone, Debug, Default)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: [u8; 32],
    pub address_merkle_tree_pubkey: [u8; 32],
    pub address_merkle_tree_root_index: u16,
}

pub fn unpack_new_address_params(
    address_params: &crate::NewAddressParamsPacked,
    remaining_accounts: &[AccountInfo],
) -> NewAddressParams {
    let address_merkle_tree_pubkey =
        remaining_accounts[address_params.address_merkle_tree_account_index as usize].key();
    let address_queue_pubkey =
        remaining_accounts[address_params.address_queue_account_index as usize].key();

    NewAddressParams {
        seed: address_params.seed,
        address_queue_pubkey: *address_queue_pubkey,
        address_merkle_tree_pubkey: *address_merkle_tree_pubkey,
        address_merkle_tree_root_index: address_params.address_merkle_tree_root_index,
    }
}

pub mod v1 {
    use light_sdk_types::address::AddressSeed;

    use super::*;

    /// Derives a single address seed for a compressed account, based on the
    /// provided multiple `seeds`, `program_id` and `merkle_tree_pubkey`.
    pub fn derive_address_seed(seeds: &[&[u8]], program_id: &Pubkey) -> AddressSeed {
        light_sdk_types::address::v1::derive_address_seed(seeds, program_id)
    }

    /// Derives an address from provided seeds. Returns that address and a singular
    /// seed.
    pub fn derive_address(
        seeds: &[&[u8]],
        merkle_tree_pubkey: &Pubkey,
        program_id: &Pubkey,
    ) -> ([u8; 32], AddressSeed) {
        light_sdk_types::address::v1::derive_address(seeds, merkle_tree_pubkey, program_id)
    }
}

#[cfg(feature = "v2")]
pub mod v2 {
    use light_sdk_types::address::AddressSeed;

    use super::*;

    /// Derives a single address seed for a compressed account, based on the
    /// provided multiple `seeds`.
    pub fn derive_address_seed(seeds: &[&[u8]]) -> AddressSeed {
        light_sdk_types::address::v2::derive_address_seed(seeds)
    }

    /// Derives an address for a compressed account, based on the provided singular
    /// `seed` and `address_tree_pubkey`:
    pub fn derive_address_from_seed(
        address_seed: &AddressSeed,
        address_tree_pubkey: &Pubkey,
        program_id: &Pubkey,
    ) -> [u8; 32] {
        light_sdk_types::address::v2::derive_address_from_seed(
            address_seed,
            address_tree_pubkey,
            program_id,
        )
    }

    /// Derives an address from provided seeds. Returns that address and a singular
    /// seed.
    pub fn derive_address(
        seeds: &[&[u8]],
        address_tree_pubkey: &Pubkey,
        program_id: &Pubkey,
    ) -> ([u8; 32], AddressSeed) {
        light_sdk_types::address::v2::derive_address(seeds, address_tree_pubkey, program_id)
    }
}
