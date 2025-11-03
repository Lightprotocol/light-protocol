use light_batched_merkle_tree::constants::ADDRESS_TREE_INIT_ROOT_40;
use light_hasher::{Hasher, Poseidon};
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
