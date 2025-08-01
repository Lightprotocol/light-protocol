use light_compressed_account::{
    instruction_data::zero_copy_set::CompressedCpiContextTrait, Pubkey,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{
    instructions::create_compressed_mint::UpdateCompressedMintInstructionData, AnchorDeserialize,
    AnchorSerialize, CTokenError,
};

/// Authority types for compressed mint updates, following SPL Token-2022 pattern
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedMintAuthorityType {
    /// Authority to mint new tokens
    MintTokens = 0,
    /// Authority to freeze token accounts
    FreezeAccount = 1,
}

impl TryFrom<u8> for CompressedMintAuthorityType {
    type Error = CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressedMintAuthorityType::MintTokens),
            1 => Ok(CompressedMintAuthorityType::FreezeAccount),
            _ => Err(CTokenError::InvalidAuthorityType),
        }
    }
}

impl From<CompressedMintAuthorityType> for u8 {
    fn from(authority_type: CompressedMintAuthorityType) -> u8 {
        authority_type as u8
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateCompressedMintInstructionDataV2 {
    pub compressed_mint_inputs: UpdateCompressedMintInstructionData,
    pub authority_type: u8,             // CompressedMintAuthorityType as u8
    pub new_authority: Option<Pubkey>,  // None = revoke authority, Some(key) = set new authority
    pub mint_authority: Option<Pubkey>, // Current mint authority (needed when updating freeze authority)
    pub cpi_context: Option<UpdateMintCpiContext>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct UpdateMintCpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_queue_index: u8,
}

impl CompressedCpiContextTrait for ZUpdateMintCpiContext<'_> {
    fn first_set_context(&self) -> u8 {
        self.first_set_context() as u8
    }

    fn set_context(&self) -> u8 {
        self.set_context() as u8
    }
}
