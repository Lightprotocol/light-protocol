use async_trait::async_trait;
use light_merkle_tree_metadata::QueueType;
use light_sdk::token::TokenDataWithMerkleContext;
use photon_api::models::TokenBalanceList;
use solana_pubkey::Pubkey;

use super::types::{Account, ProofRpcResult};
use super::{
    Address, AddressWithTree, BatchAddressUpdateIndexerResponse, Hash, IndexerError, MerkleProof,
    MerkleProofWithContext, NewAddressProofWithContext,
};

#[async_trait]
pub trait Indexer: std::marker::Send + std::marker::Sync {
    async fn get_indexer_slot(&self) -> Result<u64, IndexerError>;

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError>;

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<Account>, IndexerError>;

    async fn get_compressed_token_accounts_by_owner_v2(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError>;

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<Account, IndexerError>;

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError>;

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError>;

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError>;

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
    ) -> Result<Vec<Account>, IndexerError>;

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<TokenBalanceList, IndexerError>;

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
    ) -> Result<Vec<String>, IndexerError>;

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError>;

    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ProofRpcResult, IndexerError>;

    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
    ) -> Result<BatchAddressUpdateIndexerResponse, IndexerError>;

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
    ) -> Result<Vec<MerkleProofWithContext>, IndexerError>;

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, IndexerError>;
}
