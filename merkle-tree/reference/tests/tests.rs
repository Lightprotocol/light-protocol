use light_bounded_vec::BoundedVec;
use light_hasher::{Hasher, Keccak, Poseidon, Sha256};
use light_merkle_tree_reference::MerkleTree;

fn update<H>()
where
    H: Hasher,
{
    const HEIGHT: usize = 4;
    const CANOPY: usize = 0;

    let mut merkle_tree = MerkleTree::<H>::new(HEIGHT, CANOPY);

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
    let expected_proof = BoundedVec::from_array(&[
        H::zero_bytes()[0],
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ]);

    merkle_tree.append(&leaf1).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(0, false).unwrap(),
        expected_proof
    );

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
    let expected_proof = BoundedVec::from_array(&[
        leaf1,
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ]);

    merkle_tree.append(&leaf2).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(1, false).unwrap(),
        expected_proof
    );

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
    let expected_proof = BoundedVec::from_array(&[
        H::zero_bytes()[0],
        h1,
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ]);

    merkle_tree.append(&leaf3).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(2, false).unwrap(),
        expected_proof
    );

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
    let expected_proof =
        BoundedVec::from_array(&[leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]]);

    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(3, false).unwrap(),
        expected_proof
    );

    // Update `leaf1`.
    let new_leaf1 = [9u8; 32];

    // Updating L1 affects H1 and all parent hashes up to the root.
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
    //
    // Our Merkle tree implementation should be smart enough to fill up the
    // proof with zero bytes, so we can skip them and just define the proof as:
    // [L2, H2]
    merkle_tree.update(&new_leaf1, 0).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_proof =
        BoundedVec::from_array(&[leaf2, h2, H::zero_bytes()[2], H::zero_bytes()[3]]);

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(0, false).unwrap(),
        expected_proof
    );

    // Update `leaf2`.
    let new_leaf2 = H::hash(&[8u8; 32]).unwrap();

    // Updating L2 affects H1 and all parent hashes up to the root.
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
    // [L1, H2]
    merkle_tree.update(&new_leaf2, 1).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_proof =
        BoundedVec::from_array(&[new_leaf1, h2, H::zero_bytes()[2], H::zero_bytes()[3]]);

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(1, false).unwrap(),
        expected_proof
    );

    // Update `leaf3`.
    let new_leaf3 = H::hash(&[7u8; 32]).unwrap();

    // Updating L3 affects H1 and all parent hashes up to the root.
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
    // [L4, H1]
    merkle_tree.update(&new_leaf3, 2).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_proof =
        BoundedVec::from_array(&[leaf4, h1, H::zero_bytes()[2], H::zero_bytes()[3]]);

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(2, false).unwrap(),
        expected_proof
    );

    // Update `leaf4`.
    let new_leaf4 = H::hash(&[6u8; 32]).unwrap();

    // Updating L4 affects H1 and all parent hashes up to the root.
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
    // [L3, H1]
    merkle_tree.update(&new_leaf4, 3).unwrap();

    let h1 = H::hashv(&[&new_leaf1, &new_leaf2]).unwrap();
    let h2 = H::hashv(&[&new_leaf3, &new_leaf4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let expected_proof =
        BoundedVec::from_array(&[new_leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]]);

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_proof_of_leaf(3, false).unwrap(),
        expected_proof
    );
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
