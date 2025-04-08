#![cfg(feature = "test-only")]

use light_batched_merkle_tree::{
    changelog::{BatchChangelog, ChangelogInstructionData, create_entry, is_applicable_entry},
};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;

use serial_test::serial;

#[test]
#[serial]
fn test_basic_changelog_operations() {
    // Define test roots
    let root0 = [0u8; 32];
    let root1 = [1u8; 32];
    let root2 = [2u8; 32];
    let root3 = [3u8; 32];
    
    // Define test hash chains
    let chain0 = [10u8; 32];
    let chain1 = [11u8; 32];
    let chain2 = [12u8; 32];
    
    // Define updates
    let update0 = ChangelogInstructionData {
        old_root: root0,
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
    
    // Create changelog entries
    let entry0 = create_entry(&update0, 0, chain0, 0);
    let entry1 = create_entry(&update1, 0, chain1, 1);
    let entry2 = create_entry(&update2, 0, chain2, 2);
    
    // Test applicability checks
    assert!(is_applicable_entry(&entry0, &root0, 0));
    assert!(!is_applicable_entry(&entry1, &root0, 0));
    assert!(is_applicable_entry(&entry1, &root1, 1));
    assert!(is_applicable_entry(&entry2, &root2, 2));
    
    // Create a basic simulation of a tree with a changelog
    let mut current_root = root0;
    let mut current_seq = 0;
    let mut changelog: Vec<BatchChangelog> = Vec::new();
    
    // Process updates in-order
    println!("Processing in-order updates");
    
    // Update 0
    if is_applicable_entry(&entry0, &current_root, current_seq) {
        // Apply update
        current_root = entry0.new_root;
        current_seq += 1;
        println!("Applied update 0 directly");
    } else {
        // Store in changelog
        changelog.push(entry0);
        println!("Added update 0 to changelog");
    }
    
    // Update 1
    if is_applicable_entry(&entry1, &current_root, current_seq) {
        // Apply update
        current_root = entry1.new_root;
        current_seq += 1;
        println!("Applied update 1 directly");
    } else {
        // Store in changelog
        changelog.push(entry1);
        println!("Added update 1 to changelog");
    }
    
    // Update 2
    if is_applicable_entry(&entry2, &current_root, current_seq) {
        // Apply update
        current_root = entry2.new_root;
        current_seq += 1;
        println!("Applied update 2 directly");
    } else {
        // Store in changelog
        changelog.push(entry2);
        println!("Added update 2 to changelog");
    }
    
    // Verify in-order result
    assert_eq!(current_root, root3);
    assert_eq!(current_seq, 3);
    assert!(changelog.is_empty());
    
    // Reset for out-of-order test
    current_root = root0;
    current_seq = 0;
    changelog.clear();
    
    // Process updates out-of-order
    println!("Processing out-of-order updates");
    
    // Update 1 (out of order)
    if is_applicable_entry(&entry1, &current_root, current_seq) {
        // Apply update
        current_root = entry1.new_root;
        current_seq += 1;
        println!("Applied update 1 directly");
    } else {
        // Store in changelog
        changelog.push(entry1);
        println!("Added update 1 to changelog");
    }
    
    // Update 0 (now applicable)
    if is_applicable_entry(&entry0, &current_root, current_seq) {
        // Apply update
        current_root = entry0.new_root;
        current_seq += 1;
        println!("Applied update 0 directly");
    } else {
        // Store in changelog
        changelog.push(entry0);
        println!("Added update 0 to changelog");
    }
    
    // Apply pending updates
    println!("Checking for applicable pending updates");
    let mut applied_any = true;
    while applied_any {
        applied_any = false;
        
        // Find an applicable entry
        let position = changelog.iter().position(|entry| {
            is_applicable_entry(entry, &current_root, current_seq)
        });
        
        if let Some(idx) = position {
            // Apply the entry
            let entry = changelog.remove(idx);
            println!("Applying pending update with hash_chain_index {}", entry.hash_chain_index);
            current_root = entry.new_root;
            current_seq += 1;
            applied_any = true;
        }
    }
    
    // Update 2 (should apply directly now)
    if is_applicable_entry(&entry2, &current_root, current_seq) {
        // Apply update
        current_root = entry2.new_root;
        current_seq += 1;
        println!("Applied update 2 directly");
    } else {
        // Store in changelog
        changelog.push(entry2);
        println!("Added update 2 to changelog");
    }
    
    // Verify final state
    assert_eq!(current_root, root3);
    assert_eq!(current_seq, 3);
    assert!(changelog.is_empty());
}

#[test]
#[serial]
fn test_conflicting_updates() {
    // Define tree properties
    let root0 = [0u8; 32];
    let root1 = [1u8; 32];
    let root2 = [2u8; 32];
    let root3_v1 = [3u8; 32]; // First version
    let root3_v2 = [33u8; 32]; // Alternate version
    
    // Define hash chains
    let chain0 = [10u8; 32];
    let chain1 = [11u8; 32];
    let chain2_v1 = [12u8; 32];
    let chain2_v2 = [22u8; 32];
    
    // Define updates
    let update0 = ChangelogInstructionData {
        old_root: root0,
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
    
    // Two conflicting versions of update 2 (same hash_chain_index)
    let update2_v1 = ChangelogInstructionData {
        old_root: root2,
        new_root: root3_v1,
        hash_chain_index: 2,
        compressed_proof: CompressedProof::default(),
    };
    
    let update2_v2 = ChangelogInstructionData {
        old_root: root2,
        new_root: root3_v2,
        hash_chain_index: 2, // Same index!
        compressed_proof: CompressedProof::default(),
    };
    
    // Create changelog entries
    let entry0 = create_entry(&update0, 0, chain0, 0);
    let entry1 = create_entry(&update1, 0, chain1, 1);
    let entry2_v1 = create_entry(&update2_v1, 0, chain2_v1, 2);
    let entry2_v2 = create_entry(&update2_v2, 0, chain2_v2, 2); // Same expected sequence
    
    // Create our simulation
    let mut current_root = root0;
    let mut current_seq = 0;
    let mut changelog = std::collections::HashMap::new();
    
    println!("Testing conflicting updates");
    
    // Apply entry0 directly
    assert!(is_applicable_entry(&entry0, &current_root, current_seq));
    current_root = entry0.new_root;
    current_seq += 1;
    
    // Apply entry1 directly
    assert!(is_applicable_entry(&entry1, &current_root, current_seq));
    current_root = entry1.new_root;
    current_seq += 1;
    
    // Apply first version of entry2
    assert!(is_applicable_entry(&entry2_v1, &current_root, current_seq));
    current_root = entry2_v1.new_root;
    current_seq += 1;
    
    // Try to apply second version - should not be applicable anymore
    assert!(!is_applicable_entry(&entry2_v2, &current_root, current_seq - 1));
    
    // Let's add it to the changelog anyway
    changelog.insert(entry2_v2.hash_chain_index, entry2_v2);
    
    // Verify it can't be applied
    assert!(!is_applicable_entry(
        changelog.get(&2).unwrap(),
        &current_root,
        current_seq
    ));
    
    // Verify final state is at version 1
    assert_eq!(current_root, root3_v1);
    assert_eq!(current_seq, 3);
}