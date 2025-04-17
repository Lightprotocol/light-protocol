use light_compressed_account::instruction_data::data::{
    NewAddressParams, NewAddressParamsPacked as PackedNewAddressParams,
};

use crate::{
    instruction::{merkle_context::AddressMerkleContext, pack_accounts::PackedAccounts},
    AccountInfo,
};

pub struct AddressWithMerkleContext {
    pub address: [u8; 32],
    pub address_merkle_context: AddressMerkleContext,
}

pub fn pack_new_addresses_params(
    addresses_params: &[NewAddressParams],
    remaining_accounts: &mut PackedAccounts,
) -> Vec<PackedNewAddressParams> {
    addresses_params
        .iter()
        .map(|x| {
            let address_queue_account_index =
                remaining_accounts.insert_or_get(x.address_queue_pubkey);
            let address_merkle_tree_account_index =
                remaining_accounts.insert_or_get(x.address_merkle_tree_pubkey);
            PackedNewAddressParams {
                seed: x.seed,
                address_queue_account_index,
                address_merkle_tree_account_index,
                address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_new_address_params(
    address_params: NewAddressParams,
    remaining_accounts: &mut PackedAccounts,
) -> PackedNewAddressParams {
    pack_new_addresses_params(&[address_params], remaining_accounts)[0]
}

pub fn unpack_new_address_params(
    address_params: &PackedNewAddressParams,
    remaining_accounts: &[AccountInfo],
) -> NewAddressParams {
    let address_merkle_tree_pubkey =
        remaining_accounts[address_params.address_merkle_tree_account_index as usize].key;
    let address_queue_pubkey =
        remaining_accounts[address_params.address_queue_account_index as usize].key;
    NewAddressParams {
        seed: address_params.seed,
        address_queue_pubkey: *address_queue_pubkey,
        address_merkle_tree_pubkey: *address_merkle_tree_pubkey,
        address_merkle_tree_root_index: address_params.address_merkle_tree_root_index,
    }
}

pub mod v1 {
    use light_compressed_account::hashv_to_bn254_field_size_be;
    use light_hasher::{Hasher, Keccak};

    use crate::Pubkey;

    /// Derives a single address seed for a compressed account, based on the
    /// provided multiple `seeds`, `program_id` and `merkle_tree_pubkey`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use light_sdk::{address::derive_address, pubkey};
    ///
    /// let address = derive_address(
    ///     &[b"my_compressed_account"],
    ///     &crate::ID,
    /// );
    /// ```
    pub fn derive_address_seed(seeds: &[&[u8]], program_id: &Pubkey) -> [u8; 32] {
        let mut inputs = Vec::with_capacity(seeds.len() + 1);

        let program_id = program_id.to_bytes();
        inputs.push(program_id.as_slice());

        inputs.extend(seeds);

        let seed = hashv_to_bn254_field_size_be_legacy(inputs.as_slice());
        seed
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
        let input = [merkle_tree_pubkey.to_bytes(), *address_seed].concat();
        hashv_to_bn254_field_size_be(&[input.as_slice()])
    }

    /// Derives an address from provided seeds. Returns that address and a singular
    /// seed.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use light_sdk::{address::derive_address, pubkey};
    ///
    /// let address_merkle_context = {
    ///     address_merkle_tree_pubkey: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
    ///     address_queue_pubkey: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
    /// };
    /// let address = derive_address(
    ///     &[b"my_compressed_account"],
    ///     &address_merkle_context,
    ///     &crate::ID,
    /// );
    /// ```
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

#[cfg(test)]
mod test {
    use light_macros::pubkey;

    use super::{v1::*, *};

    #[test]
    fn test_derive_address_seed() {
        let program_id = pubkey!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

        let address_seed = derive_address_seed(&[b"foo", b"bar"], &program_id);
        assert_eq!(
            address_seed,
            [
                0, 246, 150, 3, 192, 95, 53, 123, 56, 139, 206, 179, 253, 133, 115, 103, 120, 155,
                251, 72, 250, 47, 117, 217, 118, 59, 174, 207, 49, 101, 201, 110
            ]
        );

        let address_seed = derive_address_seed(&[b"ayy", b"lmao"], &program_id);
        assert_eq!(
            address_seed,
            [
                0, 202, 44, 25, 221, 74, 144, 92, 69, 168, 38, 19, 206, 208, 29, 162, 53, 27, 120,
                214, 152, 116, 15, 107, 212, 168, 33, 121, 187, 10, 76, 233
            ]
        );
    }

    #[test]
    fn test_derive_address() {
        let address_merkle_context = AddressMerkleContext {
            address_merkle_tree_pubkey: pubkey!("11111111111111111111111111111111"),
            address_queue_pubkey: pubkey!("22222222222222222222222222222222222222222222"),
        };
        let program_id = pubkey!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

        let seeds: &[&[u8]] = &[b"foo", b"bar"];
        let expected_address_seed = [
            0, 246, 150, 3, 192, 95, 53, 123, 56, 139, 206, 179, 253, 133, 115, 103, 120, 155, 251,
            72, 250, 47, 117, 217, 118, 59, 174, 207, 49, 101, 201, 110,
        ];
        let expected_address = pubkey!("139uhyyBtEh4e1CBDJ68ooK5nCeWoncZf9HPyAfRrukA");

        let address_seed = derive_address_seed(seeds, &program_id);
        assert_eq!(address_seed, expected_address_seed);
        let address = derive_address_from_seed(
            &address_seed,
            &address_merkle_context.address_merkle_tree_pubkey,
        );
        assert_eq!(address, expected_address.to_bytes());
        let (address, address_seed) = derive_address(
            seeds,
            &address_merkle_context.address_merkle_tree_pubkey,
            &program_id,
        );
        assert_eq!(address_seed, expected_address_seed);
        assert_eq!(address, expected_address.to_bytes());

        let seeds: &[&[u8]] = &[b"ayy", b"lmao"];
        let expected_address_seed = [
            0, 202, 44, 25, 221, 74, 144, 92, 69, 168, 38, 19, 206, 208, 29, 162, 53, 27, 120, 214,
            152, 116, 15, 107, 212, 168, 33, 121, 187, 10, 76, 233,
        ];
        let expected_address = pubkey!("12bhHm6PQjbNmEn3Yu1Gq9k7XwVn2rZpzYokmLwbFazN");

        let address_seed = derive_address_seed(seeds, &program_id);
        assert_eq!(address_seed, expected_address_seed);
        let address = derive_address_from_seed(
            &address_seed,
            &address_merkle_context.address_merkle_tree_pubkey,
        );
        assert_eq!(address, expected_address.to_bytes());
        let (address, address_seed) = derive_address(
            seeds,
            &address_merkle_context.address_merkle_tree_pubkey,
            &program_id,
        );
        assert_eq!(address_seed, expected_address_seed);
        assert_eq!(address, expected_address.to_bytes());
    }
}
