use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_sparse_merkle_tree::changelog::{ChangelogEntry, ChangelogPath};

#[test]
fn test_tree_30k_elements_with_changelog() {
    const HEIGHT: usize = 32;
    let mut mt = MerkleTree::<Poseidon>::new(HEIGHT, 0);
    let num_elements = 500u64;
    let num_proofs = 60u64;
    let num_patched_proofs = (0..(num_proofs * num_elements)).sum::<u64>();
    let mut leaf = [0u8; 32];
    mt.append(&leaf).unwrap();
    let proof = mt.get_path_of_leaf(0, false).unwrap();
    let changelog: ChangelogEntry<HEIGHT> = ChangelogEntry::new(
        ChangelogPath(
            proof
                .iter()
                .cloned()
                .map(Some)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        ),
        0,
    );
    println!("num patched proofs {}", num_patched_proofs);

    for i in 1..num_patched_proofs {
        leaf[24..32].copy_from_slice(&i.to_be_bytes());
        let mut merkle_proof = mt.get_proof_of_leaf(i as usize, false).unwrap().to_vec();
        changelog
            .update_proof(i as usize, &mut merkle_proof)
            .unwrap();
    }
}
