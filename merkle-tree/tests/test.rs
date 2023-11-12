use core::mem;

use light_merkle_tree::HashFunction;
use light_merkle_tree::{
    config,
    constants::{self},
    hasher::Sha256,
    MerkleTree,
};

mod test_config {
    use anchor_lang::prelude::*;

    use super::*;

    pub(crate) struct Sha256MerkleTreeConfig;

    impl config::MerkleTreeConfig for Sha256MerkleTreeConfig {
        const ZERO_BYTES: constants::ZeroBytes = constants::sha256::ZERO_BYTES;
        const PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);
    }
}

#[test]
fn test_sha256() {
    let mut merkle_tree = {
        let mut merkle_tree: MerkleTree<Sha256, test_config::Sha256MerkleTreeConfig> =
            unsafe { mem::zeroed() };
        merkle_tree.init(3, HashFunction::Sha256);
        merkle_tree
    };

    let h = merkle_tree.hash([1; 32], [1; 32]);
    let h = merkle_tree.hash(h, h);
    assert_eq!(h, constants::sha256::ZERO_BYTES[0]);
}

#[test]
fn test_merkle_tree_insert() {
    let mut merkle_tree = {
        let mut merkle_tree: MerkleTree<Sha256, test_config::Sha256MerkleTreeConfig> =
            unsafe { mem::zeroed() };
        merkle_tree.init(3, HashFunction::Sha256);
        merkle_tree
    };

    let h1 = merkle_tree.hash([1; 32], [2; 32]);
    let h2 = merkle_tree.hash(h1, constants::sha256::ZERO_BYTES[1]);
    let h3 = merkle_tree.hash(h2, constants::sha256::ZERO_BYTES[2]);

    merkle_tree.insert([1u8; 32], [2u8; 32]);
    assert_eq!(merkle_tree.last_root(), h3);

    assert_eq!(
        merkle_tree.last_root(),
        [
            247, 106, 203, 53, 197, 22, 54, 96, 235, 103, 77, 32, 26, 225, 24, 139, 161, 98, 253,
            193, 16, 47, 34, 229, 111, 32, 89, 149, 147, 184, 120, 122
        ]
    );

    merkle_tree.insert([3u8; 32], [4u8; 32]);

    assert_eq!(
        merkle_tree.last_root(),
        [
            221, 141, 161, 139, 16, 93, 204, 253, 77, 161, 139, 239, 120, 252, 228, 149, 93, 68,
            230, 151, 142, 173, 176, 130, 234, 217, 247, 60, 49, 232, 98, 116
        ]
    )
}
