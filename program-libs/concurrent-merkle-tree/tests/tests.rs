use std::cmp;

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
use num_bigint::BigUint;
use num_traits::FromBytes;
use rand::{
    distributions::uniform::{SampleRange, SampleUniform},
    rngs::ThreadRng,
    seq::SliceRandom,
    thread_rng, Rng,
};

/// Tests whether append operations work as expected.
fn append<H, const CANOPY: usize>()
where
    H: Hasher,
{
    const HEIGHT: usize = 4;
    const CHANGELOG: usize = 32;
    const ROOTS: usize = 256;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
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
    let expected_changelog_path = ChangelogPath([Some(leaf1), Some(h1), Some(h2), Some(h3)]);
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf1, h1, h2, h3]);

    merkle_tree.append(&leaf1).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 1);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 0)
    );
    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 1);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index(), 1);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf1);

    // Appending the 2nd leaf should result in recomputing the root due to the
    // change of the `h1`, which now is a hash of the two non-zero leafs. So
    // when computing hashes from H2 up to the root, we are still going to use
    // zero bytes.
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
    let expected_changelog_path = ChangelogPath([Some(leaf2), Some(h1), Some(h2), Some(h3)]);
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf1, h1, h2, h3]);

    merkle_tree.append(&leaf2).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 2);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 1),
    );
    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 2);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index(), 2);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf2);

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
    let expected_changelog_path = ChangelogPath([Some(leaf3), Some(h2), Some(h3), Some(h4)]);
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf3, h1, h3, h4]);

    merkle_tree.append(&leaf3).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 3);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 2),
    );
    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 3);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index(), 3);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf3);

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
    let expected_changelog_path = ChangelogPath([Some(leaf4), Some(h2), Some(h3), Some(h4)]);
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf3, h1, h3, h4]);

    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 4);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 3),
    );
    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 4);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index(), 4);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf4);
}

/// Checks whether `append_with_proof` returns correct Merkle proofs.
fn append_with_proof<
    H,
    const HEIGHT: usize,
    const CHANGELOG: usize,
    const ROOTS: usize,
    const CANOPY: usize,
    const N_APPENDS: usize,
>()
where
    H: Hasher,
{
    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    for i in 0..N_APPENDS {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        let mut proof = BoundedVec::with_capacity(HEIGHT);
        merkle_tree.append_with_proof(&leaf, &mut proof).unwrap();
        reference_tree.append(&leaf).unwrap();

        let reference_proof = reference_tree.get_proof_of_leaf(i, true).unwrap();

        assert_eq!(proof.to_vec(), reference_proof);
    }
}

/// Performs invalid updates on the given Merkle tree by trying to swap all
/// parameters separately. Asserts the errors that the Merkle tree should
/// return as a part of validation of these inputs.
fn invalid_updates<H, const HEIGHT: usize, const CHANGELOG: usize>(
    rng: &mut ThreadRng,
    merkle_tree: &mut ConcurrentMerkleTree<H, HEIGHT>,
    changelog_index: usize,
    old_leaf: &[u8; 32],
    new_leaf: &[u8; 32],
    leaf_index: usize,
    proof: BoundedVec<[u8; 32]>,
) where
    H: Hasher,
{
    // This test case works only for larger changelogs, where there is a chance
    // to encounter conflicting changelog entries.
    //
    // We assume that it's going to work for changelogs with capacity greater
    // than 1. But the smaller the changelog and the more non-conflicting
    // operations are done in between, the higher the chance of this check
    // failing. If you ever encounter issues with reproducing this error, try
    // tuning your changelog size or make sure that conflicting operations are
    // done frequently enough.
    if CHANGELOG > 1 {
        let invalid_changelog_index = 0;
        let mut proof_clone = proof.clone();
        let res = merkle_tree.update(
            invalid_changelog_index,
            old_leaf,
            new_leaf,
            leaf_index,
            &mut proof_clone,
        );
        assert!(matches!(
            res,
            Err(ConcurrentMerkleTreeError::CannotUpdateLeaf)
        ));
    }

    let invalid_old_leaf: [u8; 32] = Fr::rand(rng)
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap();
    let mut proof_clone = proof.clone();
    let res = merkle_tree.update(
        changelog_index,
        &invalid_old_leaf,
        new_leaf,
        0,
        &mut proof_clone,
    );
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::InvalidProof(_, _))
    ));

    /// Generates a random value in the given range, excluding the values provided
    /// in `exclude`.
    pub fn gen_range_exclude<N, R, T>(rng: &mut N, range: R, exclude: &[T]) -> T
    where
        N: Rng,
        R: Clone + SampleRange<T>,
        T: PartialEq + SampleUniform,
    {
        loop {
            // This utility is supposed to be used only in unit tests. This `clone`
            // is harmless and necessary (can't pass a reference to range, it has
            // to be moved).
            let sample = rng.gen_range(range.clone());
            if !exclude.contains(&sample) {
                return sample;
            }
        }
    }
    let invalid_index_in_range = gen_range_exclude(rng, 0..merkle_tree.next_index(), &[leaf_index]);
    let mut proof_clone = proof.clone();
    let res = merkle_tree.update(
        changelog_index,
        old_leaf,
        new_leaf,
        invalid_index_in_range,
        &mut proof_clone,
    );
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::InvalidProof(_, _))
    ));

    // Try pointing to the leaf indices outside the range only if the tree is
    // not full. Otherwise, it doesn't make sense and even `gen_range` will
    // fail.
    let next_index = merkle_tree.next_index();
    let limit_leaves = 1 << HEIGHT;
    if next_index < limit_leaves {
        let invalid_index_outside_range = rng.gen_range(next_index..limit_leaves);
        let mut proof_clone = proof.clone();
        let res = merkle_tree.update(
            changelog_index,
            old_leaf,
            new_leaf,
            invalid_index_outside_range,
            &mut proof_clone,
        );
        assert!(matches!(
            res,
            Err(ConcurrentMerkleTreeError::CannotUpdateEmpty)
        ));
    }
}

/// Tests whether update operations work as expected.
fn update<H, const CHANGELOG: usize, const ROOTS: usize, const CANOPY: usize>()
where
    H: Hasher,
{
    const HEIGHT: usize = 4;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();
    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

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
    let expected_changelog_path = ChangelogPath([Some(leaf4), Some(h2), Some(h3), Some(h4)]);
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf3, h1, h3, h4]);

    merkle_tree.append(&leaf1).unwrap();
    reference_tree.append(&leaf1).unwrap();
    merkle_tree.append(&leaf2).unwrap();
    reference_tree.append(&leaf2).unwrap();
    merkle_tree.append(&leaf3).unwrap();
    reference_tree.append(&leaf3).unwrap();
    merkle_tree.append(&leaf4).unwrap();
    reference_tree.append(&leaf4).unwrap();

    let canopy_levels = [
        &[h4, H::zero_bytes()[3]][..],
        &[
            h3,
            H::zero_bytes()[2],
            H::zero_bytes()[2],
            H::zero_bytes()[2],
        ][..],
    ];
    let mut expected_canopy = Vec::new();

    for canopy_level in canopy_levels.iter().take(CANOPY) {
        println!("canopy_level: {:?}", canopy_level);
        expected_canopy.extend_from_slice(canopy_level);
    }

    assert_eq!(merkle_tree.changelog_index(), 4 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 3),
    );

    assert_eq!(merkle_tree.root(), reference_tree.root());
    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 4);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index(), 4);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf4);
    assert_eq!(
        merkle_tree.canopy,
        BoundedVec::from_slice(reference_tree.get_canopy().unwrap().as_slice())
    );
    assert_eq!(merkle_tree.canopy.as_slice(), expected_canopy.as_slice());

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
    let changelog_index = merkle_tree.changelog_index();

    let proof_raw = &[leaf2, h2, H::zero_bytes()[2], H::zero_bytes()[3]];
    let mut proof = BoundedVec::with_capacity(HEIGHT);
    for node in &proof_raw[..HEIGHT - CANOPY] {
        proof.push(*node).unwrap();
    }

    invalid_updates::<H, HEIGHT, CHANGELOG>(
        &mut rng,
        &mut merkle_tree,
        changelog_index,
        &leaf1,
        &new_leaf1,
        0,
        proof.clone(),
    );
    merkle_tree
        .update(changelog_index, &leaf1, &new_leaf1, 0, &mut proof)
        .unwrap();
    reference_tree.update(&new_leaf1, 0).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = ChangelogPath([Some(new_leaf1), Some(h1), Some(h3), Some(h4)]);

    let canopy_levels = [
        &[h4, H::zero_bytes()[3]][..],
        &[
            h3,
            H::zero_bytes()[2],
            H::zero_bytes()[2],
            H::zero_bytes()[2],
        ][..],
    ];
    let mut expected_canopy = Vec::new();
    for canopy_level in canopy_levels.iter().take(CANOPY) {
        expected_canopy.extend_from_slice(canopy_level);
    }

    assert_eq!(merkle_tree.changelog_index(), 5 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 0),
    );

    assert_eq!(merkle_tree.root(), reference_tree.root());
    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 5);
    assert_eq!(merkle_tree.next_index(), 4);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf4);
    assert_eq!(
        merkle_tree.canopy,
        BoundedVec::from_slice(reference_tree.get_canopy().unwrap().as_slice())
    );
    assert_eq!(merkle_tree.canopy.as_slice(), expected_canopy.as_slice());

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
    let changelog_index = merkle_tree.changelog_index();

    let proof_raw = &[new_leaf1, h2, H::zero_bytes()[2], H::zero_bytes()[3]];
    let mut proof = BoundedVec::with_capacity(HEIGHT);
    for node in &proof_raw[..HEIGHT - CANOPY] {
        proof.push(*node).unwrap();
    }

    invalid_updates::<H, HEIGHT, CHANGELOG>(
        &mut rng,
        &mut merkle_tree,
        changelog_index,
        &leaf2,
        &new_leaf2,
        1,
        proof.clone(),
    );
    merkle_tree
        .update(changelog_index, &leaf2, &new_leaf2, 1, &mut proof)
        .unwrap();
    reference_tree.update(&new_leaf2, 1).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = ChangelogPath([Some(new_leaf2), Some(h1), Some(h3), Some(h4)]);

    let canopy_levels = [
        &[h4, H::zero_bytes()[3]][..],
        &[
            h3,
            H::zero_bytes()[2],
            H::zero_bytes()[2],
            H::zero_bytes()[2],
        ][..],
    ];
    let mut expected_canopy = Vec::new();
    for canopy_level in canopy_levels.iter().take(CANOPY) {
        expected_canopy.extend_from_slice(canopy_level);
    }

    assert_eq!(merkle_tree.changelog_index(), 6 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 1),
    );

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 6);
    assert_eq!(merkle_tree.next_index(), 4);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf4);
    assert_eq!(
        merkle_tree.canopy,
        BoundedVec::from_slice(reference_tree.get_canopy().unwrap().as_slice())
    );
    assert_eq!(merkle_tree.canopy.as_slice(), expected_canopy.as_slice());

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
    let changelog_index = merkle_tree.changelog_index();

    let proof_raw = &[leaf4, h1, H::zero_bytes()[2], H::zero_bytes()[3]];
    let mut proof = BoundedVec::with_capacity(HEIGHT);
    for node in &proof_raw[..HEIGHT - CANOPY] {
        proof.push(*node).unwrap();
    }

    invalid_updates::<H, HEIGHT, CHANGELOG>(
        &mut rng,
        &mut merkle_tree,
        changelog_index,
        &leaf3,
        &new_leaf3,
        2,
        proof.clone(),
    );
    merkle_tree
        .update(changelog_index, &leaf3, &new_leaf3, 2, &mut proof)
        .unwrap();
    reference_tree.update(&new_leaf3, 2).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = ChangelogPath([Some(new_leaf3), Some(h2), Some(h3), Some(h4)]);

    let canopy_levels = [
        &[h4, H::zero_bytes()[3]][..],
        &[
            h3,
            H::zero_bytes()[2],
            H::zero_bytes()[2],
            H::zero_bytes()[2],
        ][..],
    ];
    let mut expected_canopy = Vec::new();
    for canopy_level in canopy_levels.iter().take(CANOPY) {
        expected_canopy.extend_from_slice(canopy_level);
    }

    assert_eq!(merkle_tree.changelog_index(), 7 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 2)
    );

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 7);
    assert_eq!(merkle_tree.next_index(), 4);
    assert_eq!(merkle_tree.rightmost_leaf(), leaf4);
    assert_eq!(
        merkle_tree.canopy,
        BoundedVec::from_slice(reference_tree.get_canopy().unwrap().as_slice())
    );
    assert_eq!(merkle_tree.canopy.as_slice(), expected_canopy.as_slice());

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
    let changelog_index = merkle_tree.changelog_index();

    let proof_raw = &[new_leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];
    let mut proof = BoundedVec::with_capacity(HEIGHT);
    for node in &proof_raw[..HEIGHT - CANOPY] {
        proof.push(*node).unwrap();
    }

    invalid_updates::<H, HEIGHT, CHANGELOG>(
        &mut rng,
        &mut merkle_tree,
        changelog_index,
        &leaf4,
        &new_leaf4,
        3,
        proof.clone(),
    );
    merkle_tree
        .update(changelog_index, &leaf4, &new_leaf4, 3, &mut proof)
        .unwrap();
    reference_tree.update(&new_leaf4, 3).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &new_leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = ChangelogPath([Some(new_leaf4), Some(h2), Some(h3), Some(h4)]);

    let canopy_levels = [
        &[h4, H::zero_bytes()[3]][..],
        &[
            h3,
            H::zero_bytes()[2],
            H::zero_bytes()[2],
            H::zero_bytes()[2],
        ][..],
    ];
    let mut expected_canopy = Vec::new();
    for canopy_level in canopy_levels.iter().take(CANOPY) {
        expected_canopy.extend_from_slice(canopy_level);
    }

    assert_eq!(merkle_tree.changelog_index(), 8 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_changelog_path, 3)
    );

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(merkle_tree.roots.last_index(), 8);
    assert_eq!(merkle_tree.next_index(), 4);
    assert_eq!(merkle_tree.rightmost_leaf(), new_leaf4);
    assert_eq!(
        merkle_tree.canopy,
        BoundedVec::from_slice(reference_tree.get_canopy().unwrap().as_slice())
    );
    assert_eq!(merkle_tree.canopy.as_slice(), expected_canopy.as_slice());
}

/// Tests whether appending leaves over the limit results in an explicit error.
fn overfill_tree<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 2;
    const CHANGELOG: usize = 32;
    const ROOTS: usize = 32;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    for _ in 0..4 {
        merkle_tree.append(&[4; 32]).unwrap();
    }
    assert!(matches!(
        merkle_tree.append(&[4; 32]),
        Err(ConcurrentMerkleTreeError::TreeIsFull)
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
    const CANOPY: usize = 0;

    // Our implementation of concurrent Merkle tree.
    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    // Reference implementation of Merkle tree which Solana Labs uses for
    // testing (and therefore, we as well). We use it mostly to get the Merkle
    // proofs.
    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    // Fill up the tree, producing 4 roots and changelog entries.
    for _ in 0..(1 << HEIGHT) {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        merkle_tree.append(&leaf).unwrap();
        reference_tree.append(&leaf).unwrap();
    }

    assert_eq!(merkle_tree.changelog.last_index(), 4);
    assert_eq!(merkle_tree.roots.last_index(), 4);

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
        let mut proof = BoundedVec::from_slice(
            reference_tree
                .get_proof_of_leaf(i, false)
                .unwrap()
                .as_slice(),
        );

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.changelog.last_index(), 0);
    assert_eq!(merkle_tree.roots.last_index(), 6);

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
        let mut proof = BoundedVec::from_slice(
            reference_tree
                .get_proof_of_leaf(i, false)
                .unwrap()
                .as_slice(),
        );

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.changelog.last_index(), 2);
    assert_eq!(merkle_tree.roots.last_index(), 0);

    // The latter updates should keep incrementing the counters.
    for i in 0..3 {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        let changelog_index = merkle_tree.changelog_index();
        let old_leaf = reference_tree.leaf(i);
        let mut proof = BoundedVec::from_slice(
            reference_tree
                .get_proof_of_leaf(i, false)
                .unwrap()
                .as_slice(),
        );

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.changelog.last_index(), 5);
    assert_eq!(merkle_tree.roots.last_index(), 3);
}

/// Checks whether `append_batch` is compatible with equivalent multiple
/// appends.
fn compat_batch<H, const HEIGHT: usize, const CANOPY: usize>()
where
    H: Hasher,
{
    const CHANGELOG: usize = 64;
    const ROOTS: usize = 256;

    let mut rng = thread_rng();

    let batch_limit = cmp::min(1 << HEIGHT, CHANGELOG);
    for batch_size in 1..batch_limit {
        let mut concurrent_mt_1 =
            ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
        concurrent_mt_1.init().unwrap();

        // Tree to which are going to append single leaves.
        let mut concurrent_mt_2 =
            ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
        concurrent_mt_2.init().unwrap();

        // Reference tree for checking the correctness of proofs.
        let mut reference_mt = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

        let leaves: Vec<[u8; 32]> = (0..batch_size)
            .map(|_| {
                Fr::rand(&mut rng)
                    .into_bigint()
                    .to_bytes_be()
                    .try_into()
                    .unwrap()
            })
            .collect();
        let leaves: Vec<&[u8; 32]> = leaves.iter().collect();

        // Append leaves to all Merkle tree implementations.

        // Batch append.
        concurrent_mt_1.append_batch(leaves.as_slice()).unwrap();

        // Singular appends.
        for leaf in leaves.iter() {
            concurrent_mt_2.append(leaf).unwrap();
        }

        // Singular appends to reference MT.
        for leaf in leaves.iter() {
            reference_mt.append(leaf).unwrap();
        }

        // Check whether roots are the same.
        // Skip roots which are an output of singular, non-terminal
        // appends - we don't compute them in batch appends and instead,
        // emit a "zero root" (just to appease the clients assuming that
        // root index is equal to sequence number).
        assert_eq!(
            concurrent_mt_1
                .roots
                .iter()
                .step_by(batch_size)
                .collect::<Vec<_>>()
                .as_slice(),
            concurrent_mt_2
                .roots
                .iter()
                .step_by(batch_size)
                .collect::<Vec<_>>()
                .as_slice()
        );
        assert_eq!(concurrent_mt_1.root(), reference_mt.root());
        assert_eq!(concurrent_mt_2.root(), reference_mt.root());
    }
}

fn batch_greater_than_changelog<H, const HEIGHT: usize, const CANOPY: usize>()
where
    H: Hasher,
{
    const CHANGELOG: usize = 64;
    const ROOTS: usize = 256;

    let mut rng = thread_rng();

    let mut concurrent_mt =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    concurrent_mt.init().unwrap();

    for batch_size in (CHANGELOG + 1)..(1 << HEIGHT) {
        let leaves: Vec<[u8; 32]> = (0..batch_size)
            .map(|_| {
                Fr::rand(&mut rng)
                    .into_bigint()
                    .to_bytes_be()
                    .try_into()
                    .unwrap()
            })
            .collect();
        let leaves: Vec<&[u8; 32]> = leaves.iter().collect();

        assert!(matches!(
            concurrent_mt.append_batch(leaves.as_slice()),
            Err(ConcurrentMerkleTreeError::BatchGreaterThanChangelog(_, _)),
        ));
    }
}

fn compat_canopy<H, const HEIGHT: usize>()
where
    H: Hasher,
{
    const CHANGELOG: usize = 64;
    const ROOTS: usize = 256;

    let mut rng = thread_rng();

    for canopy_depth in 1..(HEIGHT + 1) {
        let batch_limit = cmp::min(1 << HEIGHT, CHANGELOG);
        for batch_size in 1..batch_limit {
            let mut concurrent_mt_with_canopy =
                ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, canopy_depth)
                    .unwrap();
            concurrent_mt_with_canopy.init().unwrap();

            let mut concurrent_mt_without_canopy =
                ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, 0).unwrap();
            concurrent_mt_without_canopy.init().unwrap();

            let mut reference_mt_with_canopy =
                light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, canopy_depth);
            let mut reference_mt_without_canopy =
                light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, 0);

            for batch_i in 0..((1 << HEIGHT) / batch_size) {
                let leaves: Vec<[u8; 32]> = (0..batch_size)
                    .map(|_| {
                        Fr::rand(&mut rng)
                            .into_bigint()
                            .to_bytes_be()
                            .try_into()
                            .unwrap()
                    })
                    .collect();
                let leaves: Vec<&[u8; 32]> = leaves.iter().collect();

                concurrent_mt_with_canopy
                    .append_batch(leaves.as_slice())
                    .unwrap();
                concurrent_mt_without_canopy
                    .append_batch(leaves.as_slice())
                    .unwrap();

                for leaf in leaves {
                    reference_mt_with_canopy.append(leaf).unwrap();
                    reference_mt_without_canopy.append(leaf).unwrap();
                }

                for leaf_i in 0..batch_size {
                    let leaf_index = (batch_i * batch_size) + leaf_i;

                    let inter_proof_with_canopy = reference_mt_with_canopy
                        .get_proof_of_leaf(leaf_index, false)
                        .unwrap();
                    let mut proof_with_canopy = BoundedVec::with_capacity(HEIGHT);
                    for node in inter_proof_with_canopy.iter() {
                        proof_with_canopy.push(*node).unwrap();
                    }
                    let proof_without_canopy = BoundedVec::from_slice(
                        reference_mt_without_canopy
                            .get_proof_of_leaf(leaf_index, true)
                            .unwrap()
                            .as_slice(),
                    );

                    assert_eq!(
                        proof_with_canopy[..],
                        proof_without_canopy[..HEIGHT - canopy_depth]
                    );

                    concurrent_mt_with_canopy
                        .update_proof_from_canopy(leaf_index, &mut proof_with_canopy)
                        .unwrap();

                    assert_eq!(proof_with_canopy, proof_without_canopy)
                }
            }
        }
    }
}

#[test]
fn test_append_keccak_canopy_0() {
    append::<Keccak, 0>()
}

#[test]
fn test_append_poseidon_canopy_0() {
    append::<Poseidon, 0>()
}

#[test]
fn test_append_sha256_canopy_0() {
    append::<Sha256, 0>()
}

#[test]
fn test_append_with_proof_keccak_4_16_16_0_16() {
    append_with_proof::<Keccak, 4, 16, 16, 0, 16>()
}

#[test]
fn test_append_with_proof_poseidon_4_16_16_0_16() {
    append_with_proof::<Poseidon, 4, 16, 16, 0, 16>()
}

#[test]
fn test_append_with_proof_sha256_4_16_16_0_16() {
    append_with_proof::<Sha256, 4, 16, 16, 0, 16>()
}

#[test]
fn test_append_with_proof_keccak_26_1400_2800_0_200() {
    append_with_proof::<Keccak, 26, 1400, 2800, 0, 200>()
}

#[test]
fn test_append_with_proof_poseidon_26_1400_2800_0_200() {
    append_with_proof::<Poseidon, 26, 1400, 2800, 0, 200>()
}

#[test]
fn test_append_with_proof_sha256_26_1400_2800_0_200() {
    append_with_proof::<Sha256, 26, 1400, 2800, 0, 200>()
}

#[test]
fn test_append_with_proof_keccak_26_1400_2800_10_200() {
    append_with_proof::<Keccak, 26, 1400, 2800, 10, 200>()
}

#[test]
fn test_append_with_proof_poseidon_26_1400_2800_10_200() {
    append_with_proof::<Poseidon, 26, 1400, 2800, 10, 200>()
}

#[test]
fn test_append_with_proof_sha256_26_1400_2800_10_200() {
    append_with_proof::<Sha256, 26, 1400, 2800, 10, 200>()
}

#[test]
fn test_update_keccak_height_4_changelog_1_roots_256_canopy_0() {
    update::<Keccak, 1, 256, 0>()
}

#[test]
fn test_update_keccak_height_4_changelog_1_roots_256_canopy_1() {
    update::<Keccak, 1, 256, 1>()
}

#[test]
fn test_update_keccak_height_4_changelog_1_roots_256_canopy_2() {
    update::<Keccak, 1, 256, 2>()
}

#[test]
fn test_update_keccak_height_4_changelog_32_roots_256_canopy_0() {
    update::<Keccak, 32, 256, 0>()
}

#[test]
fn test_update_keccak_height_4_changelog_32_roots_256_canopy_1() {
    update::<Keccak, 32, 256, 1>()
}

#[test]
fn test_update_keccak_height_4_changelog_32_roots_256_canopy_2() {
    update::<Keccak, 32, 256, 2>()
}

#[test]
fn test_update_poseidon_height_4_changelog_1_roots_256_canopy_0() {
    update::<Poseidon, 1, 256, 0>()
}

#[test]
fn test_update_poseidon_height_4_changelog_1_roots_256_canopy_1() {
    update::<Poseidon, 1, 256, 1>()
}

#[test]
fn test_update_poseidon_height_4_changelog_1_roots_256_canopy_2() {
    update::<Poseidon, 1, 256, 2>()
}

#[test]
fn test_update_poseidon_height_4_changelog_32_roots_256_canopy_0() {
    update::<Poseidon, 32, 256, 0>()
}

#[test]
fn test_update_poseidon_height_4_changelog_32_roots_256_canopy_1() {
    update::<Poseidon, 32, 256, 1>()
}

#[test]
fn test_update_poseidon_height_4_changelog_32_roots_256_canopy_2() {
    update::<Poseidon, 32, 256, 2>()
}

#[test]
fn test_update_sha256_height_4_changelog_32_roots_256_canopy_0() {
    update::<Sha256, 32, 256, 0>()
}

#[test]
fn test_update_sha256_height_4_changelog_32_roots_256_canopy_1() {
    update::<Sha256, 32, 256, 0>()
}

#[test]
fn test_update_sha256_height_4_changelog_32_roots_256_canopy_2() {
    update::<Sha256, 32, 256, 0>()
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
fn test_compat_batch_keccak_8_canopy_0() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 0;
    compat_batch::<Keccak, HEIGHT, CANOPY>()
}

#[test]
fn test_compat_batch_poseidon_3_canopy_0() {
    const HEIGHT: usize = 3;
    const CANOPY: usize = 0;
    compat_batch::<Poseidon, HEIGHT, CANOPY>()
}

#[test]
fn test_compat_batch_poseidon_6_canopy_0() {
    const HEIGHT: usize = 6;
    const CANOPY: usize = 0;
    compat_batch::<Poseidon, HEIGHT, CANOPY>()
}

#[test]
fn test_compat_batch_sha256_8_canopy_0() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 0;
    compat_batch::<Sha256, HEIGHT, CANOPY>()
}

#[cfg(feature = "heavy-tests")]
#[test]
fn test_compat_batch_keccak_16() {
    const HEIGHT: usize = 16;
    const CANOPY: usize = 0;
    compat_batch::<Keccak, HEIGHT, CANOPY>()
}

#[cfg(feature = "heavy-tests")]
#[test]
fn test_compat_batch_poseidon_16() {
    const HEIGHT: usize = 16;
    const CANOPY: usize = 0;
    compat_batch::<Poseidon, HEIGHT, CANOPY>()
}

#[cfg(feature = "heavy-tests")]
#[test]
fn test_compat_batch_sha256_16() {
    const HEIGHT: usize = 16;
    const CANOPY: usize = 0;
    compat_batch::<Sha256, HEIGHT, CANOPY>()
}

#[test]
fn test_batch_greater_than_changelog_keccak_8_canopy_0() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 0;
    batch_greater_than_changelog::<Keccak, HEIGHT, CANOPY>()
}

#[test]
fn test_batch_greater_than_changelog_poseidon_8_canopy_0() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 0;
    batch_greater_than_changelog::<Poseidon, HEIGHT, CANOPY>()
}

#[test]
fn test_batch_greater_than_changelog_sha256_8_canopy_0() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 0;
    batch_greater_than_changelog::<Sha256, HEIGHT, CANOPY>()
}

#[test]
fn test_batch_greater_than_changelog_keccak_8_canopy_4() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 4;
    batch_greater_than_changelog::<Keccak, HEIGHT, CANOPY>()
}

#[test]
fn test_batch_greater_than_changelog_poseidon_6_canopy_3() {
    const HEIGHT: usize = 6;
    const CANOPY: usize = 3;
    batch_greater_than_changelog::<Poseidon, HEIGHT, CANOPY>()
}

#[test]
fn test_batch_greater_than_changelog_sha256_8_canopy_4() {
    const HEIGHT: usize = 8;
    const CANOPY: usize = 4;
    batch_greater_than_changelog::<Sha256, HEIGHT, CANOPY>()
}

#[test]
fn test_compat_canopy_keccak_8() {
    const HEIGHT: usize = 8;
    compat_canopy::<Keccak, HEIGHT>()
}

#[test]
fn test_compat_canopy_poseidon_6() {
    const HEIGHT: usize = 6;
    compat_canopy::<Poseidon, HEIGHT>()
}

#[cfg(feature = "heavy-tests")]
#[test]
fn test_compat_canopy_poseidon_26() {
    const HEIGHT: usize = 26;
    compat_canopy::<Poseidon, HEIGHT>()
}

#[test]
fn test_compat_canopy_sha256_8() {
    const HEIGHT: usize = 8;
    compat_canopy::<Sha256, HEIGHT>()
}

// /// Compares the internal fields of concurrent Merkle tree implementations, to
// /// ensure their consistency.
// fn compare_trees<H, const HEIGHT: usize, const MAX_ROOTS: usize>(
//     concurrent_mt: &ConcurrentMerkleTree<H, HEIGHT>,
//     spl_concurrent_mt: &spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree<
//         HEIGHT,
//         MAX_ROOTS,
//     >,
// ) where
//     H: Hasher,
// {
//     for i in 0..concurrent_mt.changelog.len() {
//         let changelog_entry = concurrent_mt.changelog[i].clone();
//         let spl_changelog_entry = spl_concurrent_mt.change_logs[i];
//         for j in 0..HEIGHT {
//             let changelog_node = changelog_entry.path[j].unwrap();
//             let spl_changelog_node = spl_changelog_entry.path[j];
//             assert_eq!(changelog_node, spl_changelog_node);
//         }
//         assert_eq!(changelog_entry.index, spl_changelog_entry.index as u64);
//     }
//     assert_eq!(
//         concurrent_mt.changelog.last_index(),
//         spl_concurrent_mt.active_index as usize
//     );
//     assert_eq!(concurrent_mt.root(), spl_concurrent_mt.get_root());
//     for i in 0..concurrent_mt.roots.len() {
//         assert_eq!(
//             concurrent_mt.roots[i],
//             spl_concurrent_mt.change_logs[i].root
//         );
//     }
//     assert_eq!(
//         concurrent_mt.roots.last_index(),
//         spl_concurrent_mt.active_index as usize
//     );
//     assert_eq!(
//         concurrent_mt.next_index(),
//         spl_concurrent_mt.rightmost_proof.index as usize
//     );
//     assert_eq!(
//         concurrent_mt.rightmost_leaf(),
//         spl_concurrent_mt.rightmost_proof.leaf
//     );
// }

// /// Checks whether our `append` and `update` implementations are compatible
// /// with `append` and `set_leaf` from `spl-concurrent-merkle-tree` crate.
// #[tokio::test(flavor = "multi_thread")]
// async fn test_spl_compat() {
//     const HEIGHT: usize = 4;
//     const CHANGELOG: usize = 64;
//     const ROOTS: usize = 256;
//     const CANOPY: usize = 0;

//     let mut rng = thread_rng();

//     // Our implementation of concurrent Merkle tree.
//     let mut concurrent_mt =
//         ConcurrentMerkleTree::<Keccak, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
//     concurrent_mt.init().unwrap();

//     // Solana Labs implementation of concurrent Merkle tree.
//     let mut spl_concurrent_mt = spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<HEIGHT, ROOTS>::new();
//     spl_concurrent_mt.initialize().unwrap();

//     // Reference implemenetation of Merkle tree which Solana Labs uses for
//     // testing (and therefore, we as well). We use it mostly to get the Merkle
//     // proofs.
//     let mut reference_tree = light_merkle_tree_reference::MerkleTree::<Keccak>::new(HEIGHT, CANOPY);

//     for i in 0..(1 << HEIGHT) {
//         let leaf: [u8; 32] = Fr::rand(&mut rng)
//             .into_bigint()
//             .to_bytes_be()
//             .try_into()
//             .unwrap();

//         concurrent_mt.append(&leaf).unwrap();
//         spl_concurrent_mt.append(leaf).unwrap();
//         reference_tree.append(&leaf).unwrap();

//         compare_trees(&concurrent_mt, &spl_concurrent_mt);

//         // For every appended leaf with index greater than 0, update the leaf 0.
//         // This is done in indexed Merkle trees[0] and it's a great test case
//         // for rightmost proof updates.
//         //
//         // [0] https://docs.aztec.network/aztec/concepts/storage/trees/indexed_merkle_tree
//         if i > 0 {
//             let new_leaf: [u8; 32] = Fr::rand(&mut rng)
//                 .into_bigint()
//                 .to_bytes_be()
//                 .try_into()
//                 .unwrap();

//             let root = concurrent_mt.root();
//             let changelog_index = concurrent_mt.changelog_index();
//             let old_leaf = reference_tree.leaf(0);
//             let mut proof = BoundedVec::from_slice(
//                 reference_tree
//                     .get_proof_of_leaf(0, false)
//                     .unwrap()
//                     .as_slice(),
//             );

//             concurrent_mt
//                 .update(changelog_index, &old_leaf, &new_leaf, 0, &mut proof)
//                 .unwrap();
//             spl_concurrent_mt
//                 .set_leaf(root, old_leaf, new_leaf, proof.as_slice(), 0_u32)
//                 .unwrap();
//             reference_tree.update(&new_leaf, 0).unwrap();

//             compare_trees(&concurrent_mt, &spl_concurrent_mt);
//         }
//     }

//     for i in 0..(1 << HEIGHT) {
//         let new_leaf: [u8; 32] = Fr::rand(&mut rng)
//             .into_bigint()
//             .to_bytes_be()
//             .try_into()
//             .unwrap();

//         let root = concurrent_mt.root();
//         let changelog_index = concurrent_mt.changelog_index();
//         let old_leaf = reference_tree.leaf(i);
//         let mut proof = BoundedVec::from_slice(
//             reference_tree
//                 .get_proof_of_leaf(i, false)
//                 .unwrap()
//                 .as_slice(),
//         );

//         concurrent_mt
//             .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
//             .unwrap();
//         spl_concurrent_mt
//             .set_leaf(root, old_leaf, new_leaf, proof.as_slice(), i as u32)
//             .unwrap();
//         reference_tree.update(&new_leaf, i).unwrap();

//         compare_trees(&concurrent_mt, &spl_concurrent_mt);
//     }
// }

fn from_bytes<
    H,
    const HEIGHT: usize,
    const CHANGELOG: usize,
    const ROOTS: usize,
    const CANOPY: usize,
>()
where
    H: Hasher,
{
    let mut bytes =
        vec![
            0u8;
            ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(HEIGHT, CHANGELOG, ROOTS, CANOPY)
        ];

    let mut rng = thread_rng();
    let mut reference_tree_1 = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    // Vector of changelog indices after each operation.
    let mut leaf_indices = CyclicBoundedVec::with_capacity(CHANGELOG);
    // Vector of roots after each operation.
    let mut roots = CyclicBoundedVec::with_capacity(CHANGELOG);
    // Vector of merkle paths we get from the reference tree after each operation.
    let mut merkle_paths = CyclicBoundedVec::with_capacity(CHANGELOG);
    // Changelog is always initialized with a changelog path consisting of zero
    // bytes. For consistency, we need to assert the 1st zero byte as the first
    // expected leaf in the changelog.
    let merkle_path = reference_tree_1.get_path_of_leaf(0, true).unwrap();
    leaf_indices.push(0);
    merkle_paths.push(merkle_path);

    {
        let mut merkle_tree =
            ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::from_bytes_zero_copy_init(
                bytes.as_mut_slice(),
                HEIGHT,
                CANOPY,
                CHANGELOG,
                ROOTS,
            )
            .unwrap();
        merkle_tree.init().unwrap();
        roots.push(merkle_tree.root());
    }

    let mut reference_tree_2 =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    reference_tree_2.init().unwrap();

    // Try to make the tree full. After each append, update a random leaf.
    // Reload the tree from bytes after each action.
    for _ in 0..(1 << HEIGHT) {
        // Reload the tree.
        let mut merkle_tree =
            ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::from_bytes_zero_copy_mut(
                bytes.as_mut_slice(),
            )
            .unwrap();

        // Append leaf.
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        let leaf_index = merkle_tree.next_index();
        merkle_tree.append(&leaf).unwrap();
        reference_tree_1.append(&leaf).unwrap();
        reference_tree_2.append(&leaf).unwrap();

        leaf_indices.push(leaf_index);
        roots.push(merkle_tree.root());
        let merkle_path = reference_tree_1.get_path_of_leaf(leaf_index, true).unwrap();
        merkle_paths.push(merkle_path);

        assert_eq!(
            merkle_tree.filled_subtrees.iter().collect::<Vec<_>>(),
            reference_tree_2.filled_subtrees.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            merkle_tree.changelog.iter().collect::<Vec<_>>(),
            reference_tree_2.changelog.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            merkle_tree.roots.iter().collect::<Vec<_>>(),
            reference_tree_2.roots.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            merkle_tree.canopy.iter().collect::<Vec<_>>(),
            reference_tree_2.canopy.iter().collect::<Vec<_>>()
        );
        assert_eq!(merkle_tree.root(), reference_tree_1.root());

        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog.first_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(changelog_entries.len(), merkle_paths.len() - 1);

        for ((leaf_index, merkle_path), changelog_entry) in leaf_indices
            .iter()
            .skip(1)
            .zip(merkle_paths.iter().skip(1))
            .zip(changelog_entries)
        {
            assert_eq!(changelog_entry.index, *leaf_index as u64);
            for (i, path_node) in merkle_path.iter().enumerate() {
                let changelog_node = changelog_entry.path[i].unwrap();
                assert_eq!(changelog_node, *path_node);
            }
        }

        for (root_1, root_2) in merkle_tree.roots.iter().zip(roots.iter()) {
            assert_eq!(root_1, root_2);
        }

        // Update random leaf.
        let leaf_index = rng.gen_range(0..reference_tree_1.leaves().len());
        let old_leaf = reference_tree_1.leaf(leaf_index);
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        let mut proof = BoundedVec::from_slice(
            reference_tree_1
                .get_proof_of_leaf(leaf_index, false)
                .unwrap()
                .as_slice(),
        );
        let changelog_index = merkle_tree.changelog_index();
        merkle_tree
            .update(
                changelog_index,
                &old_leaf,
                &new_leaf,
                leaf_index,
                &mut proof,
            )
            .unwrap();
        reference_tree_1.update(&new_leaf, leaf_index).unwrap();
        reference_tree_2
            .update(
                changelog_index,
                &old_leaf,
                &new_leaf,
                leaf_index,
                &mut proof,
            )
            .unwrap();

        assert_eq!(merkle_tree.root(), reference_tree_1.root());

        leaf_indices.push(leaf_index);
        roots.push(merkle_tree.root());
        let merkle_path = reference_tree_1.get_path_of_leaf(leaf_index, true).unwrap();
        merkle_paths.push(merkle_path);

        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog.first_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(changelog_entries.len(), merkle_paths.len() - 1);

        for ((leaf_index, merkle_path), changelog_entry) in leaf_indices
            .iter()
            .skip(1)
            .zip(merkle_paths.iter().skip(1))
            .zip(changelog_entries)
        {
            assert_eq!(changelog_entry.index, *leaf_index as u64);
            for (i, path_node) in merkle_path.iter().enumerate() {
                let changelog_node = changelog_entry.path[i].unwrap();
                assert_eq!(changelog_node, *path_node);
            }
        }

        for (root_1, root_2) in merkle_tree.roots.iter().zip(roots.iter()) {
            assert_eq!(root_1, root_2);
        }
    }

    // Keep updating random leaves in loop.
    for _ in 0..1000 {
        // Reload the tree.
        let mut merkle_tree =
            ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::from_bytes_zero_copy_mut(
                bytes.as_mut_slice(),
            )
            .unwrap();

        // Update random leaf.
        let leaf_index = rng.gen_range(0..reference_tree_1.leaves().len());
        let old_leaf = reference_tree_1.leaf(leaf_index);
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        let mut proof = BoundedVec::from_slice(
            reference_tree_1
                .get_proof_of_leaf(leaf_index, false)
                .unwrap()
                .as_slice(),
        );
        let changelog_index = merkle_tree.changelog_index();
        merkle_tree
            .update(
                changelog_index,
                &old_leaf,
                &new_leaf,
                leaf_index,
                &mut proof,
            )
            .unwrap();
        reference_tree_1.update(&new_leaf, leaf_index).unwrap();
        reference_tree_2
            .update(
                changelog_index,
                &old_leaf,
                &new_leaf,
                leaf_index,
                &mut proof,
            )
            .unwrap();

        assert_eq!(merkle_tree.root(), reference_tree_1.root());

        leaf_indices.push(leaf_index);
        roots.push(merkle_tree.root());
        let merkle_path = reference_tree_1.get_path_of_leaf(leaf_index, true).unwrap();
        merkle_paths.push(merkle_path);

        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog.first_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(changelog_entries.len(), merkle_paths.len() - 1);

        for ((leaf_index, merkle_path), changelog_entry) in leaf_indices
            .iter()
            .skip(1)
            .zip(merkle_paths.iter().skip(1))
            .zip(changelog_entries)
        {
            assert_eq!(changelog_entry.index, *leaf_index as u64);
            for (i, path_node) in merkle_path.iter().enumerate() {
                let changelog_node = changelog_entry.path[i].unwrap();
                assert_eq!(changelog_node, *path_node);
            }
        }

        for (root_1, root_2) in merkle_tree.roots.iter().zip(roots.iter()) {
            assert_eq!(root_1, root_2);
        }
    }
}

#[test]
fn test_from_bytes_keccak_8_256_256() {
    const HEIGHT: usize = 8;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    from_bytes::<Keccak, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_from_bytes_poseidon_8_256_256() {
    const HEIGHT: usize = 8;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    from_bytes::<Poseidon, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_from_bytes_sha256_8_256_256_0() {
    const HEIGHT: usize = 8;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    from_bytes::<Sha256, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

/// Tests the buffer size checks. Buffer size checks should fail any time that
/// a provided byte slice is smaller than the expected size indicated by the
/// tree metadata (height, changelog size, roots size etc.).
///
/// In case of `from_bytes_zero_copy_init`, the metadata are provided with an
/// intention of initializing them. The provided parameters influence the
/// size checks.
///
/// In case of `from_bytes_zero_copy_mut`, the metadata are read from the
/// buffer. Therefore, we end up with two phases of checks:
///
/// 1. Check of the non-dynamic fields, including the metadata structs.
///    Based on size of all non-dynamic fields of `ConcurrentMerkleTree`.
/// 2. If the check was successful, metadata are being read from the buffer.
/// 3. After reading the metadata, we check the buffer size again, now to the
///    full extent, before actually using it.
fn buffer_error<
    H,
    const HEIGHT: usize,
    const CHANGELOG: usize,
    const ROOTS: usize,
    const CANOPY: usize,
>()
where
    H: Hasher,
{
    let valid_size =
        ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(HEIGHT, CHANGELOG, ROOTS, CANOPY);

    // Check that `from_bytes_zero_copy_init` checks the bounds.
    for invalid_size in 1..valid_size {
        let mut bytes = vec![0u8; invalid_size];
        let res = ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::from_bytes_zero_copy_init(
            &mut bytes, HEIGHT, CANOPY, CHANGELOG, ROOTS,
        );
        assert!(matches!(
            res,
            Err(ConcurrentMerkleTreeError::BufferSize(_, _))
        ));
    }

    // Initialize the tree correctly.
    let mut bytes = vec![0u8; valid_size];
    ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::from_bytes_zero_copy_init(
        &mut bytes, HEIGHT, CANOPY, CHANGELOG, ROOTS,
    )
    .unwrap();

    // Check that `from_bytes_zero_copy` mut checks the bounds based on the
    // metadata in already existing Merkle tree.
    for invalid_size in 1..valid_size {
        let bytes = &mut bytes[..invalid_size];
        let res = ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::from_bytes_zero_copy_mut(bytes);
        assert!(matches!(
            res,
            Err(ConcurrentMerkleTreeError::BufferSize(_, _))
        ));
    }
}

#[test]
fn test_buffer_error_keccak_8_256_256() {
    const HEIGHT: usize = 8;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    buffer_error::<Keccak, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_buffer_error_poseidon_8_256_256() {
    const HEIGHT: usize = 8;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    buffer_error::<Poseidon, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_buffer_error_sha256_8_256_256_0() {
    const HEIGHT: usize = 8;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    buffer_error::<Sha256, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

fn height_zero<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 0;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let res = ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY);
    assert!(matches!(res, Err(ConcurrentMerkleTreeError::HeightZero)));
}

#[test]
fn test_height_zero_keccak() {
    height_zero::<Keccak>()
}

#[test]
fn test_height_zero_poseidon() {
    height_zero::<Poseidon>()
}

#[test]
fn test_height_zero_sha256() {
    height_zero::<Sha256>()
}

fn changelog_zero<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 0;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let res = ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY);
    assert!(matches!(res, Err(ConcurrentMerkleTreeError::ChangelogZero)));
}

#[test]
fn test_changelog_zero_keccak() {
    changelog_zero::<Keccak>()
}

#[test]
fn test_changelog_zero_poseidon() {
    changelog_zero::<Poseidon>()
}

#[test]
fn test_changelog_zero_sha256() {
    changelog_zero::<Sha256>()
}

fn roots_zero<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 0;
    const CANOPY: usize = 0;

    let res = ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY);
    assert!(matches!(res, Err(ConcurrentMerkleTreeError::RootsZero)));
}

#[test]
fn test_roots_zero_keccak() {
    roots_zero::<Keccak>()
}

#[test]
fn test_roots_zero_poseidon() {
    roots_zero::<Poseidon>()
}

#[test]
fn test_roots_zero_sha256() {
    roots_zero::<Sha256>()
}

fn update_with_invalid_proof<H, const HEIGHT: usize>(
    merkle_tree: &mut ConcurrentMerkleTree<H, HEIGHT>,
    proof_len: usize,
) where
    H: Hasher,
{
    // It doesn't matter what values do we use. The proof length check
    // should happend before checking its correctness.
    let mut proof = BoundedVec::from_slice(vec![[5u8; 32]; proof_len].as_slice());

    let res = merkle_tree.update(
        merkle_tree.changelog_index(),
        &H::zero_bytes()[0],
        &[4u8; 32],
        0,
        &mut proof,
    );
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::InvalidProofLength(_, _))
    ))
}

fn invalid_proof_len<H, const HEIGHT: usize, const CANOPY: usize>()
where
    H: Hasher,
{
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    // Proof sizes lower than `height - canopy`.
    for proof_len in 0..(HEIGHT - CANOPY) {
        update_with_invalid_proof(&mut merkle_tree, proof_len);
    }
    // Proof sizes greater than `height - canopy`.
    for proof_len in (HEIGHT - CANOPY + 1)..256 {
        update_with_invalid_proof(&mut merkle_tree, proof_len);
    }
}

#[test]
fn test_invalid_proof_len_keccak_height_26_canopy_0() {
    invalid_proof_len::<Keccak, 26, 0>()
}

#[test]
fn test_invalid_proof_len_keccak_height_26_canopy_10() {
    invalid_proof_len::<Keccak, 26, 10>()
}

#[test]
fn test_invalid_proof_len_poseidon_height_26_canopy_0() {
    invalid_proof_len::<Poseidon, 26, 0>()
}

#[test]
fn test_invalid_proof_len_poseidon_height_26_canopy_10() {
    invalid_proof_len::<Poseidon, 26, 10>()
}

#[test]
fn test_invalid_proof_len_sha256_height_26_canopy_0() {
    invalid_proof_len::<Sha256, 26, 0>()
}

#[test]
fn test_invalid_proof_len_sha256_height_26_canopy_10() {
    invalid_proof_len::<Sha256, 26, 10>()
}

fn invalid_proof<H, const HEIGHT: usize, const CANOPY: usize>()
where
    H: Hasher,
{
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let old_leaf = [5u8; 32];
    merkle_tree.append(&old_leaf).unwrap();

    let mut rng = thread_rng();

    let mut invalid_proof = BoundedVec::with_capacity(HEIGHT);
    for _ in 0..(HEIGHT - CANOPY) {
        let node: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        invalid_proof.push(node).unwrap();
    }

    let res = merkle_tree.update(
        merkle_tree.changelog_index(),
        &old_leaf,
        &[6u8; 32],
        0,
        &mut invalid_proof,
    );
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::InvalidProof(_, _))
    ));
}

#[test]
fn test_invalid_proof_keccak_height_26_canopy_0() {
    invalid_proof::<Keccak, 26, 0>()
}

#[test]
fn test_invalid_proof_keccak_height_26_canopy_10() {
    invalid_proof::<Keccak, 26, 10>()
}

#[test]
fn test_invalid_proof_poseidon_height_26_canopy_0() {
    invalid_proof::<Poseidon, 26, 0>()
}

#[test]
fn test_invalid_proof_poseidon_height_26_canopy_10() {
    invalid_proof::<Poseidon, 26, 10>()
}

#[test]
fn test_invalid_proof_sha256_height_26_canopy_0() {
    invalid_proof::<Sha256, 26, 0>()
}

#[test]
fn test_invalid_proof_sha256_height_26_canopy_10() {
    invalid_proof::<Sha256, 26, 10>()
}

fn update_empty<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    // Try updating all empty leaves in the empty tree.
    let mut proof = BoundedVec::from_slice(&H::zero_bytes()[..HEIGHT]);
    for leaf_index in 0..(1 << HEIGHT) {
        let old_leaf = H::zero_bytes()[0];
        let new_leaf = [5u8; 32];

        let res = merkle_tree.update(
            merkle_tree.changelog_index(),
            &old_leaf,
            &new_leaf,
            leaf_index,
            &mut proof,
        );
        assert!(matches!(
            res,
            Err(ConcurrentMerkleTreeError::CannotUpdateEmpty)
        ));
    }
}

#[test]
fn test_update_empty_keccak() {
    update_empty::<Keccak>()
}

#[test]
fn test_update_empty_poseidon() {
    update_empty::<Poseidon>()
}

#[test]
fn test_update_empty_sha256() {
    update_empty::<Sha256>()
}

fn append_empty_batch<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let res = merkle_tree.append_batch(&[]);
    assert!(matches!(res, Err(ConcurrentMerkleTreeError::EmptyLeaves)));
}

#[test]
fn test_append_empty_batch_keccak() {
    append_empty_batch::<Keccak>()
}

#[test]
fn test_append_empty_batch_poseidon() {
    append_empty_batch::<Poseidon>()
}

#[test]
fn test_append_empty_batch_sha256() {
    append_empty_batch::<Sha256>()
}

/// Reproducible only with Poseidon. Keccak and SHA256 don't return errors, as
/// they don't operate on a prime field.
#[test]
fn hasher_error() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<Poseidon, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    // Append a leaf which exceed the modulus.
    let res = merkle_tree.append(&[255_u8; 32]);
    assert!(matches!(res, Err(ConcurrentMerkleTreeError::Hasher(_))));
}

#[test]
pub fn test_100_nullify_mt() {
    for iterations in 1..100 {
        println!("iteration: {:?}", iterations);
        let mut crank_merkle_tree =
            light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon>::new(26, 10);
        let mut onchain_merkle_tree =
            ConcurrentMerkleTree::<Poseidon, 26>::new(26, 10, 10, 10).unwrap();
        onchain_merkle_tree.init().unwrap();
        assert_eq!(onchain_merkle_tree.root(), crank_merkle_tree.root());

        let mut queue = HashSet::new(6857, 2400).unwrap();
        let mut queue_indices = Vec::new();
        for i in 1..1 + iterations {
            let mut leaf = [0; 32];
            leaf[31] = i as u8;
            // onchain this is equivalent to append state (compressed pda program)
            onchain_merkle_tree.append(&leaf).unwrap();
            crank_merkle_tree.append(&leaf).unwrap();
            // onchain the equivalent is nullify state (compressed pda program)
            let leaf_bn = BigUint::from_be_bytes(&leaf);
            queue.insert(&leaf_bn, 1).unwrap();
            let (_, index) = queue.find_element(&leaf_bn, None).unwrap().unwrap();
            queue_indices.push(index);
        }
        assert_eq!(onchain_merkle_tree.root(), crank_merkle_tree.root());
        assert_eq!(
            onchain_merkle_tree.canopy,
            BoundedVec::from_slice(crank_merkle_tree.get_canopy().unwrap().as_slice())
        );

        let mut rng = rand::thread_rng();

        // Pick random queue indices to nullify.
        let queue_indices = queue_indices
            .choose_multiple(&mut rng, cmp::min(9, iterations))
            .cloned()
            .collect::<Vec<_>>();

        let change_log_index = onchain_merkle_tree.changelog_index();

        let mut nullified_leaf_indices = Vec::with_capacity(queue_indices.len());

        // Nullify the leaves we picked.
        for queue_index in queue_indices {
            let leaf_cell = queue.get_unmarked_bucket(queue_index).unwrap().unwrap();
            let leaf_index = crank_merkle_tree
                .get_leaf_index(&leaf_cell.value_bytes())
                .unwrap();

            let inter_proof = BoundedVec::from_slice(
                crank_merkle_tree
                    .get_proof_of_leaf(leaf_index, false)
                    .unwrap()
                    .as_slice(),
            );
            let mut proof = BoundedVec::with_capacity(onchain_merkle_tree.height);
            for node in inter_proof.iter() {
                proof.push(*node).unwrap();
            }
            onchain_merkle_tree
                .update(
                    change_log_index,
                    &leaf_cell.value_bytes(),
                    &[0u8; 32],
                    leaf_index,
                    &mut proof,
                )
                .unwrap();

            nullified_leaf_indices.push(leaf_index);
        }
        for leaf_index in nullified_leaf_indices {
            crank_merkle_tree.update(&[0; 32], leaf_index).unwrap();
        }
        assert_eq!(onchain_merkle_tree.root(), crank_merkle_tree.root());
        assert_eq!(
            onchain_merkle_tree.canopy,
            BoundedVec::from_slice(crank_merkle_tree.get_canopy().unwrap().as_slice())
        );
    }
}

// const LEAVES_WITH_NULLIFICATIONS: [([u8; 32], Option<usize>); 25] = [
//     (
//         [
//             9, 207, 75, 159, 247, 170, 46, 154, 178, 197, 60, 83, 191, 240, 137, 41, 36, 54, 242,
//             50, 43, 48, 56, 220, 154, 217, 138, 19, 152, 123, 86, 8,
//         ],
//         None,
//     ),
//     (
//         [
//             40, 10, 138, 159, 12, 188, 226, 84, 188, 92, 250, 11, 94, 240, 77, 158, 69, 219, 175,
//             48, 248, 181, 216, 200, 54, 38, 12, 224, 155, 40, 23, 32,
//         ],
//         None,
//     ),
//     (
//         [
//             11, 36, 94, 177, 195, 5, 4, 35, 75, 253, 31, 235, 68, 201, 79, 197, 199, 23, 214, 86,
//             196, 2, 41, 249, 246, 138, 184, 248, 245, 66, 184, 244,
//         ],
//         None,
//     ),
//     (
//         [
//             29, 3, 221, 195, 235, 46, 139, 171, 137, 7, 36, 118, 178, 198, 52, 20, 10, 131, 164, 5,
//             116, 187, 118, 186, 34, 193, 46, 6, 5, 144, 82, 4,
//         ],
//         None,
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(0),
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(1),
//     ),
//     (
//         [
//             6, 146, 149, 76, 49, 159, 84, 164, 203, 159, 181, 165, 21, 204, 111, 149, 87, 255, 46,
//             82, 162, 181, 99, 178, 247, 27, 166, 174, 212, 39, 163, 106,
//         ],
//         None,
//     ),
//     (
//         [
//             19, 135, 28, 172, 63, 129, 175, 101, 201, 97, 135, 147, 18, 78, 152, 243, 15, 154, 120,
//             153, 92, 46, 245, 82, 67, 32, 224, 141, 89, 149, 162, 228,
//         ],
//         None,
//     ),
//     (
//         [
//             4, 93, 251, 40, 246, 136, 132, 20, 175, 98, 3, 186, 159, 251, 128, 159, 219, 172, 67,
//             20, 69, 19, 66, 193, 232, 30, 121, 19, 193, 177, 143, 6,
//         ],
//         None,
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(3),
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(4),
//     ),
//     (
//         [
//             34, 229, 118, 4, 68, 219, 118, 228, 117, 70, 150, 93, 208, 215, 51, 243, 123, 48, 39,
//             228, 206, 194, 200, 232, 35, 133, 166, 222, 118, 217, 122, 228,
//         ],
//         None,
//     ),
//     (
//         [
//             24, 61, 159, 11, 70, 12, 177, 252, 244, 238, 130, 73, 202, 69, 102, 83, 33, 103, 82,
//             66, 83, 191, 149, 187, 141, 111, 253, 110, 49, 5, 47, 151,
//         ],
//         None,
//     ),
//     (
//         [
//             29, 239, 118, 17, 75, 98, 148, 167, 142, 190, 223, 175, 98, 255, 153, 111, 127, 169,
//             62, 234, 90, 89, 90, 70, 218, 161, 233, 150, 89, 173, 19, 1,
//         ],
//         None,
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(6),
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(5),
//     ),
//     (
//         [
//             45, 31, 195, 30, 201, 235, 73, 88, 57, 130, 35, 53, 202, 191, 20, 156, 125, 123, 37,
//             49, 154, 194, 124, 157, 198, 236, 233, 25, 195, 174, 157, 31,
//         ],
//         None,
//     ),
//     (
//         [
//             5, 59, 32, 123, 40, 100, 50, 132, 2, 194, 104, 95, 21, 23, 52, 56, 125, 198, 102, 210,
//             24, 44, 99, 255, 185, 255, 151, 249, 67, 167, 189, 85,
//         ],
//         None,
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(9),
//     ),
//     (
//         [
//             36, 131, 231, 53, 12, 14, 62, 144, 170, 248, 90, 226, 125, 178, 99, 87, 101, 226, 179,
//             43, 110, 130, 233, 194, 112, 209, 74, 219, 154, 48, 41, 148,
//         ],
//         None,
//     ),
//     (
//         [
//             12, 110, 79, 229, 117, 215, 178, 45, 227, 65, 183, 14, 91, 45, 170, 232, 126, 71, 37,
//             211, 160, 77, 148, 223, 50, 144, 134, 232, 83, 159, 131, 62,
//         ],
//         None,
//     ),
//     (
//         [
//             28, 57, 110, 171, 41, 144, 47, 162, 132, 221, 102, 100, 30, 69, 249, 176, 87, 134, 133,
//             207, 250, 166, 139, 16, 73, 39, 11, 139, 158, 182, 43, 68,
//         ],
//         None,
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(11),
//     ),
//     (
//         [
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0,
//         ],
//         Some(10),
//     ),
//     (
//         [
//             25, 88, 170, 121, 91, 234, 185, 213, 24, 92, 209, 146, 109, 134, 118, 242, 74, 218, 69,
//             28, 87, 154, 207, 86, 218, 48, 182, 206, 8, 9, 35, 240,
//         ],
//         None,
//     ),
// ];

// /// Test correctness of subtree updates during updates.
// /// The test data is a sequence of leaves with some nullifications
// /// and the result of a randomized tests which has triggered subtree inconsistencies.
// /// 1. Test subtree consistency with test data
// /// 2. Test subtree consistency of updating the right most leaf
// #[test]
// fn test_subtree_updates() {
//     const HEIGHT: usize = 26;
//     let mut ref_mt =
//         light_merkle_tree_reference::MerkleTree::<light_hasher::Keccak>::new(HEIGHT, 0);
//     let mut con_mt =
//         light_concurrent_merkle_tree::ConcurrentMerkleTree26::<light_hasher::Keccak>::new(
//             HEIGHT, 1400, 2400, 0,
//         )
//         .unwrap();
//     let mut spl_concurrent_mt =
//         spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<HEIGHT, 256>::new();
//     spl_concurrent_mt.initialize().unwrap();
//     con_mt.init().unwrap();
//     assert_eq!(ref_mt.root(), con_mt.root());
//     for leaf in LEAVES_WITH_NULLIFICATIONS.iter() {
//         match leaf.1 {
//             Some(index) => {
//                 let change_log_index = con_mt.changelog_index();
//                 let mut proof = BoundedVec::from_slice(
//                     ref_mt.get_proof_of_leaf(index, false).unwrap().as_slice(),
//                 );
//                 let old_leaf = ref_mt.leaf(index);
//                 let current_root = con_mt.root();
//                 spl_concurrent_mt
//                     .set_leaf(
//                         current_root,
//                         old_leaf,
//                         [0u8; 32],
//                         proof.to_array::<HEIGHT>().unwrap().as_slice(),
//                         index.try_into().unwrap(),
//                     )
//                     .unwrap();
//                 con_mt
//                     .update(change_log_index, &old_leaf, &[0u8; 32], index, &mut proof)
//                     .unwrap();
//                 ref_mt.update(&[0u8; 32], index).unwrap();
//             }
//             None => {
//                 con_mt.append(&leaf.0).unwrap();
//                 ref_mt.append(&leaf.0).unwrap();
//                 spl_concurrent_mt.append(leaf.0).unwrap();
//             }
//         }
//         assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
//         assert_eq!(spl_concurrent_mt.get_root(), con_mt.root());
//         assert_eq!(ref_mt.root(), con_mt.root());
//     }
//     let index = con_mt.next_index() - 1;
//     // test rightmost leaf edge case
//     let change_log_index = con_mt.changelog_index();
//     let mut proof =
//         BoundedVec::from_slice(ref_mt.get_proof_of_leaf(index, false).unwrap().as_slice());
//     let old_leaf = ref_mt.leaf(index);
//     let current_root = con_mt.root();
//     spl_concurrent_mt
//         .set_leaf(
//             current_root,
//             old_leaf,
//             [0u8; 32],
//             proof.to_array::<HEIGHT>().unwrap().as_slice(),
//             index.try_into().unwrap(),
//         )
//         .unwrap();
//     con_mt
//         .update(change_log_index, &old_leaf, &[0u8; 32], index, &mut proof)
//         .unwrap();
//     ref_mt.update(&[0u8; 32], index).unwrap();

//     assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
//     assert_eq!(spl_concurrent_mt.get_root(), con_mt.root());
//     assert_eq!(ref_mt.root(), con_mt.root());

//     let leaf = [3u8; 32];
//     con_mt.append(&leaf).unwrap();
//     ref_mt.append(&leaf).unwrap();
//     spl_concurrent_mt.append(leaf).unwrap();

//     assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
//     assert_eq!(spl_concurrent_mt.get_root(), con_mt.root());
//     assert_eq!(ref_mt.root(), con_mt.root());
// }

/// Tests an update of a leaf which was modified by another updates.
fn update_already_modified_leaf<
    H,
    // Number of conflicting updates of the same leaf.
    const CONFLICTS: usize,
    // Number of appends of random leaves before submitting the conflicting
    // updates.
    const RANDOM_APPENDS_BEFORE_CONFLICTS: usize,
    // Number of appends of random leaves after every single conflicting
    // update.
    const RANDOM_APPENDS_AFTER_EACH_CONFLICT: usize,
>()
where
    H: Hasher,
{
    const HEIGHT: usize = 26;
    const MAX_CHANGELOG: usize = 8;
    const MAX_ROOTS: usize = 8;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, MAX_CHANGELOG, MAX_ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();
    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    // Create tree with a single leaf.
    let first_leaf: [u8; 32] = Fr::rand(&mut rng)
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap();
    merkle_tree.append(&first_leaf).unwrap();
    reference_tree.append(&first_leaf).unwrap();

    // Save a proof of the first append.
    let outdated_changelog_index = merkle_tree.changelog_index();
    let mut outdated_proof = BoundedVec::from_slice(
        reference_tree
            .get_proof_of_leaf(0, false)
            .unwrap()
            .clone()
            .as_slice(),
    );

    let mut old_leaf = first_leaf;
    for _ in 0..CONFLICTS {
        // Update leaf. Always use an up-to-date proof.
        let mut up_to_date_proof = BoundedVec::from_slice(
            reference_tree
                .get_proof_of_leaf(0, false)
                .unwrap()
                .as_slice(),
        );
        let new_leaf = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        merkle_tree
            .update(
                merkle_tree.changelog_index(),
                &old_leaf,
                &new_leaf,
                0,
                &mut up_to_date_proof,
            )
            .unwrap();
        reference_tree.update(&new_leaf, 0).unwrap();

        old_leaf = new_leaf;

        assert_eq!(merkle_tree.root(), reference_tree.root());
    }

    // Update leaf. This time, try using an outdated proof.
    let new_leaf = Fr::rand(&mut rng)
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap();
    let res = merkle_tree.update(
        outdated_changelog_index,
        &first_leaf,
        &new_leaf,
        0,
        &mut outdated_proof,
    );
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::CannotUpdateLeaf)
    ));
}

#[test]
fn test_update_already_modified_leaf_keccak_1_0_0() {
    update_already_modified_leaf::<Keccak, 1, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_1_0_0() {
    update_already_modified_leaf::<Poseidon, 1, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_sha256_1_0_0() {
    update_already_modified_leaf::<Sha256, 1, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_keccak_1_1_1() {
    update_already_modified_leaf::<Keccak, 1, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_1_1_1() {
    update_already_modified_leaf::<Poseidon, 1, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_sha256_1_1_1() {
    update_already_modified_leaf::<Sha256, 1, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_keccak_1_2_2() {
    update_already_modified_leaf::<Keccak, 1, 2, 2>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_1_2_2() {
    update_already_modified_leaf::<Poseidon, 1, 2, 2>()
}

#[test]
fn test_update_already_modified_leaf_sha256_1_2_2() {
    update_already_modified_leaf::<Sha256, 1, 2, 2>()
}

#[test]
fn test_update_already_modified_leaf_keccak_2_0_0() {
    update_already_modified_leaf::<Keccak, 2, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_2_0_0() {
    update_already_modified_leaf::<Poseidon, 2, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_sha256_2_0_0() {
    update_already_modified_leaf::<Sha256, 2, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_keccak_2_1_1() {
    update_already_modified_leaf::<Keccak, 2, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_2_1_1() {
    update_already_modified_leaf::<Poseidon, 2, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_sha256_2_1_1() {
    update_already_modified_leaf::<Sha256, 2, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_keccak_2_2_2() {
    update_already_modified_leaf::<Keccak, 2, 2, 2>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_2_2_2() {
    update_already_modified_leaf::<Poseidon, 2, 2, 2>()
}

#[test]
fn test_update_already_modified_leaf_sha256_2_2_2() {
    update_already_modified_leaf::<Sha256, 2, 2, 2>()
}

#[test]
fn test_update_already_modified_leaf_keccak_4_0_0() {
    update_already_modified_leaf::<Keccak, 4, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_4_0_0() {
    update_already_modified_leaf::<Poseidon, 4, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_sha256_4_0_0() {
    update_already_modified_leaf::<Sha256, 4, 0, 0>()
}

#[test]
fn test_update_already_modified_leaf_keccak_4_1_1() {
    update_already_modified_leaf::<Keccak, 4, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_4_1_1() {
    update_already_modified_leaf::<Poseidon, 4, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_sha256_4_1_1() {
    update_already_modified_leaf::<Sha256, 4, 1, 1>()
}

#[test]
fn test_update_already_modified_leaf_keccak_4_4_4() {
    update_already_modified_leaf::<Keccak, 4, 4, 4>()
}

#[test]
fn test_update_already_modified_leaf_poseidon_4_4_4() {
    update_already_modified_leaf::<Poseidon, 4, 4, 4>()
}

#[test]
fn test_update_already_modified_leaf_sha256_4_4_4() {
    update_already_modified_leaf::<Sha256, 4, 4, 4>()
}

/// Checks whether the [`changelog_entries`](ConcurrentMerkleTree::changelog_entries)
/// method returns an iterator with expected entries.
///
/// We expect the `changelog_entries` method to return an iterator with entries
/// newer than the requested index.
///
/// # Examples
///
/// (In the tree) `current_index`: 1
/// (Requested) `changelog_index`: 1
/// Expected iterator: `[]` (empty)
///
/// (In the tree) `current_index`: 3
/// (Requested) `changelog_index`: 1
/// Expected iterator: `[2, 3]` (1 is skipped)
///
/// Changelog capacity: 12
/// (In the tree) `current_index`: 9
/// (Requested) `changelog_index`: 3 (lowed than `current_index`, because the
/// changelog is full and started overwriting values from the head)
/// Expected iterator: `[10, 11, 12, 13, 14, 15]` (9 is skipped)
fn changelog_entries<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 12;
    const ROOTS: usize = 16;
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    merkle_tree
        .append(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ])
        .unwrap();

    let changelog_entries = merkle_tree
        .changelog_entries(1)
        .unwrap()
        .collect::<Vec<_>>();
    assert!(changelog_entries.is_empty());

    // Try getting changelog entries out of bounds.
    for start in merkle_tree.changelog.len()..1000 {
        let changelog_entries = merkle_tree.changelog_entries(start);
        assert!(matches!(
            changelog_entries,
            Err(ConcurrentMerkleTreeError::BoundedVec(
                BoundedVecError::IterFromOutOfBounds
            ))
        ));
    }

    merkle_tree
        .append(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 2,
        ])
        .unwrap();
    merkle_tree
        .append(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 3,
        ])
        .unwrap();

    let changelog_leaves = merkle_tree
        .changelog_entries(1)
        .unwrap()
        .map(|changelog_entry| changelog_entry.path[0])
        .collect::<Vec<_>>();
    assert_eq!(
        changelog_leaves.as_slice(),
        &[
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 2
            ]),
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 3
            ])
        ]
    );

    // Try getting changelog entries out of bounds.
    for start in merkle_tree.changelog.len()..1000 {
        let changelog_entries = merkle_tree.changelog_entries(start);
        assert!(matches!(
            changelog_entries,
            Err(ConcurrentMerkleTreeError::BoundedVec(
                BoundedVecError::IterFromOutOfBounds
            ))
        ));
    }

    for i in 4_u8..16_u8 {
        merkle_tree
            .append(&[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, i,
            ])
            .unwrap();
    }

    let changelog_leaves = merkle_tree
        .changelog_entries(9)
        .unwrap()
        .map(|changelog_entry| changelog_entry.path[0])
        .collect::<Vec<_>>();
    assert_eq!(
        changelog_leaves.as_slice(),
        &[
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 10
            ]),
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 11
            ]),
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 12
            ]),
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 13
            ]),
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 14
            ]),
            Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 15
            ])
        ]
    );

    // Try getting changelog entries out of bounds.
    for start in merkle_tree.changelog.len()..1000 {
        let changelog_entries = merkle_tree.changelog_entries(start);
        assert!(matches!(
            changelog_entries,
            Err(ConcurrentMerkleTreeError::BoundedVec(
                BoundedVecError::IterFromOutOfBounds
            ))
        ));
    }
}

#[test]
fn changelog_entries_keccak() {
    changelog_entries::<Keccak>()
}

#[test]
fn changelog_entries_poseidon() {
    changelog_entries::<Poseidon>()
}

#[test]
fn changelog_entries_sha256() {
    changelog_entries::<Sha256>()
}

/// Checks whether the [`changelog_entries`](ConcurrentMerkleTree::changelog_entries)
/// method returns an iterator with expected entries.
///
/// It tests random insertions and updates and checks the consistency of leaves
/// (`path[0]`) in changelogs.
fn changelog_entries_random<
    H,
    const HEIGHT: usize,
    const CHANGELOG: usize,
    const ROOTS: usize,
    const CANOPY: usize,
>()
where
    H: Hasher,
{
    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    let changelog_entries = merkle_tree
        .changelog_entries(0)
        .unwrap()
        .collect::<Vec<_>>();
    assert!(changelog_entries.is_empty());

    // Requesting changelog entries starting from the current `changelog_index()`
    // should always return an empty iterator.
    let changelog_entries = merkle_tree
        .changelog_entries(merkle_tree.changelog_index())
        .unwrap()
        .collect::<Vec<_>>();
    assert!(changelog_entries.is_empty());

    // Vector of changelog indices after each operation.
    let mut leaf_indices = CyclicBoundedVec::with_capacity(CHANGELOG);
    // Vector of roots after each operation.
    let mut roots = CyclicBoundedVec::with_capacity(CHANGELOG);
    // Vector of merkle paths we get from the reference tree after each operation.
    let mut merkle_paths = CyclicBoundedVec::with_capacity(CHANGELOG);
    // Changelog is always initialized with a changelog path consisting of zero
    // bytes. For consistency, we need to assert the 1st zero byte as the first
    // expected leaf in the changelog.
    let merkle_path = reference_tree.get_path_of_leaf(0, true).unwrap();
    leaf_indices.push(0);
    merkle_paths.push(merkle_path);
    roots.push(merkle_tree.root());

    for _ in 0..1000 {
        // Append random leaf.
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        let leaf_index = merkle_tree.next_index();
        merkle_tree.append(&leaf).unwrap();
        reference_tree.append(&leaf).unwrap();

        leaf_indices.push(leaf_index);
        roots.push(merkle_tree.root());
        let merkle_path = reference_tree.get_path_of_leaf(leaf_index, true).unwrap();
        merkle_paths.push(merkle_path);

        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog.first_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(changelog_entries.len(), merkle_paths.len() - 1);

        for ((leaf_index, merkle_path), changelog_entry) in leaf_indices
            .iter()
            .skip(1)
            .zip(merkle_paths.iter().skip(1))
            .zip(changelog_entries)
        {
            assert_eq!(changelog_entry.index, *leaf_index as u64);
            for (i, path_node) in merkle_path.iter().enumerate() {
                let changelog_node = changelog_entry.path[i].unwrap();
                assert_eq!(changelog_node, *path_node);
            }
        }

        // Requesting changelog entries starting from the current `changelog_index()`
        // should always return an empty iterator.
        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert!(changelog_entries.is_empty());

        // Update random leaf.
        let leaf_index = rng.gen_range(0..reference_tree.leaves().len());
        let old_leaf = reference_tree.leaf(leaf_index);
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();
        let inter_proof_with_canopy = reference_tree.get_proof_of_leaf(leaf_index, false).unwrap();
        let mut proof = BoundedVec::with_capacity(HEIGHT);
        for node in inter_proof_with_canopy.iter() {
            proof.push(*node).unwrap();
        }
        merkle_tree
            .update(
                merkle_tree.changelog_index(),
                &old_leaf,
                &new_leaf,
                leaf_index,
                &mut proof,
            )
            .unwrap();
        reference_tree.update(&new_leaf, leaf_index).unwrap();

        leaf_indices.push(leaf_index);
        roots.push(merkle_tree.root());
        let merkle_path = reference_tree.get_path_of_leaf(leaf_index, true).unwrap();
        merkle_paths.push(merkle_path);

        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog.first_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(changelog_entries.len(), merkle_paths.len() - 1);

        for ((leaf_index, merkle_path), changelog_entry) in leaf_indices
            .iter()
            .skip(1)
            .zip(merkle_paths.iter().skip(1))
            .zip(changelog_entries)
        {
            assert_eq!(changelog_entry.index, *leaf_index as u64);
            for (i, path_node) in merkle_path.iter().enumerate() {
                let changelog_node = changelog_entry.path[i].unwrap();
                assert_eq!(changelog_node, *path_node);
            }
        }

        // Requesting changelog entries starting from the current `changelog_index()`
        // should always return an empty iterator.
        let changelog_entries = merkle_tree
            .changelog_entries(merkle_tree.changelog_index())
            .unwrap()
            .collect::<Vec<_>>();
        assert!(changelog_entries.is_empty());
    }
}

#[test]
fn test_changelog_entries_random_keccak_26_256_256_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    changelog_entries_random::<Keccak, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_entries_random_keccak_26_256_256_10() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 10;
    changelog_entries_random::<Keccak, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_entries_random_poseidon_26_256_256_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    changelog_entries_random::<Poseidon, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_entries_random_poseidon_26_256_256_10() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 10;
    changelog_entries_random::<Poseidon, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_entries_random_sha256_26_256_256_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    changelog_entries_random::<Sha256, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_entries_random_sha256_26_256_256_10() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 10;
    changelog_entries_random::<Sha256, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

/// When reading the tests above (`changelog_entries`, `changelog_entries_random`)
/// you might be still wondering why is skipping the **current** changelog element
/// necessary.
///
/// The explanation is that not skipping the current element might produce leaf
/// conflicts. Imagine that we insert a leaf and then we try to immediately update
/// it. Starting the iteration
///
/// This test reproduces that case and serves as a proof that skipping is the
/// right action.
fn changelog_iteration_without_skipping<
    H,
    const HEIGHT: usize,
    const CHANGELOG: usize,
    const ROOTS: usize,
    const CANOPY: usize,
>()
where
    H: Hasher,
{
    /// A broken re-implementation of `ConcurrentMerkleTree::update_proof_from_changelog`
    /// which reproduces the described issue.
    fn update_proof_from_changelog<H, const HEIGHT: usize>(
        merkle_tree: &ConcurrentMerkleTree<H, HEIGHT>,
        changelog_index: usize,
        leaf_index: usize,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError>
    where
        H: Hasher,
    {
        for changelog_entry in merkle_tree.changelog.iter_from(changelog_index).unwrap() {
            changelog_entry.update_proof(leaf_index, proof)?;
        }

        Ok(())
    }

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    let leaf: [u8; 32] = Fr::rand(&mut rng)
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap();

    merkle_tree.append(&leaf).unwrap();
    reference_tree.append(&leaf).unwrap();

    let mut proof = BoundedVec::from_slice(
        reference_tree
            .get_proof_of_leaf(0, false)
            .unwrap()
            .as_slice(),
    );

    let res =
        update_proof_from_changelog(&merkle_tree, merkle_tree.changelog_index(), 0, &mut proof);
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::CannotUpdateLeaf)
    ));
}

#[test]
fn test_changelog_interation_without_skipping_keccak_26_16_16_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 16;
    const ROOTS: usize = 16;
    const CANOPY: usize = 0;
    changelog_iteration_without_skipping::<Keccak, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_interation_without_skipping_poseidon_26_16_16_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 16;
    const ROOTS: usize = 16;
    const CANOPY: usize = 0;
    changelog_iteration_without_skipping::<Poseidon, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_changelog_interation_without_skipping_sha256_26_16_16_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 16;
    const ROOTS: usize = 16;
    const CANOPY: usize = 0;
    changelog_iteration_without_skipping::<Sha256, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

/// Tests an update with an old `changelog_index` and proof, which refers to the
/// state before the changelog wrap-around (enough new operations to overwrite
/// the whole changelog). Such an update should fail,
fn update_changelog_wrap_around<
    H,
    const HEIGHT: usize,
    const CHANGELOG: usize,
    const ROOTS: usize,
    const CANOPY: usize,
>()
where
    H: Hasher,
{
    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    // The leaf which we will want to update with an expired changelog.
    let leaf: [u8; 32] = Fr::rand(&mut rng)
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap();
    let (changelog_index, _) = merkle_tree.append(&leaf).unwrap();
    reference_tree.append(&leaf).unwrap();
    let mut proof = BoundedVec::from_slice(
        reference_tree
            .get_proof_of_leaf(0, false)
            .unwrap()
            .as_slice(),
    );

    // Perform enough appends and updates to overfill the changelog
    for i in 0..CHANGELOG {
        if i % 2 == 0 {
            // Append random leaf.
            let leaf: [u8; 32] = Fr::rand(&mut rng)
                .into_bigint()
                .to_bytes_be()
                .try_into()
                .unwrap();
            merkle_tree.append(&leaf).unwrap();
            reference_tree.append(&leaf).unwrap();
        } else {
            // Update random leaf.
            let leaf_index = rng.gen_range(1..reference_tree.leaves().len());
            let old_leaf = reference_tree.leaf(leaf_index);
            let new_leaf: [u8; 32] = Fr::rand(&mut rng)
                .into_bigint()
                .to_bytes_be()
                .try_into()
                .unwrap();
            let mut proof = BoundedVec::from_slice(
                reference_tree
                    .get_proof_of_leaf(leaf_index, false)
                    .unwrap()
                    .as_slice(),
            );
            merkle_tree
                .update(
                    merkle_tree.changelog_index(),
                    &old_leaf,
                    &new_leaf,
                    leaf_index,
                    &mut proof,
                )
                .unwrap();
            reference_tree.update(&new_leaf, leaf_index).unwrap();
        }
    }

    // Try to update the original `leaf` with an outdated proof and changelog
    // index. Expect an error.
    let new_leaf: [u8; 32] = Fr::rand(&mut rng)
        .into_bigint()
        .to_bytes_be()
        .try_into()
        .unwrap();

    let res = merkle_tree.update(changelog_index, &leaf, &new_leaf, 0, &mut proof);
    assert!(matches!(
        res,
        Err(ConcurrentMerkleTreeError::InvalidProof(_, _))
    ));

    // Try to update the original `leaf` with an up-to-date proof and changelog
    // index. Expect a success.
    let changelog_index = merkle_tree.changelog_index();
    let mut proof = BoundedVec::from_slice(
        reference_tree
            .get_proof_of_leaf(0, false)
            .unwrap()
            .as_slice(),
    );
    merkle_tree
        .update(changelog_index, &leaf, &new_leaf, 0, &mut proof)
        .unwrap();
}

#[test]
fn test_update_changelog_wrap_around_keccak_26_256_512_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    update_changelog_wrap_around::<Keccak, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_update_changelog_wrap_around_poseidon_26_256_512_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    update_changelog_wrap_around::<Poseidon, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_update_changelog_wrap_around_sha256_26_256_512_0() {
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 256;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;
    update_changelog_wrap_around::<Sha256, HEIGHT, CHANGELOG, ROOTS, CANOPY>()
}

#[test]
fn test_append_batch() {
    let mut tree = ConcurrentMerkleTree::<Sha256, 2>::new(2, 2, 2, 1).unwrap();
    tree.init().unwrap();
    let leaf_0 = [0; 32];
    let leaf_1 = [1; 32];
    tree.append_batch(&[&leaf_0, &leaf_1]).unwrap();
    let change_log_0 = &tree
        .changelog
        .get(tree.changelog.first_index())
        .unwrap()
        .path;
    let change_log_1 = &tree
        .changelog
        .get(tree.changelog.last_index())
        .unwrap()
        .path;
    let path_0 = ChangelogPath([Some(leaf_0), None]);
    let path_1 = ChangelogPath([
        Some(leaf_1),
        Some(Sha256::hashv(&[&leaf_0, &leaf_1]).unwrap()),
    ]);

    assert_eq!(change_log_1, &path_1);
    assert_eq!(change_log_0, &path_0);
}

/// Tests that updating proof with changelog entries with incomplete paths (coming
/// from batched appends) works.
#[test]
fn test_append_batch_and_update() {
    let mut tree = ConcurrentMerkleTree::<Sha256, 3>::new(3, 10, 10, 0).unwrap();
    tree.init().unwrap();

    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<Sha256>::new(3, 0);

    // Append two leaves.
    let leaf_0 = [0; 32];
    let leaf_1 = [1; 32];
    tree.append_batch(&[&leaf_0, &leaf_1]).unwrap();
    reference_tree.append(&leaf_0).unwrap();
    reference_tree.append(&leaf_1).unwrap();

    let changelog_index = tree.changelog_index();
    let mut proof_leaf_0 = BoundedVec::from_slice(
        reference_tree
            .get_proof_of_leaf(0, false)
            .unwrap()
            .as_slice(),
    );
    let mut proof_leaf_1 = BoundedVec::from_slice(
        reference_tree
            .get_proof_of_leaf(1, false)
            .unwrap()
            .as_slice(),
    );

    // Append another two leaves.
    let leaf_2 = [2; 32];
    let leaf_3 = [3; 32];
    tree.append_batch(&[&leaf_2, &leaf_3]).unwrap();
    reference_tree.append(&leaf_2).unwrap();
    reference_tree.append(&leaf_3).unwrap();

    let changelog_entry_leaf_2 = &tree.changelog[3];
    // Make sure that the non-terminal changelog entry has `None` nodes.
    assert_eq!(
        changelog_entry_leaf_2.path,
        ChangelogPath([Some([2; 32]), None, None])
    );
    let changelog_entry_leaf_3 = &tree.changelog[4];
    // And that the terminal one has no `None` nodes.
    assert_eq!(
        changelog_entry_leaf_3.path,
        ChangelogPath([
            Some([3; 32]),
            Some([
                39, 243, 47, 187, 250, 194, 251, 187, 206, 88, 177, 7, 82, 20, 75, 90, 116, 70,
                212, 185, 30, 75, 169, 15, 253, 238, 48, 94, 145, 89, 128, 232
            ]),
            Some([
                211, 95, 81, 105, 147, 137, 218, 126, 236, 124, 229, 235, 2, 100, 12, 109, 49, 140,
                245, 26, 227, 158, 202, 137, 11, 188, 123, 132, 236, 181, 218, 104
            ])
        ])
    );

    // The tree (only the used fragment) looks like:
    //
    //       _ H2 _
    //     /        \
    //    H0        H1
    //   /   \    /    \
    // L0    L1  L2     L3

    // Update `leaf_0`. Expect a success.
    let new_leaf_0 = [10; 32];
    tree.update(changelog_index, &leaf_0, &new_leaf_0, 0, &mut proof_leaf_0)
        .unwrap();

    // Update `leaf_1`. Expect a success.
    let new_leaf_1 = [20; 32];
    tree.update(changelog_index, &leaf_1, &new_leaf_1, 1, &mut proof_leaf_1)
        .unwrap();
}

/// Makes sure canopy works by:
///
/// 1. Appending 3 leaves.
/// 2. Updating the first leaf.
/// 3. Updating the second leaf.
fn update_with_canopy<H>()
where
    H: Hasher,
{
    let mut tree = ConcurrentMerkleTree::<H, 2>::new(2, 2, 2, 1).unwrap();
    tree.init().unwrap();
    let leaf_0 = [0; 32];
    let leaf_1 = [1; 32];
    let leaf_2 = [2; 32];
    tree.append(&leaf_0).unwrap();
    tree.append(&leaf_1).unwrap();
    tree.append(&leaf_2).unwrap();
    let old_canopy = tree.canopy.as_slice()[0];

    let new_leaf_0 = [1; 32];
    let mut leaf_0_proof = BoundedVec::with_capacity(2);
    leaf_0_proof.push(leaf_1).unwrap();
    tree.update(
        tree.changelog_index(),
        &leaf_0,
        &new_leaf_0,
        0,
        &mut leaf_0_proof,
    )
    .unwrap();
    let new_canopy = tree.canopy.as_slice()[0];

    assert_ne!(old_canopy, new_canopy);

    let new_leaf_2 = [3; 32];
    let mut leaf_2_proof = BoundedVec::with_capacity(2);
    leaf_2_proof.push([0; 32]).unwrap();
    tree.update(
        tree.changelog_index(),
        &leaf_2,
        &new_leaf_2,
        2,
        &mut leaf_2_proof,
    )
    .unwrap();
}

#[test]
fn test_update_with_canopy_keccak() {
    update_with_canopy::<Keccak>()
}

#[test]
fn test_update_with_canopy_poseidon() {
    update_with_canopy::<Poseidon>()
}

#[test]
fn test_update_with_canopy_sha256() {
    update_with_canopy::<Sha256>()
}
