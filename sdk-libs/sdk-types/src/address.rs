#[derive(Debug, PartialEq, Clone, Copy)]
pub struct AddressSeed(pub [u8; 32]);

impl From<[u8; 32]> for AddressSeed {
    fn from(value: [u8; 32]) -> Self {
        AddressSeed(value)
    }
}

impl From<AddressSeed> for [u8; 32] {
    fn from(address_seed: AddressSeed) -> Self {
        address_seed.0
    }
}

pub type CompressedAddress = [u8; 32];
pub mod v1 {
    use light_hasher::{
        hash_to_field_size::hashv_to_bn254_field_size_be_const_array, Hasher, Keccak,
    };

    use super::AddressSeed;

    /// Derives a single address seed for a compressed account, based on the
    /// provided multiple `seeds`, `program_id` and `address_tree_pubkey`.
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
    pub fn derive_address_seed(seeds: &[&[u8]], program_id: &[u8; 32]) -> AddressSeed {
        let mut inputs: [&[u8]; 16] = [&[]; 16];

        inputs[0] = program_id.as_slice();

        for (i, seed) in seeds.iter().enumerate() {
            inputs[i + 1] = seed;
        }

        let seed = hashv_to_bn254_field_size_be_legacy(inputs.as_slice());
        AddressSeed(seed)
    }

    fn hashv_to_bn254_field_size_be_legacy(bytes: &[&[u8]]) -> [u8; 32] {
        let mut hashed_value: [u8; 32] = Keccak::hashv(bytes)
            .expect("Keccak::hashv should be infallible when keccak feature is enabled");
        // Truncates to 31 bytes so that value is less than bn254 Fr modulo
        // field size.
        hashed_value[0] = 0;
        hashed_value
    }

    /// Derives an address for a compressed account, based on the provided singular
    /// `seed` and `address_tree_pubkey`:
    pub(crate) fn derive_address_from_seed(
        address_seed: &AddressSeed,
        address_tree_pubkey: &[u8; 32],
    ) -> [u8; 32] {
        let input = [address_tree_pubkey.as_slice(), address_seed.0.as_slice()];
        hashv_to_bn254_field_size_be_const_array::<3>(input.as_slice()).unwrap()
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
        address_tree_pubkey: &[u8; 32],
        program_id: &[u8; 32],
    ) -> ([u8; 32], AddressSeed) {
        let address_seed = derive_address_seed(seeds, program_id);
        let address = derive_address_from_seed(&address_seed, address_tree_pubkey);

        (address, address_seed)
    }
}

pub mod v2 {
    use light_hasher::hash_to_field_size::hashv_to_bn254_field_size_be_const_array;

    use super::AddressSeed;

    /// Derives a single address seed for a compressed account, based on the
    /// provided multiple `seeds`, and `address_tree_pubkey`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use light_sdk_types::address::v2::derive_address_seed;
    ///
    /// let address = derive_address_seed(
    ///     &[b"my_compressed_account".as_slice()],
    /// );
    /// ```
    pub fn derive_address_seed(seeds: &[&[u8]]) -> AddressSeed {
        // Max 16 seeds + 1 for bump
        AddressSeed(hashv_to_bn254_field_size_be_const_array::<17>(seeds).unwrap())
    }

    /// Derives an address for a compressed account, based on the provided singular
    /// `seed` and `address_tree_pubkey`:
    pub fn derive_address_from_seed(
        address_seed: &AddressSeed,
        address_tree_pubkey: &[u8; 32],
        program_id: &[u8; 32],
    ) -> [u8; 32] {
        light_compressed_account::address::derive_address(
            &address_seed.0,
            address_tree_pubkey,
            program_id,
        )
    }

    /// Derives an address from provided seeds. Returns that address and a singular
    /// seed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use light_sdk_types::{address::v2::derive_address};
    /// use solana_pubkey::pubkey;
    ///
    /// let program_id = pubkey!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");
    /// let address_tree_pubkey = pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");
    ///
    /// let (address, address_seed) = derive_address(
    ///     &[b"my_compressed_account".as_slice()],
    ///     &address_tree_pubkey.to_bytes(),
    ///     &program_id.to_bytes(),
    /// );
    /// ```
    pub fn derive_address(
        seeds: &[&[u8]],
        address_tree_pubkey: &[u8; 32],
        program_id: &[u8; 32],
    ) -> ([u8; 32], AddressSeed) {
        let address_seed = derive_address_seed(seeds);
        let address = derive_address_from_seed(&address_seed, address_tree_pubkey, program_id);
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
            .into()
        );

        let address_seed = derive_address_seed(&[b"ayy", b"lmao"], &program_id);
        assert_eq!(
            address_seed,
            [
                0, 202, 44, 25, 221, 74, 144, 92, 69, 168, 38, 19, 206, 208, 29, 162, 53, 27, 120,
                214, 152, 116, 15, 107, 212, 168, 33, 121, 187, 10, 76, 233
            ]
            .into()
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
        assert_eq!(address_seed, expected_address_seed.into());
        let (address, address_seed) = derive_address(
            seeds,
            &address_tree_info.address_merkle_tree_pubkey,
            &program_id,
        );
        assert_eq!(address_seed, expected_address_seed.into());
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
        assert_eq!(address_seed, expected_address_seed.into());
        let (address, address_seed) = derive_address(
            seeds,
            &address_tree_info.address_merkle_tree_pubkey,
            &program_id,
        );
        assert_eq!(address_seed, expected_address_seed.into());
        assert_eq!(address, expected_address);
    }

    #[test]
    fn test_v2_derive_address_seed() {
        let seeds: &[&[u8]] = &[b"foo", b"bar"];
        let address_seed = super::v2::derive_address_seed(seeds);

        assert_eq!(
            address_seed.0,
            [
                0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237, 154, 181, 246, 54,
                131, 21, 150, 248, 106, 75, 26, 80, 147, 245, 3, 23, 136, 56
            ]
        );

        let seeds: &[&[u8]] = &[b"ayy", b"lmao"];
        let address_seed = super::v2::derive_address_seed(seeds);

        assert_eq!(
            address_seed.0,
            [
                0, 224, 206, 65, 137, 189, 70, 157, 163, 133, 247, 140, 198, 252, 169, 250, 18, 18,
                16, 189, 164, 131, 225, 113, 197, 225, 64, 81, 175, 154, 221, 28
            ]
        );
    }

    #[test]
    fn test_v2_derive_address() {
        use light_macros::pubkey_array;
        let address_tree_pubkey = [0u8; 32];
        let program_id = pubkey_array!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

        let seeds: &[&[u8]] = &[b"foo", b"bar"];

        let expected_address_seed = [
            0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237, 154, 181, 246, 54, 131,
            21, 150, 248, 106, 75, 26, 80, 147, 245, 3, 23, 136, 56,
        ];
        let expected_address = [
            0, 16, 227, 141, 38, 32, 23, 82, 252, 50, 202, 3, 183, 186, 236, 133, 86, 112, 59, 23,
            128, 162, 11, 84, 91, 127, 179, 208, 25, 178, 1, 240,
        ];

        let address_seed = super::v2::derive_address_seed(seeds);
        assert_eq!(address_seed.0, expected_address_seed);

        let (address, address_seed) =
            super::v2::derive_address(seeds, &address_tree_pubkey, &program_id);
        assert_eq!(address_seed.0, expected_address_seed);
        assert_eq!(address, expected_address);

        let seeds: &[&[u8]] = &[b"ayy", b"lmao"];

        let expected_address_seed = [
            0, 224, 206, 65, 137, 189, 70, 157, 163, 133, 247, 140, 198, 252, 169, 250, 18, 18, 16,
            189, 164, 131, 225, 113, 197, 225, 64, 81, 175, 154, 221, 28,
        ];
        let expected_address = [
            0, 226, 28, 142, 199, 153, 126, 212, 37, 54, 82, 232, 244, 161, 108, 12, 67, 84, 111,
            66, 107, 111, 8, 126, 153, 233, 239, 192, 83, 117, 25, 6,
        ];

        let address_seed = super::v2::derive_address_seed(seeds);
        assert_eq!(address_seed.0, expected_address_seed);

        let (address, address_seed) =
            super::v2::derive_address(seeds, &address_tree_pubkey, &program_id);
        assert_eq!(address_seed.0, expected_address_seed);
        assert_eq!(address, expected_address);
    }
}
