use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use forester_utils::forester_epoch::TreeAccounts;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, Indexer, StateMerkleTreeAccounts},
    rpc::Rpc,
};
use light_compressed_account::TreeType;
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_program_test::indexer::{
    state_tree::StateMerkleTreeBundle, TestIndexer, TestIndexerExtensions,
};
use light_sdk::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT};
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
    use super::*;
    pub trait Sealed {}
    impl Sealed for TestIndexer {}
    impl Sealed for PhotonIndexer {}
}

#[async_trait]
pub trait IndexerType<R: Rpc>: Indexer + sealed::Sealed {
    fn rpc_phantom(&self) -> PhantomData<R> {
        PhantomData
    }
    fn handle_state_bundle(
        &mut self,
        new_merkle_tree: Pubkey,
        new_queue: Pubkey,
        new_cpi_context: Pubkey,
    );

    fn handle_address_bundle(&mut self, new_merkle_tree: &Keypair, new_queue: &Keypair);

    async fn finalize_batch_address_tree_update(
        &mut self,
        rpc: &mut R,
        new_merkle_tree_pubkey: Pubkey,
    );

    async fn update_test_indexer_after_nullification(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        batch_index: usize,
    );

    async fn update_test_indexer_after_append(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        output_queue: Pubkey,
    );
}

#[async_trait]
impl<R: Rpc> IndexerType<R> for TestIndexer {
    fn handle_state_bundle(
        &mut self,
        new_merkle_tree: Pubkey,
        new_queue: Pubkey,
        new_cpi_context: Pubkey,
    ) {
        let state_bundle = StateMerkleTreeBundle {
            rollover_fee: 0,
            accounts: StateMerkleTreeAccounts {
                merkle_tree: new_merkle_tree,
                nullifier_queue: new_queue,
                cpi_context: new_cpi_context,
            },
            tree_type: TreeType::StateV1,
            output_queue_elements: vec![],
            merkle_tree: Box::new(MerkleTree::<Poseidon>::new(
                STATE_MERKLE_TREE_HEIGHT,
                STATE_MERKLE_TREE_CANOPY_DEPTH,
            )),
            input_leaf_indices: vec![],
            num_inserted_batches: 0,
            output_queue_batch_size: None,
        };
        self.add_state_bundle(state_bundle);
    }

    fn handle_address_bundle(&mut self, new_merkle_tree: &Keypair, new_queue: &Keypair) {
        self.add_address_merkle_tree_accounts(new_merkle_tree, new_queue, None);
    }

    async fn finalize_batch_address_tree_update(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
    ) {
        let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        self.finalize_batched_address_tree_update(merkle_tree_pubkey, account.data.as_mut_slice())
            .await;
    }

    async fn update_test_indexer_after_nullification(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        batch_index: usize,
    ) {
        let state_merkle_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        let batch = &merkle_tree.queue_batches.batches[batch_index];
        let batch_size = batch.zkp_batch_size;
        let leaf_indices_tx_hashes =
            state_merkle_tree_bundle.input_leaf_indices[..batch_size as usize].to_vec();
        for leaf_info in leaf_indices_tx_hashes.iter() {
            let index = leaf_info.leaf_index as usize;
            let leaf = leaf_info.leaf;
            let mut index_32_bytes = [0u8; 32];
            index_32_bytes[24..].copy_from_slice(index.to_be_bytes().as_slice());

            let nullifier = Poseidon::hashv(&[&leaf, &index_32_bytes, &leaf_info.tx_hash]).unwrap();

            state_merkle_tree_bundle.input_leaf_indices.remove(0);
            let result = state_merkle_tree_bundle
                .merkle_tree
                .update(&nullifier, index);
            if result.is_err() {
                let num_missing_leaves =
                    (index + 1) - state_merkle_tree_bundle.merkle_tree.rightmost_index;
                state_merkle_tree_bundle
                    .merkle_tree
                    .append_batch(&vec![&[0u8; 32]; num_missing_leaves])
                    .unwrap();
                state_merkle_tree_bundle
                    .merkle_tree
                    .update(&nullifier, index)
                    .unwrap();
            }
        }
    }

    async fn update_test_indexer_after_append(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        output_queue_pubkey: Pubkey,
    ) {
        let state_merkle_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        let (merkle_tree_next_index, root) = {
            let mut merkle_tree_account =
                rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                merkle_tree_account.data.as_mut_slice(),
                &merkle_tree_pubkey.into(),
            )
            .unwrap();
            (
                merkle_tree.next_index as usize,
                *merkle_tree.root_history.last().unwrap(),
            )
        };

        let zkp_batch_size = {
            let mut output_queue_account =
                rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
            let output_queue =
                BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                    .unwrap();

            output_queue.batch_metadata.zkp_batch_size
        };

        let leaves = state_merkle_tree_bundle.output_queue_elements.to_vec();
        let batch_update_leaves = leaves[0..zkp_batch_size as usize].to_vec();

        for (i, (new_leaf, _)) in batch_update_leaves.iter().enumerate() {
            let index = merkle_tree_next_index + i - zkp_batch_size as usize;
            // This is dangerous it should call self.get_leaf_by_index() but it
            // can t for mutable borrow
            // TODO: call a get_leaf_by_index equivalent, we could move the method to the reference merkle tree
            let leaf = state_merkle_tree_bundle
                .merkle_tree
                .get_leaf(index)
                .unwrap_or_default();
            if leaf == [0u8; 32] {
                let result = state_merkle_tree_bundle.merkle_tree.update(new_leaf, index);
                if result.is_err() && state_merkle_tree_bundle.merkle_tree.rightmost_index == index
                {
                    state_merkle_tree_bundle
                        .merkle_tree
                        .append(new_leaf)
                        .unwrap();
                } else {
                    result.unwrap();
                }
            }
        }
        assert_eq!(
            root,
            state_merkle_tree_bundle.merkle_tree.root(),
            "update indexer after append root invalid"
        );

        for _ in 0..zkp_batch_size {
            state_merkle_tree_bundle.output_queue_elements.remove(0);
        }
    }
}

// Implementation for PhotonIndexer - no-op
#[async_trait]
impl<R: Rpc> IndexerType<R> for PhotonIndexer {
    fn handle_state_bundle(
        &mut self,
        _new_merkle_tree: Pubkey,
        _new_queue: Pubkey,
        _new_cpi_context: Pubkey,
    ) {
        // No-op for production indexer
    }

    fn handle_address_bundle(&mut self, _new_merkle_tree: &Keypair, _new_queue: &Keypair) {
        // No-op for production indexer
    }

    async fn finalize_batch_address_tree_update(
        &mut self,
        _rpc: &mut R,
        _new_merkle_tree_pubkey: Pubkey,
    ) {
        // No-op for production indexer
    }

    async fn update_test_indexer_after_nullification(
        &mut self,
        _rpc: &mut R,
        _merkle_tree_pubkey: Pubkey,
        _batch_index: usize,
    ) {
        // No-op for production indexer
    }

    async fn update_test_indexer_after_append(
        &mut self,
        _rpc: &mut R,
        _merkle_tree_pubkey: Pubkey,
        _output_queue: Pubkey,
    ) {
        // No-op for production indexer
    }
}

pub async fn rollover_state_merkle_tree<R: Rpc, I: IndexerType<R>>(
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

    let mut indexer_lock = indexer.lock().await;
    indexer_lock.handle_state_bundle(
        new_merkle_tree_keypair.pubkey(),
        new_nullifier_queue_keypair.pubkey(),
        new_cpi_signature_keypair.pubkey(),
    );

    Ok(())
}

pub async fn rollover_address_merkle_tree<R: Rpc, I: IndexerType<R>>(
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

    let mut indexer_lock = indexer.lock().await;
    indexer_lock.handle_address_bundle(&new_merkle_tree_keypair, &new_nullifier_queue_keypair);

    Ok(())
}

pub async fn finalize_batch_address_tree_update<R: Rpc, I: IndexerType<R>>(
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    new_merkle_tree_pubkey: Pubkey,
) -> Result<(), ForesterError> {
    let mut indexer_lock = indexer.lock().await;
    indexer_lock
        .finalize_batch_address_tree_update(rpc, new_merkle_tree_pubkey)
        .await;

    Ok(())
}

pub async fn update_test_indexer_after_nullification<R: Rpc, I: IndexerType<R>>(
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    batch_index: usize,
) -> Result<(), ForesterError> {
    let mut indexer_lock = indexer.lock().await;
    indexer_lock
        .update_test_indexer_after_nullification(rpc, merkle_tree_pubkey, batch_index)
        .await;

    Ok(())
}

pub async fn update_test_indexer_after_append<R: Rpc, I: IndexerType<R>>(
    rpc: &mut R,
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    output_queue: Pubkey,
) -> Result<(), ForesterError> {
    let mut indexer_lock = indexer.lock().await;
    indexer_lock
        .update_test_indexer_after_append(rpc, merkle_tree_pubkey, output_queue)
        .await;

    Ok(())
}
