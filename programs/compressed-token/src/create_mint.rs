use anchor_lang::prelude::Pubkey;
use anchor_lang::{prelude::borsh, AnchorDeserialize, AnchorSerialize};
use light_compressed_account::hash_to_bn254_field_size_be;
use light_hasher::{errors::HasherError, Hasher, Poseidon};

// TODO: add is native_compressed, this means that the compressed mint is always synced with the spl mint
//      compressed mint accounts which are not native_compressed can be not in sync the spl mint account is the source of truth
// Order is optimized for hashing.
// freeze_authority option is skipped if None.
#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CompressedMint {
    /// Pda with seed address of compressed mint
    pub spl_mint: Pubkey,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Extension, necessary for mint to.
    pub is_decompressed: bool,
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: Option<Pubkey>,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
    // Not necessary.
    // /// Is `true` if this structure has been initialized
    // pub is_initialized: bool,
    pub num_extensions: u8, // TODO: check again how token22 does it
}

impl CompressedMint {
    pub fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        let hashed_spl_mint = hash_to_bn254_field_size_be(self.spl_mint.to_bytes().as_slice());
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..].copy_from_slice(self.supply.to_be_bytes().as_slice());

        let hashed_mint_authority;
        let hashed_mint_authority_option = if let Some(mint_authority) = self.mint_authority {
            hashed_mint_authority =
                hash_to_bn254_field_size_be(mint_authority.to_bytes().as_slice());
            Some(&hashed_mint_authority)
        } else {
            None
        };

        let hashed_freeze_authority;
        let hashed_freeze_authority_option = if let Some(freeze_authority) = self.freeze_authority {
            hashed_freeze_authority =
                hash_to_bn254_field_size_be(freeze_authority.to_bytes().as_slice());
            Some(&hashed_freeze_authority)
        } else {
            None
        };

        Self::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            self.decimals,
            self.is_decompressed,
            &hashed_mint_authority_option,
            &hashed_freeze_authority_option,
            self.num_extensions,
        )
    }

    pub fn hash_with_hashed_values(
        hashed_spl_mint: &[u8; 32],
        supply_bytes: &[u8; 32],
        decimals: u8,
        is_decompressed: bool,
        hashed_mint_authority: &Option<&[u8; 32]>,
        hashed_freeze_authority: &Option<&[u8; 32]>,
        num_extensions: u8,
    ) -> std::result::Result<[u8; 32], HasherError> {
        let mut hash_inputs = vec![hashed_spl_mint.as_slice(), supply_bytes.as_slice()];

        // Add decimals with prefix if not 0
        let mut decimals_bytes = [0u8; 32];
        if decimals != 0 {
            decimals_bytes[30] = 1; // decimals prefix
            decimals_bytes[31] = decimals;
            hash_inputs.push(&decimals_bytes[..]);
        }

        // Add is_decompressed with prefix if true
        let mut is_decompressed_bytes = [0u8; 32];
        if is_decompressed {
            is_decompressed_bytes[30] = 2; // is_decompressed prefix
            is_decompressed_bytes[31] = 1; // true as 1
            hash_inputs.push(&is_decompressed_bytes[..]);
        }

        // Add mint authority if present
        if let Some(hashed_mint_authority) = hashed_mint_authority {
            hash_inputs.push(hashed_mint_authority.as_slice());
        }

        // Add freeze authority if present
        let empty_authority = [0u8; 32];
        if let Some(hashed_freeze_authority) = hashed_freeze_authority {
            // If there is freeze authority but no mint authority, add empty mint authority
            if hashed_mint_authority.is_none() {
                hash_inputs.push(&empty_authority[..]);
            }
            hash_inputs.push(hashed_freeze_authority.as_slice());
        }

        // Add num_extensions with prefix if not 0
        let mut num_extensions_bytes = [0u8; 32];
        if num_extensions != 0 {
            num_extensions_bytes[30] = 3; // num_extensions prefix
            num_extensions_bytes[31] = num_extensions;
            hash_inputs.push(&num_extensions_bytes[..]);
        }

        Poseidon::hashv(hash_inputs.as_slice())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_equivalency_of_hash_functions() {
        let compressed_mint = CompressedMint {
            spl_mint: Pubkey::new_unique(),
            supply: 1000000,
            decimals: 6,
            is_decompressed: false,
            mint_authority: Some(Pubkey::new_unique()),
            freeze_authority: Some(Pubkey::new_unique()),
            num_extensions: 2,
        };

        let hash_result = compressed_mint.hash().unwrap();

        // Test with hashed values
        let hashed_spl_mint =
            hash_to_bn254_field_size_be(compressed_mint.spl_mint.to_bytes().as_slice());
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..].copy_from_slice(compressed_mint.supply.to_be_bytes().as_slice());

        let hashed_mint_authority = hash_to_bn254_field_size_be(
            compressed_mint
                .mint_authority
                .unwrap()
                .to_bytes()
                .as_slice(),
        );
        let hashed_freeze_authority = hash_to_bn254_field_size_be(
            compressed_mint
                .freeze_authority
                .unwrap()
                .to_bytes()
                .as_slice(),
        );

        let hash_with_hashed_values = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            compressed_mint.decimals,
            compressed_mint.is_decompressed,
            &Some(&hashed_mint_authority),
            &Some(&hashed_freeze_authority),
            compressed_mint.num_extensions,
        )
        .unwrap();

        assert_eq!(hash_result, hash_with_hashed_values);
    }

    #[test]
    fn test_equivalency_without_optional_fields() {
        let compressed_mint = CompressedMint {
            spl_mint: Pubkey::new_unique(),
            supply: 500000,
            decimals: 0,
            is_decompressed: false,
            mint_authority: None,
            freeze_authority: None,
            num_extensions: 0,
        };

        let hash_result = compressed_mint.hash().unwrap();

        let hashed_spl_mint =
            hash_to_bn254_field_size_be(compressed_mint.spl_mint.to_bytes().as_slice());
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..].copy_from_slice(compressed_mint.supply.to_be_bytes().as_slice());

        let hash_with_hashed_values = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            compressed_mint.decimals,
            compressed_mint.is_decompressed,
            &None,
            &None,
            compressed_mint.num_extensions,
        )
        .unwrap();

        assert_eq!(hash_result, hash_with_hashed_values);
    }

    fn equivalency_of_hash_functions_rnd_iters<const ITERS: usize>() {
        let mut rng = rand::thread_rng();

        for _ in 0..ITERS {
            let compressed_mint = CompressedMint {
                spl_mint: Pubkey::new_unique(),
                supply: rng.gen(),
                decimals: rng.gen_range(0..=18),
                is_decompressed: rng.gen_bool(0.5),
                mint_authority: if rng.gen_bool(0.5) {
                    Some(Pubkey::new_unique())
                } else {
                    None
                },
                freeze_authority: if rng.gen_bool(0.5) {
                    Some(Pubkey::new_unique())
                } else {
                    None
                },
                num_extensions: rng.gen_range(0..=10),
            };

            let hash_result = compressed_mint.hash().unwrap();

            let hashed_spl_mint =
                hash_to_bn254_field_size_be(compressed_mint.spl_mint.to_bytes().as_slice());
            let mut supply_bytes = [0u8; 32];
            supply_bytes[24..].copy_from_slice(compressed_mint.supply.to_be_bytes().as_slice());

            let hashed_mint_authority;
            let hashed_mint_authority_option =
                if let Some(mint_authority) = compressed_mint.mint_authority {
                    hashed_mint_authority =
                        hash_to_bn254_field_size_be(mint_authority.to_bytes().as_slice());
                    Some(&hashed_mint_authority)
                } else {
                    None
                };

            let hashed_freeze_authority;
            let hashed_freeze_authority_option =
                if let Some(freeze_authority) = compressed_mint.freeze_authority {
                    hashed_freeze_authority =
                        hash_to_bn254_field_size_be(freeze_authority.to_bytes().as_slice());
                    Some(&hashed_freeze_authority)
                } else {
                    None
                };

            let hash_with_hashed_values = CompressedMint::hash_with_hashed_values(
                &hashed_spl_mint,
                &supply_bytes,
                compressed_mint.decimals,
                compressed_mint.is_decompressed,
                &hashed_mint_authority_option,
                &hashed_freeze_authority_option,
                compressed_mint.num_extensions,
            )
            .unwrap();

            assert_eq!(hash_result, hash_with_hashed_values);
        }
    }

    #[test]
    fn test_equivalency_random_iterations() {
        equivalency_of_hash_functions_rnd_iters::<1000>();
    }

    #[test]
    fn test_hash_collision_detection() {
        let mut vec_previous_hashes = Vec::new();

        // Base compressed mint
        let base_mint = CompressedMint {
            spl_mint: Pubkey::new_unique(),
            supply: 1000000,
            decimals: 6,
            is_decompressed: false,
            mint_authority: None,
            freeze_authority: None,
            num_extensions: 0,
        };

        let base_hash = base_mint.hash().unwrap();
        vec_previous_hashes.push(base_hash);

        // Different spl_mint
        let mut mint1 = base_mint.clone();
        mint1.spl_mint = Pubkey::new_unique();
        let hash1 = mint1.hash().unwrap();
        assert_to_previous_hashes(hash1, &mut vec_previous_hashes);

        // Different supply
        let mut mint2 = base_mint.clone();
        mint2.supply = 2000000;
        let hash2 = mint2.hash().unwrap();
        assert_to_previous_hashes(hash2, &mut vec_previous_hashes);

        // Different decimals
        let mut mint3 = base_mint.clone();
        mint3.decimals = 9;
        let hash3 = mint3.hash().unwrap();
        assert_to_previous_hashes(hash3, &mut vec_previous_hashes);

        // Different is_decompressed
        let mut mint4 = base_mint.clone();
        mint4.is_decompressed = true;
        let hash4 = mint4.hash().unwrap();
        assert_to_previous_hashes(hash4, &mut vec_previous_hashes);

        // Different mint_authority
        let mut mint5 = base_mint.clone();
        mint5.mint_authority = Some(Pubkey::new_unique());
        let hash5 = mint5.hash().unwrap();
        assert_to_previous_hashes(hash5, &mut vec_previous_hashes);

        // Different freeze_authority
        let mut mint6 = base_mint.clone();
        mint6.freeze_authority = Some(Pubkey::new_unique());
        let hash6 = mint6.hash().unwrap();
        assert_to_previous_hashes(hash6, &mut vec_previous_hashes);

        // Different num_extensions
        let mut mint7 = base_mint.clone();
        mint7.num_extensions = 5;
        let hash7 = mint7.hash().unwrap();
        assert_to_previous_hashes(hash7, &mut vec_previous_hashes);

        // Multiple fields different
        let mut mint8 = base_mint.clone();
        mint8.decimals = 18;
        mint8.is_decompressed = true;
        mint8.mint_authority = Some(Pubkey::new_unique());
        mint8.freeze_authority = Some(Pubkey::new_unique());
        mint8.num_extensions = 3;
        let hash8 = mint8.hash().unwrap();
        assert_to_previous_hashes(hash8, &mut vec_previous_hashes);
    }

    #[test]
    fn test_authority_hash_collision_prevention() {
        // This is a critical security test: ensuring that different authority combinations
        // with the same pubkey don't produce the same hash
        let same_pubkey = Pubkey::new_unique();

        let base_mint = CompressedMint {
            spl_mint: Pubkey::new_unique(),
            supply: 1000000,
            decimals: 6,
            is_decompressed: false,
            mint_authority: None,
            freeze_authority: None,
            num_extensions: 0,
        };

        // Case 1: None mint_authority, Some freeze_authority
        let mut mint1 = base_mint.clone();
        mint1.mint_authority = None;
        mint1.freeze_authority = Some(same_pubkey);
        let hash1 = mint1.hash().unwrap();

        // Case 2: Some mint_authority, None freeze_authority (using same pubkey)
        let mut mint2 = base_mint.clone();
        mint2.mint_authority = Some(same_pubkey);
        mint2.freeze_authority = None;
        let hash2 = mint2.hash().unwrap();

        // These must be different hashes to prevent authority confusion
        assert_ne!(
            hash1, hash2,
            "CRITICAL: Hash collision between different authority configurations!"
        );

        // Case 3: Both authorities present (should also be different)
        let mut mint3 = base_mint.clone();
        mint3.mint_authority = Some(same_pubkey);
        mint3.freeze_authority = Some(same_pubkey);
        let hash3 = mint3.hash().unwrap();

        assert_ne!(
            hash1, hash3,
            "Hash collision between freeze-only and both authorities!"
        );
        assert_ne!(
            hash2, hash3,
            "Hash collision between mint-only and both authorities!"
        );

        // Test with different pubkeys for good measure
        let different_pubkey = Pubkey::new_unique();
        let mut mint4 = base_mint.clone();
        mint4.mint_authority = Some(same_pubkey);
        mint4.freeze_authority = Some(different_pubkey);
        let hash4 = mint4.hash().unwrap();

        assert_ne!(
            hash1, hash4,
            "Hash collision with different freeze authority!"
        );
        assert_ne!(hash2, hash4, "Hash collision with different authorities!");
        assert_ne!(hash3, hash4, "Hash collision with mixed authorities!");
    }

    fn assert_to_previous_hashes(hash: [u8; 32], previous_hashes: &mut Vec<[u8; 32]>) {
        for previous_hash in previous_hashes.iter() {
            assert_ne!(hash, *previous_hash, "Hash collision detected!");
        }
        previous_hashes.push(hash);
    }
}
