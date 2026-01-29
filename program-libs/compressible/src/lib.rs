pub mod compression_info;
pub mod config;
pub mod error;
pub mod registry_instructions;
pub mod rent;

/// Decompressed PDA discriminator - marks a compressed account as a decompressed PDA placeholder.
/// When a CMint or other PDA is decompressed to a Solana account, the compressed account
/// stores this discriminator and the PDA pubkey (hashed) to preserve the address.
pub const DECOMPRESSED_PDA_DISCRIMINATOR: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 0];

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, data::PackedAddressTreeInfo,
};

/// Proof data for instruction params when creating new compressed accounts.
/// Used in the INIT flow - pass directly to instruction data.
/// All accounts use the same address tree, so only one `address_tree_info` is needed.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateAccountsProof {
    /// The validity proof.
    pub proof: ValidityProof,
    /// Single packed address tree info (all accounts use same tree).
    pub address_tree_info: PackedAddressTreeInfo,
    /// Output state tree index for new compressed accounts.
    pub output_state_tree_index: u8,
    /// State merkle tree index (needed for mint creation decompress validation).
    /// This is optional to maintain backwards compatibility.
    pub state_tree_index: Option<u8>,
    /// Offset in remaining_accounts where Light system accounts start.
    pub system_accounts_offset: u8,
}
