use std::{cmp, mem};

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::{
    changelog::ChangelogEntry, errors::ConcurrentMerkleTreeError, event::ChangelogEvent,
    ConcurrentMerkleTree,
};
use light_hasher::{Hasher, Keccak, Poseidon, Sha256};
use light_merkle_tree_reference::store::Store;
use rand::thread_rng;
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 0, 1)
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 1, 2),
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 2, 3),
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 3, 4),
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 3, 4),
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
        .update(changelog_index, &leaf1, &new_leaf1, 0, &mut proof)
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 0, 5),
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
        .update(changelog_index, &leaf2, &new_leaf2, 1, &mut proof)
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 1, 6),
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
        .update(changelog_index, &leaf3, &new_leaf3, 2, &mut proof)
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 2, 7)
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
        .update(changelog_index, &leaf4, &new_leaf4, 3, &mut proof)
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
        ChangelogEntry::new(expected_root, expected_changelog_path, 3, 8)
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
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
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
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
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
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
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
        let first_changelog_index = concurrent_mt_1.append_batch(leaves.as_slice()).unwrap();
        let changelog_event_1 = concurrent_mt_1
            .get_changelog_event([0u8; 32], first_changelog_index, batch_size)
            .unwrap();
        let changelog_event_1 = match changelog_event_1 {
            ChangelogEvent::V1(changelog_event_1) => changelog_event_1,
        };

        let mut changelog_index = 0;
        for leaf in leaves.iter() {
            changelog_index = concurrent_mt_2.append(leaf).unwrap();
        }
        let changelog_event_2 = concurrent_mt_2
            .get_changelog_event([0u8; 32], changelog_index, 1)
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
        // for batch_size in 1..(1 << HEIGHT) {
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
                .update(changelog_index, &old_leaf, &new_leaf, 0, &mut proof)
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
            .update(changelog_index, &old_leaf, &new_leaf, i, &mut proof)
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
        ConcurrentMerkleTree::<H, HEIGHT>::from_bytes(
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
    const HEIGHT: usize = 22;
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
        let changelog_event = merkle_tree.get_changelog_event([0u8; 32], i, 1).unwrap();
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
