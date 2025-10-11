pub mod v1 {
    use light_hasher::{hash_to_field_size::hashv_to_bn254_field_size_be, Hasher, Keccak};

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
    pub fn derive_address_seed(seeds: &[&[u8]], program_id: &[u8; 32]) -> [u8; 32] {
        let mut inputs = Vec::with_capacity(seeds.len() + 1);

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
        merkle_tree_pubkey: &[u8; 32],
    ) -> [u8; 32] {
        let input = [merkle_tree_pubkey.as_slice(), address_seed.as_slice()];
        hashv_to_bn254_field_size_be(input.as_slice())
    }

    /// Derives an address from provided seeds. Returns that address and a singular
    /// seed.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use light_sdk::{address::derive_address, pubkey};
    ///
    /// let address_tree_info = {
    ///     address_merkle_tree_pubkey: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
    ///     address_queue_pubkey: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
    /// };
    /// let (address, address_seed) = derive_address(
    ///     &[b"my_compressed_account"],
    ///     &address_tree_info,
    ///     &crate::ID,
    /// );
    /// ```
    pub fn derive_address(
        seeds: &[&[u8]],
        merkle_tree_pubkey: &[u8; 32],
        program_id: &[u8; 32],
    ) -> ([u8; 32], [u8; 32]) {
        let address_seed = derive_address_seed(seeds, program_id);
        let address = derive_address_from_seed(&address_seed, merkle_tree_pubkey);

        (address, address_seed)
    }
}

#[cfg(test)]
mod test {
    use super::v1::*;

    #[allow(dead_code)]
    #[derive(Debug)]
    struct AddressTreeInfo {
        pub address_merkle_tree_pubkey: [u8; 32],
        pub address_queue_pubkey: [u8; 32],
    }

    #[test]
    fn test_derive_address_seed() {
        use light_macros::pubkey_array;
        let program_id = pubkey_array!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

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
        use light_macros::pubkey_array;
        let address_tree_info = AddressTreeInfo {
            address_merkle_tree_pubkey: [0; 32],
            address_queue_pubkey: [0; 32],
        };
        let program_id = pubkey_array!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

        let seeds: &[&[u8]] = &[b"foo", b"bar"];
        let expected_address_seed = [
            0, 246, 150, 3, 192, 95, 53, 123, 56, 139, 206, 179, 253, 133, 115, 103, 120, 155, 251,
            72, 250, 47, 117, 217, 118, 59, 174, 207, 49, 101, 201, 110,
        ];
        let expected_address = [
            0, 141, 60, 24, 250, 156, 15, 250, 237, 196, 171, 243, 182, 10, 8, 66, 147, 57, 27,
            209, 222, 86, 109, 234, 161, 219, 142, 43, 121, 104, 16, 63,
        ];

        let address_seed = derive_address_seed(seeds, &program_id);
        assert_eq!(address_seed, expected_address_seed);
        let (address, address_seed) = derive_address(
            seeds,
            &address_tree_info.address_merkle_tree_pubkey,
            &program_id,
        );
        assert_eq!(address_seed, expected_address_seed);
        assert_eq!(address, expected_address);

        let seeds: &[&[u8]] = &[b"ayy", b"lmao"];
        let expected_address_seed = [
            0, 202, 44, 25, 221, 74, 144, 92, 69, 168, 38, 19, 206, 208, 29, 162, 53, 27, 120, 214,
            152, 116, 15, 107, 212, 168, 33, 121, 187, 10, 76, 233,
        ];
        let expected_address = [
            0, 104, 207, 102, 176, 61, 126, 178, 11, 174, 213, 195, 17, 36, 71, 95, 0, 231, 179,
            87, 218, 195, 114, 84, 47, 97, 176, 93, 106, 175, 72, 115,
        ];

        let address_seed = derive_address_seed(seeds, &program_id);
        assert_eq!(address_seed, expected_address_seed);
        let (address, address_seed) = derive_address(
            seeds,
            &address_tree_info.address_merkle_tree_pubkey,
            &program_id,
        );
        assert_eq!(address_seed, expected_address_seed);
        assert_eq!(address, expected_address);
    }
}
