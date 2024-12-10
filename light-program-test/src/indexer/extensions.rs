use account_compression::initialize_address_merkle_tree::Pubkey;
use forester_utils::{
    AddressMerkleTreeAccounts, AddressMerkleTreeBundle, StateMerkleTreeAccounts,
    StateMerkleTreeBundle,
};
use light_client::indexer::{Indexer, NewAddressProofWithContext};
use light_client::rpc::RpcConnection;
use light_sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_sdk::event::PublicTransactionEvent;
use light_sdk::proof::ProofRpcResult;
use light_sdk::token::TokenDataWithMerkleContext;
use solana_sdk::signature::Keypair;

pub trait TestIndexerExtensions<R: RpcConnection>: Indexer<R> {
    fn account_nullified(&mut self, merkle_tree_pubkey: Pubkey, account_hash: &str);

    fn address_tree_updated(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext,
    );

    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts>;

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle>;

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle>;

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle>;

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle>;

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithMerkleContext>;

    fn get_payer(&self) -> &Keypair;

    fn get_group_pda(&self) -> &Pubkey;

    fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> impl std::future::Future<Output = ProofRpcResult>;

    fn add_address_merkle_tree_accounts(
        &mut self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts;

    fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext>;

    fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<TokenDataWithMerkleContext>;

    fn add_state_bundle(&mut self, state_bundle: StateMerkleTreeBundle);

    fn add_event_and_compressed_accounts(
        &mut self,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    );
}
