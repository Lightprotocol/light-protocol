//! ## Addresses
//! Address seed is 32 bytes. Multiple seeds are hashed
//! into a single 32 bytes seed that is passed into the light system program for address creation.
//! Addresses are created independently from compressed accounts.
//! This means that an address can be used in a compressed account but does not have to be used.
//!
//! ### Address uniqueness
//! Every address can only be created once per address tree.
//! Addresses over all address trees are unique but
//! address seeds can be reused in different address trees.
//! If your program security requires global address uniqueness over all address trees,
//! the used address Merkle tree must be checked.
//! If your program just requires addresses to identify accounts but not uniqueness over all address trees
//! the used address Merkle tree does not need to be checked.
//!
//!
//! ### Create address example
//! ```ignore
//! let packed_address_tree_info = instruction_data.address_tree_info;
//! let tree_accounts = cpi_accounts.tree_accounts();
//!
//! let address_tree_pubkey = tree_accounts[address_tree_info
//!    .address_merkle_tree_pubkey_index as usize]
//!    .key();
//!
//! let (address, address_seed) = derive_address(
//!     &[b"counter"],
//!     &address_tree_pubkey,
//!     &crate::ID,
//! );
//!
//! // Used in cpi to light-system program
//! // to insert the new address into the address merkle tree.
//! let new_address_params = packed_address_tree_info
//!     .into_new_address_params_packed(address_seed);
//! ```

pub use light_compressed_account::instruction_data::data::NewAddressParams;
/// Struct passed into the light system program cpi to create a new address.
pub use light_compressed_account::instruction_data::data::NewAddressParamsPacked as PackedNewAddressParams;
#[cfg(feature = "v2")]
pub use light_compressed_account::instruction_data::data::{
    NewAddressParamsAssigned, NewAddressParamsAssignedPacked, PackedReadOnlyAddress,
    ReadOnlyAddress,
};

pub mod v1 {

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
        light_sdk_types::address::v1::derive_address_seed(seeds, &program_id.to_bytes())
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
        merkle_tree_pubkey: &Pubkey,
        program_id: &Pubkey,
    ) -> ([u8; 32], [u8; 32]) {
        light_sdk_types::address::v1::derive_address(
            seeds,
            &merkle_tree_pubkey.to_bytes(),
            &program_id.to_bytes(),
        )
    }
}

#[cfg(test)]
mod test {
    use solana_pubkey::pubkey;

    use super::v1::*;
    use crate::instruction::AddressTreeInfo;

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
        let address_tree_info = AddressTreeInfo {
            tree: pubkey!("11111111111111111111111111111111"),
            queue: pubkey!("22222222222222222222222222222222222222222222"),
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
        let (address, address_seed) = derive_address(seeds, &address_tree_info.tree, &program_id);
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
        let (address, address_seed) = derive_address(seeds, &address_tree_info.tree, &program_id);
        assert_eq!(address_seed, expected_address_seed);
        assert_eq!(address, expected_address.to_bytes());
    }
}
