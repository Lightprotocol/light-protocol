use light_hasher::{hash_to_field_size::hashv_to_bn254_field_size_be, Hasher, Keccak};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

// Define derive_address function locally
pub fn derive_address(
    seed: &[u8; 32],
    merkle_tree_pubkey: &[u8; 32],
    program_id_bytes: &[u8; 32],
) -> [u8; 32] {
    let slices = [
        seed.as_slice(),
        merkle_tree_pubkey.as_slice(),
        program_id_bytes.as_slice(),
    ];

    light_hasher::hash_to_field_size::hashv_to_bn254_field_size_be_const_array::<4>(&slices)
        .unwrap()
}

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
    use super::*;

    /// Derives a single address seed for a compressed account, based on the
    /// provided multiple `seeds`, `program_id` and `merkle_tree_pubkey`.
    pub fn derive_address_seed(seeds: &[&[u8]], program_id: &Pubkey) -> [u8; 32] {
        let mut inputs = Vec::with_capacity(seeds.len() + 1);
        inputs.push(program_id.as_slice());
        inputs.extend(seeds);
        hashv_to_bn254_field_size_be_legacy(inputs.as_slice())
    }

    fn hashv_to_bn254_field_size_be_legacy(bytes: &[&[u8]]) -> [u8; 32] {
        let mut hashed_value: [u8; 32] = Keccak::hashv(bytes).unwrap();
        // Truncates to 31 bytes so that value is less than bn254 Fr modulo
        // field size.
        hashed_value[0] = 0;
        hashed_value
    }

    /// Derives an address for a compressed account, based on the provided singular
    /// `seed` and `merkle_tree_pubkey`:
    pub(crate) fn derive_address_from_seed(
        address_seed: &[u8; 32],
        merkle_tree_pubkey: &Pubkey,
    ) -> [u8; 32] {
        let input = [merkle_tree_pubkey.as_slice(), address_seed].concat();
        hashv_to_bn254_field_size_be(&[input.as_slice()])
    }

    /// Derives an address from provided seeds. Returns that address and a singular
    /// seed.
    pub fn derive_address(
        seeds: &[&[u8]],
        merkle_tree_pubkey: &Pubkey,
        program_id: &Pubkey,
    ) -> ([u8; 32], [u8; 32]) {
        let address_seed = derive_address_seed(seeds, program_id);
        let address = derive_address_from_seed(&address_seed, merkle_tree_pubkey);

        (address, address_seed)
    }
}
