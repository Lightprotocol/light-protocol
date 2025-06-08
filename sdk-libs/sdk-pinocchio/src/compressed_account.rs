// Types copied from light-compressed-account to avoid dependency
use crate::{BorshDeserialize, BorshSerialize};

#[derive(Debug, PartialEq, Default, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccount {
    pub owner: [u8; 32],
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

#[derive(Debug, PartialEq, Default, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[derive(Debug, PartialEq, Default, Clone, BorshSerialize, BorshDeserialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub read_only: bool,
}

#[derive(Debug, PartialEq, Default, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

#[derive(Debug, PartialEq, Default, Clone, BorshSerialize, BorshDeserialize)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

#[derive(Debug, Default, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct CompressedAccountInfo {
    pub address: Option<[u8; 32]>,
    pub input: Option<InAccountInfo>,
    pub output: Option<OutAccountInfo>,
}

#[derive(Debug, Default, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct InAccountInfo {
    pub data_hash: [u8; 32],
    pub lamports: u64,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub discriminator: [u8; 8],
}

#[derive(Debug, Default, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct OutAccountInfo {
    pub data_hash: [u8; 32],
    pub lamports: u64,
    pub output_merkle_tree_index: u8,
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
}

#[derive(Debug, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<crate::CompressedProof>,
    pub new_address_params: Vec<crate::NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

#[derive(Debug, Default, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct CompressedCpiContext {
    pub cpi_context_account_index: u8,
    pub cpi_signature_account_index: u8,
}

// Constants for discriminators
pub const DISCRIMINATOR_INVOKE_CPI: [u8; 8] = [49, 212, 191, 129, 39, 194, 43, 196];
