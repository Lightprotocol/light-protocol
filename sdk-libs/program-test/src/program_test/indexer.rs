use async_trait::async_trait;
use light_client::indexer::{
    Address, AddressWithTree, BatchAddressUpdateIndexerResponse, Hash, Indexer, IndexerError,
    MerkleProof, MerkleProofWithContext, NewAddressProofWithContext, ProofRpcResult,
    ProofRpcResultV2,
};
use light_compressed_account::{compressed_account::CompressedAccountWithMerkleContext, QueueType};
use light_sdk::token::TokenDataWithMerkleContext;
use photon_api::models::{Account, TokenBalanceList};
use solana_sdk::pubkey::Pubkey;

use crate::program_test::LightProgramTest;

#[async_trait]
impl Indexer for LightProgramTest {
    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ProofRpcResult, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_validity_proof(hashes, new_addresses_with_trees)
            .await?)
    }

    async fn get_validity_proof_v2(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ProofRpcResultV2, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_validity_proof_v2(hashes, new_addresses_with_trees)
            .await?)
    }

    async fn get_indexer_slot(&self) -> Result<u64, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_indexer_slot()
            .await?)
    }

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_compressed_account_proofs(hashes)
            .await?)
    }
    // TODO: implement get_compressed_accounts_by_owner
    async fn get_compressed_accounts_by_owner_v2(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<CompressedAccountWithMerkleContext>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_accounts_by_owner_v2(owner)
            .await?)
    }

    async fn get_compressed_token_accounts_by_owner_v2(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_accounts_by_owner_v2(owner, mint)
            .await?)
    }

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<Account, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_account(address, hash)
            .await?)
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_accounts_by_owner(owner, mint)
            .await?)
    }

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_account_balance(address, hash)
            .await?)
    }

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_account_balance(address, hash)
            .await?)
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
    ) -> Result<Vec<Account>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_compressed_accounts(addresses, hashes)
            .await?)
    }

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<TokenBalanceList, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compressed_token_balances_by_owner(owner, mint)
            .await?)
    }

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
    ) -> Result<Vec<String>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_compression_signatures_for_account(hash)
            .await?)
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_new_address_proofs(merkle_tree_pubkey, addresses)
            .await?)
    }

    async fn get_multiple_new_address_proofs_h40(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_multiple_new_address_proofs_h40(merkle_tree_pubkey, addresses)
            .await?)
    }

    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
    ) -> Result<BatchAddressUpdateIndexerResponse, IndexerError> {
        Ok(self
            .indexer
            .as_mut()
            .ok_or(IndexerError::NotInitialized)?
            .get_address_queue_with_proofs(merkle_tree_pubkey, zkp_batch_size)
            .await?)
    }

    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        queue_type: QueueType,
        num_elements: u16,
        start_offset: Option<u64>,
    ) -> Result<Vec<MerkleProofWithContext>, IndexerError> {
        Ok(self
            .indexer
            .as_mut()
            .ok_or(IndexerError::NotInitialized)?
            .get_queue_elements(merkle_tree_pubkey, queue_type, num_elements, start_offset)
            .await?)
    }

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        Ok(self
            .indexer
            .as_ref()
            .ok_or(IndexerError::NotInitialized)?
            .get_subtrees(merkle_tree_pubkey)
            .await?)
    }
}
