use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

// TODO: add borsh compat test TokenMetadataUi TokenMetadata
/// Ui Token metadata with Strings instead of bytes.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct TokenMetadataUi {
    /// The authority that can sign to update the metadata
    pub update_authority: Option<Pubkey>,
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    pub metadata: MetadataUi,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<AdditionalMetadataUi>,
    /// 0: Poseidon, 1: Sha256, 2: Keccak256, 3: Sha256Flat
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
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
