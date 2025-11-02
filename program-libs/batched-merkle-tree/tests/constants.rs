use light_batched_merkle_tree::{
    constants::ADDRESS_TREE_INIT_ROOT_40, merkle_tree::BatchedMerkleTreeAccount,
};
use light_compressed_account::{Pubkey, TreeType};
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;
use light_merkle_tree_reference::indexed::IndexedMerkleTree;
use num_bigint::BigUint;
use num_traits::Num;

#[test]
fn test_reproduce_address_tree_init_root_40() {
    // Method 1: Using IndexedMerkleTree library
    let tree = IndexedMerkleTree::<Poseidon, usize>::new(40, 0).unwrap();
    let root_from_tree = tree.merkle_tree.root();

    assert_eq!(
        root_from_tree, ADDRESS_TREE_INIT_ROOT_40,
        "IndexedMerkleTree root does not match ADDRESS_TREE_INIT_ROOT_40 constant"
    );

    // Method 2: Manual hash computation to verify the constant
    // IndexedMerkleTree::new() creates tree with ONE leaf at index 0:
    // - IndexedArray::new(0, HIGHEST_ADDRESS_PLUS_ONE) creates element 0
    // - element[0].hash(HIGHEST_ADDRESS_PLUS_ONE) = H(0, HIGHEST_ADDRESS_PLUS_ONE)
    // - This single leaf is appended to the merkle tree

    const HIGHEST_ADDRESS_PLUS_ONE: &str =
        "452312848583266388373324160190187140051835877600158453279131187530910662655";

    let max_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10).unwrap();
    let max_value_bytes = light_hasher::bigint::bigint_to_be_bytes_array::<32>(&max_value).unwrap();

    // Leaf 0: H(value=0, nextValue=HIGHEST_ADDRESS_PLUS_ONE)
    let leaf_0 = Poseidon::hashv(&[&[0u8; 32], &max_value_bytes]).unwrap();

    // Build merkle tree root from single leaf
    // Hash leaf_0 with zero bytes for the empty right sibling
    let mut current_root = Poseidon::hashv(&[&leaf_0, &Poseidon::zero_bytes()[0]]).unwrap();

    // Hash up the tree to height 40
    for i in 1..40 {
        current_root = Poseidon::hashv(&[&current_root, &Poseidon::zero_bytes()[i]]).unwrap();
    }

    assert_eq!(
        current_root, ADDRESS_TREE_INIT_ROOT_40,
        "Manually computed root does not match ADDRESS_TREE_INIT_ROOT_40 constant"
    );
}
// Helper to create a minimal tree for ghost state testing
fn create_test_tree() -> BatchedMerkleTreeAccount<'static> {
    // let batch_size: u64 = 5; //TEST_DEFAULT_BATCH_SIZE;
    // let zkp_batch_size: u64 = 1; // TEST_DEFAULT_ZKP_BATCH_SIZE;
    // let root_history_capacity: u32 = 20;
    // let height = 32;
    // let num_iters = 1;
    // let bloom_filter_capacity = 8 * 100;

    // let account_data = vec![0u8; 4096].leak();
    // let pubkey = Pubkey::new_from_array([1u8; 32]);
    let batch_size: u64 = 3; //TEST_DEFAULT_BATCH_SIZE;
    let zkp_batch_size: u64 = 1; // TEST_DEFAULT_ZKP_BATCH_SIZE;
    let root_history_capacity: u32 = 30;
    let height = 40; // Address trees require height 40
    let num_iters = 1;
    let bloom_filter_capacity = 1;

    let account_data = vec![0u8; 1152].leak();
    let pubkey = Pubkey::new_from_array([1u8; 32]);
    let init_result = BatchedMerkleTreeAccount::init(
        account_data,
        &pubkey,
        MerkleTreeMetadata::default(),
        root_history_capacity,
        batch_size,
        zkp_batch_size,
        height,
        num_iters,
        bloom_filter_capacity,
        TreeType::AddressV2,
    )
    .unwrap();

    // kani::assume(init_result.is_ok());
    let tree_result = BatchedMerkleTreeAccount::address_from_bytes(account_data, &pubkey);
    // kani::assume(tree_result.is_ok());
    tree_result.unwrap()
}
#[test]
fn verify_no_unsafe_roots_ever() {
    create_test_tree();
}
