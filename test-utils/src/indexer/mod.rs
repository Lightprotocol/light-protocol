pub mod test_indexer;

use num_bigint::BigUint;
use std::fmt::Debug;
pub use test_indexer::create_mint_helper;
pub use test_indexer::AddressMerkleTreeAccounts;
pub use test_indexer::AddressMerkleTreeBundle;
pub use test_indexer::StateMerkleTreeAccounts;
pub use test_indexer::StateMerkleTreeBundle;
pub use test_indexer::TestIndexer;
pub use test_indexer::TokenDataWithContext;

use crate::rpc::rpc_connection::RpcConnection;
use account_compression::initialize_address_merkle_tree::{
    Error as AccountCompressionError, Pubkey,
};
use light_hash_set::HashSetError;
use light_indexed_merkle_tree::array::IndexedElement;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use thiserror::Error;

pub trait Indexer: Sync + Send + Clone + Debug + 'static {
    fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> impl std::future::Future<Output = Result<Vec<MerkleProof>, IndexerError>> + Send + Sync;

    fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send + Sync;

    fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        address: [u8; 32],
    ) -> impl std::future::Future<Output = Result<NewAddressProofWithContext, IndexerError>> + Send + Sync;

    fn account_nullified(&mut self, _merkle_tree_pubkey: Pubkey, _account_hash: &str) {}

    fn address_tree_updated(
        &mut self,
        _merkle_tree_pubkey: [u8; 32],
        _context: NewAddressProofWithContext,
    ) {
    }
}

#[derive(Debug)]
pub struct MerkleProof {
    pub hash: String,
    pub leaf_index: u32,
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
