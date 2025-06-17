use crate::{AnchorDeserialize, AnchorSerialize};
pub use light_compressed_account::instruction_data::compressed_proof::CompressedProof;

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub proof_by_index: bool,
}

// TODO: get from light-compressed-account
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedCpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    pub cpi_context_account_index: u8,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
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
    pub owner: [u8; 32],
    /// Index of change compressed account in output compressed accounts. In
    /// case that the delegate didn't spend the complete delegated compressed
    /// account balance the change compressed account will be delegated to her
    /// as well.
    pub delegate_change_account_index: Option<u8>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub mint: [u8; 32],
    /// Is required if the signer is delegate,
    /// -> delegate is authority account,
    /// owner = Some(owner) is the owner of the token account.
    pub delegated_transfer: Option<DelegatedTransfer>,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    pub is_compress: bool,
    pub compress_or_decompress_amount: Option<u64>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub lamports_change_account_merkle_tree_index: Option<u8>,
    pub with_transaction_hash: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct PackedTokenTransferOutputData {
    pub owner: [u8; 32],
    pub amount: u64,
    pub lamports: Option<u64>,
    pub merkle_tree_index: u8,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct TokenTransferOutputData {
    pub owner: [u8; 32],
    pub amount: u64,
    pub lamports: Option<u64>,
    pub merkle_tree: [u8; 32],
}
