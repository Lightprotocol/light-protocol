use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{
    hash_cache::HashCache,
    state::{
        token_metadata_hash, token_metadata_hash_with_hashed_values, AdditionalMetadata, Metadata,
    },
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

macro_rules! impl_hash_token_metadata {
    (
        $self:ident,
        $mint_param:ident: $mint_type:ty,
        $hash_cache:ident,
        $get_hashed_mint:expr,
        $map_additional_metadata:expr,
        $map_update_authority:expr,
        $hash_fn:ident
    ) => {{
        let metadata_hash = light_hasher::DataHasher::hash::<H>(&$self.metadata)
            .map_err(|_| CTokenError::InvalidAccountData)?;

        let additional_metadata: arrayvec::ArrayVec<(&[u8], &[u8]), 32> =
            if let Some(ref additional_metadata) = $self.additional_metadata {
                additional_metadata
                    .iter()
                    .map($map_additional_metadata)
                    .collect()
            } else {
                arrayvec::ArrayVec::new()
            };

        let hashed_update_authority = $map_update_authority
            .map(|update_authority| $hash_cache.get_or_hash_pubkey(&update_authority));

        let hashed_mint = $get_hashed_mint;

        $hash_fn::<H>(
            hashed_update_authority
                .as_ref()
                .map(|h: &[u8; 32]| h.as_slice()),
            hashed_mint.as_slice(),
            metadata_hash.as_slice(),
            &additional_metadata,
            $self.version,
        )
        .map_err(|_| CTokenError::InvalidAccountData)
    }};
}

// TODO: double check hashing scheme, add tests with partial data
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
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
        hash_cache: &mut HashCache,
    ) -> Result<[u8; 32], CTokenError> {
        impl_hash_token_metadata!(
            self,
            mint: light_compressed_account::Pubkey,
            hash_cache,
            hash_cache.get_or_hash_mint(&mint.into())?,
            |item| (item.key.as_slice(), item.value.as_slice()),
            self.update_authority.map(|value|value.to_bytes()),
            token_metadata_hash
        )
    }
}

impl ZTokenMetadataInstructionData<'_> {
    pub fn hash_token_metadata<H: light_hasher::Hasher>(
        &self,
        hashed_mint: &[u8; 32],
        hash_cache: &mut HashCache,
    ) -> Result<[u8; 32], CTokenError> {
        impl_hash_token_metadata!(
            self,
            hashed_mint: &[u8; 32],
            hash_cache,
            *hashed_mint,
            |item| (item.key, item.value),
            (self.update_authority.map(|value| (*value).to_bytes())),
            token_metadata_hash_with_hashed_values
        )
    }
}
