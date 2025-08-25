use light_compressed_account::{hash_to_bn254_field_size_be, Pubkey};
use light_hasher::{errors::HasherError, sha256::Sha256BE, Hasher, Poseidon};
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopy, ZeroCopyMut};
use solana_msg::msg;
use zerocopy::IntoBytes;

use crate::{
    hash_cache::HashCache,
    instructions::mint_action::CompressedMintInstructionData,
    state::{ExtensionStruct, ZExtensionStructMut},
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

// Order is optimized for hashing.
// freeze_authority option is skipped if None.
#[repr(C)]
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopyMut, ZeroCopy,
)]
pub struct CompressedMint {
    /// Version for upgradability
    pub version: u8,
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
    pub extensions: Option<Vec<ExtensionStruct>>,
}

// TODO: unify code if possible
// use nested token metadata layout for data extension
impl CompressedMint {
    #[allow(dead_code)]
    pub fn hash(&self) -> std::result::Result<[u8; 32], CTokenError> {
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

        let mint_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            self.decimals,
            self.is_decompressed,
            &hashed_mint_authority_option,
            &hashed_freeze_authority_option,
            self.version,
        )?;
        // TODO: consider to make hasher generic. could use version for that.
        if let Some(extensions) = self.extensions.as_ref() {
            let mut extension_hashchain = [0u8; 32];
            for extension in extensions {
                if self.version == 0 {
                    extension_hashchain = Poseidon::hashv(&[
                        extension_hashchain.as_slice(),
                        extension.hash::<Poseidon>()?.as_slice(),
                    ])?;
                } else if self.version == 1 {
                    extension_hashchain = Sha256BE::hashv(&[
                        extension_hashchain.as_slice(),
                        extension.hash::<Sha256BE>()?.as_slice(),
                    ])?;
                } else {
                    return Err(CTokenError::InvalidTokenDataVersion);
                }
            }
            if self.version == 0 {
                Ok(Poseidon::hashv(&[
                    mint_hash.as_slice(),
                    extension_hashchain.as_slice(),
                ])?)
            } else if self.version == 1 {
                Ok(Sha256BE::hashv(&[
                    mint_hash.as_slice(),
                    extension_hashchain.as_slice(),
                ])?)
            } else {
                return Err(CTokenError::InvalidTokenDataVersion);
            }
        } else if self.version == 0 {
            Ok(mint_hash)
        } else if self.version == 1 {
            // Truncate hash to 248 bits
            let mut hash = mint_hash;
            hash[0] = 0;
            Ok(hash)
        } else {
            return Err(CTokenError::InvalidTokenDataVersion);
        }
    }
    pub fn hash_with_hashed_values(
        hashed_spl_mint: &[u8; 32],
        supply_bytes: &[u8; 32],
        decimals: u8,
        is_decompressed: bool,
        hashed_mint_authority: &Option<&[u8; 32]>,
        hashed_freeze_authority: &Option<&[u8; 32]>,
        version: u8,
    ) -> std::result::Result<[u8; 32], CTokenError> {
        if version == 0 {
            Ok(CompressedMint::hash_with_hashed_values_inner::<Poseidon>(
                hashed_spl_mint,
                supply_bytes,
                decimals,
                is_decompressed,
                hashed_mint_authority,
                hashed_freeze_authority,
                version,
            )?)
        } else if version == 1 {
            Ok(CompressedMint::hash_with_hashed_values_inner::<Sha256BE>(
                hashed_spl_mint,
                supply_bytes,
                decimals,
                is_decompressed,
                hashed_mint_authority,
                hashed_freeze_authority,
                version,
            )?)
        } else {
            Err(CTokenError::InvalidTokenDataVersion)
        }
    }
    fn hash_with_hashed_values_inner<H: Hasher>(
        hashed_spl_mint: &[u8; 32],
        supply_bytes: &[u8; 32],
        decimals: u8,
        is_decompressed: bool,
        hashed_mint_authority: &Option<&[u8; 32]>,
        hashed_freeze_authority: &Option<&[u8; 32]>,
        version: u8,
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

        // Add version with prefix if not 0
        let mut num_extensions_bytes = [0u8; 32];
        if version != 0 {
            num_extensions_bytes[30] = 3; // version prefix
            num_extensions_bytes[31] = version;
            hash_inputs.push(&num_extensions_bytes[..]);
        }

        let hash = H::hashv(hash_inputs.as_slice())?;

        Ok(hash)
    }
}

impl ZCompressedMintMut<'_> {
    pub fn hash(&self, hash_cache: &mut HashCache) -> std::result::Result<[u8; 32], CTokenError> {
        // let hashed_spl_mint = hash_to_bn254_field_size_be(self.spl_mint.to_bytes().as_slice());
        let hashed_spl_mint = hash_cache.get_or_hash_mint(&self.spl_mint.into())?;
        let mut supply_bytes = [0u8; 32];
        // TODO: copy from slice
        self.supply
            .as_bytes()
            .iter()
            .rev()
            .zip(supply_bytes[24..].iter_mut())
            .for_each(|(x, y)| *y = *x);

        let hashed_mint_authority;
        let hashed_mint_authority_option = if let Some(mint_authority) =
            self.mint_authority.as_ref()
        {
            // TODO: skip if sha is selected
            hashed_mint_authority = hash_cache.get_or_hash_pubkey(&(*mint_authority).to_bytes());
            Some(&hashed_mint_authority)
        } else {
            None
        };

        let hashed_freeze_authority;
        let hashed_freeze_authority_option =
            if let Some(freeze_authority) = self.freeze_authority.as_ref() {
                // TODO: skip if sha is selected
                hashed_freeze_authority =
                    hash_cache.get_or_hash_pubkey(&(*freeze_authority).to_bytes());

                Some(&hashed_freeze_authority)
            } else {
                None
            };

        let mut mint_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            self.decimals,
            self.is_decompressed(),
            &hashed_mint_authority_option,
            &hashed_freeze_authority_option,
            self.version,
        )?;
        msg!("mint_hash {:?}", mint_hash);

        // Compute extension hash chain if extensions exist
        if let Some(extensions) = self.extensions.as_ref() {
            let mut extension_hashchain = [0u8; 32];
            for extension in extensions.as_slice() {
                // Let each extension determine its own hash method based on its version
                let extension_hash = match extension {
                    ZExtensionStructMut::TokenMetadata(token_metadata) => {
                        if *token_metadata.version == 0 {
                            extension.hash::<Poseidon>()?
                        } else if *token_metadata.version == 1 {
                            extension.hash::<Sha256BE>()?
                        } else {
                            return Err(CTokenError::InvalidTokenDataVersion);
                        }
                    }
                    _ => return Err(CTokenError::UnsupportedExtension),
                };
                msg!("ZCompressedMintMut extension hash: {:?} ", extension_hash);

                if self.version == 0 {
                    extension_hashchain = Poseidon::hashv(&[
                        extension_hashchain.as_slice(),
                        extension_hash.as_slice(),
                    ])?;
                } else if self.version == 1 {
                    extension_hashchain = Sha256BE::hashv(&[
                        extension_hashchain.as_slice(),
                        extension_hash.as_slice(),
                    ])?;
                } else {
                    msg!("invalid version ");
                    return Err(CTokenError::InvalidTokenDataVersion);
                }
            }
            msg!(
                "ZCompressedMintMut extension_hashchain: {:?} ",
                extension_hashchain
            );

            if self.version == 0 {
                Ok(Poseidon::hashv(&[
                    mint_hash.as_slice(),
                    extension_hashchain.as_slice(),
                ])?)
            } else if self.version == 1 {
                Ok(Sha256BE::hashv(&[
                    mint_hash.as_slice(),
                    extension_hashchain.as_slice(),
                ])?)
            } else {
                Err(CTokenError::InvalidTokenDataVersion)
            }
        } else if self.version == 0 {
            Ok(mint_hash)
        } else if self.version == 1 {
            // Truncate hash to 248 bits
            mint_hash[0] = 0;
            Ok(mint_hash)
        } else {
            Err(CTokenError::InvalidTokenDataVersion)
        }
    }
}
// Implementation for zero-copy mutable CompressedMint
impl ZCompressedMintMut<'_> {
    /// Set all fields of the CompressedMint struct at once
    #[inline]
    pub fn set(
        &mut self,
        ix_data: &<CompressedMintInstructionData as ZeroCopyAt<'_>>::ZeroCopyAt,
        is_decompressed: bool,
    ) -> Result<(), CTokenError> {
        self.version = ix_data.version;
        self.spl_mint = ix_data.spl_mint;
        self.supply = ix_data.supply;
        self.decimals = ix_data.decimals;
        self.is_decompressed = if is_decompressed { 1 } else { 0 };
        msg!("set1");
        if let Some(self_mint_authority) = self.mint_authority.as_deref_mut() {
            *self_mint_authority = *ix_data
                .mint_authority
                .ok_or(CTokenError::InstructionDataExpectedMintAuthority)?;
        }
        msg!("set2");
        if self.mint_authority.is_some() && ix_data.mint_authority.is_none() {
            return Err(CTokenError::ZeroCopyExpectedMintAuthority);
        }
        msg!("set3");

        if let Some(self_freeze_authority) = self.freeze_authority.as_deref_mut() {
            *self_freeze_authority = *ix_data
                .freeze_authority
                .ok_or(CTokenError::InstructionDataExpectedFreezeAuthority)?;
        }
        msg!("set4");
        if self.freeze_authority.is_some() && ix_data.freeze_authority.is_none() {
            return Err(CTokenError::ZeroCopyExpectedFreezeAuthority);
        }
        // extensions are handled separately
        Ok(())
    }
}
