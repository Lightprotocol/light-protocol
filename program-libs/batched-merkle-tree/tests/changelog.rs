#![cfg(feature = "test-only")]

use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_batched_merkle_tree::changelog::{
    BatchChangelog, 
    ChangelogInstructionData,
    create_entry,
    is_applicable_entry
};

use std::collections::HashMap;

#[test]
fn test_batch_changelog_functionality() {
    // Create test parameters
    let empty_root = [0u8; 32];
    let root1 = [1u8; 32];
    let root2 = [2u8; 32];
    let root3 = [3u8; 32];
    
    let chain0 = [10u8; 32];
    let chain1 = [11u8; 32];
    let chain2 = [12u8; 32];
    
    // Create test updates
    let update0 = ChangelogInstructionData {
        old_root: empty_root,
        new_root: root1,
        hash_chain_index: 0,
        compressed_proof: CompressedProof::default(),
    };
    
    let update1 = ChangelogInstructionData {
        old_root: root1,
        new_root: root2,
        hash_chain_index: 1,
        compressed_proof: CompressedProof::default(),
    };
    
    let update2 = ChangelogInstructionData {
        old_root: root2,
        new_root: root3,
        hash_chain_index: 2,
        compressed_proof: CompressedProof::default(),
    };
    
    // Test creating and checking entries
    let entry0 = create_entry(&update0, 0, chain0, 0);
    let entry1 = create_entry(&update1, 0, chain1, 1);
    let entry2 = create_entry(&update2, 0, chain2, 2);
    
    // Entry 0 should be applicable to empty root with sequence 0
    assert!(is_applicable_entry(&entry0, &empty_root, 0));
    
    // Entry 1 should not be applicable to empty root
    assert!(!is_applicable_entry(&entry1, &empty_root, 0));
    
    // Entry 1 should be applicable to root1 with sequence 1
    assert!(is_applicable_entry(&entry1, &root1, 1));
    
    // Entry 2 should be applicable to root2 with sequence 2
    assert!(is_applicable_entry(&entry2, &root2, 2));
    
    // Test overwriting entries in a map
    let mut changelog_map: HashMap<u16, BatchChangelog> = HashMap::new();
    
    // Insert entry 1
    changelog_map.insert(entry1.hash_chain_index, entry1);
    assert_eq!(changelog_map.len(), 1);
    
    // Create a different entry with the same hash_chain_index
    let entry1_alt = BatchChangelog {
        old_root: root1,
        new_root: [42u8; 32], // Different new root
        leaves_hash_chain: [99u8; 32],
        hash_chain_index: 1, // Same index
        pending_batch_index: 0,
        _padding: [0u8; 5],
        expected_seq: 1,
    };
    
    // Insert the alternate entry - should replace the original
    changelog_map.insert(entry1_alt.hash_chain_index, entry1_alt);
    assert_eq!(changelog_map.len(), 1); // Still just one entry
    
    // Check that the entry was replaced
    let retrieved = changelog_map.get(&1).unwrap();
    assert_eq!(retrieved.new_root, [42u8; 32]);
    assert_eq!(retrieved.leaves_hash_chain, [99u8; 32]);
}

// The on-chain flow would use the BatchedMerkleTreeAccount to store and process
// changelog entries, which happens in the update instruction handlers.