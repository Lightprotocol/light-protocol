// use num_bigint::BigUint;
// use solana_sdk::signature::Keypair;
// use std::fmt::Debug;
//
// use account_compression::initialize_address_merkle_tree::{
//     Error as AccountCompressionError, Pubkey,
// };
// use light_client::rpc::RpcConnection;
// use light_compressed_token::TokenData;
// use light_hash_set::HashSetError;
// use light_hasher::Poseidon;
// use light_indexed_merkle_tree::array::{IndexedArray, IndexedElement};
// use light_indexed_merkle_tree::reference::IndexedMerkleTree;
// use light_merkle_tree_reference::MerkleTree;
// use light_system_program::invoke::processor::CompressedProof;
// use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
// use light_system_program::sdk::event::PublicTransactionEvent;
// use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
// use thiserror::Error;
//
// #[derive(Debug, Clone)]
// pub struct TokenDataWithContext {
//     pub token_data: TokenData,
//     pub compressed_account: CompressedAccountWithMerkleContext,
// }
//
// #[derive(Debug, Default)]
// pub struct BatchedTreeProofRpcResult {
//     pub proof: Option<CompressedProof>,
//     // If none -> proof by index, else included in zkp
//     pub root_indices: Vec<Option<u16>>,
//     pub address_root_indices: Vec<u16>,
// }
//
// #[derive(Debug, Default)]
// pub struct ProofRpcResult {
//     pub proof: CompressedProof,
//     pub root_indices: Vec<Option<u16>>,
//     pub address_root_indices: Vec<u16>,
// }
//
//
// pub trait Indexer<R: RpcConnection>: Sync + Send + Debug + 'static {
//     /// Returns queue elements from the queue with the given pubkey. For input
//     /// queues account compression program does not store queue elements in the
//     /// account data but only emits these in the public transaction event. The
//     /// indexer needs the queue elements to create batch update proofs.
//     fn get_queue_elements(
//         &self,
//         pubkey: [u8; 32],
//         batch: u64,
//         start_offset: u64,
//         end_offset: u64,
//     ) -> impl std::future::Future<Output = Result<Vec<[u8; 32]>, IndexerError>> + Send + Sync;
//
//     fn get_subtrees(
//         &self,
//         merkle_tree_pubkey: [u8; 32],
//     ) -> impl std::future::Future<Output = Result<Vec<[u8; 32]>, IndexerError>> + Send + Sync;
//
//     fn get_multiple_compressed_account_proofs(
//         &self,
//         hashes: Vec<String>,
//     ) -> impl std::future::Future<Output = Result<Vec<MerkleProof>, IndexerError>> + Send + Sync;
//
//     fn get_rpc_compressed_accounts_by_owner(
//         &self,
//         owner: &Pubkey,
//     ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send + Sync;
//
//     fn get_multiple_new_address_proofs(
//         &self,
//         merkle_tree_pubkey: [u8; 32],
//         addresses: Vec<[u8; 32]>,
//     ) -> impl std::future::Future<Output = Result<Vec<NewAddressProofWithContext<16>>, IndexerError>>
//            + Send
//            + Sync;
//     fn get_multiple_new_address_proofs_full(
//         &self,
//         merkle_tree_pubkey: [u8; 32],
//         addresses: Vec<[u8; 32]>,
//     ) -> impl std::future::Future<Output = Result<Vec<NewAddressProofWithContext<40>>, IndexerError>>
//            + Send
//            + Sync;
//
//     fn account_nullified(&mut self, _merkle_tree_pubkey: Pubkey, _account_hash: &str) {}
//
//     fn address_tree_updated(
//         &mut self,
//         _merkle_tree_pubkey: Pubkey,
//         _context: &NewAddressProofWithContext<16>,
//     ) {
//     }
//
//     fn get_state_merkle_tree_accounts(&self, _pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts> {
//         unimplemented!()
//     }
//
//     fn add_event_and_compressed_accounts(
//         &mut self,
//         _slot: u64,
//         _event: &PublicTransactionEvent,
//     ) -> (
//         Vec<CompressedAccountWithMerkleContext>,
//         Vec<TokenDataWithContext>,
//     ) {
//         unimplemented!()
//     }
//
//     fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle> {
//         unimplemented!()
//     }
//
//     fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle> {
//         unimplemented!()
//     }
//
//     fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
//         unimplemented!()
//     }
//
//     fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle> {
//         unimplemented!()
//     }
//
//     fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithContext> {
//         unimplemented!()
//     }
//
//     fn get_payer(&self) -> &Keypair {
//         unimplemented!()
//     }
//
//     fn get_group_pda(&self) -> &Pubkey {
//         unimplemented!()
//     }
//
//     #[allow(async_fn_in_trait)]
//     async fn create_proof_for_compressed_accounts(
//         &mut self,
//         _compressed_accounts: Option<Vec<[u8; 32]>>,
//         _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
//         _new_addresses: Option<&[[u8; 32]]>,
//         _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
//         _rpc: &mut R,
//     ) -> ProofRpcResult {
//         unimplemented!()
//     }
//
//     #[allow(async_fn_in_trait)]
//     async fn create_proof_for_compressed_accounts2(
//         &mut self,
//         _compressed_accounts: Option<Vec<[u8; 32]>>,
//         _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
//         _new_addresses: Option<&[[u8; 32]]>,
//         _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
//         _rpc: &mut R,
//     ) -> BatchedTreeProofRpcResult {
//         unimplemented!()
//     }
//
//     fn add_address_merkle_tree_accounts(
//         &mut self,
//         _merkle_tree_keypair: &Keypair,
//         _queue_keypair: &Keypair,
//         _owning_program_id: Option<Pubkey>,
//     ) -> AddressMerkleTreeAccounts {
//         unimplemented!()
//     }
//
//     fn get_compressed_accounts_by_owner(
//         &self,
//         _owner: &Pubkey,
//     ) -> Vec<CompressedAccountWithMerkleContext> {
//         unimplemented!()
//     }
//
//     fn get_compressed_token_accounts_by_owner(&self, _owner: &Pubkey) -> Vec<TokenDataWithContext> {
//         unimplemented!()
//     }
//
//     fn add_state_bundle(&mut self, _state_bundle: StateMerkleTreeBundle) {
//         unimplemented!()
//     }
// }
//
//
// #[derive(Error, Debug)]
// pub enum IndexerError {
//     #[error("RPC Error: {0}")]
//     RpcError(#[from] solana_client::client_error::ClientError),
//     #[error("failed to deserialize account data")]
//     DeserializeError(#[from] solana_sdk::program_error::ProgramError),
//     #[error("failed to copy merkle tree")]
//     CopyMerkleTreeError(#[from] std::io::Error),
//     #[error(transparent)]
//     AccountCompressionError(#[from] AccountCompressionError),
//     #[error(transparent)]
//     HashSetError(#[from] HashSetError),
//     #[error(transparent)]
//     PhotonApiError(PhotonApiErrorWrapper),
//     #[error("error: {0:?}")]
//     Custom(String),
//     #[error("unknown error")]
//     Unknown,
// }
//
// #[derive(Error, Debug)]
// pub enum PhotonApiErrorWrapper {
//     #[error(transparent)]
//     GetCompressedAccountProofPostError(#[from] PhotonApiError<GetCompressedAccountProofPostError>),
// }
//
// impl From<PhotonApiError<GetCompressedAccountProofPostError>> for IndexerError {
//     fn from(err: PhotonApiError<GetCompressedAccountProofPostError>) -> Self {
//         IndexerError::PhotonApiError(PhotonApiErrorWrapper::GetCompressedAccountProofPostError(
//             err,
//         ))
//     }
// }
