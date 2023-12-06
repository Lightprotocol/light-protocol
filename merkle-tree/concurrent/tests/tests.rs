use light_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::{Hasher, Poseidon, Sha256};

#[tokio::test]
async fn test_sha256() {
    let mut merkle_tree = ConcurrentMerkleTree::<Sha256, 3, 3>::default();

    merkle_tree.init();

    let h1 = Sha256::hashv(&[&[1; 32], &[2; 32]]).unwrap();
    println!("h1: {h1:?}");
    let h2 = Sha256::hashv(&[&h1, &Sha256::zero_bytes()[1]]).unwrap();
    println!("h2: {h2:?}");
    let h3 = Sha256::hashv(&[&h2, &Sha256::zero_bytes()[2]]).unwrap();
    println!("h3: {h3:?}");
    let h4 = Sha256::hashv(&[&h3, &Sha256::zero_bytes()[3]]).unwrap();
    println!("h4: {h4:?}");

    merkle_tree.append([1u8; 32]).unwrap();
    merkle_tree.append([2u8; 32]).unwrap();

    println!("ROOTS: {:?}", merkle_tree.roots);
    println!("CHANGELOG: {:?}", merkle_tree.changelog);

    assert_eq!(merkle_tree.roots[merkle_tree.current_index as usize], h3);
}
