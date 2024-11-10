use std::{fmt::Debug, future::Future};

use light_concurrent_merkle_tree::light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement},
    reference::IndexedMerkleTree,
};
use light_merkle_tree_reference::MerkleTree;
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext, event::PublicTransactionEvent,
    proof::ProofRpcResult, token::TokenDataWithMerkleContext,
};
use num_bigint::BigUint;
use photon_api::models::{
    GetCompressionSignaturesForAddressPostRequestParams,
    GetCompressionSignaturesForOwnerPostRequestParams,
    GetCompressionSignaturesForTokenOwnerPostRequestParams,
    GetLatestCompressionSignaturesPostRequestParams, SignatureInfo, TransactionInfo,
};
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use crate::rpc::RpcConnection;

pub mod photon_indexer;
pub mod test_indexer;

pub use photon_indexer::PhotonIndexer;
pub use test_indexer::TestIndexer;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("RPC Error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
    #[error("failed to deserialize account data")]
    DeserializeError(#[from] solana_sdk::program_error::ProgramError),
    #[error("failed to copy merkle tree")]
    CopyMerkleTreeError(#[from] std::io::Error),
    #[error("error: {0:?}")]
    Custom(String),
    #[error("unknown error")]
    Unknown,
}

/// Indexer trait defining interface for interacting with Light Protocol RPCs.
///
/// Two implementations are provided:
/// - PhotonIndexer: Production implementation using remote RPC
/// - TestIndexer: Test implementation with local state management
pub trait Indexer<R: RpcConnection>: Sync + Send + Debug + 'static {
    // Core Account Operations
    /// Returns compressed accounts for a given owner public key, with optional filters and data slice
    fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext>;

    /// Fetches a compressed account by its hash or address
    async fn get_compressed_account(
        &self,
        hash: String,
    ) -> Result<CompressedAccountWithMerkleContext, IndexerError>;

    /// Fetches multiple compressed accounts by their hashes
    async fn get_multiple_compressed_accounts(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<CompressedAccountWithMerkleContext>, IndexerError>;

    // Balance Operations
    /// Gets token balance for a compressed account
    async fn get_compressed_account_balance(&self, hash: String) -> Result<u64, IndexerError>;

    /// Gets total compressed balance for an owner
    async fn get_compressed_balance_by_owner(&self, owner: &Pubkey) -> Result<u64, IndexerError>;

    // Proof Operations
    /// Gets merkle proof context for a compressed account
    async fn get_compressed_account_proof(&self, hash: String)
        -> Result<MerkleProof, IndexerError>;

    /// Gets merkle proof contexts for multiple compressed accounts
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError>;

    /// Gets validity proof for compressed accounts and new addresses with merkle tree context
    async fn get_validity_proof(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> ProofRpcResult;

    // Transaction Operations
    /// Gets transaction details with compression information including opened/closed accounts
    async fn get_transaction_with_compression_info(
        &self,
        signature: String,
    ) -> Result<TransactionInfo, IndexerError>;

    // Signature Operations
    /// Gets latest compression signatures with optional cursor and limit
    async fn get_latest_compression_signatures(
        &self,
        params: GetLatestCompressionSignaturesPostRequestParams,
    ) -> Result<Vec<String>, IndexerError>;

    /// Gets latest non-voting signatures with optional limit
    async fn get_latest_non_voting_signatures(&self) -> Result<Vec<String>, IndexerError>;

    // Health Operations
    /// Returns indexer health status
    async fn get_indexer_health(&self) -> Result<bool, IndexerError>;

    /// Returns current indexer slot
    async fn get_indexer_slot(&self) -> Result<u64, IndexerError>;
}

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub hash: String,
    pub leaf_index: u64,
    pub merkle_tree: String,
    pub proof: Vec<[u8; 32]>,
    pub root_seq: u64,
}

// For consistency with the Photon API.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct NewAddressProofWithContext {
    pub merkle_tree: [u8; 32],
    pub root: [u8; 32],
    pub root_seq: u64,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; 16],
    pub new_low_element: Option<IndexedElement<usize>>,
    pub new_element: Option<IndexedElement<usize>>,
    pub new_element_next_value: Option<BigUint>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub nullifier_queue: Pubkey,
    pub cpi_context: Pubkey,
}

#[derive(Debug, Clone, Copy)]
pub struct AddressMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
}

#[derive(Debug, Clone)]
pub struct StateMerkleTreeBundle {
    pub rollover_fee: u64,
    pub merkle_tree: Box<MerkleTree<Poseidon>>,
    pub accounts: StateMerkleTreeAccounts,
}

#[derive(Debug, Clone)]
pub struct AddressMerkleTreeBundle {
    pub rollover_fee: u64,
    pub merkle_tree: Box<IndexedMerkleTree<Poseidon, usize>>,
    pub indexed_array: Box<IndexedArray<Poseidon, usize>>,
    pub accounts: AddressMerkleTreeAccounts,
}
