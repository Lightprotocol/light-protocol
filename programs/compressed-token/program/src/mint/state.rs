use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{hash_to_bn254_field_size_be, Pubkey};
use light_hasher::{errors::HasherError, Hasher, Poseidon};
use light_zero_copy::ZeroCopyMut;
use zerocopy::IntoBytes;

// Order is optimized for hashing.
// freeze_authority option is skipped if None.
#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize, ZeroCopyMut)]
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
    pub num_extensions: u8,
}

impl CompressedMint {
    #[allow(dead_code)]
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

impl ZCompressedMintMut<'_> {
    pub fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        let hashed_spl_mint = hash_to_bn254_field_size_be(self.spl_mint.to_bytes().as_slice());
        let mut supply_bytes = [0u8; 32];
        // TODO: copy from slice
        self.supply
            .as_bytes()
            .iter()
            .rev()
            .zip(supply_bytes[24..].iter_mut())
            .for_each(|(x, y)| *y = *x);

        let hashed_mint_authority;
        let hashed_mint_authority_option =
            if let Some(mint_authority) = self.mint_authority.as_ref() {
                hashed_mint_authority =
                    hash_to_bn254_field_size_be(mint_authority.to_bytes().as_slice());
                Some(&hashed_mint_authority)
            } else {
                None
            };

        let hashed_freeze_authority;
        let hashed_freeze_authority_option =
            if let Some(freeze_authority) = self.freeze_authority.as_ref() {
                hashed_freeze_authority =
                    hash_to_bn254_field_size_be(freeze_authority.to_bytes().as_slice());
                Some(&hashed_freeze_authority)
            } else {
                None
            };

        CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            self.decimals,
            self.is_decompressed(),
            &hashed_mint_authority_option,
            &hashed_freeze_authority_option,
            *self.num_extensions,
        )
    }
}
