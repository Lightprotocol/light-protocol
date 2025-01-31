use account_compression::initialize_address_merkle_tree::Pubkey;
use async_trait::async_trait;
use light_client::{
    indexer::{
        AddressMerkleTreeAccounts, AddressMerkleTreeBundle, Indexer, NewAddressProofWithContext,
        ProofOfLeaf, StateMerkleTreeAccounts, StateMerkleTreeBundle,
    },
    rpc::RpcConnection,
};
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext, event::PublicTransactionEvent,
    proof::BatchedTreeProofRpcResult, token::TokenDataWithMerkleContext,
};
use solana_sdk::signature::Keypair;

#[async_trait]
pub trait TestIndexerExtensions<R: RpcConnection>: Indexer<R> {
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
        context: &NewAddressProofWithContext<16>,
    );

    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts>;

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle>;

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle>;

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle>;

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithMerkleContext>;

    fn get_payer(&self) -> &Keypair;

    fn get_group_pda(&self) -> &Pubkey;

    async fn create_proof_for_compressed_accounts2(
        &mut self,
        compressed_accounts: Option<Vec<[u8; 32]>>,
        state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> BatchedTreeProofRpcResult;

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

    fn get_proof_by_index(&mut self, merkle_tree_pubkey: Pubkey, index: u64) -> ProofOfLeaf;

    async fn update_test_indexer_after_append(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        output_queue_pubkey: Pubkey,
        num_inserted_zkps: u64,
    );

    async fn update_test_indexer_after_nullification(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        batch_index: usize,
    );

    async fn finalize_batched_address_tree_update(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        account_data: &mut [u8],
    );
}
