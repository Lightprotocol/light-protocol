use std::fmt::Debug;

use light_concurrent_merkle_tree::light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement},
    reference::IndexedMerkleTree,
};
use light_merkle_tree_reference::MerkleTree;
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext,
    event::PublicTransactionEvent,
    proof::ProofRpcResult,
    token::{TokenData, TokenDataWithMerkleContext},
};
use num_bigint::BigUint;
use photon_api::{
    apis::default_api::{
        GetCompressedAccountsByOwnerPostError, GetMultipleCompressedAccountProofsPostError,
        GetMultipleNewAddressProofsV2PostError,
    },
    models::GetLatestCompressionSignaturesPostRequestParams,
};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use crate::rpc::RpcConnection;

pub mod photon_indexer;
pub mod test_indexer;

pub use photon_indexer::PhotonIndexer;
pub use test_indexer::TestIndexer;

pub trait RpcRequirements: RpcConnection + Send + Sync + 'static {}
impl<T> RpcRequirements for T where T: RpcConnection + Send + Sync + 'static {}

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

impl From<GetCompressedAccountsByOwnerPostError> for IndexerError {
    fn from(error: GetCompressedAccountsByOwnerPostError) -> Self {
        IndexerError::Custom(format!("{:?}", error))
    }
}

impl From<GetMultipleCompressedAccountProofsPostError> for IndexerError {
    fn from(error: GetMultipleCompressedAccountProofsPostError) -> Self {
        IndexerError::Custom(format!("{:?}", error))
    }
}

impl From<GetMultipleNewAddressProofsV2PostError> for IndexerError {
    fn from(error: GetMultipleNewAddressProofsV2PostError) -> Self {
        IndexerError::Custom(format!("{:?}", error))
    }
}

impl
    From<
        photon_api::apis::Error<
            photon_api::apis::default_api::GetCompressedAccountsByOwnerPostError,
        >,
    > for IndexerError
{
    fn from(
        e: photon_api::apis::Error<
            photon_api::apis::default_api::GetCompressedAccountsByOwnerPostError,
        >,
    ) -> Self {
        IndexerError::Custom(e.to_string())
    }
}

impl From<photon_api::apis::Error<GetMultipleCompressedAccountProofsPostError>> for IndexerError {
    fn from(e: photon_api::apis::Error<GetMultipleCompressedAccountProofsPostError>) -> Self {
        IndexerError::Custom(e.to_string())
    }
}

impl From<photon_api::apis::Error<GetMultipleNewAddressProofsV2PostError>> for IndexerError {
    fn from(e: photon_api::apis::Error<GetMultipleNewAddressProofsV2PostError>) -> Self {
        IndexerError::Custom(e.to_string())
    }
}

/// Indexer trait defining interface for interacting with Light Protocol RPCs.
///
/// Two implementations are provided:
/// - PhotonIndexer: Production implementation using remote RPC
/// - TestIndexer: Test implementation with local state management
pub trait Indexer: Sync + Send + Debug + 'static {
    type Rpc: RpcRequirements;

    // Core Account Operations
    fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext>;

    fn get_compressed_account(
        &self,
        hash: String,
    ) -> impl std::future::Future<Output = Result<CompressedAccountWithMerkleContext, IndexerError>> + Send;

    fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send;

    fn get_multiple_compressed_accounts(
        &self,
        hashes: Vec<String>,
    ) -> impl std::future::Future<
        Output = Result<Vec<CompressedAccountWithMerkleContext>, IndexerError>,
    > + Send;

    // Balance Operations
    fn get_compressed_account_balance(
        &self,
        hash: String,
    ) -> impl std::future::Future<Output = Result<u64, IndexerError>> + Send;

    fn get_compressed_balance_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<u64, IndexerError>> + Send;

    // Proof Operations
    fn get_compressed_account_proof(
        &self,
        hash: String,
    ) -> impl std::future::Future<Output = Result<MerkleProof, IndexerError>> + Send;

    fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> impl std::future::Future<Output = Result<Vec<MerkleProof>, IndexerError>> + Send;

    fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> impl std::future::Future<Output = Result<Vec<NewAddressProofWithContext>, IndexerError>> + Send;

    fn get_validity_proof(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut Self::Rpc,
    ) -> impl std::future::Future<Output = ProofRpcResult> + Send;

    // Transaction Operations
    fn get_transaction_with_compression_info(
        &self,
        signature: String,
    ) -> impl std::future::Future<Output = Result<TransactionInfo, IndexerError>> + Send;

    // Signature Operations
    fn get_latest_compression_signatures(
        &self,
        params: GetLatestCompressionSignaturesPostRequestParams,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send;

    fn get_latest_non_voting_signatures(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send;

    // Health Operations
    fn get_indexer_health(
        &self,
    ) -> impl std::future::Future<Output = Result<bool, IndexerError>> + Send;

    fn get_indexer_slot(
        &self,
    ) -> impl std::future::Future<Output = Result<u64, IndexerError>> + Send;

    // State Management
    fn add_event_and_compressed_accounts(
        &mut self,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    );
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

#[derive(Debug, Clone)]
pub struct TransactionInfo {
    pub compression_info: CompressionInfo,
    pub transaction: Value, // Using serde_json::Value for the "any" type
}

#[derive(Debug, Clone)]
pub struct CompressionInfo {
    pub closed_accounts: Vec<AccountWithTokenData>,
    pub opened_accounts: Vec<AccountWithTokenData>,
}

#[derive(Debug, Clone)]
pub struct AccountWithTokenData {
    pub account: CompressedAccountWithMerkleContext,
    pub optional_token_data: Option<TokenData>,
}
