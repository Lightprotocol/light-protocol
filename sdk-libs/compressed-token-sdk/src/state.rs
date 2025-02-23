#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_compressed_account::{
    compressed_account::{CompressedAccountWithMerkleContext, PackedMerkleContext},
    instruction_data::{compressed_proof::CompressedProof, cpi_context::CompressedCpiContext},
};
use solana_program::pubkey::Pubkey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Clone)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TokenDataWithMerkleContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedTokenInstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub mint: Pubkey,

    pub delegated_transfer: Option<bool>,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub output_compressed_accounts: Vec<bool>,
    pub is_compress: bool,
    pub compress_or_decompress_amount: Option<u64>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub lamports_change_account_merkle_tree_index: Option<u8>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InputTokenDataWithContext {
    pub amount: u64,
    pub delegate_index: Option<u8>,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub lamports: Option<u64>,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

/// Struct to provide the owner when the delegate is signer of the transaction.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DelegatedTransfer {
    pub owner: Pubkey,
    /// Index of change compressed account in output compressed accounts. In
    /// case that the delegate didn't spend the complete delegated compressed
    /// account balance the change compressed account will be delegated to her
    /// as well.
    pub delegate_change_account_index: Option<u8>,
}
