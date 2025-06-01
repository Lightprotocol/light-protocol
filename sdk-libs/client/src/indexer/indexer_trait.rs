use std::fmt::Debug;

use async_trait::async_trait;
use light_merkle_tree_metadata::QueueType;
use solana_pubkey::Pubkey;

use super::{
    types::{Account, TokenAccount, TokenBalance, ValidityProofWithContext},
    Address, AddressWithTree, BatchAddressUpdateIndexerResponse, Hash, IndexerError, MerkleProof,
    MerkleProofWithContext, NewAddressProofWithContext,
};

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Context {
    pub slot: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Response<T: Clone + PartialEq + Default + Debug> {
    pub context: Context,
    pub value: T,
}

impl<T: Clone + PartialEq + Default + Debug> Response<T> {
    pub fn value(&self) -> &T {
        &self.value
    }
    pub fn indexer_slot(&self) -> u64 {
        self.context.slot
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResponseWithCursor<
    T: Clone + PartialEq + Default + Debug,
    C: Clone + PartialEq + Default + Debug,
> {
    pub context: Context,
    pub value: T,
    pub cursor: C,
}

impl<T: Clone + PartialEq + Default + Debug, C: Clone + PartialEq + Default + Debug>
    ResponseWithCursor<T, C>
{
    pub fn value(&self) -> &T {
        &self.value
    }
    pub fn indexer_slot(&self) -> u64 {
        self.context.slot
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IndexerRpcConfig {
    pub slot: u64,
    pub retry_config: RetryConfig,
}
impl IndexerRpcConfig {
    pub fn new(slot: u64) -> Self {
        Self {
            slot,
            retry_config: RetryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetryConfig {
    pub num_retries: u32,
    pub delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            num_retries: 10,
            delay_ms: 400,
            max_delay_ms: 8000,
        }
    }
}

#[async_trait]
pub trait Indexer: std::marker::Send + std::marker::Sync {
    // No response type needed
    async fn get_indexer_slot(&self, config: Option<RetryConfig>) -> Result<u64, IndexerError>;

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<MerkleProof>>, IndexerError>;

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<ResponseWithCursor<Vec<Account>, [u8; 32]>, IndexerError>;

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Account>, IndexerError>;

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<ResponseWithCursor<Vec<TokenAccount>, [u8; 32]>, IndexerError>;

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError>;

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError>;

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<Account>>, IndexerError>;

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<ResponseWithCursor<Vec<TokenBalance>, Option<String>>, IndexerError>;

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<String>>, IndexerError>;

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<NewAddressProofWithContext>>, IndexerError>;

    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ValidityProofWithContext>, IndexerError>;

    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<BatchAddressUpdateIndexerResponse>, IndexerError>;

    /// Returns queue elements from the queue with the given merkle tree pubkey. For input
    /// queues account compression program does not store queue elements in the
    /// account data but only emits these in the public transaction event. The
    /// indexer needs the queue elements to create batch update proofs.
    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        queue_type: QueueType,
        num_elements: u16,
        start_offset: Option<u64>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<MerkleProofWithContext>>, IndexerError>;

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Vec<[u8; 32]>>, IndexerError>;
}
