use light_compressed_account::Pubkey;
use light_hasher::{
    hash_to_field_size::hashv_to_bn254_field_size_be_const_array, sha256::Sha256BE,
    to_byte_array::ToByteArray, DataHasher, HasherError, Poseidon,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use solana_msg::msg;

use crate::{AnchorDeserialize, AnchorSerialize};

// TODO: decide whether to keep Shaflat
pub enum Version {
    Poseidon,
    Sha256,
    Keccak256,
    Sha256Flat,
}

impl TryFrom<u8> for Version {
    type Error = HasherError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Version::Poseidon),
            1 => Ok(Version::Sha256),
            // 2 => Ok(Version::Keccak256),
            // 3 => Ok(Version::Sha256Flat),
            // TODO: use real error
            _ => Err(HasherError::InvalidInputLength(value as usize, 3)),
        }
    }
}

// TODO: test deserialization equivalence
/// Used for onchain serialization
#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct TokenMetadata {
    // TODO: decide whether to move down for more efficient zero copy. Or impl manual zero copy.
    /// The authority that can sign to update the metadata
    pub update_authority: Option<Pubkey>,
    // TODO: decide whether to keep this.
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    pub metadata: Metadata,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<AdditionalMetadata>,
    // TODO: decide whether to do this on this or MintAccount level
    /// 0: Poseidon, 1: Sha256, 2: Keccak256, 3: Sha256Flat
    pub version: u8,
}

impl TokenMetadata {
    pub fn hash(&self) -> Result<[u8; 32], HasherError> {
        match Version::try_from(self.version)? {
            Version::Poseidon => {
                msg!("poseidon");
                <Self as DataHasher>::hash::<Poseidon>(self)
            }
            Version::Sha256 => {
                msg!("sha256");
                <Self as DataHasher>::hash::<Sha256BE>(self)
            }
            _ => {
                msg!(
                                        "TokenMetadata hash version not supported {} (0 Poseidon, 1 Sha256 are supported).",
                                        self.version
                                   );
                Err(HasherError::InvalidNumFields)
            } // Version::Keccak256 => <Self as DataHasher>::hash::<Keccak>(self),
              // Version::Keccak256 => <Self as DataHasher>::hash::<Keccak>(self),
              // Version::Sha256Flat => self.sha_flat(),
        }
    }
}

fn token_metadata_hash_inner<H: light_hasher::Hasher, const HASHED: bool>(
    update_authority: Option<&[u8]>,
    mint: &[u8],
    metadata_hash: &[u8],
    additional_metadata: &[(&[u8], &[u8])],
    version: u8,
) -> Result<[u8; 32], HasherError> {
    let mut vec = [[0u8; 32]; 5];
    let mut slice_vec: [&[u8]; 5] = [&[]; 5];

    if let Some(update_authority) = update_authority {
        if HASHED {
            vec[0].copy_from_slice(update_authority);
        } else {
            vec[0].copy_from_slice(
                hashv_to_bn254_field_size_be_const_array::<2>(&[update_authority])?.as_slice(),
            );
        }
    }

    if HASHED {
        vec[1].copy_from_slice(mint);
    } else {
        vec[1] = hashv_to_bn254_field_size_be_const_array::<2>(&[mint])?;
    }

    for (key, value) in additional_metadata {
        vec[3] = H::hashv(&[vec[3].as_slice(), key, value])?;
    }
    vec[4][31] = version;

    slice_vec[0] = vec[0].as_slice();
    slice_vec[1] = vec[1].as_slice();
    slice_vec[2] = metadata_hash;
    slice_vec[3] = vec[3].as_slice();
    slice_vec[4] = vec[4].as_slice();

    // Omit empty slice
    if vec[4] == [0u8; 32] {
        H::hashv(&slice_vec[..4])
    } else {
        H::hashv(slice_vec.as_slice())
    }
}

pub fn token_metadata_hash<H: light_hasher::Hasher>(
    update_authority: Option<&[u8]>,
    mint: &[u8],
    metadata_hash: &[u8],
    additional_metadata: &[(&[u8], &[u8])],
    version: u8,
) -> Result<[u8; 32], HasherError> {
    token_metadata_hash_inner::<H, false>(
        update_authority,
        mint,
        metadata_hash,
        additional_metadata,
        version,
    )
}

pub fn token_metadata_hash_with_hashed_values<H: light_hasher::Hasher>(
    hashed_update_authority: Option<&[u8]>,
    hashed_mint: &[u8],
    metadata_hash: &[u8],
    additional_metadata: &[(&[u8], &[u8])],
    version: u8,
) -> Result<[u8; 32], HasherError> {
    token_metadata_hash_inner::<H, true>(
        hashed_update_authority,
        hashed_mint,
        metadata_hash,
        additional_metadata,
        version,
    )
}

macro_rules! impl_token_metadata_hasher {
    ($type_name:ty $(,$op:tt)?) => {
        impl DataHasher for $type_name {
            fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], HasherError> {
                let metadata_hash = DataHasher::hash::<H>(&self.metadata)?;
                let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> = self
                    .additional_metadata
                    .iter()
                    .map(|item| (item.key.as_ref(), item.value.as_ref()))
                    .collect();

                token_metadata_hash::<H>(
                    self.update_authority.as_ref().map(|auth| auth.as_ref()),
                    self.mint.as_ref(),
                    metadata_hash.as_slice(),
                    &additional_metadata,
                    $($op)?self.version,
                )
            }
        }
    };
}

impl_token_metadata_hasher!(TokenMetadata);
impl_token_metadata_hasher!(ZTokenMetadataMut<'_>, *);
impl_token_metadata_hasher!(ZTokenMetadata<'_>);

// TODO: if version 0 we check all string len for less than 31 bytes
#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct Metadata {
    /// The longer name of the token
    pub name: Vec<u8>,
    /// The shortened symbol for the token
    pub symbol: Vec<u8>,
    /// The URI pointing to richer metadata
    pub uri: Vec<u8>,
}

macro_rules! impl_metadata_hasher {
    ($type_name:ty) => {
        impl ToByteArray for $type_name {
            const NUM_FIELDS: usize = 3;

            fn to_byte_array(&self) -> Result<[u8; 32], light_hasher::HasherError> {
                DataHasher::hash::<light_hasher::Poseidon>(self)
            }
        }

        impl DataHasher for $type_name {
            fn hash<H>(&self) -> Result<[u8; 32], light_hasher::HasherError>
            where
                H: light_hasher::Hasher,
            {
                use light_hasher::hash_to_field_size::hash_to_bn254_field_size_be;

                let name_hash = hash_to_bn254_field_size_be(self.name.as_ref());
                let symbol_hash = hash_to_bn254_field_size_be(self.symbol.as_ref());
                let uri_hash = hash_to_bn254_field_size_be(self.uri.as_ref());

                H::hashv(&[
                    name_hash.as_slice(),
                    symbol_hash.as_slice(),
                    uri_hash.as_slice(),
                ])
            }
        }
    };
}

impl_metadata_hasher!(Metadata);
impl_metadata_hasher!(ZMetadata<'_>);
impl_metadata_hasher!(ZMetadataMut<'_>);

#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct AdditionalMetadata {
    /// The key of the metadata
    pub key: Vec<u8>,
    /// The value of the metadata
    pub value: Vec<u8>,
}
