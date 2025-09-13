pub use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use tracing::{debug, error, info, trace};

use super::{account_parser, address, context::BatchContext, state, types::BatchReadyState};
use crate::Result;

#[derive(Debug)]
pub struct BatchProcessor<R: Rpc> {
    context: BatchContext<R>,
    tree_type: TreeType,
}

impl<R: Rpc> BatchProcessor<R> {
    pub fn new(context: BatchContext<R>, tree_type: TreeType) -> Self {
        Self { context, tree_type }
    }

    pub async fn process(&self) -> Result<usize> {
        trace!(
            "Starting batch processing for tree type: {:?}",
            self.tree_type
        );
        let state = self.verify_batch_ready().await;

        match state {
            BatchReadyState::AddressReadyForAppend { merkle_tree_data } => {
                trace!(
                    "Processing address append for tree: {}",
                    self.context.merkle_tree
                );

                let batch_hash = format!(
                    "address_batch_{}_{}",
                    self.context.merkle_tree, self.context.epoch
                );
                {
                    let mut cache = self.context.ops_cache.lock().await;
                    if cache.contains(&batch_hash) {
                        debug!("Skipping already processed address batch: {}", batch_hash);
                        return Ok(0);
                    }
                    cache.add(&batch_hash);
                }

                let result = address::process_batch(&self.context, merkle_tree_data).await;

                if let Err(ref e) = result {
                    error!(
                        "Address append failed for tree {}: {:?}",
                        self.context.merkle_tree, e
                    );
                }

                let mut cache = self.context.ops_cache.lock().await;
                cache.cleanup_by_key(&batch_hash);
                trace!("Cache cleaned up for batch: {}", batch_hash);

                result
            }
            BatchReadyState::StateReadyForAppend {
                merkle_tree_data,
                output_queue_data,
            } => {
                trace!(
                    "Process state append for tree: {}",
                    self.context.merkle_tree
                );
                let result = self
                    .process_state_append(merkle_tree_data, output_queue_data)
                    .await;
                if let Err(ref e) = result {
                    error!(
                        "State append failed for tree {}: {:?}",
                        self.context.merkle_tree, e
                    );
                }
                result
            }
            BatchReadyState::StateReadyForNullify { merkle_tree_data } => {
                trace!(
                    "Processing batch for nullify, tree: {}",
                    self.context.merkle_tree
                );
                let result = self.process_state_nullify(merkle_tree_data).await;
                if let Err(ref e) = result {
                    error!(
                        "State nullify failed for tree {}: {:?}",
                        self.context.merkle_tree, e
                    );
                }
                result
            }
            BatchReadyState::BothReady {
                merkle_tree_data,
                output_queue_data,
            } => {
                info!(
                    "Processing both nullify and append in parallel for tree: {}",
                    self.context.merkle_tree
                );
                self.process_state(merkle_tree_data, output_queue_data)
                    .await
            }
            BatchReadyState::NotReady => {
                trace!(
                    "Batch not ready for processing, tree: {}",
                    self.context.merkle_tree
                );
                Ok(0)
            }
        }
    }

    async fn verify_batch_ready(&self) -> BatchReadyState {
        let rpc = match self.context.rpc_pool.get_connection().await {
            Ok(rpc) => rpc,
            Err(_) => return BatchReadyState::NotReady,
        };

        let merkle_tree_account = rpc
            .get_account(self.context.merkle_tree)
            .await
            .ok()
            .flatten();

        let output_queue_account = if self.tree_type == TreeType::StateV2 {
            rpc.get_account(self.context.output_queue)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        account_parser::determine_batch_state(
            self.tree_type,
            self.context.merkle_tree,
            merkle_tree_account,
            output_queue_account,
        )
    }

    async fn process_state_nullify(&self, merkle_tree_data: ParsedMerkleTreeData) -> Result<usize> {
        let batch_hash = format!(
            "state_nullify_hybrid_{}_{}",
            self.context.merkle_tree, self.context.epoch
        );

        {
            let mut cache = self.context.ops_cache.lock().await;
            if cache.contains(&batch_hash) {
                trace!(
                    "Skipping already processed state nullify batch (hybrid): {}",
                    batch_hash
                );
                return Ok(0);
            }
            cache.add(&batch_hash);
        }

        let empty_append_data = ParsedQueueData {
            zkp_batch_size: 0,
            pending_batch_index: 0,
            num_inserted_zkps: 0,
            current_zkp_batch_index: 0,
            leaves_hash_chains: Vec::new(), // No append operations
        };

        let result = self
            .process_state(merkle_tree_data, empty_append_data)
            .await;

        trace!(
            "State nullify operation (hybrid) completed for tree: {}",
            self.context.merkle_tree
        );
        let mut cache = self.context.ops_cache.lock().await;
        cache.cleanup_by_key(&batch_hash);
        trace!("Cache cleaned up for batch: {}", batch_hash);

        result
    }

    async fn process_state_append(
        &self,
        merkle_tree_data: ParsedMerkleTreeData,
        output_queue_data: ParsedQueueData,
    ) -> Result<usize> {
        let batch_hash = format!(
            "state_append_hybrid_{}_{}",
            self.context.merkle_tree, self.context.epoch
        );
        {
            let mut cache = self.context.ops_cache.lock().await;
            if cache.contains(&batch_hash) {
                trace!(
                    "Skipping already processed state append batch (hybrid): {}",
                    batch_hash
                );
                return Ok(0);
            }
            cache.add(&batch_hash);
        }
        let empty_nullify_data = ParsedMerkleTreeData {
            next_index: merkle_tree_data.next_index,
            current_root: merkle_tree_data.current_root,
            root_history: merkle_tree_data.root_history.clone(),
            zkp_batch_size: merkle_tree_data.zkp_batch_size,
            pending_batch_index: merkle_tree_data.pending_batch_index,
            num_inserted_zkps: merkle_tree_data.num_inserted_zkps,
            current_zkp_batch_index: merkle_tree_data.current_zkp_batch_index,
            leaves_hash_chains: Vec::new(), // No nullify operations
        };

        let result = self
            .process_state(empty_nullify_data, output_queue_data)
            .await;

        trace!(
            "State append operation (hybrid) completed for tree: {}",
            self.context.merkle_tree
        );

        let mut cache = self.context.ops_cache.lock().await;
        cache.cleanup_by_key(&batch_hash);

        result
    }

    async fn process_state(
        &self,
        merkle_tree_data: ParsedMerkleTreeData,
        output_queue_data: ParsedQueueData,
    ) -> Result<usize> {
        info!("Processing state operations with hybrid approach: sequential changelogs, parallel proofs");

        let _ = super::changelog_cache::get_changelog_cache().await;

        let (nullify_proofs, append_proofs) =
            state::generate_state_inputs(&self.context, merkle_tree_data, output_queue_data)
                .await?;

        let mut success_count = 0;

        if let Err(e) = state::submit_nullify_transaction(&self.context, nullify_proofs).await {
            error!("Nullify transaction failed: {:?}", e);
            return Err(anyhow::anyhow!(
                "Cannot proceed with append after nullify failure: {:?}",
                e
            ));
        } else {
            success_count += 1;
            debug!("Nullify transaction completed successfully");
        }

        if let Err(e) = state::submit_append_transaction(&self.context, append_proofs).await {
            error!("Append transaction failed: {:?}", e);
        } else {
            success_count += 1;
            debug!("Append transaction completed successfully");
        }

        info!(
            "Processing completed for tree {}, {} operations succeeded",
            self.context.merkle_tree, success_count
        );

        Ok(success_count)
    }
}
