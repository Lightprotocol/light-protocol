use async_trait::async_trait;
use solana_pubkey::Pubkey;

use super::LightClient;
use crate::indexer::{
    Address, AddressWithTree, BatchAddressUpdateIndexerResponse, CompressedAccount,
    CompressedTokenAccount, GetCompressedAccountsByOwnerConfig,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions, Hash, Indexer, IndexerError,
    IndexerRpcConfig, Items, ItemsWithCursor, MerkleProof, NewAddressProofWithContext,
    OwnerBalance, PaginatedOptions, Response, RetryConfig, SignatureWithMetadata, TokenBalance,
    ValidityProofWithContext,
};

#[async_trait]
impl Indexer for LightClient {
    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ValidityProofWithContext>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_validity_proof(hashes, new_addresses_with_trees, config)
            .await?)
    }

    async fn get_indexer_slot(&self, config: Option<RetryConfig>) -> Result<u64, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_indexer_slot(config)
            .await?)
    }

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<MerkleProof>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_compressed_account_proofs(hashes, config)
            .await?)
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedAccountsByOwnerConfig>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedAccount>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_accounts_by_owner(owner, options, config)
            .await?)
    }

    async fn get_compressed_account(
        &self,
        address: Address,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<CompressedAccount>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_account(address, config)
            .await?)
    }

    async fn get_compressed_account_by_hash(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Option<CompressedAccount>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_account_by_hash(hash, config)
            .await?)
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedTokenAccount>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_accounts_by_owner(owner, options, config)
            .await?)
    }

    async fn get_compressed_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_balance(address, hash, config)
            .await?)
    }

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_account_balance(address, hash, config)
            .await?)
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<Option<CompressedAccount>>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_compressed_accounts(addresses, hashes, config)
            .await?)
    }

    async fn get_compressed_token_balances_by_owner_v2(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenBalance>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_balances_by_owner_v2(owner, options, config)
            .await?)
    }

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<SignatureWithMetadata>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compression_signatures_for_account(hash, config)
            .await?)
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<NewAddressProofWithContext>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_new_address_proofs(merkle_tree_pubkey, addresses, config)
            .await?)
    }

    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
        start_offset: Option<u64>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<BatchAddressUpdateIndexerResponse>, IndexerError> {
        Ok(self
            .indexer
            .as_mut()
            .ok_or(IndexerError::NotInitialized)?
            .get_address_queue_with_proofs(merkle_tree_pubkey, zkp_batch_size, start_offset, config)
            .await?)
    }

    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        output_queue_start_index: Option<u64>,
        output_queue_limit: Option<u16>,
        input_queue_start_index: Option<u64>,
        input_queue_limit: Option<u16>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<crate::indexer::QueueElementsResult>, IndexerError> {
        Ok(self
            .indexer
            .as_mut()
            .ok_or(IndexerError::NotInitialized)?
            .get_queue_elements(
                merkle_tree_pubkey,
                output_queue_start_index,
                output_queue_limit,
                input_queue_start_index,
                input_queue_limit,
                config,
            )
            .await?)
    }

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<[u8; 32]>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_subtrees(merkle_tree_pubkey, config)
            .await?)
    }

    async fn get_compressed_balance_by_owner(
        &self,
        owner: &Pubkey,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_balance_by_owner(owner, config)
            .await?)
    }

    async fn get_compressed_mint_token_holders(
        &self,
        mint: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<OwnerBalance>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_mint_token_holders(mint, options, config)
            .await?)
    }

    async fn get_compressed_token_accounts_by_delegate(
        &self,
        delegate: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<CompressedTokenAccount>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_accounts_by_delegate(delegate, options, config)
            .await?)
    }

    async fn get_compression_signatures_for_address(
        &self,
        address: &[u8; 32],
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compression_signatures_for_address(address, options, config)
            .await?)
    }

    async fn get_compression_signatures_for_owner(
        &self,
        owner: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compression_signatures_for_owner(owner, options, config)
            .await?)
    }

    async fn get_compression_signatures_for_token_owner(
        &self,
        owner: &Pubkey,
        options: Option<PaginatedOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compression_signatures_for_token_owner(owner, options, config)
            .await?)
    }

    async fn get_indexer_health(&self, config: Option<RetryConfig>) -> Result<bool, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_indexer_health(config)
            .await?)
    }
}
