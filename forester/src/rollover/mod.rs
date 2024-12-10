mod operations;
mod state;

use crate::errors::ForesterError;
use crate::photon_indexer::PhotonIndexer;
use crate::rollover::operations::{
    perform_address_merkle_tree_rollover, perform_state_merkle_tree_rollover_forester,
};
use crate::ForesterConfig;
use account_compression::utils::constants::{
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
};
use forester_utils::forester_epoch::TreeAccounts;
use forester_utils::{StateMerkleTreeAccounts, StateMerkleTreeBundle};
use light_client::indexer::Indexer;
use light_client::rpc::RpcConnection;
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_program_test::indexer::{TestIndexer, TestIndexerExtensions};
pub use operations::{get_tree_fullness, is_tree_ready_for_rollover};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
pub use state::RolloverState;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::log::info;

mod sealed {
    use super::*;
    use crate::photon_indexer::PhotonIndexer;
    use light_program_test::indexer::TestIndexer;
    pub trait Sealed {}
    impl<R: RpcConnection> Sealed for TestIndexer<R> {}
    impl<R: RpcConnection> Sealed for PhotonIndexer<R> {}
}

pub trait IndexerType<R: RpcConnection>: sealed::Sealed {
    fn handle_state_bundle(
        indexer: &mut impl Indexer<R>,
        new_merkle_tree: Pubkey,
        new_queue: Pubkey,
        new_cpi_context: Pubkey,
    ) where
        Self: Sized;

    fn handle_address_bundle(
        indexer: &mut impl Indexer<R>,
        new_merkle_tree: &Keypair,
        new_queue: &Keypair,
    ) where
        Self: Sized;
}

impl<R: RpcConnection> IndexerType<R> for TestIndexer<R> {
    fn handle_state_bundle(
        indexer: &mut impl Indexer<R>,
        new_merkle_tree: Pubkey,
        new_queue: Pubkey,
        new_cpi_context: Pubkey,
    ) {
        if let Some(test_indexer) = (indexer as &mut dyn Any).downcast_mut::<TestIndexer<R>>() {
            let state_bundle = StateMerkleTreeBundle {
                rollover_fee: 0,
                accounts: StateMerkleTreeAccounts {
                    merkle_tree: new_merkle_tree,
                    nullifier_queue: new_queue,
                    cpi_context: new_cpi_context,
                },
                merkle_tree: Box::new(MerkleTree::<Poseidon>::new(
                    STATE_MERKLE_TREE_HEIGHT as usize,
                    STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                )),
            };
            test_indexer.add_state_bundle(state_bundle);
        }
    }

    fn handle_address_bundle(
        indexer: &mut impl Indexer<R>,
        new_merkle_tree: &Keypair,
        new_queue: &Keypair,
    ) {
        if let Some(test_indexer) = (indexer as &mut dyn Any).downcast_mut::<TestIndexer<R>>() {
            test_indexer.add_address_merkle_tree_accounts(new_merkle_tree, new_queue, None);
        }
    }
}

// Implementation for PhotonIndexer - no-op
impl<R: RpcConnection> IndexerType<R> for PhotonIndexer<R> {
    fn handle_state_bundle(
        _indexer: &mut impl Indexer<R>,
        _new_merkle_tree: Pubkey,
        _new_queue: Pubkey,
        _new_cpi_context: Pubkey,
    ) {
        // No-op for production indexer
    }

    fn handle_address_bundle(
        _indexer: &mut impl Indexer<R>,
        _new_merkle_tree: &Keypair,
        _new_queue: &Keypair,
    ) {
        // No-op for production indexer
    }
}

pub async fn rollover_state_merkle_tree<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    config: Arc<ForesterConfig>,
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    tree_accounts: &TreeAccounts,
) -> Result<(), ForesterError> {
    let new_nullifier_queue_keypair = Keypair::new();
    let new_merkle_tree_keypair = Keypair::new();
    let new_cpi_signature_keypair = Keypair::new();

    let rollover_signature = perform_state_merkle_tree_rollover_forester(
        &config.payer_keypair,
        &config.derivation_pubkey,
        rpc,
        &new_nullifier_queue_keypair,
        &new_merkle_tree_keypair,
        &new_cpi_signature_keypair,
        &tree_accounts.merkle_tree,
        &tree_accounts.queue,
        &Pubkey::default(),
    )
    .await?;

    info!("State rollover signature: {:?}", rollover_signature);

    I::handle_state_bundle(
        &mut *indexer.lock().await,
        new_merkle_tree_keypair.pubkey(),
        new_nullifier_queue_keypair.pubkey(),
        new_cpi_signature_keypair.pubkey(),
    );

    Ok(())
}

pub async fn rollover_address_merkle_tree<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    config: Arc<ForesterConfig>,
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    tree_accounts: &TreeAccounts,
) -> Result<(), ForesterError> {
    let new_nullifier_queue_keypair = Keypair::new();
    let new_merkle_tree_keypair = Keypair::new();

    let rollover_signature = perform_address_merkle_tree_rollover(
        &config.payer_keypair,
        &config.derivation_pubkey,
        rpc,
        &new_nullifier_queue_keypair,
        &new_merkle_tree_keypair,
        &tree_accounts.merkle_tree,
        &tree_accounts.queue,
    )
    .await?;

    info!("Address rollover signature: {:?}", rollover_signature);

    I::handle_address_bundle(
        &mut *indexer.lock().await,
        &new_merkle_tree_keypair,
        &new_nullifier_queue_keypair,
    );

    Ok(())
}
