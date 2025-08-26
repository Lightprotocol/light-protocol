use light_compressed_account::Pubkey;
use light_hasher::{errors::HasherError, sha256::Sha256BE, Hasher, Poseidon};
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopy, ZeroCopyMut};
use zerocopy::IntoBytes;

use crate::{
    hash_cache::HashCache, instructions::mint_action::CompressedMintInstructionData,
    state::ExtensionStruct, AnchorDeserialize, AnchorSerialize, CTokenError,
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

macro_rules! impl_compressed_mint_hash {
    (
        $self:ident,
        $hash_cache:ident,
        $is_decompressed:expr
        $(,$deref_op:tt)?
    ) => {{
        let hashed_spl_mint = $hash_cache.get_or_hash_mint(&$self.spl_mint.into())?;
        let mut supply_bytes = [0u8; 32];
        $self.supply
            .as_bytes()
            .iter()
            .rev()
            .zip(supply_bytes[24..].iter_mut())
            .for_each(|(x, y)| *y = *x);

        let hashed_mint_authority;
        let hashed_mint_authority_option =
            if let Some(mint_authority) = $self.mint_authority.as_ref() {
                hashed_mint_authority = $hash_cache.get_or_hash_pubkey(&($($deref_op)?mint_authority).to_bytes());
                Some(&hashed_mint_authority)
            } else {
                None
            };

        let hashed_freeze_authority;
        let hashed_freeze_authority_option = if let Some(freeze_authority) =
            $self.freeze_authority.as_ref()
        {
            hashed_freeze_authority = $hash_cache.get_or_hash_pubkey(&($($deref_op)?freeze_authority).to_bytes());
            Some(&hashed_freeze_authority)
        } else {
            None
        };

        let mint_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            $self.decimals,
            $is_decompressed,
            &hashed_mint_authority_option,
            &hashed_freeze_authority_option,
            $self.version,
        )?;

        if let Some(extensions) = $self.extensions.as_ref() {
            let mut extension_hashchain = [0u8; 32];
            for extension in extensions.as_slice() {
                let extension_hash = if $self.version == 0 {
                    extension.hash::<Poseidon>()?
                } else if $self.version == 1 {
                    extension.hash::<Sha256BE>()?
                } else {
                    return Err(CTokenError::InvalidTokenDataVersion);
                };

                if $self.version == 0 {
                    extension_hashchain = Poseidon::hashv(&[
                        extension_hashchain.as_slice(),
                        extension_hash.as_slice(),
                    ])?;
                } else if $self.version == 1 {
                    extension_hashchain = Sha256BE::hashv(&[
                        extension_hashchain.as_slice(),
                        extension_hash.as_slice(),
                    ])?;
                } else {
                    return Err(CTokenError::InvalidTokenDataVersion);
                }
            }
            if $self.version == 0 {
                Ok(Poseidon::hashv(&[
                    mint_hash.as_slice(),
                    extension_hashchain.as_slice(),
                ])?)
            } else if $self.version == 1 {
                Ok(Sha256BE::hashv(&[
                    mint_hash.as_slice(),
                    extension_hashchain.as_slice(),
                ])?)
            } else {
                return Err(CTokenError::InvalidTokenDataVersion);
            }
        } else {
            Ok(mint_hash)
        }
    }};
}

// TODO: unify code if possible
// use nested token metadata layout for data extension
impl CompressedMint {
    #[allow(dead_code)]
    pub fn hash(&self) -> Result<[u8; 32], CTokenError> {
        let mut hash_cache = HashCache::new();
        self.hash_with_cache(&mut hash_cache)
    }

    pub fn hash_with_cache(&self, hash_cache: &mut HashCache) -> Result<[u8; 32], CTokenError> {
        impl_compressed_mint_hash!(self, hash_cache, self.is_decompressed)
    }

    pub fn hash_with_hashed_values(
        hashed_spl_mint: &[u8; 32],
        supply_bytes: &[u8; 32],
        decimals: u8,
        is_decompressed: bool,
        hashed_mint_authority: &Option<&[u8; 32]>,
        hashed_freeze_authority: &Option<&[u8; 32]>,
        version: u8,
    ) -> Result<[u8; 32], CTokenError> {
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
    ) -> Result<[u8; 32], HasherError> {
        // Note: ArrayVec causes lifetime issues.
        let mut hash_inputs: [&[u8]; 8] = [&[]; 8];

        hash_inputs[0] = hashed_spl_mint.as_slice();
        hash_inputs[1] = supply_bytes.as_slice();
        let mut input_count = 2;

        // Add decimals with prefix if not 0
        let mut decimals_bytes = [0u8; 32];
        if decimals != 0 {
            decimals_bytes[30] = 1; // decimals prefix
            decimals_bytes[31] = decimals;
            hash_inputs[input_count] = &decimals_bytes[..];
            input_count += 1;
        }

        // Add is_decompressed with prefix if true
        let mut is_decompressed_bytes = [0u8; 32];
        if is_decompressed {
            is_decompressed_bytes[30] = 2; // is_decompressed prefix
            is_decompressed_bytes[31] = 1; // true as 1
            hash_inputs[input_count] = &is_decompressed_bytes[..];
            input_count += 1;
        }

        // Add mint authority if present
        if let Some(hashed_mint_authority) = hashed_mint_authority {
            hash_inputs[input_count] = hashed_mint_authority.as_slice();
            input_count += 1;
        }

        // Add freeze authority if present
        let empty_authority = [0u8; 32];
        if let Some(hashed_freeze_authority) = hashed_freeze_authority {
            // If there is freeze authority but no mint authority, add empty mint authority
            if hashed_mint_authority.is_none() {
                hash_inputs[input_count] = &empty_authority[..];
                input_count += 1;
            }
            hash_inputs[input_count] = hashed_freeze_authority.as_slice();
            input_count += 1;
        }

        // Add version with prefix if not 0
        let mut num_extensions_bytes = [0u8; 32];
        if version != 0 {
            num_extensions_bytes[30] = 3; // version prefix
            num_extensions_bytes[31] = version;
            hash_inputs[input_count] = &num_extensions_bytes[..];
            input_count += 1;
        }

        let hash = H::hashv(&hash_inputs[..input_count])?;

        Ok(hash)
    }
}

impl ZCompressedMintMut<'_> {
    pub fn hash(&self, hash_cache: &mut HashCache) -> Result<[u8; 32], CTokenError> {
        impl_compressed_mint_hash!(
            self,
            hash_cache,
            self.is_decompressed(),
            *
        )
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

        if let Some(self_mint_authority) = self.mint_authority.as_deref_mut() {
            *self_mint_authority = *ix_data
                .mint_authority
                .ok_or(CTokenError::InstructionDataExpectedMintAuthority)?;
        }

        if self.mint_authority.is_some() && ix_data.mint_authority.is_none() {
            return Err(CTokenError::ZeroCopyExpectedMintAuthority);
        }

        if let Some(self_freeze_authority) = self.freeze_authority.as_deref_mut() {
            *self_freeze_authority = *ix_data
                .freeze_authority
                .ok_or(CTokenError::InstructionDataExpectedFreezeAuthority)?;
        }

        if self.freeze_authority.is_some() && ix_data.freeze_authority.is_none() {
            return Err(CTokenError::ZeroCopyExpectedFreezeAuthority);
        }
        // extensions are handled separately
        Ok(())
    }
}
