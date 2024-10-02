use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use light_bounded_vec::{BoundedVec, BoundedVecError, CyclicBoundedVec};
use light_concurrent_merkle_tree::{
    changelog::{ChangelogEntry, ChangelogPath},
    errors::ConcurrentMerkleTreeError,
    zero_copy::ConcurrentMerkleTreeZeroCopyMut,
    ConcurrentMerkleTree,
};
use light_hash_set::HashSet;
use light_hasher::{Hasher, Keccak, Poseidon, Sha256};
use light_utils::rand::gen_range_exclude;
use num_bigint::BigUint;
use num_traits::FromBytes;
use rand::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};
use std::cmp;

#[test]
fn test_case() {
    const BATCH_SIZE: usize = 10;
    const HEIGHT: usize = 4;
    // must be at least batch size
    const CHANGELOG: usize = 32;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<Poseidon, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let leaves: [[u8; 32]; BATCH_SIZE] = [Fr::rand(&mut thread_rng())
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap(); BATCH_SIZE];
    println!("leaves {:?}", leaves);

    for i in 0..merkle_tree.filled_subtrees.len() {
        println!("subtree {}: {:?}", i, merkle_tree.filled_subtrees[i]);
    }
    let leaves_ref = leaves.iter().map(|x| x).collect::<Vec<&[u8; 32]>>();
    merkle_tree.append_batch(leaves_ref.as_slice()).unwrap();

    println!("root: {:?}", merkle_tree.root());
    for i in 0..merkle_tree.filled_subtrees.len() {
        println!("subtree {}: {:?}", i, merkle_tree.filled_subtrees[i]);
    }
}
