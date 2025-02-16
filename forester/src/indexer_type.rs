use std::{any::Any, sync::Arc};

use async_trait::async_trait;
use forester_utils::forester_epoch::TreeAccounts;
use light_client::{
    indexer::{
        photon_indexer::PhotonIndexer, Indexer, StateMerkleTreeAccounts, StateMerkleTreeBundle,
    },
    rpc::RpcConnection,
};
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_program_test::indexer::{TestIndexer, TestIndexerExtensions};
use light_sdk::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use tokio::sync::Mutex;
use tracing::info;

use crate::{
    errors::ForesterError,
    rollover::{perform_address_merkle_tree_rollover, perform_state_merkle_tree_rollover_forester},
    ForesterConfig,
};

mod sealed {
    use light_client::rpc::merkle_tree::MerkleTreeExt;

    use super::*;
    pub trait Sealed {}
    impl<R: RpcConnection + MerkleTreeExt> Sealed for TestIndexer<R> {}
    impl<R: RpcConnection> Sealed for PhotonIndexer<R> {}
}

#[async_trait]
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

    async fn finalize_batch_address_tree_update(
        rpc: &mut R,
        indexer: &mut impl Indexer<R>,
        new_merkle_tree_pubkey: Pubkey,
    ) where
        Self: Sized;

    async fn update_test_indexer_after_nullification(
        rpc: &mut R,
        indexer: &mut impl Indexer<R>,
        merkle_tree_pubkey: Pubkey,
        batch_index: usize,
    ) where
        Self: Sized;

    async fn update_test_indexer_after_append(
        rpc: &mut R,
        indexer: &mut impl Indexer<R>,
        merkle_tree_pubkey: Pubkey,
        output_queue: Pubkey,
        num_inserted_zkps: u64,
    ) where
        Self: Sized;
}

#[async_trait]
impl<R: RpcConnection + light_client::rpc::merkle_tree::MerkleTreeExt> IndexerType<R>
    for TestIndexer<R>
{
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
                version: 1,
                output_queue_elements: vec![],
                merkle_tree: Box::new(MerkleTree::<Poseidon>::new(
                    STATE_MERKLE_TREE_HEIGHT,
                    STATE_MERKLE_TREE_CANOPY_DEPTH,
                )),
                input_leaf_indices: vec![],
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

    async fn finalize_batch_address_tree_update(
        rpc: &mut R,
        indexer: &mut impl Indexer<R>,
        merkle_tree_pubkey: Pubkey,
    ) {
        if let Some(test_indexer) = (indexer as &mut dyn Any).downcast_mut::<TestIndexer<R>>() {
            let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
            test_indexer
                .finalize_batched_address_tree_update(
                    merkle_tree_pubkey,
                    account.data.as_mut_slice(),
                )
                .await;
        }
    }

    async fn update_test_indexer_after_nullification(
        rpc: &mut R,
        indexer: &mut impl Indexer<R>,
        merkle_tree_pubkey: Pubkey,
        batch_index: usize,
    ) where
        Self: Sized,
    {
        if let Some(test_indexer) = (indexer as &mut dyn Any).downcast_mut::<TestIndexer<R>>() {
            test_indexer
                .update_test_indexer_after_nullification(rpc, merkle_tree_pubkey, batch_index)
                .await;
        }
    }

    async fn update_test_indexer_after_append(
        rpc: &mut R,
        indexer: &mut impl Indexer<R>,
        merkle_tree_pubkey: Pubkey,
        output_queue: Pubkey,
        num_inserted_zkps: u64,
    ) where
        Self: Sized,
    {
        if let Some(test_indexer) = (indexer as &mut dyn Any).downcast_mut::<TestIndexer<R>>() {
            test_indexer
                .update_test_indexer_after_append(
                    rpc,
                    merkle_tree_pubkey,
                    output_queue,
                    num_inserted_zkps,
                )
                .await;
        }
    }
}

// Implementation for PhotonIndexer - no-op
#[async_trait]
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

    async fn finalize_batch_address_tree_update(
        _rpc: &mut R,
        _indexer: &mut impl Indexer<R>,
        _new_merkle_tree_pubkey: Pubkey,
    ) {
        // No-op for production indexer
    }

    async fn update_test_indexer_after_nullification(
        _rpc: &mut R,
        _indexer: &mut impl Indexer<R>,
        _merkle_tree_pubkey: Pubkey,
        _batch_index: usize,
    ) {
        // No-op for production indexer
    }

    async fn update_test_indexer_after_append(
        _rpc: &mut R,
        _indexer: &mut impl Indexer<R>,
        _merkle_tree_pubkey: Pubkey,
        _output_queue: Pubkey,
        _num_inserted_zkps: u64,
    ) {
        // No-op for production indexer
    }
}

pub async fn rollover_state_merkle_tree<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    config: Arc<ForesterConfig>,
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    tree_accounts: &TreeAccounts,
    epoch: u64,
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
        epoch,
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
    epoch: u64,
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
        epoch,
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

pub async fn finalize_batch_address_tree_update<
    R: RpcConnection,
    I: Indexer<R> + IndexerType<R>,
>(
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    new_merkle_tree_pubkey: Pubkey,
) -> Result<(), ForesterError> {
    I::finalize_batch_address_tree_update(
        &mut *rpc,
        &mut *indexer.lock().await,
        new_merkle_tree_pubkey,
    )
    .await;

    Ok(())
}

pub async fn update_test_indexer_after_nullification<
    R: RpcConnection,
    I: Indexer<R> + IndexerType<R>,
>(
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    batch_index: usize,
) -> Result<(), ForesterError> {
    I::update_test_indexer_after_nullification(
        &mut *rpc,
        &mut *indexer.lock().await,
        merkle_tree_pubkey,
        batch_index,
    )
    .await;

    Ok(())
}

pub async fn update_test_indexer_after_append<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    output_queue: Pubkey,
    num_inserted_zkps: u64,
) -> Result<(), ForesterError> {
    I::update_test_indexer_after_append(
        &mut *rpc,
        &mut *indexer.lock().await,
        merkle_tree_pubkey,
        output_queue,
        num_inserted_zkps,
    )
    .await;

    Ok(())
}
