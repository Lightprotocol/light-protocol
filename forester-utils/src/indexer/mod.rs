use account_compression::initialize_address_merkle_tree::{
    Error as AccountCompressionError, Pubkey,
};
use async_trait::async_trait;
use light_client::rpc::RpcConnection;
use light_compressed_token::TokenData;
use light_hash_set::HashSetError;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::{IndexedArray, IndexedElement};
use light_indexed_merkle_tree::reference::IndexedMerkleTree;
use light_merkle_tree_reference::MerkleTree;
use light_system_program::invoke::processor::CompressedProof;
use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_system_program::sdk::event::PublicTransactionEvent;
use num_bigint::BigUint;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use solana_sdk::signature::Keypair;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub struct TokenDataWithContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}

#[derive(Debug)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub address_root_indices: Vec<u16>,
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
    pub rollover_fee: i64,
    pub merkle_tree: Box<MerkleTree<Poseidon>>,
    pub accounts: StateMerkleTreeAccounts,
}

#[derive(Debug, Clone)]
pub struct AddressMerkleTreeBundle {
    pub rollover_fee: i64,
    pub merkle_tree: Box<IndexedMerkleTree<Poseidon, usize>>,
    pub indexed_array: Box<IndexedArray<Poseidon, usize>>,
    pub accounts: AddressMerkleTreeAccounts,
}

#[async_trait]
pub trait Indexer<R: RpcConnection>: Send + Sync {
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError>;

    async fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError>;

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError>;

    async fn account_nullified(&self, merkle_tree_pubkey: Pubkey, account_hash: &str);

    async fn address_tree_updated(
        &self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext,
    );

    async fn get_state_merkle_tree_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Vec<StateMerkleTreeAccounts>;

    async fn add_event_and_compressed_accounts(
        &self,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithContext>,
    );

    async fn get_state_merkle_trees(&self) -> Vec<StateMerkleTreeBundle>;

    async fn get_address_merkle_trees(&self) -> Vec<AddressMerkleTreeBundle>;

    async fn get_token_compressed_accounts(&self) -> Vec<TokenDataWithContext>;

    fn get_payer(&self) -> &Keypair;

    fn get_group_pda(&self) -> &Pubkey;

    async fn create_proof_for_compressed_accounts(
        &self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &R,
    ) -> ProofRpcResult;

    async fn add_address_merkle_tree_accounts(
        &self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts;

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext>;

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<TokenDataWithContext>;

    async fn add_state_bundle(&self, state_bundle: StateMerkleTreeBundle);
    async fn add_address_bundle(&self, address_bundle: AddressMerkleTreeBundle);

    async fn clear_state_trees(&self);
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

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("RPC Error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
    #[error("failed to deserialize account data")]
    DeserializeError(#[from] solana_sdk::program_error::ProgramError),
    #[error("failed to copy merkle tree")]
    CopyMerkleTreeError(#[from] std::io::Error),
    #[error(transparent)]
    AccountCompressionError(#[from] AccountCompressionError),
    #[error(transparent)]
    HashSetError(#[from] HashSetError),
    #[error(transparent)]
    PhotonApiError(PhotonApiErrorWrapper),
    #[error("error: {0:?}")]
    Custom(String),
    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum PhotonApiErrorWrapper {
    #[error(transparent)]
    GetCompressedAccountProofPostError(#[from] PhotonApiError<GetCompressedAccountProofPostError>),
}

impl From<PhotonApiError<GetCompressedAccountProofPostError>> for IndexerError {
    fn from(err: PhotonApiError<GetCompressedAccountProofPostError>) -> Self {
        IndexerError::PhotonApiError(PhotonApiErrorWrapper::GetCompressedAccountProofPostError(
            err,
        ))
    }
}
