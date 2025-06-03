use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_sparse_merkle_tree::merkle_tree::SparseMerkleTree;

const HEIGHT: usize = 5;
#[test]
fn test_sparse_merkle_tree() {
    let mut merkle_tree = SparseMerkleTree::<Poseidon, HEIGHT>::new_empty();
    let mut reference_merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, 0);
    for i in 0..1 << HEIGHT {
        let mut leaf = [0u8; 32];
        leaf[24..].copy_from_slice(&(i as u64).to_be_bytes());
        println!("i: {}, leaf: {:?}", i, leaf);
        merkle_tree.append(leaf);
        reference_merkle_tree.append(&leaf).unwrap();
        assert_eq!(merkle_tree.root(), reference_merkle_tree.root());
        assert_eq!(merkle_tree.get_next_index(), i + 1);
        let subtrees = merkle_tree.get_subtrees();
        let reference_subtrees = reference_merkle_tree.get_subtrees();
        assert_eq!(subtrees.to_vec(), reference_subtrees);
    }
}
