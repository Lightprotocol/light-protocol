use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof, zero_copy_set::CompressedCpiContextTrait,
    },
    Pubkey,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{
    instructions::{
        create_compressed_mint::CompressedMintInstructionData, mint_to_compressed::MintToAction,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateAuthority {
    pub new_authority: Option<Pubkey>, // None = revoke authority, Some(key) = set new authority
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateSplMintAction {
    pub mint_bump: u8,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct DecompressedRecipient {
    pub account_index: u8, // Index into remaining accounts for the recipient token account
    pub amount: u64,
    pub compressible_config: Option<crate::instructions::extensions::compressible::CompressibleExtensionInstructionData>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToDecompressedAction {
    pub recipient: DecompressedRecipient,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    UpdateMintAuthority(UpdateAuthority),
    UpdateFreezeAuthority(UpdateAuthority),
    CreateSplMint(CreateSplMintAction),
    MintToDecompressed(MintToDecompressedAction),
    UpdateMetadata,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintActionCompressedInstructionData {
    pub create_mint: bool,
    /// Only used if create mint
    pub mint_bump: u8,
    /// Only set if mint already exists
    pub leaf_index: u32,
    /// Only set if mint already exists
    pub prove_by_index: bool,
    /// If create mint, root index of address proof
    /// If mint already exists, root index of validity proof
    /// If proof by index not used.
    pub root_index: u16,
    pub compressed_address: [u8; 32],
    /// If some -> no input because we create mint
    pub mint: CompressedMintInstructionData,
    pub actions: Vec<Action>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<CpiContext>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct CpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    // Used as address tree index if create mint
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_queue_index: u8,
    pub token_out_queue_index: u8,
    // Index of the compressed account that should receive the new address (0 = mint, 1+ = token accounts)
    pub assigned_account_index: u8,
}
impl CompressedCpiContextTrait for ZCpiContext<'_> {
    fn first_set_context(&self) -> u8 {
        self.first_set_context() as u8
    }

    fn set_context(&self) -> u8 {
        self.set_context() as u8
    }
}
