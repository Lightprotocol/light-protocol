#![cfg(feature = "test-only")]

// Import existing test utils
use light_batched_merkle_tree::{
    changelog::{BatchChangelog, ChangelogInstructionData, is_applicable_entry},
};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use serial_test::serial;

// Mock BatchedMerkleTreeAccount
#[derive(Debug)]
struct MockMerkleTreeAccount {
    root_history: Vec<[u8; 32]>,
    hash_chain_stores: Vec<Vec<[u8; 32]>>,
    sequence_number: u64,
    changelog_entries: Vec<BatchChangelog>,
}

impl MockMerkleTreeAccount {
    fn new() -> Self {
        let initial_root = [0u8; 32];
        let mut root_history = Vec::new();
        root_history.push(initial_root);
        
        let hash_chain_stores = vec![vec![], vec![]];
        
        Self {
            root_history,
            hash_chain_stores,
            sequence_number: 0,
            changelog_entries: Vec::new(),
        }
    }
    
    fn last_root(&self) -> [u8; 32] {
        self.root_history[self.root_history.len() - 1]
    }
    
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }
    
    fn add_changelog_entry(
        &mut self,
        old_root: [u8; 32],
        new_root: [u8; 32],
        leaves_hash_chain: [u8; 32],
        hash_chain_index: u16,
        pending_batch_index: u8,
        expected_seq: u64,
    ) {
        // Create and store a real changelog entry
        let entry = BatchChangelog {
            old_root,
            new_root,
            leaves_hash_chain,
            hash_chain_index,
            pending_batch_index,
            _padding: [0u8; 5],
            expected_seq,
        };
        
        // Check if we already have an entry for this hash_chain_index and batch_index
        let position = self.changelog_entries.iter().position(|e| 
            e.hash_chain_index == hash_chain_index && 
            e.pending_batch_index == pending_batch_index
        );
        
        if let Some(idx) = position {
            // Replace existing entry
            self.changelog_entries[idx] = entry;
        } else {
            // Add new entry
            self.changelog_entries.push(entry);
        }
    }
    
    fn process_changelog(&mut self) -> usize {
        let current_root = self.last_root();
        let current_seq = self.get_sequence_number();
        
        // Find applicable entries
        let mut applied_count = 0;
        
        // Process entries in order (this is simplified compared to real implementation)
        let mut to_apply = Vec::new();
        for entry in &self.changelog_entries {
            if is_applicable_entry(entry, &current_root, current_seq) {
                to_apply.push(*entry);
            }
        }
        
        // Apply all valid entries
        for entry in to_apply {
            // Update the tree
            self.update_tree(entry.new_root);
            applied_count += 1;
            
            // Remove processed entry
            self.changelog_entries.retain(|e| !(
                e.hash_chain_index == entry.hash_chain_index && 
                e.pending_batch_index == entry.pending_batch_index
            ));
        }
        
        applied_count
    }
    
    fn update_tree(&mut self, new_root: [u8; 32]) {
        self.root_history.push(new_root);
        self.sequence_number += 1;
    }
}

// Helper function to create test instruction data
fn create_test_instruction(
    old_root: [u8; 32],
    new_root: [u8; 32],
    hash_chain_index: u16
) -> ChangelogInstructionData {
    ChangelogInstructionData {
        old_root,
        new_root,
        hash_chain_index,
        compressed_proof: CompressedProof::default(),
    }
}

#[test]
#[serial]
fn test_changelog_integration() {
    // Create a mock tree account
    let mut tree = MockMerkleTreeAccount::new();
    
    // Get the initial root
    let initial_root = tree.last_root();
    println!("Initial root: {:?}", initial_root);
    
    // Create a sequence of roots that would be produced by hash operations
    let root1 = [1u8; 32]; // Pretend this is the root after first update
    let root2 = [2u8; 32]; // Root after second update
    let root3 = [3u8; 32]; // Root after third update
    
    // Create test hash chains
    let hash_chain0 = [10u8; 32];
    let hash_chain1 = [11u8; 32];
    let hash_chain2 = [12u8; 32];
    
    // Set up hash chains
    tree.hash_chain_stores[0].push(hash_chain0);
    tree.hash_chain_stores[0].push(hash_chain1);
    tree.hash_chain_stores[0].push(hash_chain2);
    
    // Test 1: Add an entry to the changelog
    tree.add_changelog_entry(
        initial_root,
        root1,
        hash_chain0,
        0, // hash_chain_index
        0, // pending_batch_index
        0, // expected_seq
    );
    
    // Process the changelog and apply valid entries
    let applied_count = tree.process_changelog();
    assert_eq!(applied_count, 1);
    
    // Verify the update was applied
    assert_eq!(tree.last_root(), root1);
    assert_eq!(tree.get_sequence_number(), 1);
    
    // Test 2: Add entries that depend on each other
    tree.add_changelog_entry(
        root2, // Out of order - depends on root1
        root3,
        hash_chain2,
        2, // hash_chain_index
        0, // pending_batch_index
        2, // expected_seq = 2
    );
    
    tree.add_changelog_entry(
        root1, // Current root
        root2,
        hash_chain1,
        1, // hash_chain_index
        0, // pending_batch_index
        1, // expected_seq = 1
    );
    
    // Process the changelog - should only apply the root1->root2 update
    let applied_count = tree.process_changelog();
    assert_eq!(applied_count, 1);
    
    // Verify update1 was applied
    assert_eq!(tree.last_root(), root2);
    assert_eq!(tree.get_sequence_number(), 2);
    
    // Process again - now the root2->root3 update should apply
    let applied_count = tree.process_changelog();
    assert_eq!(applied_count, 1);
    
    // Verify both updates were applied
    assert_eq!(tree.last_root(), root3);
    assert_eq!(tree.get_sequence_number(), 3);
    
    println!("All tests passed!");
}

#[test]
#[serial]
fn test_overwriting_changelog_entries() {
    // Create a mock tree account
    let mut tree = MockMerkleTreeAccount::new();
    
    // Get the initial root
    let initial_root = tree.last_root();
    
    // Create some test roots
    let root1 = [1u8; 32];
    let root1_updated = [11u8; 32]; // Updated version of root1
    
    // Create test hash chains
    let hash_chain0 = [10u8; 32];
    
    // Store these hash chains in the tree's hash_chain_store
    tree.hash_chain_stores[0].push(hash_chain0);
    
    // Add an entry to the changelog
    tree.add_changelog_entry(
        initial_root,
        root1,
        hash_chain0,
        0, // hash_chain_index
        0, // pending_batch_index
        0, // expected_seq
    );
    
    // Verify we have one entry
    assert_eq!(tree.changelog_entries.len(), 1);
    
    // Overwrite with a new entry
    tree.add_changelog_entry(
        initial_root,
        root1_updated, // Different new_root
        hash_chain0,
        0, // Same hash_chain_index
        0, // Same pending_batch_index
        0, // Same expected_seq
    );
    
    // Verify we still have only one entry (overwritten)
    assert_eq!(tree.changelog_entries.len(), 1);
    
    // Process the changelog
    let applied_count = tree.process_changelog();
    assert_eq!(applied_count, 1);
    
    // Verify the updated entry was applied
    assert_eq!(tree.last_root(), root1_updated);
}

#[test]
#[serial]
fn test_multi_batch_changelog() {
    // Create a mock tree account
    let mut tree = MockMerkleTreeAccount::new();
    
    // Create test roots for two different batches
    let batch0_root1 = [1u8; 32];
    let batch1_root1 = [101u8; 32];
    
    // Create test hash chains for two batches
    let batch0_hash_chain = [10u8; 32];
    let batch1_hash_chain = [110u8; 32];
    
    // Store hash chains
    tree.hash_chain_stores[0].push(batch0_hash_chain);
    tree.hash_chain_stores[1].push(batch1_hash_chain);
    
    // Add entries for both batches
    let initial_root = tree.last_root();
    
    tree.add_changelog_entry(
        initial_root,
        batch0_root1,
        batch0_hash_chain,
        0, // hash_chain_index
        0, // pending_batch_index
        0, // expected_seq
    );
    
    tree.add_changelog_entry(
        batch0_root1, // Depends on first batch being processed
        batch1_root1,
        batch1_hash_chain,
        0, // hash_chain_index
        1, // pending_batch_index (different batch)
        1, // expected_seq
    );
    
    // Process the changelog
    let applied_count = tree.process_changelog();
    assert_eq!(applied_count, 1);
    
    // Verify first batch update applied
    assert_eq!(tree.last_root(), batch0_root1);
    
    // Process again for second batch
    let applied_count = tree.process_changelog();
    assert_eq!(applied_count, 1);
    
    // Verify second batch update applied
    assert_eq!(tree.last_root(), batch1_root1);
}