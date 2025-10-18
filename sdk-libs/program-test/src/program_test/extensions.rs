use anchor_lang::solana_program::pubkey::Pubkey;
use async_trait::async_trait;
use light_client::indexer::{
    AddressMerkleTreeAccounts, MerkleProof, NewAddressProofWithContext, StateMerkleTreeAccounts,
};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_event::event::PublicTransactionEvent;
use light_sdk::token::TokenDataWithMerkleContext;
use solana_sdk::signature::Keypair;

use crate::{
    indexer::{
        address_tree::AddressMerkleTreeBundle, state_tree::StateMerkleTreeBundle,
        TestIndexerExtensions,
    },
    program_test::LightProgramTest,
};

#[async_trait]
impl TestIndexerExtensions for LightProgramTest {
    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        self.indexer()
            .expect("Indexer not initialized")
            .get_address_merkle_trees()
    }

    fn get_address_merkle_tree(
        &self,
        merkle_tree_pubkey: Pubkey,
    ) -> Option<&AddressMerkleTreeBundle> {
        self.indexer()
            .expect("Indexer not initialized")
            .get_address_merkle_tree(merkle_tree_pubkey)
    }

    fn add_compressed_accounts_with_token_data(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .add_compressed_accounts_with_token_data(slot, event)
    }

    fn account_nullified(&mut self, merkle_tree_pubkey: Pubkey, account_hash: &str) {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .account_nullified(merkle_tree_pubkey, account_hash)
    }

    fn address_tree_updated(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext,
    ) {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .address_tree_updated(merkle_tree_pubkey, context)
    }

    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts> {
        self.indexer()
            .expect("Indexer not initialized")
            .get_state_merkle_tree_accounts(pubkeys)
    }

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle> {
        self.indexer()
            .expect("Indexer not initialized")
            .get_state_merkle_trees()
    }

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle> {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .get_state_merkle_trees_mut()
    }

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle> {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .get_address_merkle_trees_mut()
    }

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithMerkleContext> {
        self.indexer()
            .expect("Indexer not initialized")
            .get_token_compressed_accounts()
    }

    fn get_group_pda(&self) -> &Pubkey {
        self.indexer()
            .expect("Indexer not initialized")
            .get_group_pda()
    }

    fn add_address_merkle_tree_accounts(
        &mut self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .add_address_merkle_tree_accounts(merkle_tree_keypair, queue_keypair, owning_program_id)
    }

    fn get_compressed_accounts_with_merkle_context_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.indexer()
            .expect("Indexer not initialized")
            .get_compressed_accounts_with_merkle_context_by_owner(owner)
    }

    fn add_state_bundle(&mut self, state_bundle: StateMerkleTreeBundle) {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .add_state_bundle(state_bundle)
    }

    fn add_event_and_compressed_accounts(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    ) {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .add_event_and_compressed_accounts(slot, event)
    }

    fn get_proof_by_index(&mut self, merkle_tree_pubkey: Pubkey, index: u64) -> MerkleProof {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .get_proof_by_index(merkle_tree_pubkey, index)
    }

    async fn finalize_batched_address_tree_update(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        account_data: &mut [u8],
    ) {
        self.indexer_mut()
            .expect("Indexer not initialized")
            .finalize_batched_address_tree_update(merkle_tree_pubkey, account_data)
            .await
    }
}
