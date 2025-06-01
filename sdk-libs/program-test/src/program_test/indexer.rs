use async_trait::async_trait;
use light_client::indexer::{
    Account, Address, AddressWithTree, BatchAddressUpdateIndexerResponse,
    GetCompressedAccountsByOwnerConfig, GetCompressedTokenAccountsByOwnerOrDelegateOptions, Hash,
    Indexer, IndexerError, IndexerRpcConfig, Items, ItemsWithCursor, MerkleProof,
    MerkleProofWithContext, NewAddressProofWithContext, OwnerBalance, PaginatedOptions, Response,
    RetryConfig, SignatureWithMetadata, TokenAccount, TokenBalance, ValidityProofWithContext,
};
use light_compressed_account::QueueType;
use solana_sdk::pubkey::Pubkey;

use crate::program_test::LightProgramTest;

#[async_trait]
impl Indexer for LightProgramTest {
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
    ) -> Result<Response<ItemsWithCursor<Account>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_accounts_by_owner(owner, options, config)
            .await?)
    }

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Account>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_account(address, hash, config)
            .await?)
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenAccount>>, IndexerError> {
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
        let indexer = self.indexer.as_ref().ok_or(IndexerError::NotInitialized)?;
        Ok(Indexer::get_compressed_balance(indexer, address, hash, config).await?)
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
    ) -> Result<Response<Items<Account>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_compressed_accounts(addresses, hashes, config)
            .await?)
    }

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenBalance>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_balances_by_owner(owner, options, config)
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
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<BatchAddressUpdateIndexerResponse>, IndexerError> {
        Ok(self
            .indexer
            .as_mut()
            .ok_or(IndexerError::NotInitialized)?
            .get_address_queue_with_proofs(merkle_tree_pubkey, zkp_batch_size, config)
            .await?)
    }

    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        queue_type: QueueType,
        num_elements: u16,
        start_offset: Option<u64>,
        config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<MerkleProofWithContext>>, IndexerError> {
        Ok(self
            .indexer
            .as_mut()
            .ok_or(IndexerError::NotInitialized)?
            .get_queue_elements(
                merkle_tree_pubkey,
                queue_type,
                num_elements,
                start_offset,
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

    // New required trait methods
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
    ) -> Result<Response<ItemsWithCursor<TokenAccount>>, IndexerError> {
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
