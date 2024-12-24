use std::fmt::Debug;

use account_compression::initialize_address_merkle_tree::{
    Error as AccountCompressionError, Pubkey,
};
use async_trait::async_trait;
use light_client::rpc::RpcConnection;
use light_compressed_token::TokenData;
use light_hash_set::HashSetError;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement},
    reference::IndexedMerkleTree,
};
use light_merkle_tree_reference::MerkleTree;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::CompressedAccountWithMerkleContext, event::PublicTransactionEvent},
};
use num_bigint::BigUint;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use solana_sdk::signature::Keypair;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct TokenDataWithContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}

#[derive(Debug, Default)]
pub struct BatchedTreeProofRpcResult {
    pub proof: Option<CompressedProof>,
    // If none -> proof by index, else included in zkp
    pub root_indices: Vec<Option<u16>>,
    pub address_root_indices: Vec<u16>,
}

#[derive(Debug, Default)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<Option<u16>>,
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
    pub version: u64,
    pub output_queue_elements: Vec<[u8; 32]>,
    /// leaf index, leaf, tx hash
    pub input_leaf_indices: Vec<(u32, [u8; 32], [u8; 32])>,
}

#[derive(Debug, Clone)]
pub struct AddressMerkleTreeBundle {
    pub rollover_fee: i64,
    pub merkle_tree: Box<IndexedMerkleTree<Poseidon, usize>>,
    pub indexed_array: Box<IndexedArray<Poseidon, usize>>,
    pub accounts: AddressMerkleTreeAccounts,
    pub queue_elements: Vec<[u8; 32]>,
}

pub struct ProofOfLeaf {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
}

#[async_trait]
pub trait Indexer<R: RpcConnection>: Sync + Send + Debug + 'static {
    /// Returns queue elements from the queue with the given pubkey. For input
    /// queues account compression program does not store queue elements in the
    /// account data but only emits these in the public transaction event. The
    /// indexer needs the queue elements to create batch update proofs.
    async fn get_queue_elements(
        &self,
        pubkey: [u8; 32],
        batch: u64,
        start_offset: u64,
        end_offset: u64,
    ) -> Result<Vec<[u8; 32]>, IndexerError>;

    fn get_proof_by_index(&mut self, _merkle_tree_pubkey: Pubkey, _index: u64) -> ProofOfLeaf {
        unimplemented!("get_proof_by_index not implemented")
    }

    fn get_proofs_by_indices(
        &mut self,
        _merkle_tree_pubkey: Pubkey,
        _indices: &[u64],
    ) -> Vec<ProofOfLeaf> {
        unimplemented!("get_proof_by_index not implemented")
    }

    fn get_leaf_indices_tx_hashes(
        &mut self,
        _merkle_tree_pubkey: Pubkey,
        _zkp_batch_size: usize,
    ) -> Vec<(u32, [u8; 32], [u8; 32])> {
        unimplemented!();
    }

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, IndexerError>;

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
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError>;

    async fn get_multiple_new_address_proofs_full(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError>;

    fn account_nullified(&mut self, _merkle_tree_pubkey: Pubkey, _account_hash: &str) {}

    fn address_tree_updated(
        &mut self,
        _merkle_tree_pubkey: Pubkey,
        _context: &NewAddressProofWithContext<16>,
    ) {
    }

    fn get_state_merkle_tree_accounts(&self, _pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts> {
        unimplemented!()
    }

    fn add_event_and_compressed_accounts(
        &mut self,
        _slot: u64,
        _event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithContext>,
    ) {
        unimplemented!()
    }

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle> {
        unimplemented!()
    }

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle> {
        unimplemented!()
    }

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        unimplemented!()
    }

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle> {
        unimplemented!()
    }

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithContext> {
        unimplemented!()
    }

    fn get_payer(&self) -> &Keypair {
        unimplemented!()
    }

    fn get_group_pda(&self) -> &Pubkey {
        unimplemented!()
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        _compressed_accounts: Option<Vec<[u8; 32]>>,
        _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> ProofRpcResult {
        unimplemented!()
    }

    async fn create_proof_for_compressed_accounts2(
        &mut self,
        _compressed_accounts: Option<Vec<[u8; 32]>>,
        _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> BatchedTreeProofRpcResult {
        unimplemented!()
    }

    fn add_address_merkle_tree_accounts(
        &mut self,
        _merkle_tree_keypair: &Keypair,
        _queue_keypair: &Keypair,
        _owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        unimplemented!()
    }

    fn get_compressed_accounts_by_owner(
        &self,
        _owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        unimplemented!()
    }

    fn get_compressed_token_accounts_by_owner(&self, _owner: &Pubkey) -> Vec<TokenDataWithContext> {
        unimplemented!()
    }

    fn add_state_bundle(&mut self, _state_bundle: StateMerkleTreeBundle) {
        unimplemented!()
    }

    async fn update_test_indexer_after_append(
        &mut self,
        _rpc: &mut R,
        _merkle_tree_pubkey: Pubkey,
        _output_queue_pubkey: Pubkey,
        _num_inserted_zkps: u64,
    ) {
        unimplemented!()
    }

    async fn update_test_indexer_after_nullification(
        &mut self,
        _rpc: &mut R,
        _merkle_tree_pubkey: Pubkey,
        _batch_index: usize,
    ) {
        unimplemented!()
    }
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
#[derive(Clone, Debug, PartialEq)]
pub struct NewAddressProofWithContext<const NET_HEIGHT: usize> {
    pub merkle_tree: [u8; 32],
    pub root: [u8; 32],
    pub root_seq: u64,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; NET_HEIGHT],
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
