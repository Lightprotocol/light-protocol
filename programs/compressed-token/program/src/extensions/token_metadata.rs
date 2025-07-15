use anchor_lang::prelude::ProgramError;
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_hasher::{
    hash_to_field_size::hashv_to_bn254_field_size_be_const_array, DataHasher, HasherError, Poseidon,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

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
            2 => Ok(Version::Keccak256),
            3 => Ok(Version::Sha256Flat),
            // TODO: use real error
            _ => Err(HasherError::InvalidInputLength(value as usize, 3)),
        }
    }
}

// TODO: impl string for zero copy
// TODO: test deserialization equivalence
/// Used for onchain serialization
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
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
            Version::Poseidon => <Self as DataHasher>::hash::<Poseidon>(self),
            _ => unimplemented!("TokenMetadata hash version not supported {}", self.version),
            // Version::Sha256 => <Self as DataHasher>::hash::<Sha256>(self),
            // Version::Keccak256 => <Self as DataHasher>::hash::<Keccak>(self),
            // Version::Sha256Flat => self.sha_flat(),
        }
    }
}

fn token_metadata_hash<H: light_hasher::Hasher>(
    update_authority: Option<&[u8]>,
    mint: &[u8],
    metadata_hash: &[u8],
    additional_metadata: &[(&[u8], &[u8])],
    version: u8,
) -> Result<[u8; 32], HasherError> {
    let mut vec = [[0u8; 32]; 5];
    let mut slice_vec: [&[u8]; 5] = [&[]; 5];

    if let Some(update_authority) = update_authority {
        vec[0].copy_from_slice(
            hashv_to_bn254_field_size_be_const_array::<2>(&[update_authority])?.as_slice(),
        );
    }

    vec[1] = hashv_to_bn254_field_size_be_const_array::<2>(&[mint])?;

    for (key, value) in additional_metadata {
        // TODO: add check is poseidon and throw meaningful error.
        vec[3] = H::hashv(&[vec[3].as_slice(), key, value])?;
    }
    vec[4][31] = version;

    slice_vec[0] = vec[0].as_slice();
    slice_vec[1] = vec[2].as_slice();
    slice_vec[2] = metadata_hash;
    slice_vec[3] = vec[3].as_slice();
    slice_vec[4] = vec[4].as_slice();

    if vec[4] != [0u8; 32] {
        H::hashv(&slice_vec[..4])
    } else {
        H::hashv(slice_vec.as_slice())
    }
}

fn token_metadata_hash_with_hashed_values<H: light_hasher::Hasher>(
    hashed_update_authority: Option<&[u8; 32]>,
    hashed_mint: &[u8; 32],
    metadata_hash: &[u8],
    additional_metadata: &[(&[u8], &[u8])],
    version: u8,
) -> Result<[u8; 32], HasherError> {
    let mut vec = [[0u8; 32]; 5];
    let mut slice_vec: [&[u8]; 5] = [&[]; 5];

    if let Some(hashed_update_authority) = hashed_update_authority {
        vec[0] = *hashed_update_authority;
    }

    vec[1] = *hashed_mint;

    for (key, value) in additional_metadata {
        // TODO: add check is poseidon and throw meaningful error.
        vec[3] = H::hashv(&[vec[3].as_slice(), key, value])?;
    }
    vec[4][31] = version;

    slice_vec[0] = vec[0].as_slice();
    slice_vec[1] = vec[2].as_slice();
    slice_vec[2] = metadata_hash;
    slice_vec[3] = vec[3].as_slice();
    slice_vec[4] = vec[4].as_slice();

    if vec[4] != [0u8; 32] {
        H::hashv(&slice_vec[..4])
    } else {
        H::hashv(slice_vec.as_slice())
    }
}

impl DataHasher for TokenMetadata {
    fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let metadata_hash = light_hasher::DataHasher::hash::<H>(&self.metadata)?;
        let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> = self
            .additional_metadata
            .iter()
            .map(|item| (item.key.as_slice(), item.value.as_slice()))
            .collect();

        token_metadata_hash::<H>(
            self.update_authority.as_ref().map(|auth| (*auth).as_ref()),
            self.mint.as_ref(),
            metadata_hash.as_slice(),
            &additional_metadata,
            self.version,
        )
    }
}

impl DataHasher for ZTokenMetadataMut<'_> {
    fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let metadata_hash = light_hasher::DataHasher::hash::<H>(&self.metadata)?;
        let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> = self
            .additional_metadata
            .iter()
            .map(|item| (&*item.key, &*item.value))
            .collect();

        token_metadata_hash::<H>(
            self.update_authority.as_ref().map(|auth| (*auth).as_ref()),
            self.mint.as_ref(),
            metadata_hash.as_slice(),
            &additional_metadata,
            *self.version,
        )
    }
}

impl DataHasher for ZTokenMetadata<'_> {
    fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let metadata_hash = light_hasher::DataHasher::hash::<H>(&self.metadata)?;
        let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> = self
            .additional_metadata
            .iter()
            .map(|item| (item.key, item.value))
            .collect();

        token_metadata_hash::<H>(
            self.update_authority.as_ref().map(|auth| (*auth).as_ref()),
            self.mint.as_ref(),
            metadata_hash.as_slice(),
            &additional_metadata,
            self.version,
        )
    }
}

// TODO: if version 0 we check all string len for less than 31 bytes
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct Metadata {
    /// The longer name of the token
    pub name: Vec<u8>,
    /// The shortened symbol for the token
    pub symbol: Vec<u8>,
    /// The URI pointing to richer metadata
    pub uri: Vec<u8>,
}

// Manual LightHasher implementation for Metadata struct
impl light_hasher::to_byte_array::ToByteArray for Metadata {
    const NUM_FIELDS: usize = 3;

    fn to_byte_array(&self) -> Result<[u8; 32], light_hasher::HasherError> {
        light_hasher::DataHasher::hash::<light_hasher::Poseidon>(self)
    }
}

impl light_hasher::DataHasher for Metadata {
    fn hash<H>(&self) -> Result<[u8; 32], light_hasher::HasherError>
    where
        H: light_hasher::Hasher,
    {
        use light_hasher::hash_to_field_size::hash_to_bn254_field_size_be;

        // Hash each Vec<u8> field using as_slice() and hash_to_bn254_field_size_be for consistency
        let name_hash = hash_to_bn254_field_size_be(self.name.as_slice());
        let symbol_hash = hash_to_bn254_field_size_be(self.symbol.as_slice());
        let uri_hash = hash_to_bn254_field_size_be(self.uri.as_slice());

        H::hashv(&[
            name_hash.as_slice(),
            symbol_hash.as_slice(),
            uri_hash.as_slice(),
        ])
    }
}

// Manual LightHasher implementation for ZMetadata ZStruct
impl light_hasher::to_byte_array::ToByteArray for ZMetadata<'_> {
    const NUM_FIELDS: usize = 3;

    fn to_byte_array(&self) -> Result<[u8; 32], light_hasher::HasherError> {
        light_hasher::DataHasher::hash::<light_hasher::Poseidon>(self)
    }
}

impl light_hasher::DataHasher for ZMetadata<'_> {
    fn hash<H>(&self) -> Result<[u8; 32], light_hasher::HasherError>
    where
        H: light_hasher::Hasher,
    {
        use light_hasher::hash_to_field_size::hash_to_bn254_field_size_be;

        // Hash each &[u8] slice field using hash_to_bn254_field_size_be for consistency
        let name_hash = hash_to_bn254_field_size_be(self.name);
        let symbol_hash = hash_to_bn254_field_size_be(self.symbol);
        let uri_hash = hash_to_bn254_field_size_be(self.uri);

        H::hashv(&[
            name_hash.as_slice(),
            symbol_hash.as_slice(),
            uri_hash.as_slice(),
        ])
    }
}

impl light_hasher::to_byte_array::ToByteArray for ZMetadataMut<'_> {
    const NUM_FIELDS: usize = 3;

    fn to_byte_array(&self) -> Result<[u8; 32], light_hasher::HasherError> {
        light_hasher::DataHasher::hash::<light_hasher::Poseidon>(self)
    }
}

impl light_hasher::DataHasher for ZMetadataMut<'_> {
    fn hash<H>(&self) -> Result<[u8; 32], light_hasher::HasherError>
    where
        H: light_hasher::Hasher,
    {
        use light_hasher::hash_to_field_size::hash_to_bn254_field_size_be;

        // Hash each &[u8] slice field using hash_to_bn254_field_size_be for consistency
        let name_hash = hash_to_bn254_field_size_be(self.name);
        let symbol_hash = hash_to_bn254_field_size_be(self.symbol);
        let uri_hash = hash_to_bn254_field_size_be(self.uri);

        H::hashv(&[
            name_hash.as_slice(),
            symbol_hash.as_slice(),
            uri_hash.as_slice(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct AdditionalMetadata {
    /// The key of the metadata
    pub key: Vec<u8>,
    /// The value of the metadata
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct TokenMetadataInstructionData {
    pub update_authority: Option<Pubkey>,
    pub metadata: Metadata,
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
    pub version: u8,
}

impl TokenMetadataInstructionData {
    pub fn hash_token_metadata<H: light_hasher::Hasher>(
        &self,
        mint: light_compressed_account::Pubkey,
        context: &mut TokenContext,
    ) -> Result<[u8; 32], anchor_lang::solana_program::program_error::ProgramError> {
        let metadata_hash = light_hasher::DataHasher::hash::<H>(&self.metadata).map_err(|_| {
            anchor_lang::solana_program::program_error::ProgramError::InvalidAccountData
        })?;

        let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> =
            if let Some(ref additional_metadata) = self.additional_metadata {
                additional_metadata
                    .iter()
                    .map(|item| (item.key.as_slice(), item.value.as_slice()))
                    .collect()
            } else {
                arrayvec::ArrayVec::new()
            };

        let hashed_update_authority = self
            .update_authority
            .map(|update_authority| context.get_or_hash_pubkey(&update_authority.into()));

        let hashed_mint = context.get_or_hash_mint(&mint.into())?;

        token_metadata_hash::<H>(
            hashed_update_authority
                .as_ref()
                .map(|h: &[u8; 32]| h.as_slice()),
            hashed_mint.as_slice(),
            metadata_hash.as_slice(),
            &additional_metadata,
            self.version,
        )
        .map_err(|_| anchor_lang::solana_program::program_error::ProgramError::InvalidAccountData)
    }
}

impl ZTokenMetadataInstructionData<'_> {
    pub fn hash_token_metadata<H: light_hasher::Hasher>(
        &self,
        hashed_mint: &[u8; 32],
        context: &mut TokenContext,
    ) -> Result<[u8; 32], anchor_lang::solana_program::program_error::ProgramError> {
        let metadata_hash = light_hasher::DataHasher::hash::<H>(&self.metadata).map_err(|_| {
            anchor_lang::solana_program::program_error::ProgramError::InvalidAccountData
        })?;

        let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> =
            if let Some(ref additional_metadata) = self.additional_metadata {
                additional_metadata
                    .iter()
                    .map(|item| (item.key, item.value))
                    .collect()
            } else {
                arrayvec::ArrayVec::new()
            };

        let hashed_update_authority = self
            .update_authority
            .map(|update_authority| context.get_or_hash_pubkey(&(*update_authority).into()));

        token_metadata_hash_with_hashed_values::<H>(
            hashed_update_authority.as_ref(),
            hashed_mint,
            metadata_hash.as_slice(),
            &additional_metadata,
            self.version,
        )
        .map_err(|_| anchor_lang::solana_program::program_error::ProgramError::InvalidAccountData)
    }
}

use crate::shared::context::TokenContext;

pub fn create_output_token_metadata(
    token_metadata_data: &ZTokenMetadataInstructionData<'_>,
    token_metadata: &mut ZTokenMetadataMut<'_>,
    mint: Pubkey,
) -> Result<[u8; 32], ProgramError> {
    if let Some(ref mut authority) = token_metadata.update_authority {
        **authority = *token_metadata_data
            .update_authority
            .ok_or(ProgramError::InvalidInstructionData)?;
    }
    token_metadata
        .metadata
        .name
        .copy_from_slice(token_metadata_data.metadata.name);
    token_metadata
        .metadata
        .symbol
        .copy_from_slice(token_metadata_data.metadata.symbol);
    token_metadata
        .metadata
        .uri
        .copy_from_slice(token_metadata_data.metadata.uri);

    // Set mint
    *token_metadata.mint = mint;

    // Set version
    *token_metadata.version = token_metadata_data.version;

    // Set additional metadata if provided
    if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
        for (i, item) in additional_metadata.iter().enumerate() {
            token_metadata.additional_metadata[i]
                .key
                .copy_from_slice(item.key);
            token_metadata.additional_metadata[i]
                .value
                .copy_from_slice(item.value);
        }
    }

    // Use the zero-copy mut struct for hashing
    let hash = token_metadata
        .hash::<light_hasher::Poseidon>()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(hash)
}
