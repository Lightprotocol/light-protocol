use std::fmt::Debug;
use async_trait::async_trait;
use light_concurrent_merkle_tree::light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement},
    reference::IndexedMerkleTree,
};
use light_merkle_tree_reference::MerkleTree;
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext,
    proof::ProofRpcResult
};
use num_bigint::BigUint;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;
use light_sdk::token::TokenData;
use crate::rpc::RpcConnection;

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

    fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, IndexerError>;

    // fn add_event_and_compressed_accounts(
    //     &mut self,
    //     slot: u64,
    //     event: &PublicTransactionEvent,
    // ) -> (
    //     Vec<CompressedAccountWithMerkleContext>,
    //     Vec<TokenDataWithMerkleContext>,
    // );

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<Vec<[u8; 32]>>,
        state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> ProofRpcResult;

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError>;

    async fn get_compressed_accounts_by_owner(
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

    // TODO: remove?
    fn get_proofs_by_indices(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        indices: &[u64],
    ) -> Vec<ProofOfLeaf>;

    // TODO: remove?
    fn get_leaf_indices_tx_hashes(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        zkp_batch_size: usize,
    ) -> Vec<(u32, [u8; 32], [u8; 32])>;

    // TODO: remove?
    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle>;
}

#[derive(Debug, Clone)]
pub struct TokenDataWithMerkleContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
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

// use std::{fmt::Debug, future::Future};
//
// use light_concurrent_merkle_tree::light_hasher::Poseidon;
// use light_indexed_merkle_tree::{
//     array::{IndexedArray, IndexedElement},
//     reference::IndexedMerkleTree,
// };
// use light_merkle_tree_reference::MerkleTree;
// use light_sdk::{
//     compressed_account::CompressedAccountWithMerkleContext, event::PublicTransactionEvent,
//     proof::ProofRpcResult, token::TokenDataWithMerkleContext,
// };
// use num_bigint::BigUint;
// use solana_sdk::pubkey::Pubkey;
// use thiserror::Error;
//
// use crate::rpc::RpcConnection;
//
// #[derive(Error, Debug)]
// pub enum IndexerError {
//     #[error("RPC Error: {0}")]
//     RpcError(#[from] solana_client::client_error::ClientError),
//     #[error("failed to deserialize account data")]
//     DeserializeError(#[from] solana_sdk::program_error::ProgramError),
//     #[error("failed to copy merkle tree")]
//     CopyMerkleTreeError(#[from] std::io::Error),
//     #[error("error: {0:?}")]
//     Custom(String),
//     #[error("unknown error")]
//     Unknown,
// }
//
// pub trait Indexer<R: RpcConnection>: Sync + Send + Debug + 'static {
//     fn add_event_and_compressed_accounts(
//         &mut self,
//         event: &PublicTransactionEvent,
//     ) -> (
//         Vec<CompressedAccountWithMerkleContext>,
//         Vec<TokenDataWithMerkleContext>,
//     );
//
//     fn create_proof_for_compressed_accounts(
//         &mut self,
//         compressed_accounts: Option<&[[u8; 32]]>,
//         state_merkle_tree_pubkeys: Option<&[Pubkey]>,
//         new_addresses: Option<&[[u8; 32]]>,
//         address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
//         rpc: &mut R,
//     ) -> impl Future<Output = ProofRpcResult>;
//
//     fn get_compressed_accounts_by_owner(
//         &self,
//         owner: &Pubkey,
//     ) -> Vec<CompressedAccountWithMerkleContext>;
// }
//
// #[derive(Debug, Clone)]
// pub struct MerkleProof {
//     pub hash: String,
//     pub leaf_index: u64,
//     pub merkle_tree: String,
//     pub proof: Vec<[u8; 32]>,
//     pub root_seq: u64,
// }
//
// // For consistency with the Photon API.
// #[derive(Clone, Default, Debug, PartialEq)]
// pub struct NewAddressProofWithContext {
//     pub merkle_tree: [u8; 32],
//     pub root: [u8; 32],
//     pub root_seq: u64,
//     pub low_address_index: u64,
//     pub low_address_value: [u8; 32],
//     pub low_address_next_index: u64,
//     pub low_address_next_value: [u8; 32],
//     pub low_address_proof: [[u8; 32]; 16],
//     pub new_low_element: Option<IndexedElement<usize>>,
//     pub new_element: Option<IndexedElement<usize>>,
//     pub new_element_next_value: Option<BigUint>,
// }
//
// #[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
// pub struct StateMerkleTreeAccounts {
//     pub merkle_tree: Pubkey,
//     pub nullifier_queue: Pubkey,
//     pub cpi_context: Pubkey,
// }
//
// #[derive(Debug, Clone, Copy)]
// pub struct AddressMerkleTreeAccounts {
//     pub merkle_tree: Pubkey,
//     pub queue: Pubkey,
// }
//
// #[derive(Debug, Clone)]
// pub struct StateMerkleTreeBundle {
//     pub rollover_fee: u64,
//     pub merkle_tree: Box<MerkleTree<Poseidon>>,
//     pub accounts: StateMerkleTreeAccounts,
// }
//
// #[derive(Debug, Clone)]
// pub struct AddressMerkleTreeBundle {
//     pub rollover_fee: u64,
//     pub merkle_tree: Box<IndexedMerkleTree<Poseidon, usize>>,
//     pub indexed_array: Box<IndexedArray<Poseidon, usize>>,
//     pub accounts: AddressMerkleTreeAccounts,
// }
