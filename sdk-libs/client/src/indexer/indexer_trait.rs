use async_trait::async_trait;
use solana_pubkey::Pubkey;

use super::{
    response::{Items, ItemsWithCursor, Response},
    types::{
        CompressedAccount, CompressedTokenAccount, OwnerBalance, QueueElementsResult,
        SignatureWithMetadata, TokenBalance, ValidityProofWithContext,
    },
    Address, AddressWithTree, BatchAddressUpdateIndexerResponse,
    GetCompressedAccountsByOwnerConfig, GetCompressedTokenAccountsByOwnerOrDelegateOptions, Hash,
    IndexerError, IndexerRpcConfig, MerkleProof, NewAddressProofWithContext, PaginatedOptions,
    RetryConfig,
};
// TODO: remove all references in input types.
#[async_trait]
pub trait Indexer: std::marker::Send + std::marker::Sync {
    /// Returns the compressed account with the given address or hash.
    async fn get_compressed_account(
        &self,
        address: Address,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<CompressedAccount>>, IndexerError>;

    /// Returns the compressed account with the given address or hash.
    async fn get_compressed_account_by_hash(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<CompressedAccount>>, IndexerError>;

    /// Returns the owner’s compressed accounts.
    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedAccountsByOwnerConfig>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedAccount>>, IndexerError>;

    /// Returns the balance for the compressed account with the given address or hash.
    async fn get_compressed_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError>;

    /// Returns the total balance of the owner’s compressed accounts.
    async fn get_compressed_balance_by_owner(
        &self,
        owner: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError>;

    /// Returns the owner balances for a given mint in descending order.
    async fn get_compressed_mint_token_holders(
        &self,
        mint: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<OwnerBalance>>, IndexerError>;

    /// Returns the balance for a given token account.
    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError>;

    /// Returns the compressed token accounts that are partially or fully delegated to the given delegate.
    async fn get_compressed_token_accounts_by_delegate(
        &self,
        delegate: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedTokenAccount>>, IndexerError>;

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedTokenAccount>>, IndexerError>;

    /// Returns the token balances for a given owner.
    async fn get_compressed_token_balances_by_owner_v2(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenBalance>>, IndexerError>;

    /// Returns the token balances for a given owner.
    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<SignatureWithMetadata>>, IndexerError>;

    /// Return the signatures of the transactions that
    /// closed or opened a compressed account with the given address.
    async fn get_compression_signatures_for_address(
        &self,
        address: &[u8; 32],
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError>;

    /// Returns the signatures of the transactions that
    /// have modified an owner’s compressed accounts.
    async fn get_compression_signatures_for_owner(
        &self,
        owner: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError>;

    /// Returns the signatures of the transactions that
    /// have modified an owner’s compressed token accounts.
    async fn get_compression_signatures_for_token_owner(
        &self,
        owner: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError>;

    /// Returns an error if the indexer is stale
    /// by more than a configurable number of blocks.
    /// Otherwise, it returns ok.
    async fn get_indexer_health(&self, config: Option<RetryConfig>) -> Result<bool, IndexerError>;

    /// Returns the slot of the last block indexed by the indexer.
    async fn get_indexer_slot(&self, config: Option<RetryConfig>) -> Result<u64, IndexerError>;

    // /// Returns the signatures of the latest transactions that used the compression program.
    // async fn getLatestCompressionSignatures

    // /// Returns the signatures of the latest transactions that are not voting transactions.
    // getLatestNonVotingSignatures

    /// Returns multiple proofs used by the compression program to verify the accounts’ validity.
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<MerkleProof>>, IndexerError>;

    /// Returns multiple compressed accounts with the given addresses or hashes.
    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<Option<CompressedAccount>>>, IndexerError>;

    /// Returns proofs that the new addresses are not taken already and can be created.
    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<NewAddressProofWithContext>>, IndexerError>;

    /// Returns a single ZK Proof used by the compression program
    /// to verify that the given accounts are valid and that
    /// the new addresses can be created.
    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ValidityProofWithContext>, IndexerError>;

    // TODO: in different pr:
    //      replace zkp_batch_size with PaginatedOptions
    //      - return type should be ItemsWithCursor
    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
        start_offset: Option<u64>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<BatchAddressUpdateIndexerResponse>, IndexerError>;

    // TODO: in different pr:
    //      replace num_elements & start_queue_index with PaginatedOptions
    //      - return type should be ItemsWithCursor
    /// Returns queue elements from the queue with the given merkle tree pubkey.
    /// Can fetch from output queue (append), input queue (nullify), or both atomically.
    /// For input queues account compression program does not store queue elements in the
    /// account data but only emits these in the public transaction event. The
    /// indexer needs the queue elements to create batch update proofs.
    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        output_queue_start_index: Option<u64>,
        output_queue_limit: Option<u16>,
        input_queue_start_index: Option<u64>,
        input_queue_limit: Option<u16>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<QueueElementsResult>, IndexerError>;

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<[u8; 32]>>, IndexerError>;
}
