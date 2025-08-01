#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::processor::v2::tree_cache::TREE_CACHE;
    use light_hasher::Poseidon;
    use light_sparse_merkle_tree::SparseMerkleTree;
    use solana_sdk::pubkey::Pubkey;
    
    #[tokio::test]
    async fn test_tree_cache_basic() {
        // Create a test tree
        let tree = SparseMerkleTree::<Poseidon, 32>::new_empty();
        let pubkey = Pubkey::new_unique();
        
        // Cache the tree
        TREE_CACHE.update(pubkey, &tree).await.unwrap();
        
        // Retrieve from cache
        let snapshot = TREE_CACHE.get(&pubkey).await.unwrap();
        assert_eq!(snapshot.root, tree.root());
        assert_eq!(snapshot.next_index, tree.get_next_index());
        assert_eq!(snapshot.height, 32);
    }
    
    #[tokio::test]
    async fn test_tree_cache_update() {
        let pubkey = Pubkey::new_unique();
        
        // Create and cache initial tree
        let mut tree = SparseMerkleTree::<Poseidon, 32>::new_empty();
        TREE_CACHE.update(pubkey, &tree).await.unwrap();
        
        let initial_snapshot = TREE_CACHE.get(&pubkey).await.unwrap();
        let initial_seq = initial_snapshot.sequence_number;
        
        // Append some leaves
        tree.append([1u8; 32]);
        tree.append([2u8; 32]);
        
        // Update cache
        TREE_CACHE.update(pubkey, &tree).await.unwrap();
        
        // Verify updates
        let updated_snapshot = TREE_CACHE.get(&pubkey).await.unwrap();
        assert_ne!(updated_snapshot.root, initial_snapshot.root);
        assert_eq!(updated_snapshot.next_index, 2);
        assert!(updated_snapshot.sequence_number > initial_seq);
    }
    
    #[tokio::test]
    async fn test_tree_snapshot_to_tree() {
        let pubkey = Pubkey::new_unique();
        
        // Create tree with some data
        let mut original_tree = SparseMerkleTree::<Poseidon, 32>::new_empty();
        original_tree.append([10u8; 32]);
        original_tree.append([20u8; 32]);
        original_tree.append([30u8; 32]);
        
        // Cache it
        TREE_CACHE.update(pubkey, &original_tree).await.unwrap();
        
        // Get snapshot and recreate tree
        let snapshot = TREE_CACHE.get(&pubkey).await.unwrap();
        let recreated_tree = snapshot.to_tree::<Poseidon, 32>().unwrap();
        
        // Verify they match
        assert_eq!(recreated_tree.root(), original_tree.root());
        assert_eq!(recreated_tree.get_next_index(), original_tree.get_next_index());
        assert_eq!(recreated_tree.get_subtrees(), original_tree.get_subtrees());
    }
    
    #[tokio::test]
    async fn test_parallel_tree_operations() {
        use tokio::sync::Arc;
        
        let pubkey = Pubkey::new_unique();
        let tree = SparseMerkleTree::<Poseidon, 32>::new_empty();
        
        // Cache the tree
        TREE_CACHE.update(pubkey, &tree).await.unwrap();
        
        // Simulate parallel operations
        let cache = Arc::new(TREE_CACHE.clone());
        let pk1 = pubkey.clone();
        let pk2 = pubkey.clone();
        
        let handle1 = tokio::spawn(async move {
            // Operation 1: Read tree state
            let snapshot = TREE_CACHE.get(&pk1).await.unwrap();
            let tree1 = snapshot.to_tree::<Poseidon, 32>().unwrap();
            assert_eq!(tree1.get_next_index(), 0);
        });
        
        let handle2 = tokio::spawn(async move {
            // Operation 2: Also read tree state
            let snapshot = TREE_CACHE.get(&pk2).await.unwrap();
            let tree2 = snapshot.to_tree::<Poseidon, 32>().unwrap();
            assert_eq!(tree2.get_next_index(), 0);
        });
        
        // Both should succeed without conflicts
        handle1.await.unwrap();
        handle2.await.unwrap();
    }
}

#[cfg(test)]
mod shared_tree_tests {
    use super::super::*;
    use crate::processor::v2::{
        shared_tree_proof_generator::SharedTreeProofGenerator,
        tree_cache::TREE_CACHE,
    };
    use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
    use light_hasher::Poseidon;
    use solana_sdk::pubkey::Pubkey;
    
    fn create_test_merkle_data() -> ParsedMerkleTreeData {
        ParsedMerkleTreeData {
            next_index: 0,
            current_root: [0u8; 32],
            root_history: vec![[0u8; 32]],
            zkp_batch_size: 10,
            pending_batch_index: 0,
            num_inserted_zkps: 0,
            current_zkp_batch_index: 0,
            leaves_hash_chains: vec![],
        }
    }
    
    fn create_test_queue_data() -> ParsedQueueData {
        ParsedQueueData {
            zkp_batch_size: 10,
            pending_batch_index: 0,
            num_inserted_zkps: 0,
            current_zkp_batch_index: 2,
            leaves_hash_chains: vec![[1u8; 32], [2u8; 32]], // 2 leaves to append
        }
    }
    
    #[tokio::test]
    async fn test_cache_invalidation() {
        let pubkey = Pubkey::new_unique();
        let tree = light_sparse_merkle_tree::SparseMerkleTree::<Poseidon, 32>::new_empty();
        
        // Cache the tree
        TREE_CACHE.update(pubkey, &tree).await.unwrap();
        assert!(TREE_CACHE.get(&pubkey).await.is_some());
        
        // Invalidate
        TREE_CACHE.invalidate(&pubkey).await;
        assert!(TREE_CACHE.get(&pubkey).await.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        // Add multiple trees
        for i in 0..5 {
            let pubkey = Pubkey::new_unique();
            let tree = light_sparse_merkle_tree::SparseMerkleTree::<Poseidon, 32>::new_empty();
            TREE_CACHE.update(pubkey, &tree).await.unwrap();
        }
        
        // Clear all
        TREE_CACHE.clear().await;
        
        // Verify all are gone
        // (We can't easily check this without exposing internal state,
        // but at least verify clear doesn't panic)
    }
}