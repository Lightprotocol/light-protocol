use anchor_lang::solana_program::pubkey::Pubkey;
use async_trait::async_trait;
use light_client::indexer::{
    AddressMerkleTreeAccounts, MerkleProof, NewAddressProofWithContext, StateMerkleTreeAccounts,
};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_event::event::PublicTransactionEvent;
use light_sdk::token::TokenDataWithMerkleContext;
use solana_sdk::signature::Keypair;

use super::{address_tree::AddressMerkleTreeBundle, state_tree::StateMerkleTreeBundle};

#[async_trait]
pub trait TestIndexerExtensions {
    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle>;

    fn get_address_merkle_tree(
        &self,
        merkle_tree_pubkey: Pubkey,
    ) -> Option<&AddressMerkleTreeBundle>;

    fn add_compressed_accounts_with_token_data(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    );

    fn account_nullified(&mut self, merkle_tree_pubkey: Pubkey, account_hash: &str);

    fn address_tree_updated(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext,
    );

    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts>;

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle>;

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle>;

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle>;

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithMerkleContext>;

    fn get_group_pda(&self) -> &Pubkey;

    fn add_address_merkle_tree_accounts(
        &mut self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts;

    fn get_compressed_accounts_with_merkle_context_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext>;

    fn add_state_bundle(&mut self, state_bundle: StateMerkleTreeBundle);

    fn add_event_and_compressed_accounts(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    );

    fn get_proof_by_index(&mut self, merkle_tree_pubkey: Pubkey, index: u64) -> MerkleProof;

    #[cfg(feature = "devenv")]
    async fn finalize_batched_address_tree_update(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        account_data: &mut [u8],
    );
}
