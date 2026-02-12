use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, merkle_tree_ref::BatchedMerkleTreeRef,
};
use light_compressed_account::{pubkey::Pubkey, TreeType};
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;

#[test]
fn test_merkle_tree_ref_matches_mutable() {
    let mut account_data = vec![0u8; 3376];
    let batch_size = 5;
    let zkp_batch_size = 1;
    let root_history_len = 10;
    let num_iter = 1;
    let bloom_filter_capacity = 8000;
    let height = 40;
    let pubkey = Pubkey::new_unique();

    // Initialize via mutable path.
    let _account = BatchedMerkleTreeAccount::init(
        &mut account_data,
        &pubkey,
        MerkleTreeMetadata::default(),
        root_history_len,
        batch_size,
        zkp_batch_size,
        height,
        num_iter,
        bloom_filter_capacity,
        TreeType::AddressV2,
    )
    .unwrap();

    // Collect expected values from the mutable path.
    let expected_metadata;
    let expected_height;
    let expected_tree_type;
    let expected_seq;
    let expected_pubkey;
    let mut expected_roots = Vec::new();
    let expected_bf0;
    let expected_bf1;
    {
        let tree_mut =
            BatchedMerkleTreeAccount::address_from_bytes(&mut account_data, &pubkey).unwrap();
        expected_metadata = *tree_mut.get_metadata();
        expected_height = tree_mut.height;
        expected_tree_type = tree_mut.tree_type;
        expected_seq = tree_mut.sequence_number;
        expected_pubkey = *tree_mut.pubkey();
        for i in 0..root_history_len as usize {
            expected_roots.push(tree_mut.get_root_by_index(i).copied());
        }
        expected_bf0 = tree_mut.bloom_filter_stores[0].to_vec();
        expected_bf1 = tree_mut.bloom_filter_stores[1].to_vec();
    }

    // Read via immutable ref.
    let tree_ref = BatchedMerkleTreeRef::address_from_bytes(&account_data, &pubkey).unwrap();

    // Metadata should match (use Deref trait).
    assert_eq!(*tree_ref, expected_metadata);
    assert_eq!(tree_ref.height, expected_height);
    assert_eq!(tree_ref.tree_type, expected_tree_type);
    assert_eq!(tree_ref.sequence_number, expected_seq);
    assert_eq!(*tree_ref.pubkey(), expected_pubkey);

    // Root history should match (using root_history() accessor).
    for (i, expected) in expected_roots.iter().enumerate() {
        assert_eq!(Some(tree_ref.root_history()[i]), *expected);
    }

    // Bloom filter stores should match.
    assert_eq!(tree_ref.bloom_filter_stores[0], expected_bf0.as_slice());
    assert_eq!(tree_ref.bloom_filter_stores[1], expected_bf1.as_slice());

    // Non-inclusion check should work.
    let random_value = [42u8; 32];
    tree_ref
        .check_input_queue_non_inclusion(&random_value)
        .unwrap();
}
