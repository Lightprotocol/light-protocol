use ark_crypto_primitives::crh::TwoToOneCRH;
use ark_crypto_primitives::merkle_tree::{Config, MerkleTree, Path};
use crate::common::*;
// mod constraints_test;

#[derive(Clone)]
pub struct MerkleConfig;
impl Config for MerkleConfig {
    // Our Merkle tree relies on two hashes: one to hash leaves, and one to hash pairs
    // of internal nodes.
    type LeafHash = LeafHash;
    type TwoToOneHash = TwoToOneHash;
}

/// A Merkle tree containing account information.
pub type SimpleMerkleTree = MerkleTree<MerkleConfig>;
/// The root of the account Merkle tree.
pub type Root = <TwoToOneHash as TwoToOneCRH>::Output;
/// A membership proof for a given account.
pub type SimplePath = Path<MerkleConfig>;
