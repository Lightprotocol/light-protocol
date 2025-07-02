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

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateAuthority {
    pub new_authority: Option<Pubkey>, // None = revoke authority, Some(key) = set new authority
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateSplMintAction {
    pub mint_bump: u8,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct DecompressedRecipient {
    pub account_index: u8, // Index into remaining accounts for the recipient token account
    pub amount: u64,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToDecompressedAction {
    pub recipient: DecompressedRecipient,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateMetadataFieldAction {
    pub extension_index: u8, // Index of the TokenMetadata extension in the extensions array
    pub field_type: u8,      // 0=Name, 1=Symbol, 2=Uri, 3=Custom key
    pub key: Vec<u8>,        // Empty for Name/Symbol/Uri, key string for custom fields
    pub value: Vec<u8>,      // UTF-8 encoded value
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateMetadataAuthorityAction {
    pub extension_index: u8, // Index of the TokenMetadata extension in the extensions array
    pub new_authority: Pubkey, // Use zero bytes to set to None
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct RemoveMetadataKeyAction {
    pub extension_index: u8, // Index of the TokenMetadata extension in the extensions array
    pub key: Vec<u8>,        // UTF-8 encoded key to remove
    pub idempotent: u8,      // 0=false, 1=true - don't error if key doesn't exist
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub enum Action {
    /// Mint compressed tokens to compressed accounts.
    MintTo(MintToAction),
    /// Update mint authority of a compressed mint account.
    UpdateMintAuthority(UpdateAuthority),
    /// Update freeze authority of a compressed mint account.
    UpdateFreezeAuthority(UpdateAuthority),
    /// Create an spl mint for a cmint.
    /// - existing supply is minted to a token pool account.
    /// - mint and freeze authority are a ctoken pda.
    /// - is an spl-token-2022 mint account.
    CreateSplMint(CreateSplMintAction),
    /// Mint ctokens from a cmint to a ctoken solana account
    /// (tokens are not compressed but not spl tokens).
    MintToDecompressed(MintToDecompressedAction),
    UpdateMetadataField(UpdateMetadataFieldAction),
    UpdateMetadataAuthority(UpdateMetadataAuthorityAction),
    RemoveMetadataKey(RemoveMetadataKeyAction),
}

#[repr(C)]
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
    pub token_pool_bump: u8,
    pub token_pool_index: u8,
    pub actions: Vec<Action>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<CpiContext>,
}

#[repr(C)]
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

impl CpiContext {
    /// Specific helper for creating a cmint as last use of cpi context.
    pub fn last_cpi_create_mint(
        address_tree_index: usize,
        output_state_queue_index: usize,
        mint_account_index: usize,
    ) -> Self {
        Self {
            set_context: false,
            first_set_context: false,
            in_tree_index: address_tree_index as u8,
            in_queue_index: 0, // unused
            out_queue_index: output_state_queue_index as u8,
            token_out_queue_index: output_state_queue_index as u8,
            assigned_account_index: mint_account_index as u8,
        }
    }
}
