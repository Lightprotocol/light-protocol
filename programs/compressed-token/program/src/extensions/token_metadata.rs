use anchor_lang::prelude::ProgramError;
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_hasher::{
    hash_to_field_size::hashv_to_bn254_field_size_be_const_array, DataHasher, Hasher, HasherError,
    Keccak, Poseidon, Sha256,
};
use light_sdk::LightHasher;
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
            Version::Sha256 => <Self as DataHasher>::hash::<Sha256>(self),
            Version::Keccak256 => <Self as DataHasher>::hash::<Keccak>(self),
            Version::Sha256Flat => self.sha_flat(),
        }
    }
    fn sha_flat(&self) -> Result<[u8; 32], HasherError> {
        use borsh::BorshSerialize;
        let vec = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        Sha256::hash(vec.as_slice())
    }
}

impl DataHasher for TokenMetadata {
    fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let mut vec = [[0u8; 32]; 5];
        let mut slice_vec: [&[u8]; 5] = [&[]; 5];
        if let Some(update_authority) = self.update_authority {
            vec[0].copy_from_slice(
                hashv_to_bn254_field_size_be_const_array::<2>(&[&update_authority.to_bytes()])?
                    .as_slice(),
            );
        }

        vec[1] = hashv_to_bn254_field_size_be_const_array::<2>(&[&self.mint.to_bytes()])?;
        vec[2] = self.metadata.hash::<H>()?;

        for additional_metadata in &self.additional_metadata {
            // TODO: add check is poseidon and throw meaningful error.
            vec[3] = H::hashv(&[
                vec[3].as_slice(),
                additional_metadata.key.as_slice(),
                additional_metadata.value.as_slice(),
            ])?;
        }
        vec[4][31] = self.version;

        slice_vec[0] = vec[0].as_slice();
        slice_vec[1] = vec[1].as_slice();
        slice_vec[2] = vec[2].as_slice();
        slice_vec[3] = vec[3].as_slice();

        slice_vec[4] = vec[4].as_slice();
        if vec[4] != [0u8; 32] {
            H::hashv(&slice_vec[..4])
        } else {
            H::hashv(slice_vec.as_slice())
        }
    }
}

impl DataHasher for ZTokenMetadata<'_> {
    fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let mut vec = [[0u8; 32]; 5];
        let mut slice_vec: [&[u8]; 5] = [&[]; 5];
        if let Some(update_authority) = self.update_authority {
            vec[0].copy_from_slice(
                hashv_to_bn254_field_size_be_const_array::<2>(&[&update_authority.to_bytes()])?
                    .as_slice(),
            );
        }

        vec[1] = hashv_to_bn254_field_size_be_const_array::<2>(&[&self.mint.to_bytes()])?;
        vec[2] = self.metadata.hash::<H>()?;

        for additional_metadata in &self.additional_metadata {
            // TODO: add check is poseidon and throw meaningful error.
            vec[3] = H::hashv(&[
                vec[3].as_slice(),
                additional_metadata.key,
                additional_metadata.value,
            ])?;
        }
        vec[4][31] = self.version;

        slice_vec[0] = vec[0].as_slice();
        slice_vec[1] = vec[1].as_slice();
        slice_vec[2] = vec[2].as_slice();
        slice_vec[3] = vec[3].as_slice();

        slice_vec[4] = vec[4].as_slice();
        if vec[4] != [0u8; 32] {
            H::hashv(&slice_vec[..4])
        } else {
            H::hashv(slice_vec.as_slice())
        }
    }
}

// TODO: add borsh compat test TokenMetadataUi TokenMetadata
/// Ui Token metadata with Strings instead of bytes.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct TokenMetadataUi {
    // TODO: decide whether to move down for more efficient zero copy. Or impl manual zero copy.
    /// The authority that can sign to update the metadata
    pub update_authority: Option<Pubkey>,
    // TODO: decide whether to keep this.
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    pub metadata: MetadataUi,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<AdditionalMetadataUi>,
    // TODO: decide whether to do this on this or MintAccount level
    /// 0: Poseidon, 1: Sha256, 2: Keccak256, 3: Sha256Flat
    pub version: u8,
}

#[derive(Debug, LightHasher, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MetadataUi {
    /// The longer name of the token
    pub name: String,
    /// The shortened symbol for the token
    pub symbol: String,
    /// The URI pointing to richer metadata
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct AdditionalMetadataUi {
    /// The key of the metadata
    pub key: String,
    /// The value of the metadata
    pub value: String,
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

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct AdditionalMetadata {
    /// The key of the metadata
    pub key: Vec<u8>,
    /// The value of the metadata
    pub value: Vec<u8>,
}

// Small instruction data input.
// TODO: impl hash fn that is consistent with full hash fn, then we can add it to the instruction data enum
pub struct SmallTokenMetadata {
    /// The authority that can sign to update the metadata
    pub update_authority: Option<Pubkey>,
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    pub metadata_hash: [u8; 32],
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
    /// 0: Poseidon, 1: Sha256, 2: Keccak256, 3: Sha256Flat
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct TokenMetadataInstructionData {
    pub update_authority: Option<Pubkey>,
    pub metadata: Metadata,
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
    pub version: u8,
}
use light_zero_copy::ZeroCopyNew;

pub fn create_output_token_metadata<'a>(
    token_metadata_data: &ZTokenMetadataInstructionData<'a>,
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    start_offset: usize,
) -> Result<usize, ProgramError> {
    let cpi_data = output_compressed_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let additional_metadata_configs =
        if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
            additional_metadata
                .iter()
                .map(|item| AdditionalMetadataConfig {
                    key: item.key.len() as u32,
                    value: item.value.len() as u32,
                })
                .collect()
        } else {
            vec![]
        };

    let config = TokenMetadataConfig {
        update_authority: (token_metadata_data.update_authority.is_some(), ()),
        metadata: MetadataConfig {
            name: token_metadata_data.metadata.name.len() as u32,
            symbol: token_metadata_data.metadata.symbol.len() as u32,
            uri: token_metadata_data.metadata.uri.len() as u32,
        },
        additional_metadata: additional_metadata_configs,
    };
    let byte_len = TokenMetadata::byte_len(&config);
    let end_offset = start_offset + byte_len;

    let (mut token_metadata, _) =
        TokenMetadata::new_zero_copy(&mut cpi_data.data[start_offset..end_offset], config)?;
    if let Some(mut authority) = token_metadata.update_authority {
        *authority = *token_metadata_data
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

    // Set version
    *token_metadata.version = token_metadata_data.version;

    // Set additional metadata if provided
    if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
        for (i, item) in additional_metadata.iter().enumerate() {
            token_metadata.additional_metadata[i]
                .key
                .copy_from_slice(&item.key);
            token_metadata.additional_metadata[i]
                .value
                .copy_from_slice(&item.value);
        }
    }

    Ok(end_offset)
}

// #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
// pub struct EfficientTokenMetadata {
//     // TODO: decide whether to keep this.
//     /// The associated mint, used to counter spoofing to be sure that metadata
//     /// belongs to a particular mint
//     pub mint: Pubkey,
//     pub metadata: EfficientMetadata,
//     /// The authority that can sign to update the metadata
//     pub update_authority: Option<Pubkey>,
//     /// Any additional metadata about the token as key-value pairs. The program
//     /// must avoid storing the same key twice.
//     pub additional_metadata: Vec<EfficientAdditionalMetadata>,
//     // TODO: decide whether to do this on this or MintAccount level
//     /// 0: Poseidon, 1: Sha256, 2: Keccak256, 3: Sha256Flat
//     pub version: u8,
// }

// #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
// pub struct EfficientMetadata {
//     /// The longer name of the token
//     pub name: [u8; 32],
//     /// The shortened symbol for the token
//     pub symbol: [u8; 32],
//     /// The URI pointing to richer metadata
//     pub uri: [u8; 32],
// }

// #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut)]
// pub struct EfficientAdditionalMetadata {
//     /// The key of the metadata
//     pub key: [u8; 32],
//     /// The value of the metadata
//     pub value: [u8; 32],
// }
