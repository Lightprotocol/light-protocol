use core::mem;

use light_hasher::{Hasher, Poseidon, Sha256};
use light_merkle_tree::{config, HashFunction, MerkleTree};

mod test_config {
    use anchor_lang::prelude::*;

    use super::*;

    pub(crate) struct Sha256MerkleTreeConfig;

    impl config::MerkleTreeConfig for Sha256MerkleTreeConfig {
        const PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);
    }

    pub(crate) struct PoseidonMerkleTreeConfig;

    impl config::MerkleTreeConfig for PoseidonMerkleTreeConfig {
        const PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);
    }
}

#[test]
fn test_sha256() {
    let mut merkle_tree = {
        let mut merkle_tree: MerkleTree<Sha256, test_config::Sha256MerkleTreeConfig> =
            unsafe { mem::zeroed() };
        merkle_tree.init(3, HashFunction::Sha256).unwrap();
        merkle_tree
    };

    let h = merkle_tree.hash([1; 32], [1; 32]).unwrap();
    let h = merkle_tree.hash(h, h).unwrap();
    assert_eq!(h, Sha256::zero_bytes()[0]);
}

#[test]
fn test_sha256_insert() {
    let mut merkle_tree = {
        let mut merkle_tree: MerkleTree<Sha256, test_config::Sha256MerkleTreeConfig> =
            unsafe { mem::zeroed() };
        merkle_tree.init(3, HashFunction::Sha256).unwrap();
        merkle_tree
    };

    let h1 = merkle_tree.hash([1; 32], [2; 32]).unwrap();
    let h2 = merkle_tree.hash(h1, Sha256::zero_bytes()[1]).unwrap();
    let h3 = merkle_tree.hash(h2, Sha256::zero_bytes()[2]).unwrap();

    merkle_tree.insert([1u8; 32], [2u8; 32]).unwrap();
    assert_eq!(merkle_tree.last_root(), h3);

    assert_eq!(
        merkle_tree.last_root(),
        [
            247, 106, 203, 53, 197, 22, 54, 96, 235, 103, 77, 32, 26, 225, 24, 139, 161, 98, 253,
            193, 16, 47, 34, 229, 111, 32, 89, 149, 147, 184, 120, 122
        ]
    );

    merkle_tree.insert([3u8; 32], [4u8; 32]).unwrap();

    assert_eq!(
        merkle_tree.last_root(),
        [
            221, 141, 161, 139, 16, 93, 204, 253, 77, 161, 139, 239, 120, 252, 228, 149, 93, 68,
            230, 151, 142, 173, 176, 130, 234, 217, 247, 60, 49, 232, 98, 116
        ]
    )
}

#[test]
fn test_poseidon() {
    let mut merkle_tree = {
        let mut merkle_tree: MerkleTree<Poseidon, test_config::PoseidonMerkleTreeConfig> =
            unsafe { mem::zeroed() };
        merkle_tree.init(3, HashFunction::Poseidon).unwrap();
        merkle_tree
    };

    let h = merkle_tree.hash([1; 32], [1; 32]).unwrap();
    let h = merkle_tree.hash(h, h).unwrap();
    assert_eq!(h, Poseidon::zero_bytes()[0]);
}

#[test]
fn test_poseidon_insert() {
    let mut merkle_tree = {
        let mut merkle_tree: MerkleTree<Poseidon, test_config::PoseidonMerkleTreeConfig> =
            unsafe { mem::zeroed() };
        merkle_tree.init(3, HashFunction::Poseidon).unwrap();
        merkle_tree
    };

    let h1 = merkle_tree.hash([1; 32], [2; 32]).unwrap();
    let h2 = merkle_tree.hash(h1, Poseidon::zero_bytes()[1]).unwrap();
    let h3 = merkle_tree.hash(h2, Poseidon::zero_bytes()[2]).unwrap();

    merkle_tree.insert([1u8; 32], [2u8; 32]).unwrap();
    assert_eq!(merkle_tree.last_root(), h3);

    assert_eq!(
        merkle_tree.last_root(),
        [
            22, 94, 72, 41, 55, 76, 132, 167, 194, 100, 125, 135, 173, 48, 186, 192, 157, 132, 215,
            98, 17, 157, 248, 70, 97, 72, 215, 26, 225, 23, 243, 153
        ]
    );

    merkle_tree.insert([3u8; 32], [4u8; 32]).unwrap();

    assert_eq!(
        merkle_tree.last_root(),
        [
            48, 33, 5, 138, 97, 146, 194, 191, 119, 7, 17, 178, 236, 250, 1, 144, 253, 17, 213,
            164, 29, 75, 194, 242, 166, 138, 247, 216, 66, 17, 104, 50
        ]
    )
}
