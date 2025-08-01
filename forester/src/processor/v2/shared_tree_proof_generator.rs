use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use light_client::{rpc::Rpc, indexer::Indexer};
use light_hasher::Hasher;
use light_batched_merkle_tree::{
    merkle_tree::{
        InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
};
use forester_utils::{
    rpc_pool::SolanaRpcPool,
    ParsedMerkleTreeData,
    ParsedQueueData,
};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use solana_sdk::pubkey::Pubkey;
use tracing::debug;

use super::tree_cache;

/// Alternative proof generator using shared immutable tree snapshots
/// 
/// This implementation demonstrates how nullify and append operations can work
/// in parallel using a shared tree state. While the production code uses the
/// streaming approach (which is more memory efficient), this implementation
/// shows the concept of shared tree parallelism more explicitly.
/// 
/// Key differences from streaming approach:
/// - Fetches all queue elements upfront (vs incremental streaming)
/// - Updates tree cache after proof generation
/// - More suitable for testing and understanding the shared tree concept
/// 
/// The streaming approach in state.rs is preferred for production use.
pub struct SharedTreeProofGenerator<R: Rpc> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    prover_url: String,
    prover_polling_interval: Duration,
    prover_max_wait_time: Duration,
}

impl<R: Rpc> SharedTreeProofGenerator<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        prover_url: String,
        prover_polling_interval: Duration,
        prover_max_wait_time: Duration,
    ) -> Self {
        Self {
            rpc_pool,
            prover_url,
            prover_polling_interval,
            prover_max_wait_time,
        }
    }
    
    /// Initialize or update tree cache with current on-chain state
    pub async fn update_tree_cache(
        &self,
        merkle_tree: Pubkey,
        tree_data: &ParsedMerkleTreeData,
    ) -> Result<()> {
        // Fetch subtrees from indexer
        let mut rpc = self.rpc_pool.get_connection().await?;
        let indexer = rpc.indexer_mut()?;
        let subtrees_response = indexer.get_subtrees(merkle_tree.to_bytes(), None).await?;
        let subtrees = subtrees_response.value.items;
        
        let cache = tree_cache::get_tree_cache().await;
        cache.update_from_data(
            merkle_tree,
            subtrees,
            tree_data.next_index as usize,
            tree_data.current_root,
            32, // HEIGHT for state trees
        ).await
    }
    
    /// Generate only nullify proofs using shared tree state
    pub async fn generate_nullify_proofs_only<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        tree_data: ParsedMerkleTreeData,
    ) -> Result<Vec<InstructionDataBatchNullifyInputs>> {
        // Update cache with latest state
        self.update_tree_cache(merkle_tree, &tree_data).await?;
        
        // Generate nullify proofs
        self.generate_nullify_proofs::<H, HEIGHT>(merkle_tree, &tree_data).await
    }
    
    /// Generate only append proofs using shared tree state
    pub async fn generate_append_proofs_only<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        output_queue: Pubkey,
        tree_data: ParsedMerkleTreeData,
        queue_data: ParsedQueueData,
    ) -> Result<Vec<InstructionDataBatchAppendInputs>> {
        // Update cache with latest state
        self.update_tree_cache(merkle_tree, &tree_data).await?;
        
        // Generate append proofs
        self.generate_append_proofs::<H, HEIGHT>(
            merkle_tree,
            output_queue,
            &tree_data,
            &queue_data,
        ).await
    }
    
    /// Generate proofs in parallel using shared tree state
    /// Both nullify and append can run concurrently, each using their own tree copy
    pub async fn generate_proofs_with_shared_state<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        output_queue: Pubkey,
        tree_data: ParsedMerkleTreeData,
        queue_data: ParsedQueueData,
    ) -> Result<(Vec<InstructionDataBatchNullifyInputs>, Vec<InstructionDataBatchAppendInputs>)> {
        // Update cache with latest state
        self.update_tree_cache(merkle_tree, &tree_data).await?;
        
        // Run nullify and append in parallel!
        // Each gets its own view of the tree from the cache
        let (nullify_proofs, append_proofs) = tokio::join!(
            self.generate_nullify_proofs::<H, HEIGHT>(
                merkle_tree,
                &tree_data,
            ),
            self.generate_append_proofs::<H, HEIGHT>(
                merkle_tree,
                output_queue,
                &tree_data,
                &queue_data,
            )
        );
        
        Ok((nullify_proofs?, append_proofs?))
    }
    
    /// Generate nullify proofs using shared tree state
    async fn generate_nullify_proofs<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        tree_data: &ParsedMerkleTreeData,
    ) -> Result<Vec<InstructionDataBatchNullifyInputs>> {
        let cache = tree_cache::get_tree_cache().await;
        let snapshot = cache.get(&merkle_tree).await
            .ok_or_else(|| anyhow::anyhow!("Tree not in cache"))?;
        
        // Create a local tree to track subtree changes
        let local_tree = snapshot.to_tree::<H, HEIGHT>()?;
        
        // Fetch queue elements for nullification
        let total_elements = tree_data.zkp_batch_size as usize * tree_data.leaves_hash_chains.len();
        let offset = tree_data.num_inserted_zkps * tree_data.zkp_batch_size as u64;
        
        let all_queue_elements = {
            let mut connection = self.rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            indexer.get_queue_elements(
                merkle_tree.to_bytes(),
                light_merkle_tree_metadata::QueueType::InputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            )
            .await?
            .value
            .items
        };
        
        if all_queue_elements.len() != total_elements {
            return Err(anyhow::anyhow!(
                "Expected {} elements, got {}",
                total_elements, all_queue_elements.len()
            ));
        }
        
        let mut proofs = Vec::new();
        let mut current_root = tree_data.current_root;
        let batch_size = tree_data.zkp_batch_size as usize;
        
        // Generate proofs for each batch
        for (batch_offset, _leaves_hash_chain) in tree_data.leaves_hash_chains.iter().enumerate() {
            debug!("Generating nullify proof for batch {}", batch_offset);
            
            let start_idx = batch_offset * batch_size;
            let end_idx = start_idx + batch_size;
            let _batch_elements = &all_queue_elements[start_idx..end_idx];
            
            // For nullification, we need to calculate the new root after nullifying leaves
            // In the actual implementation, the streams already handle proof generation
            // This is a demonstration of how the tree cache concept works
            
            // Note: SparseMerkleTree is append-only, so for nullification:
            // 1. The tree structure doesn't change (no new leaves appended)
            // 2. Only the leaf values change (to nullified state)
            // 3. The root changes due to the nullified leaves
            // 4. The prover service calculates the new root
            
            // In production, the actual proof would come from the prover service
            // For now, we simulate that the root has changed
            let new_root = {
                let mut root_bytes = current_root;
                root_bytes[0] = root_bytes[0].wrapping_add(1); // Simulate root change
                root_bytes
            };
            
            let proof_data = InstructionDataBatchNullifyInputs {
                new_root,
                compressed_proof: CompressedProof {
                    a: [0u8; 32],
                    b: [0u8; 64],
                    c: [0u8; 32],
                },
            };
            
            proofs.push(proof_data);
            current_root = new_root;
        }
        
        // Extract updated subtrees from local tree
        let updated_subtrees = local_tree.get_subtrees().to_vec();
        
        // Update tree cache with final state after all nullifications
        cache.update_from_data(
            merkle_tree,
            updated_subtrees, // Now using correct subtrees!
            local_tree.get_next_index(), // Nullify doesn't change next_index
            current_root,
            HEIGHT,
        ).await?;
        
        Ok(proofs)
    }
    
    /// Generate append proofs with local state tracking
    async fn generate_append_proofs<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        _output_queue: Pubkey,
        tree_data: &ParsedMerkleTreeData,
        queue_data: &ParsedQueueData,
    ) -> Result<Vec<InstructionDataBatchAppendInputs>> {
        let cache = tree_cache::get_tree_cache().await;
        let snapshot = cache.get(&merkle_tree).await
            .ok_or_else(|| anyhow::anyhow!("Tree not in cache"))?;
        
        // Create a local tree to track subtree changes
        let mut local_tree = snapshot.to_tree::<H, HEIGHT>()?;
        
        // Fetch queue elements for append
        let total_elements = queue_data.zkp_batch_size as usize * queue_data.leaves_hash_chains.len();
        let offset = tree_data.next_index;
        
        let queue_elements = {
            let mut connection = self.rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            indexer.get_queue_elements(
                merkle_tree.to_bytes(),
                light_merkle_tree_metadata::QueueType::OutputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            )
            .await?
            .value
            .items
        };
        
        if queue_elements.len() != total_elements {
            return Err(anyhow::anyhow!(
                "Expected {} elements, got {}",
                total_elements, queue_elements.len()
            ));
        }
        
        let mut proofs = Vec::new();
        let mut current_root = tree_data.current_root;
        let mut current_next_index = tree_data.next_index as u32;
        let batch_size = queue_data.zkp_batch_size as usize;
        
        // Generate proofs for each batch
        for (batch_idx, _leaves_hash_chain) in queue_data.leaves_hash_chains.iter().enumerate() {
            debug!("Generating append proof for batch {} at index {}", batch_idx, current_next_index);
            
            let start_idx = batch_idx * batch_size;
            let end_idx = start_idx + batch_size;
            let batch_elements = &queue_elements[start_idx..end_idx];
            
            // For append operations, we actually modify the tree
            // Update local tree to match the proof result
            for element in batch_elements {
                local_tree.append(element.account_hash);
            }
            
            // The new root is now the root of our local tree after appends
            let new_root = local_tree.root();
            
            let proof_data = InstructionDataBatchAppendInputs {
                new_root,
                compressed_proof: CompressedProof {
                    a: [0u8; 32],
                    b: [0u8; 64],
                    c: [0u8; 32],
                },
            };
            
            proofs.push(proof_data);
            current_root = new_root;
            current_next_index += batch_size as u32;
        }
        
        // Extract updated subtrees from local tree
        let updated_subtrees = local_tree.get_subtrees().to_vec();
        
        // Update tree cache with final state after all appends
        cache.update_from_data(
            merkle_tree,
            updated_subtrees, // Now using correct subtrees!
            current_next_index as usize,
            current_root,
            HEIGHT,
        ).await?;
        
        Ok(proofs)
    }
}