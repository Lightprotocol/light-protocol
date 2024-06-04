pub mod test_indexer;

pub use test_indexer::create_mint_helper;
pub use test_indexer::AddressMerkleTreeAccounts;
pub use test_indexer::AddressMerkleTreeBundle;
pub use test_indexer::StateMerkleTreeAccounts;
pub use test_indexer::StateMerkleTreeBundle;
pub use test_indexer::TestIndexer;
pub use test_indexer::TokenDataWithContext;

use account_compression::initialize_address_merkle_tree::{
    Error as AccountCompressionError, Pubkey,
};
use light_hash_set::HashSetError;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use thiserror::Error;

pub trait Indexer: Sync + Send + 'static {
    fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> impl std::future::Future<Output = Result<Vec<MerkleProof>, IndexerError>> + Send + Sync;

    fn account_nullified(&mut self, merkle_tree_pubkey: Pubkey, account_hash: &str);
}

#[derive(Debug)]
pub struct MerkleProof {
    pub hash: String,
    pub leaf_index: u32,
    pub merkle_tree: String,
    pub proof: Vec<[u8; 32]>,
    pub root_seq: i64,
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
