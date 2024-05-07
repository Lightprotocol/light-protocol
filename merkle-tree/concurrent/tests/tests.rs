use std::{cmp, mem};

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::{
    changelog::ChangelogEntry, errors::ConcurrentMerkleTreeError, event::ChangelogEvent,
    ConcurrentMerkleTree,
};
use light_hash_set::HashSet;
use light_hasher::{Hasher, Keccak, Poseidon, Sha256};
use light_merkle_tree_reference::store::Store;
use num_bigint::BigUint;
use num_traits::FromBytes;
use rand::{thread_rng, Rng};
use solana_program::pubkey::Pubkey;

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
    let expected_changelog_path = [leaf1, h1, h2, h3];
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf1, h1, h2, h3]);

    merkle_tree.append(&leaf1).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 1);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 0)
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 1);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index, 1);
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
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf1, h1, h2, h3]);

    merkle_tree.append(&leaf2).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 2);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 1),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 2);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index, 2);
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
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf3, h1, h3, h4]);

    merkle_tree.append(&leaf3).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 3);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 2),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 3);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index, 3);
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
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf3, h1, h3, h4]);

    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 4);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 3),
    );
    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 4);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index, 4);
    assert_eq!(merkle_tree.rightmost_leaf, leaf4);
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
    let expected_filled_subtrees = BoundedVec::from_array(&[leaf3, h1, h3, h4]);

    merkle_tree.append(&leaf1).unwrap();
    merkle_tree.append(&leaf2).unwrap();
    merkle_tree.append(&leaf3).unwrap();
    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.changelog_index(), 4 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 3),
    );

    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 4);
    assert_eq!(merkle_tree.filled_subtrees, expected_filled_subtrees);
    assert_eq!(merkle_tree.next_index, 4);
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
    let mut proof = BoundedVec::from_array(&[leaf2, h2, H::zero_bytes()[2], H::zero_bytes()[3]]);

    let changelog_index = merkle_tree.changelog_index();
    merkle_tree
        .update(changelog_index, &leaf1, &new_leaf1, 0, &mut proof, false)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf1, h1, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 5 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 0),
    );

    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 5);
    assert_eq!(merkle_tree.next_index, 4);
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
    let mut proof =
        BoundedVec::from_array(&[new_leaf1, h2, H::zero_bytes()[2], H::zero_bytes()[3]]);

    let changelog_index = merkle_tree.changelog_index();
    merkle_tree
        .update(changelog_index, &leaf2, &new_leaf2, 1, &mut proof, false)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf2, h1, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 6 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 1),
    );

    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 6);
    assert_eq!(merkle_tree.next_index, 4);
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
    let mut proof = BoundedVec::from_array(&[leaf4, h1, H::zero_bytes()[2], H::zero_bytes()[3]]);

    let changelog_index = merkle_tree.changelog_index();
    merkle_tree
        .update(changelog_index, &leaf3, &new_leaf3, 2, &mut proof, false)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf3, h2, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 7 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 2)
    );

    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 7);
    assert_eq!(merkle_tree.next_index, 4);
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
    let mut proof =
        BoundedVec::from_array(&[new_leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]]);

    let changelog_index = merkle_tree.root_index();
    merkle_tree
        .update(changelog_index, &leaf4, &new_leaf4, 3, &mut proof, false)
        .unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &new_leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_changelog_path = [new_leaf4, h2, h3, h4];

    assert_eq!(merkle_tree.changelog_index(), 8 % CHANGELOG);
    assert_eq!(
        merkle_tree.changelog[merkle_tree.changelog_index()],
        ChangelogEntry::new(expected_root, expected_changelog_path, 3)
    );

    assert_eq!(merkle_tree.root().unwrap(), expected_root);
    assert_eq!(merkle_tree.current_root_index, 8);
    assert_eq!(merkle_tree.next_index, 4);
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
    const CANOPY: usize = 0;

    let mut merkle_tree =
        ConcurrentMerkleTree::<H, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    merkle_tree.init().unwrap();

    for _ in 0..4 {
        merkle_tree.append(&[4; 32]).unwrap();
    }
    assert!(matches!(
        merkle_tree.append(&[4; 32]),
        Err(ConcurrentMerkleTreeError::TreeFull)
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
        let mut proof = reference_tree.get_proof_of_leaf(i, false).unwrap();

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof, false)
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
        let mut proof = reference_tree.get_proof_of_leaf(i, false).unwrap();

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof, false)
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
        let mut proof = reference_tree.get_proof_of_leaf(i, false).unwrap();

        merkle_tree
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof, false)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();
    }

    assert_eq!(merkle_tree.current_changelog_index, 5);
    assert_eq!(merkle_tree.current_root_index, 3);
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

        // Store to which we are passing the changelog events from `concurrent_mt_1`.
        // We will get proofs from it and validate against proofs from `reference_mt`.
        let mut store = Store::<H>::default();

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
        let (first_changelog_index, first_sequence_number) =
            concurrent_mt_1.append_batch(leaves.as_slice()).unwrap();
        let changelog_event_1 = concurrent_mt_1
            .get_changelog_event(
                [0u8; 32],
                first_changelog_index,
                first_sequence_number,
                batch_size,
            )
            .unwrap();
        let changelog_event_1 = match changelog_event_1 {
            ChangelogEvent::V1(changelog_event_1) => changelog_event_1,
        };

        let mut changelog_index = 0;
        let mut sequence_number = 0;
        for leaf in leaves.iter() {
            (changelog_index, sequence_number) = concurrent_mt_2.append(leaf).unwrap();
        }
        let changelog_event_2 = concurrent_mt_2
            .get_changelog_event([0u8; 32], changelog_index, sequence_number, 1)
            .unwrap();
        let changelog_event_2 = match changelog_event_2 {
            ChangelogEvent::V1(changelog_event_2) => changelog_event_2,
        };

        for leaf in leaves {
            reference_mt.append(leaf).unwrap();
        }

        for path in changelog_event_1.paths.iter() {
            for node in path {
                store.add_node(node.node, node.index.try_into().unwrap());
            }
        }

        // Check wether the last Merkle paths are the same.
        let changelog_path_1 = changelog_event_1.paths.last().unwrap();
        let changelog_path_2 = changelog_event_2.paths.last().unwrap();
        assert_eq!(changelog_path_1, changelog_path_2);

        // Check whether roots are the same.
        assert_eq!(concurrent_mt_1.root().unwrap(), reference_mt.root());
        assert_eq!(concurrent_mt_2.root().unwrap(), reference_mt.root(),);
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

                    let mut proof_with_canopy = reference_mt_with_canopy
                        .get_proof_of_leaf(leaf_index, false)
                        .unwrap();
                    let proof_without_canopy = reference_mt_without_canopy
                        .get_proof_of_leaf(leaf_index, true)
                        .unwrap();

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
fn test_update_keccak_height_4_changelog_1_roots_256_canopy_0() {
    update::<Keccak, 1, 256, 0>()
}

#[test]
fn test_update_keccak_height_4_changelog_32_roots_256_canopy_0() {
    update::<Keccak, 32, 256, 0>()
}

#[test]
fn test_update_poseidon_height_4_changelog_1_roots_256_canopy_0() {
    update::<Poseidon, 1, 256, 0>()
}

#[test]
fn test_update_poseidon_height_4_changelog_32_roots_256_canopy_0() {
    update::<Poseidon, 32, 256, 0>()
}

#[test]
fn test_update_sha256_height_4_changelog_32_roots_256_canopy_0() {
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

/// Compares the internal fields of concurrent Merkle tree implementations, to
/// ensure their consistency.
fn compare_trees<H, const HEIGHT: usize, const MAX_ROOTS: usize>(
    concurrent_mt: &ConcurrentMerkleTree<H, HEIGHT>,
    spl_concurrent_mt: &spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree<
        HEIGHT,
        MAX_ROOTS,
    >,
) where
    H: Hasher,
{
    for i in 0..concurrent_mt.current_changelog_index as usize {
        let changelog_entry = concurrent_mt.changelog[i].clone();
        let spl_changelog_entry = spl_concurrent_mt.change_logs[i];
        assert_eq!(changelog_entry.root, spl_changelog_entry.root);
        assert_eq!(changelog_entry.path.as_slice(), spl_changelog_entry.path);
        assert_eq!(changelog_entry.index, spl_changelog_entry.index as u64);
    }
    assert_eq!(
        concurrent_mt.current_changelog_index,
        spl_concurrent_mt.active_index as usize
    );
    assert_eq!(concurrent_mt.root().unwrap(), spl_concurrent_mt.get_root());
    for i in 0..concurrent_mt.current_root_index as usize {
        assert_eq!(
            concurrent_mt.roots[i],
            spl_concurrent_mt.change_logs[i].root
        );
    }
    assert_eq!(
        concurrent_mt.current_root_index,
        spl_concurrent_mt.active_index as usize
    );
    assert_eq!(
        concurrent_mt.next_index,
        spl_concurrent_mt.rightmost_proof.index as usize
    );
    assert_eq!(
        concurrent_mt.rightmost_leaf,
        spl_concurrent_mt.rightmost_proof.leaf
    );
}

/// Checks whether our `append` and `update` implementations are compatible
/// with `append` and `set_leaf` from `spl-concurrent-merkle-tree` crate.
#[tokio::test(flavor = "multi_thread")]
async fn test_spl_compat() {
    const HEIGHT: usize = 4;
    const CHANGELOG: usize = 64;
    const ROOTS: usize = 256;
    const CANOPY: usize = 0;

    let mut rng = thread_rng();

    // Our implementation of concurrent Merkle tree.
    let mut concurrent_mt =
        ConcurrentMerkleTree::<Keccak, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY).unwrap();
    concurrent_mt.init().unwrap();

    // Solana Labs implementation of concurrent Merkle tree.
    let mut spl_concurrent_mt = spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<HEIGHT, ROOTS>::new();
    spl_concurrent_mt.initialize().unwrap();

    // Reference implemenetation of Merkle tree which Solana Labs uses for
    // testing (and therefore, we as well). We use it mostly to get the Merkle
    // proofs.
    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<Keccak>::new(HEIGHT, CANOPY);

    for i in 0..(1 << HEIGHT) {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        concurrent_mt.append(&leaf).unwrap();
        spl_concurrent_mt.append(leaf).unwrap();
        reference_tree.append(&leaf).unwrap();

        compare_trees(&concurrent_mt, &spl_concurrent_mt);

        // For every appended leaf with index greater than 0, update the leaf 0.
        // This is done in indexed Merkle trees[0] and it's a great test case
        // for rightmost proof updates.
        //
        // [0] https://docs.aztec.network/concepts/advanced/data_structures/indexed_merkle_tree
        if i > 0 {
            let new_leaf: [u8; 32] = Fr::rand(&mut rng)
                .into_bigint()
                .to_bytes_be()
                .try_into()
                .unwrap();

            let root = concurrent_mt.root().unwrap();
            let changelog_index = concurrent_mt.changelog_index();
            let old_leaf = reference_tree.leaf(0);
            let mut proof = reference_tree.get_proof_of_leaf(0, false).unwrap();

            concurrent_mt
                .update(changelog_index, &old_leaf, &new_leaf, 0, &mut proof, false)
                .unwrap();
            spl_concurrent_mt
                .set_leaf(root, old_leaf, new_leaf, proof.as_slice(), 0 as u32)
                .unwrap();
            reference_tree.update(&new_leaf, 0).unwrap();

            compare_trees(&concurrent_mt, &spl_concurrent_mt);
        }
    }

    for i in 0..(1 << HEIGHT) {
        let new_leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        let root = concurrent_mt.root().unwrap();
        let changelog_index = concurrent_mt.changelog_index();
        let old_leaf = reference_tree.leaf(i);
        let mut proof = reference_tree.get_proof_of_leaf(i, false).unwrap();

        concurrent_mt
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof, false)
            .unwrap();
        spl_concurrent_mt
            .set_leaf(root, old_leaf, new_leaf, proof.as_slice(), i as u32)
            .unwrap();
        reference_tree.update(&new_leaf, i).unwrap();

        compare_trees(&concurrent_mt, &spl_concurrent_mt);
    }
}

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
    let mut bytes_struct = vec![0u8; mem::size_of::<ConcurrentMerkleTree<H, HEIGHT>>()];
    let mut bytes_filled_subtrees = vec![0u8; mem::size_of::<[u8; 32]>() * HEIGHT];
    let mut bytes_changelog = vec![0u8; mem::size_of::<ChangelogEntry<HEIGHT>>() * CHANGELOG];
    let mut bytes_roots = vec![0u8; mem::size_of::<[u8; 32]>() * ROOTS];
    let mut bytes_canopy = vec![
        0u8;
        mem::size_of::<[u8; 32]>()
            * ConcurrentMerkleTree::<H, HEIGHT>::canopy_size(CANOPY)
    ];

    let merkle_tree = unsafe {
        ConcurrentMerkleTree::<H, HEIGHT>::from_bytes_init(
            bytes_struct.as_mut_slice(),
            bytes_filled_subtrees.as_mut_slice(),
            bytes_changelog.as_mut_slice(),
            bytes_roots.as_mut_slice(),
            bytes_canopy.as_mut_slice(),
            HEIGHT,
            CHANGELOG,
            ROOTS,
            CANOPY,
        )
        .unwrap()
    };
    merkle_tree.init().unwrap();
    let mut reference_tree = light_merkle_tree_reference::MerkleTree::<H>::new(HEIGHT, CANOPY);

    let mut rng = thread_rng();

    for _ in 0..(1 << HEIGHT) {
        let leaf: [u8; 32] = Fr::rand(&mut rng)
            .into_bigint()
            .to_bytes_be()
            .try_into()
            .unwrap();

        merkle_tree.append(&leaf).unwrap();
        reference_tree.append(&leaf).unwrap();

        assert_eq!(merkle_tree.root().unwrap(), reference_tree.root());
    }

    let merkle_tree = unsafe {
        ConcurrentMerkleTree::<H, HEIGHT>::copy_from_bytes(
            bytes_struct.as_slice(),
            bytes_filled_subtrees.as_slice(),
            bytes_changelog.as_slice(),
            bytes_roots.as_slice(),
            bytes_canopy.as_slice(),
        )
        .unwrap()
    };
    assert_eq!(merkle_tree.root().unwrap(), reference_tree.root());
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

#[test]
fn test_changelog_event_v1() {
    const HEIGHT: usize = 26;
    const MAX_CHANGELOG: usize = 8;
    const MAX_ROOTS: usize = 8;
    const CANOPY: usize = 0;

    let pubkey = [0u8; 32];

    // Fill up the Merkle tree with random leaves.
    let mut merkle_tree =
        ConcurrentMerkleTree::<Keccak, HEIGHT>::new(HEIGHT, MAX_CHANGELOG, MAX_ROOTS, CANOPY)
            .unwrap();
    merkle_tree.init().unwrap();
    let mut spl_merkle_tree =
        spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<
            HEIGHT,
            MAX_CHANGELOG,
        >::new();
    spl_merkle_tree.initialize().unwrap();

    let leaves = 8;

    for i in 0..leaves {
        merkle_tree.append(&[(i + 1) as u8; 32]).unwrap();
        spl_merkle_tree.append([(i + 1) as u8; 32]).unwrap();
    }

    for i in 0..leaves {
        let changelog_event = merkle_tree.get_changelog_event([0u8; 32], i, i, 1).unwrap();
        let changelog_event = match changelog_event {
            ChangelogEvent::V1(changelog_event) => changelog_event,
        };

        let spl_changelog_entry = Box::new(spl_merkle_tree.change_logs[i]);
        let spl_changelog_event: Box<spl_account_compression::ChangeLogEvent> =
            Box::<spl_account_compression::ChangeLogEvent>::from((
                spl_changelog_entry,
                Pubkey::new_from_array(pubkey),
                i as u64,
            ));

        match *spl_changelog_event {
            spl_account_compression::ChangeLogEvent::V1(
                spl_account_compression::events::ChangeLogEventV1 {
                    id, path, index, ..
                },
            ) => {
                assert_eq!(id.to_bytes(), changelog_event.id);
                assert_eq!(path.len(), changelog_event.paths[0].len());
                for j in 0..HEIGHT {
                    assert_eq!(path[j].node, changelog_event.paths[0][j].node);
                    assert_eq!(path[j].index, changelog_event.paths[0][j].index);
                }
                assert_eq!(index, changelog_event.index);
            }
        }
    }
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
        assert_eq!(
            onchain_merkle_tree.root().unwrap(),
            crank_merkle_tree.root()
        );

        let mut queue = HashSet::<u16>::new(6857, 4800, 2400).unwrap();
        for i in 1..1 + iterations {
            let mut leaf = [0; 32];
            leaf[31] = i as u8;
            // onchain this is equivalent to append state (compressed pda program)
            onchain_merkle_tree.append(&leaf).unwrap();
            crank_merkle_tree.append(&leaf).unwrap();
            // onchain the equivalent is nullify state (compressed pda program)
            queue.insert(&BigUint::from_be_bytes(&leaf), 1).unwrap();
        }
        assert_eq!(
            onchain_merkle_tree.root().unwrap(),
            crank_merkle_tree.root()
        );

        let mut rng = rand::thread_rng();
        let change_log_index = onchain_merkle_tree.changelog_index();

        let mut nullified_leaf_indices = vec![0];
        for _ in 1..std::cmp::min(9, iterations) {
            let mut leaf = [0u8; 32];
            let mut leaf_index = 0;
            while nullified_leaf_indices.contains(&leaf_index) {
                let index = rng.gen_range(0..std::cmp::min(9, iterations));
                leaf = queue.by_value_index(index, None).unwrap().value_bytes();
                leaf_index = crank_merkle_tree.get_leaf_index(&leaf).unwrap().clone();
            }

            nullified_leaf_indices.push(leaf_index);
            let mut proof0 = crank_merkle_tree
                .get_proof_of_leaf(leaf_index, false)
                .unwrap();
            onchain_merkle_tree
                .update(
                    change_log_index,
                    &leaf,
                    &[0u8; 32],
                    leaf_index,
                    &mut proof0,
                    false,
                )
                .unwrap();
        }
        nullified_leaf_indices.remove(0);
        for leaf_index in nullified_leaf_indices {
            crank_merkle_tree.update(&[0; 32], leaf_index).unwrap();
        }
        assert_eq!(
            onchain_merkle_tree.root().unwrap(),
            crank_merkle_tree.root()
        );
    }
}

const LEAVES_WITH_NULLIFICATIONS: [([u8; 32], Option<usize>); 25] = [
    (
        [
            9, 207, 75, 159, 247, 170, 46, 154, 178, 197, 60, 83, 191, 240, 137, 41, 36, 54, 242,
            50, 43, 48, 56, 220, 154, 217, 138, 19, 152, 123, 86, 8,
        ],
        None,
    ),
    (
        [
            40, 10, 138, 159, 12, 188, 226, 84, 188, 92, 250, 11, 94, 240, 77, 158, 69, 219, 175,
            48, 248, 181, 216, 200, 54, 38, 12, 224, 155, 40, 23, 32,
        ],
        None,
    ),
    (
        [
            11, 36, 94, 177, 195, 5, 4, 35, 75, 253, 31, 235, 68, 201, 79, 197, 199, 23, 214, 86,
            196, 2, 41, 249, 246, 138, 184, 248, 245, 66, 184, 244,
        ],
        None,
    ),
    (
        [
            29, 3, 221, 195, 235, 46, 139, 171, 137, 7, 36, 118, 178, 198, 52, 20, 10, 131, 164, 5,
            116, 187, 118, 186, 34, 193, 46, 6, 5, 144, 82, 4,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(0),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(1),
    ),
    (
        [
            6, 146, 149, 76, 49, 159, 84, 164, 203, 159, 181, 165, 21, 204, 111, 149, 87, 255, 46,
            82, 162, 181, 99, 178, 247, 27, 166, 174, 212, 39, 163, 106,
        ],
        None,
    ),
    (
        [
            19, 135, 28, 172, 63, 129, 175, 101, 201, 97, 135, 147, 18, 78, 152, 243, 15, 154, 120,
            153, 92, 46, 245, 82, 67, 32, 224, 141, 89, 149, 162, 228,
        ],
        None,
    ),
    (
        [
            4, 93, 251, 40, 246, 136, 132, 20, 175, 98, 3, 186, 159, 251, 128, 159, 219, 172, 67,
            20, 69, 19, 66, 193, 232, 30, 121, 19, 193, 177, 143, 6,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(3),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(4),
    ),
    (
        [
            34, 229, 118, 4, 68, 219, 118, 228, 117, 70, 150, 93, 208, 215, 51, 243, 123, 48, 39,
            228, 206, 194, 200, 232, 35, 133, 166, 222, 118, 217, 122, 228,
        ],
        None,
    ),
    (
        [
            24, 61, 159, 11, 70, 12, 177, 252, 244, 238, 130, 73, 202, 69, 102, 83, 33, 103, 82,
            66, 83, 191, 149, 187, 141, 111, 253, 110, 49, 5, 47, 151,
        ],
        None,
    ),
    (
        [
            29, 239, 118, 17, 75, 98, 148, 167, 142, 190, 223, 175, 98, 255, 153, 111, 127, 169,
            62, 234, 90, 89, 90, 70, 218, 161, 233, 150, 89, 173, 19, 1,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(6),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(5),
    ),
    (
        [
            45, 31, 195, 30, 201, 235, 73, 88, 57, 130, 35, 53, 202, 191, 20, 156, 125, 123, 37,
            49, 154, 194, 124, 157, 198, 236, 233, 25, 195, 174, 157, 31,
        ],
        None,
    ),
    (
        [
            5, 59, 32, 123, 40, 100, 50, 132, 2, 194, 104, 95, 21, 23, 52, 56, 125, 198, 102, 210,
            24, 44, 99, 255, 185, 255, 151, 249, 67, 167, 189, 85,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(9),
    ),
    (
        [
            36, 131, 231, 53, 12, 14, 62, 144, 170, 248, 90, 226, 125, 178, 99, 87, 101, 226, 179,
            43, 110, 130, 233, 194, 112, 209, 74, 219, 154, 48, 41, 148,
        ],
        None,
    ),
    (
        [
            12, 110, 79, 229, 117, 215, 178, 45, 227, 65, 183, 14, 91, 45, 170, 232, 126, 71, 37,
            211, 160, 77, 148, 223, 50, 144, 134, 232, 83, 159, 131, 62,
        ],
        None,
    ),
    (
        [
            28, 57, 110, 171, 41, 144, 47, 162, 132, 221, 102, 100, 30, 69, 249, 176, 87, 134, 133,
            207, 250, 166, 139, 16, 73, 39, 11, 139, 158, 182, 43, 68,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(11),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(10),
    ),
    (
        [
            25, 88, 170, 121, 91, 234, 185, 213, 24, 92, 209, 146, 109, 134, 118, 242, 74, 218, 69,
            28, 87, 154, 207, 86, 218, 48, 182, 206, 8, 9, 35, 240,
        ],
        None,
    ),
];

/// Test correctness of subtree updates during updates.
/// The test data is a sequence of leaves with some nullifications
/// and the result of a randomized tests which has triggered subtree inconsistencies.
/// 1. Test subtree consistency with test data
/// 2. Test subtree consistency of updating the right most leaf
#[test]
fn test_subtree_updates() {
    const HEIGHT: usize = 26;
    let mut ref_mt =
        light_merkle_tree_reference::MerkleTree::<light_hasher::Keccak>::new(HEIGHT, 0);
    let mut con_mt =
        light_concurrent_merkle_tree::ConcurrentMerkleTree26::<light_hasher::Keccak>::new(
            HEIGHT, 1400, 2400, 0,
        )
        .unwrap();
    let mut spl_concurrent_mt =
        spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<HEIGHT, 256>::new();
    spl_concurrent_mt.initialize().unwrap();
    con_mt.init().unwrap();
    assert_eq!(ref_mt.root(), con_mt.root().unwrap());
    for (_, leaf) in LEAVES_WITH_NULLIFICATIONS.iter().enumerate() {
        match leaf.1 {
            Some(index) => {
                let change_log_index = con_mt.changelog_index();
                let mut proof = ref_mt.get_proof_of_leaf(index, false).unwrap();
                let old_leaf = ref_mt.leaf(index);
                let current_root = con_mt.root().unwrap();
                spl_concurrent_mt
                    .set_leaf(
                        current_root,
                        old_leaf,
                        [0u8; 32],
                        proof.to_array::<HEIGHT>().unwrap().as_slice(),
                        index.try_into().unwrap(),
                    )
                    .unwrap();
                con_mt
                    .update(
                        change_log_index,
                        &old_leaf,
                        &[0u8; 32],
                        index,
                        &mut proof,
                        true,
                    )
                    .unwrap();
                ref_mt.update(&[0u8; 32], index).unwrap();
            }
            None => {
                con_mt.append(&leaf.0).unwrap();
                ref_mt.append(&leaf.0).unwrap();
                spl_concurrent_mt.append(leaf.0).unwrap();
            }
        }
        assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
        assert_eq!(spl_concurrent_mt.get_root(), con_mt.root().unwrap());
        assert_eq!(ref_mt.root(), con_mt.root().unwrap());
    }
    let index = con_mt.next_index() - 1;
    // test rightmost leaf edge case
    let change_log_index = con_mt.changelog_index();
    let mut proof = ref_mt.get_proof_of_leaf(index, false).unwrap();
    let old_leaf = ref_mt.leaf(index);
    let current_root = con_mt.root().unwrap();
    spl_concurrent_mt
        .set_leaf(
            current_root,
            old_leaf,
            [0u8; 32],
            proof.to_array::<HEIGHT>().unwrap().as_slice(),
            index.try_into().unwrap(),
        )
        .unwrap();
    con_mt
        .update(
            change_log_index,
            &old_leaf,
            &[0u8; 32],
            index,
            &mut proof,
            true,
        )
        .unwrap();
    ref_mt.update(&[0u8; 32], index).unwrap();

    assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
    assert_eq!(spl_concurrent_mt.get_root(), con_mt.root().unwrap());
    assert_eq!(ref_mt.root(), con_mt.root().unwrap());

    let leaf = [3u8; 32];
    con_mt.append(&leaf).unwrap();
    ref_mt.append(&leaf).unwrap();
    spl_concurrent_mt.append(leaf).unwrap();

    assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
    assert_eq!(spl_concurrent_mt.get_root(), con_mt.root().unwrap());
    assert_eq!(ref_mt.root(), con_mt.root().unwrap());
}
