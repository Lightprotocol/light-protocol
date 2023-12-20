use core::mem;

use light_hasher::{zero_bytes, Poseidon, Sha256};
use light_sparse_merkle_tree::{config, HashFunction, MerkleTree};

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

    let h = merkle_tree.hash([0; 32], [0; 32]).unwrap();
    let h = merkle_tree.hash(h, h).unwrap();
    assert_eq!(h, zero_bytes::sha256::ZERO_BYTES[2]);
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
    let h2 = merkle_tree
        .hash(h1, zero_bytes::sha256::ZERO_BYTES[1])
        .unwrap();
    let h3 = merkle_tree
        .hash(h2, zero_bytes::sha256::ZERO_BYTES[2])
        .unwrap();

    merkle_tree.insert([1u8; 32], [2u8; 32]).unwrap();
    assert_eq!(merkle_tree.last_root(), h3);

    assert_eq!(
        merkle_tree.last_root(),
        [
            126, 24, 18, 163, 12, 124, 250, 179, 21, 106, 71, 81, 61, 52, 130, 118, 198, 143, 229,
            139, 246, 110, 172, 232, 92, 107, 161, 203, 59, 156, 229, 135
        ]
    );

    merkle_tree.insert([3u8; 32], [4u8; 32]).unwrap();

    assert_eq!(
        merkle_tree.last_root(),
        [
            26, 247, 6, 93, 36, 204, 134, 225, 221, 0, 17, 242, 118, 241, 7, 46, 118, 201, 127,
            135, 72, 127, 234, 204, 88, 209, 40, 54, 38, 141, 1, 99
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

    let h = merkle_tree.hash([0; 32], [0; 32]).unwrap();
    let h = merkle_tree.hash(h, h).unwrap();
    assert_eq!(h, zero_bytes::poseidon::ZERO_BYTES[2]);
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
    let h2 = merkle_tree
        .hash(h1, zero_bytes::poseidon::ZERO_BYTES[1])
        .unwrap();
    let h3 = merkle_tree
        .hash(h2, zero_bytes::poseidon::ZERO_BYTES[2])
        .unwrap();

    merkle_tree.insert([1u8; 32], [2u8; 32]).unwrap();
    assert_eq!(merkle_tree.last_root(), h3);

    assert_eq!(
        merkle_tree.last_root(),
        [
            28, 212, 110, 126, 27, 0, 170, 111, 30, 9, 154, 40, 116, 206, 213, 210, 21, 19, 30,
            108, 220, 78, 74, 161, 64, 217, 26, 196, 53, 228, 19, 145
        ]
    );

    merkle_tree.insert([3u8; 32], [4u8; 32]).unwrap();

    assert_eq!(
        merkle_tree.last_root(),
        [
            5, 157, 162, 35, 207, 42, 190, 6, 11, 97, 171, 221, 228, 40, 81, 188, 245, 68, 13, 47,
            220, 62, 218, 83, 32, 163, 166, 215, 53, 92, 58, 46
        ]
    )
}
