use std::mem;

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use light_concurrent_merkle_tree::{
    changelog::ChangelogEntry,
    hash::{compute_root, validate_proof},
    ConcurrentMerkleTree,
};
use light_hasher::{errors::HasherError, Hasher, Keccak, Poseidon, Sha256};
use rand::thread_rng;

/// Tests whether append operations work as expected.
fn append<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 4;
    const CHANGELOG: usize = 32;
    const ROOTS: usize = 256;

    let mut merkle_tree = ConcurrentMerkleTree::<H, HEIGHT, CHANGELOG, ROOTS>::default();
    merkle_tree.init().unwrap();

    let leaf1 = H::hash(&[1u8; 32]).unwrap();

    // The hash of our new leaf and its sibling (a zero value).
    //
    //    H1
    //  /    \
    // L1   Z[0]
    let h1 = H::hashv(&[&leaf1, &H::zero_bytes()[0]]).unwrap();

    // The hash of `h1` and its sibling (a subtree represented by `Z[1]`).
    //
    //          H2
    //      /-/    \-\
    //    H1          Z[1]
    //  /    \      /      \
    // L1   Z[0]   Z[0]   Z[0]
    //
    // `Z[1]` represents the whole subtree on the right from `h2`. In the next
    // examples, we are just going to show empty subtrees instead of the whole
    // hierarchy.
    let h2 = H::hashv(&[&h1, &H::zero_bytes()[1]]).unwrap();

    // The hash of `h3` and its sibling (a subtree represented by `Z[2]`).
    //
    //          H3
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let h3 = H::hashv(&[&h2, &H::zero_bytes()[2]]).unwrap();

    // The hash of `h4` and its sibling (a subtree represented by `Z[3]`),
    // which is the root.
    //
    //              R
    //           /     \
    //          H3    Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let expected_root = H::hashv(&[&h3, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [leaf1, h1, h2, h3];
    let expected_proof = [
        H::zero_bytes()[0],
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    merkle_tree.append(&leaf1).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 1);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 0)
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 1);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 1);
    assert_eq!(merkle_tree.rightmost_leaf, leaf1);

    // Appending the 2nd leaf should result in recomputing the root due to the
    // change of the `h1`, which now is a hash of the two non-zero leafs. So
    // when computing all hashes up to the root, we are still going to use
    // zero bytes from 1 to 8.
    //
    // The other subtrees still remain the same.
    //
    //              R
    //           /     \
    //          H3    Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //   H1    Z[1]
    //  /  \
    // L1  L2
    let leaf2 = H::hash(&[2u8; 32]).unwrap();

    let h1 = H::hashv(&[&leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&h1, &H::zero_bytes()[1]]).unwrap();
    let h3 = H::hashv(&[&h2, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h3, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [leaf2, h1, h2, h3];
    let expected_proof = [
        leaf1,
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    merkle_tree.append(&leaf2).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 2);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 1),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 2);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 2);
    assert_eq!(merkle_tree.rightmost_leaf, leaf2);

    // Appending the 3rd leaf alters the next subtree on the right.
    // Instead of using Z[1], we will end up with the hash of the new leaf and
    // Z[0].
    //
    // The other subtrees still remain the same.
    //
    //               R
    //            /     \
    //           H4    Z[3]
    //         /    \
    //       H3    Z[2]
    //     /    \
    //   H1      H2
    //  /  \    /  \
    // L1  L2  L3  Z[0]
    let leaf3 = H::hash(&[3u8; 32]).unwrap();

    let h1 = H::hashv(&[&leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &H::zero_bytes()[0]]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [leaf3, h2, h3, h4];
    let expected_proof = [
        H::zero_bytes()[0],
        h1,
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    merkle_tree.append(&leaf3).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 3);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 2),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 3);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 3);
    assert_eq!(merkle_tree.rightmost_leaf, leaf3);

    // Appending the 4th leaf alters the next subtree on the right.
    // Instead of using Z[1], we will end up with the hash of the new leaf and
    // Z[0].
    //
    // The other subtrees still remain the same.
    //
    //               R
    //            /     \
    //           H4    Z[3]
    //         /    \
    //       H3    Z[2]
    //     /    \
    //   H1      H2
    //  /  \    /  \
    // L1  L2  L3  L4
    let leaf4 = H::hash(&[4u8; 32]).unwrap();

    let h1 = H::hashv(&[&leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [leaf4, h2, h3, h4];
    let expected_proof = [leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 4);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 3),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 4);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, leaf4);
}

/// Tests whether update operations work as expected.
fn update<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 4;
    const CHANGELOG: usize = 32;
    const ROOTS: usize = 256;

    let mut merkle_tree = ConcurrentMerkleTree::<H, HEIGHT, CHANGELOG, ROOTS>::default();
    merkle_tree.init().unwrap();

    let leaf1 = H::hash(&[1u8; 32]).unwrap();
    let leaf2 = H::hash(&[2u8; 32]).unwrap();
    let leaf3 = H::hash(&[3u8; 32]).unwrap();
    let leaf4 = H::hash(&[4u8; 32]).unwrap();

    // Append 4 leaves.
    //
    //               R
    //            /     \
    //           H4    Z[3]
    //         /    \
    //       H3    Z[2]
    //     /    \
    //   H1      H2
    //  /  \    /  \
    // L1  L2  L3  L4
    let h1 = H::hashv(&[&leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [leaf4, h2, h3, h4];
    let expected_proof = [leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    merkle_tree.append(&leaf1).unwrap();
    merkle_tree.append(&leaf2).unwrap();
    merkle_tree.append(&leaf3).unwrap();
    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 4);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 3),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 4);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, leaf4);

    // Replace `leaf1`.
    let new_leaf1 = [9u8; 32];

    // Replacing L1 affects H1 and all parent hashes up to the root.
    //
    //                R
    //             /     \
    //           *H4*   Z[3]
    //          /    \
    //       *H3*   Z[2]
    //      /    \
    //   *H1*     H2
    //   /  \    /  \
    // *L1* L2  L3  L4
    //
    // Merkle proof for the replaced leaf L1 is:
    // [L2, H2, Z[2], Z[3]]
    let proof = &[leaf2, h2, H::zero_bytes()[2], H::zero_bytes()[3]];

    let changelog_index = merkle_tree.changelog_index();
    merkle_tree
        .update(changelog_index, &leaf1, &new_leaf1, 0, proof)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf1, h1, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 5);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 0),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 5);
    // `rightmost_*` variables should remain unchanged.
    // Note that we didn't create a new `expected_proof` here. We just re-used
    // the previous one.
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, leaf4);

    // Replace `leaf2`.
    let new_leaf2 = H::hash(&[8u8; 32]).unwrap();

    // Replacing L2 affects H1 and all parent hashes up to the root.
    //
    //               R
    //            /     \
    //          *H4*   Z[3]
    //         /    \
    //      *H3*   Z[2]
    //     /    \
    //  *H1*     H2
    //  /  \    /  \
    // L1 *L2* L3  L4
    //
    // Merkle proof for the replaced leaf L2 is:
    // [L1, H2, Z[2], Z[3]]
    let proof = &[new_leaf1, h2, H::zero_bytes()[2], H::zero_bytes()[3]];

    let changelog_index = merkle_tree.changelog_index();
    merkle_tree
        .update(changelog_index, &leaf2, &new_leaf2, 1, proof)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf2, h1, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 6);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 1),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 6);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, leaf4);

    // Replace `leaf3`.
    let new_leaf3 = H::hash(&[7u8; 32]).unwrap();

    // Replacing L3 affects H1 and all parent hashes up to the root.
    //
    //               R
    //            /     \
    //          *H4*   Z[3]
    //         /    \
    //      *H3*   Z[2]
    //     /    \
    //   H1     *H2*
    //  /  \    /  \
    // L1  L2 *L3* L4
    //
    // Merkle proof for the replaced leaf L3 is:
    // [L4, H1, Z[2], Z[3]]
    let proof = &[leaf4, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    let changelog_index = merkle_tree.changelog_index();
    merkle_tree
        .update(changelog_index, &leaf3, &new_leaf3, 2, proof)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf3, h2, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 7);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 2)
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 7);
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, leaf4);

    // Replace `leaf4`.
    let new_leaf4 = H::hash(&[6u8; 32]).unwrap();

    // Replacing L4 affects H1 and all parent hashes up to the root.
    //
    //               R
    //            /     \
    //          *H4*   Z[3]
    //         /    \
    //      *H3*   Z[2]
    //     /    \
    //   H1     *H2*
    //  /  \    /  \
    // L1  L2  L3 *L4*
    //
    // Merkle proof for the replaced leaf L4 is:
    // [L3, H1, Z[2], Z[3]]
    let proof = &[new_leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    let changelog_index = merkle_tree.root_index();
    merkle_tree
        .update(changelog_index, &leaf4, &new_leaf4, 3, proof)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &new_leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf4, h2, h3, h4];
    let expected_proof = [new_leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    assert_eq!(merkle_tree.changelog_index(), 8);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::<HEIGHT>::new(expected_root, expected_changelog_path, 3)
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 8);
    // This time `rightmost_*` fields should be changed.
    assert_eq!(merkle_tree.rightmost_proof, expected_proof);
    assert_eq!(merkle_tree.rightmost_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, new_leaf4);
}

/// Tests whether appending leaves over the limit results in an explicit error.
fn overfill_tree<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 2;
    const CHANGELOG: usize = 32;
    const ROOTS: usize = 32;

    let mut merkle_tree = ConcurrentMerkleTree::<H, HEIGHT, CHANGELOG, ROOTS>::default();
    merkle_tree.init().unwrap();

    for _ in 0..4 {
        merkle_tree.append(&[4; 32]).unwrap();
    }
    assert!(matches!(
        merkle_tree.append(&[4; 32]),
        Err(HasherError::TreeFull)
    ));
}

/// Tests whether performing enough updates to overfill the changelog and root
/// buffer results in graceful reset of the counters.
fn overfill_changelog_and_roots<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 2;
    const CHANGELOG: usize = 6;
    const ROOTS: usize = 8;

    // Our implementation of concurrent Merkle tree.
    let mut merkle_tree = ConcurrentMerkleTree::<H, HEIGHT, CHANGELOG, ROOTS>::default();
    merkle_tree.init().unwrap();

    // Reference implementation of Merkle tree which Solana Labs uses for
    // testing (and therefore, we as well). We use it mostly to get the Merkle
    // proofs.
    // let leaves = vec![spl_concurrent_merkle_tree::node::EMPTY; 1 << HEIGHT];
    let mut reference_tree =
        light_merkle_tree_reference::MerkleTree::<H, HEIGHT, ROOTS>::new().unwrap();

    let mut rng = thread_rng();

    // Fill up the tree, producing 4 roots and changelog entries.
    for i in 0..(1 << HEIGHT) {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        merkle_tree.append(&leaf).unwrap();
        reference_tree.update(&leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.current_changelog_index, 4);
    assert_eq!(merkle_tree.current_root_index, 4);

    // Update 2 leaves to fill up the changelog. Its counter should reach the
    // modulus and get reset.
    for i in 0..2 {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        let changelog_index = merkle_tree.changelog_index();
        let old_leaf = reference_tree.leaf(i);
        let proof = reference_tree.get_proof_of_leaf(i);

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.current_changelog_index, 0);
    assert_eq!(merkle_tree.current_root_index, 6);

    // Update another 2 leaves to fill up the root. Its counter should reach
    // the modulus and get reset. The previously reset counter should get
    // incremented.
    for i in 0..2 {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        let changelog_index = merkle_tree.changelog_index();
        let old_leaf = reference_tree.leaf(i);
        let proof = reference_tree.get_proof_of_leaf(i);

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.current_changelog_index, 2);
    assert_eq!(merkle_tree.current_root_index, 0);

    // The latter updates should keep incrementing the counters.
    for i in 0..3 {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        let changelog_index = merkle_tree.changelog_index();
        let old_leaf = reference_tree.leaf(i);
        let proof = reference_tree.get_proof_of_leaf(i);

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.current_changelog_index, 5);
    assert_eq!(merkle_tree.current_root_index, 3);
}

/// Tests the Merkle tree without changelog, which in fact takes away the
/// support for concurrent updates. But that's alright in case we want to use
/// the tree as append-only (e.g. transactions).
fn without_changelog<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 2;
    const CHANGELOG: usize = 0;
    const ROOTS: usize = 8;

    // Our implementation of concurrent Merkle tree.
    let mut merkle_tree = ConcurrentMerkleTree::<H, HEIGHT, CHANGELOG, ROOTS>::default();
    merkle_tree.init().unwrap();

    // Reference implementation of Merkle tree which Solana Labs uses for
    // testing (and therefore, we as well). We use it mostly to get the Merkle
    // proofs.
    let mut reference_tree =
        light_merkle_tree_reference::MerkleTree::<H, HEIGHT, ROOTS>::new().unwrap();

    let mut rng = thread_rng();

    for i in 0..(1 << HEIGHT) {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        merkle_tree.append(&leaf).unwrap();
        reference_tree.update(&leaf, i).unwrap();
    }

    for i in 0..32 {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        // If `ì` is greater than possible number of leaves, apply a modulus.
        let i = i % (1 << HEIGHT);

        let changelog_index = merkle_tree.changelog_index();
        let old_leaf = reference_tree.leaf(i);
        let proof = reference_tree.get_proof_of_leaf(i);

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }
}

/// Tests the case outlined in the [indexed Merkle tree] examples. It does so
/// without defining any abstraction for indexed MTs, instead it just does all
/// necessary operations manually to ensure the correctness of the concurrent
/// MT implementation.
///
/// [indexed Merkle tree](https://docs.aztec.network/concepts/advanced/data_structures/indexed_merkle_tree)
fn nullifiers<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 4;
    const CHANGELOG: usize = 8;
    const ROOTS: usize = 256;

    // Our implementation of concurrent Merkle tree.
    let mut merkle_tree = ConcurrentMerkleTree::<H, HEIGHT, CHANGELOG, ROOTS>::default();
    merkle_tree.init().unwrap();

    // Reference implementation of Merkle tree which Solana Labs uses for
    // testing (and therefore, we as well). We use it mostly to get the Merkle
    // proofs.
    let mut reference_tree =
        light_merkle_tree_reference::MerkleTree::<H, HEIGHT, ROOTS>::new().unwrap();

    // Append a "zero indexed leaf".
    let zero_indexed_leaf =
        H::hashv(&[&[0u8; 32], &[0u8; mem::size_of::<usize>()], &[0u8; 32]]).unwrap();
    merkle_tree.append(&zero_indexed_leaf).unwrap();
    reference_tree.update(&zero_indexed_leaf, 0).unwrap();

    // Append the first nullifier (30).
    let low_nullifier_index: usize = 0;
    let nullifier1_index: usize = 1;
    let nullifier1: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 30,
    ];
    let low_nullifier_leaf1 = H::hashv(&[
        &[0u8; 32],                                // value
        nullifier1_index.to_le_bytes().as_slice(), // next index
        nullifier1.as_slice(),                     // next value
    ])
    .unwrap();
    let low_nullifier_proof = reference_tree.get_proof_of_leaf(low_nullifier_index);
    let nullifier1_leaf = H::hashv(&[
        nullifier1.as_slice(),            // value
        0_usize.to_le_bytes().as_slice(), // next index
        &[0u8; 32],                       // next value
    ])
    .unwrap();
    // Update the low nullifier.
    merkle_tree
        .update(
            merkle_tree.changelog_index(),
            &zero_indexed_leaf,
            &low_nullifier_leaf1,
            low_nullifier_index,
            &low_nullifier_proof,
        )
        .unwrap();
    reference_tree
        .update(&low_nullifier_leaf1, low_nullifier_index)
        .unwrap();
    println!(
        "onchain root after updating 1st low nullifier: {:?}",
        merkle_tree.root().unwrap()
    );
    println!(
        "offchain root after updating 1st low nullifier: {:?}",
        reference_tree.root().unwrap()
    );
    // Append the new nullifier.
    merkle_tree.append(&nullifier1_leaf).unwrap();
    reference_tree
        .update(&nullifier1_leaf, nullifier1_index)
        .unwrap();
    println!(
        "onchain root after appending 1st nullifier: {:?}",
        merkle_tree.root().unwrap()
    );
    println!(
        "offchain root after updating 1st nullifier: {:?}",
        reference_tree.root().unwrap()
    );
    println!(
        "low_nullifier_proof onchain: {:?}",
        merkle_tree.rightmost_proof
    );

    assert_eq!(merkle_tree.root().unwrap(), reference_tree.root().unwrap());
    assert_eq!(
        merkle_tree.rightmost_proof,
        reference_tree.get_proof_of_leaf(nullifier1_index)
    );

    // Append the second nullifier (10).
    let low_nullifier_index: usize = 0;
    let nullifier2_index: usize = 2;
    let nullifier2: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 10,
    ];
    let low_nullifier_leaf2 = H::hashv(&[
        &[0u8; 32],                                // value
        nullifier2_index.to_le_bytes().as_slice(), // next index
        nullifier2.as_slice(),
    ])
    .unwrap();
    let low_nullifier_proof = reference_tree.get_proof_of_leaf(low_nullifier_index);
    let expected_root = compute_root::<H, HEIGHT>(
        &zero_indexed_leaf,
        low_nullifier_index,
        &low_nullifier_proof,
    )
    .unwrap();
    println!("expected root: {expected_root:?}");
    let h1 = H::hashv(&[zero_indexed_leaf.as_slice(), nullifier1_leaf.as_slice()]).unwrap();
    println!("h1: {h1:?}");
    let h2 = H::hashv(&[h1.as_slice(), H::zero_bytes()[1].as_slice()]).unwrap();
    println!("h2: {h2:?}");
    let h3 = H::hashv(&[h2.as_slice(), H::zero_bytes()[2].as_slice()]).unwrap();
    println!("h3: {h3:?}");
    let expected_root_manual = H::hashv(&[h3.as_slice(), H::zero_bytes()[3].as_slice()]).unwrap();
    println!("expected root manual: {expected_root_manual:?}");
    println!("root: {:?}", merkle_tree.root().unwrap());
    println!("offchain roots: {:?}", reference_tree.roots);
    println!(
        "onchain roots: {:?}",
        &merkle_tree.roots[..merkle_tree.current_root_index as usize + 1]
    );
    let nullifier2_leaf = H::hashv(&[
        nullifier2.as_slice(),
        0_usize.to_le_bytes().as_slice(),
        &[0u8; 32],
    ])
    .unwrap();
    // Update the low nullifier.
    merkle_tree
        .update(
            merkle_tree.changelog_index(),
            &low_nullifier_leaf1,
            &low_nullifier_leaf2,
            low_nullifier_index,
            &low_nullifier_proof,
        )
        .unwrap();
    reference_tree
        .update(&low_nullifier_leaf2, low_nullifier_index)
        .unwrap();
    assert_eq!(merkle_tree.root().unwrap(), reference_tree.root().unwrap());
    // Append the new nullifier.
    merkle_tree.append(&nullifier2_leaf).unwrap();
    reference_tree
        .update(&nullifier2_leaf, nullifier2_index)
        .unwrap();

    assert_eq!(merkle_tree.root().unwrap(), reference_tree.root().unwrap());
    assert_eq!(
        merkle_tree.rightmost_proof,
        reference_tree.get_proof_of_leaf(nullifier1_index)
    );
}

#[test]
fn test_append_keccak() {
    append::<Keccak>()
}

#[test]
fn test_append_poseidon() {
    append::<Poseidon>()
}

#[test]
fn test_append_sha256() {
    append::<Sha256>()
}

#[test]
fn test_update_keccak() {
    update::<Keccak>()
}

#[test]
fn test_update_poseidon() {
    update::<Poseidon>()
}

#[test]
fn test_update_sha256() {
    update::<Sha256>()
}

#[test]
fn test_overfill_tree_keccak() {
    overfill_tree::<Keccak>()
}

#[test]
fn test_overfill_tree_poseidon() {
    overfill_tree::<Poseidon>()
}

#[test]
fn test_overfill_tree_sha256() {
    overfill_tree::<Sha256>()
}

#[test]
fn test_overfill_changelog_keccak() {
    overfill_changelog_and_roots::<Keccak>()
}

#[test]
fn test_without_changelog_keccak() {
    without_changelog::<Keccak>()
}

#[test]
fn test_nullifiers_keccak() {
    nullifiers::<Keccak>()
}

#[test]
fn test_nullifiers_poseidon() {
    nullifiers::<Poseidon>()
}

#[test]
fn test_nullifiers_sha256() {
    nullifiers::<Sha256>()
}

/// Checks whether our `append` and `update` implementations are compatible
/// with `append` and `set_leaf` from `spl-concurrent-merkle-tree` crate.
#[tokio::test(flavor = "multi_thread")]
async fn test_spl_compat() {
    const HEIGHT: usize = 4;
    const CHANGELOG: usize = 64;
    const ROOTS: usize = 256;

    let mut rng = thread_rng();

    // Our implementation of concurrent Merkle tree.
    let mut concurrent_mt = ConcurrentMerkleTree::<Keccak, HEIGHT, CHANGELOG, ROOTS>::default();
    concurrent_mt.init().unwrap();

    // Solana Labs implementation of concurrent Merkle tree.
    let mut spl_concurrent_mt = spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<HEIGHT, ROOTS>::new();
    spl_concurrent_mt.initialize().unwrap();

    // Reference implemenetation of Merkle tree which Solana Labs uses for
    // testing (and therefore, we as well). We use it mostly to get the Merkle
    // proofs.
    let mut reference_tree =
        light_merkle_tree_reference::MerkleTree::<Keccak, HEIGHT, ROOTS>::new().unwrap();

    for i in 0..(1 << HEIGHT) {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        concurrent_mt.append(&leaf).unwrap();
        spl_concurrent_mt.append(leaf).unwrap();
        reference_tree.update(&leaf, i).unwrap();

        let root = concurrent_mt.root().unwrap();
        assert_eq!(root, spl_concurrent_mt.get_change_log().root,);
        assert_eq!(root, reference_tree.root().unwrap());
    }

    for i in 0..(1 << HEIGHT) {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        let root = concurrent_mt.root().unwrap();
        let root_index = concurrent_mt.root_index();
        let old_leaf = reference_tree.leaf(i);
        let proof = reference_tree.get_proof_of_leaf(i);

        concurrent_mt
            .update(root_index, &old_leaf, &new_leaf, i, &proof)
            .unwrap();
        spl_concurrent_mt
            .set_leaf(root, old_leaf, new_leaf, proof.as_slice(), i as u32)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();

        let root = concurrent_mt.root().unwrap();
        assert_eq!(root, spl_concurrent_mt.get_change_log().root,);
        assert_eq!(root, reference_tree.root().unwrap());
    }
}
