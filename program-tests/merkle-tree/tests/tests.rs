use light_hasher::{zero_bytes::poseidon::ZERO_BYTES, Hasher, Keccak, Poseidon, Sha256};
use light_merkle_tree_reference::MerkleTree;

fn append<H>(canopy_depth: usize)
where
    H: Hasher,
{
    const HEIGHT: usize = 4;

    let mut mt = MerkleTree::<H>::new(4, canopy_depth);

    let leaf_1 = [1_u8; 32];
    mt.append(&leaf_1).unwrap();

    // The hash of our new leaf and its sibling (a zero value).
    //
    //    H1
    //  /    \
    // L1   Z[0]
    let h1 = H::hashv(&[&leaf_1, &H::zero_bytes()[0]]).unwrap();

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
    assert_eq!(mt.root(), expected_root);

    // The Merkle path of L1 consists of nodes from L1 up to the root.
    // In this case: L1, H1, H2, H3.
    //
    //               R
    //            /     \
    //          *H3*    Z[3]
    //         /    \
    //       *H2*   Z[2]
    //      /    \
    //    *H1*   Z[1]
    //   /    \
    // *L1*   Z[0]
    let expected_merkle_path = &[leaf_1, h1, h2, h3];

    let full_merkle_path = mt.get_path_of_leaf(0, true).unwrap();
    assert_eq!(full_merkle_path.as_slice(), expected_merkle_path);

    let partial_merkle_path = mt.get_path_of_leaf(0, false).unwrap();
    assert_eq!(
        partial_merkle_path.as_slice(),
        &expected_merkle_path[..HEIGHT - canopy_depth]
    );

    // The Merkle proof consists of siblings of L1 and all its parent
    // nodes. In this case, these are just zero bytes: Z[0], Z[1], Z[2],
    // Z[3].
    //
    //              R
    //           /     \
    //          H3   *Z[3]*
    //        /    \
    //       H2  *Z[2]*
    //     /    \
    //    H1  *Z[1]*
    //  /    \
    // L1  *Z[0]*
    let expected_merkle_proof = &H::zero_bytes()[..HEIGHT];

    let full_merkle_proof = mt.get_proof_of_leaf(0, true).unwrap();
    assert_eq!(full_merkle_proof.as_slice(), expected_merkle_proof);

    let partial_merkle_proof = mt.get_proof_of_leaf(0, false).unwrap();
    assert_eq!(
        partial_merkle_proof.as_slice(),
        &expected_merkle_proof[..HEIGHT - canopy_depth]
    );

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
    let leaf_2 = H::hash(&[2u8; 32]).unwrap();
    mt.append(&leaf_2).unwrap();

    let h1 = H::hashv(&[&leaf_1, &leaf_2]).unwrap();
    let h2 = H::hashv(&[&h1, &H::zero_bytes()[1]]).unwrap();
    let h3 = H::hashv(&[&h2, &H::zero_bytes()[2]]).unwrap();

    let expected_root = H::hashv(&[&h3, &H::zero_bytes()[3]]).unwrap();
    assert_eq!(mt.root(), expected_root);

    // The Merkle path of L2 consists of nodes from L2 up to the root.
    // In this case: L2, H1, H2, H3.
    //
    //               R
    //            /     \
    //          *H3*    Z[3]
    //         /    \
    //       *H2*   Z[2]
    //      /    \
    //    *H1*   Z[1]
    //   /    \
    //  L1   *L2*
    let expected_merkle_path = &[leaf_2, h1, h2, h3];

    let full_merkle_path = mt.get_path_of_leaf(1, true).unwrap();
    assert_eq!(full_merkle_path.as_slice(), expected_merkle_path);

    let partial_merkle_path = mt.get_path_of_leaf(1, false).unwrap();
    assert_eq!(
        partial_merkle_path.as_slice(),
        &expected_merkle_path[..HEIGHT - canopy_depth]
    );

    // The Merkle proof consists of siblings of L2 and all its parent
    // nodes. In this case, these are: L1, Z[1], Z[2], Z[3].
    //
    //               R
    //            /     \
    //           H3   *Z[3]*
    //         /    \
    //        H2  *Z[2]*
    //      /    \
    //     H1  *Z[1]*
    //   /    \
    // *L1*    L2
    let expected_merkle_proof = &[
        leaf_1,
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    let full_merkle_proof = mt.get_proof_of_leaf(1, true).unwrap();
    assert_eq!(full_merkle_proof.as_slice(), expected_merkle_proof);

    let partial_merkle_proof = mt.get_proof_of_leaf(1, false).unwrap();
    assert_eq!(
        partial_merkle_proof.as_slice(),
        &expected_merkle_proof[..HEIGHT - canopy_depth]
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
    let leaf_3 = H::hash(&[3u8; 32]).unwrap();
    mt.append(&leaf_3).unwrap();

    let h1 = H::hashv(&[&leaf_1, &leaf_2]).unwrap();
    let h2 = H::hashv(&[&leaf_3, &H::zero_bytes()[0]]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();

    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    assert_eq!(mt.root(), expected_root);

    // The Merkle path of L3 consists of nodes from L3 up to the root.
    // In this case: L3, H2, H3, H4.
    //
    //               R
    //            /     \
    //          *H4*   Z[3]
    //         /    \
    //      *H3*   Z[2]
    //     /    \
    //   H1     *H2*
    //  /  \    /  \
    // L1  L2 *L3*  Z[0]
    let expected_merkle_path = &[leaf_3, h2, h3, h4];

    let full_merkle_path = mt.get_path_of_leaf(2, true).unwrap();
    assert_eq!(full_merkle_path.as_slice(), expected_merkle_path);

    let partial_merkle_path = mt.get_path_of_leaf(2, false).unwrap();
    assert_eq!(
        partial_merkle_path.as_slice(),
        &expected_merkle_path[..HEIGHT - canopy_depth]
    );

    // The Merkle proof consists of siblings of L2 and all its parent
    // nodes. In this case, these are: Z[0], H1, Z[2], Z[3].
    //
    //               R
    //            /     \
    //           H4   *Z[3]*
    //         /    \
    //       H3   *Z[2]*
    //     /    \
    //  *H1*     H2
    //  /  \    /  \
    // L1  L2  L3 *Z[0]*
    let expected_merkle_proof = &[
        H::zero_bytes()[0],
        h1,
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    let full_merkle_proof = mt.get_proof_of_leaf(2, true).unwrap();
    assert_eq!(full_merkle_proof.as_slice(), expected_merkle_proof);

    let partial_merkle_proof = mt.get_proof_of_leaf(2, false).unwrap();
    assert_eq!(
        partial_merkle_proof.as_slice(),
        &expected_merkle_proof[..HEIGHT - canopy_depth]
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
    let leaf_4 = H::hash(&[4u8; 32]).unwrap();
    mt.append(&leaf_4).unwrap();

    let h1 = H::hashv(&[&leaf_1, &leaf_2]).unwrap();
    let h2 = H::hashv(&[&leaf_3, &leaf_4]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();

    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    assert_eq!(mt.root(), expected_root);
}

#[test]
fn test_append_keccak_4_0() {
    append::<Keccak>(0)
}

#[test]
fn test_append_poseidon_4_0() {
    append::<Poseidon>(0)
}

#[test]
fn test_append_sha256_4_0() {
    append::<Sha256>(0)
}

#[test]
fn test_append_keccak_4_1() {
    append::<Keccak>(1)
}

#[test]
fn test_append_poseidon_4_1() {
    append::<Poseidon>(1)
}

#[test]
fn test_append_sha256_4_1() {
    append::<Sha256>(1)
}

#[test]
fn test_append_keccak_4_2() {
    append::<Keccak>(2)
}

#[test]
fn test_append_poseidon_4_2() {
    append::<Poseidon>(2)
}

#[test]
fn test_append_sha256_4_2() {
    append::<Sha256>(2)
}

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
    // The Merkle path is:
    // [L1, H1, H2, H3]
    let expected_path = vec![leaf1, h1, h2, h3];
    let expected_proof = vec![
        H::zero_bytes()[0],
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    merkle_tree.append(&leaf1).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(0, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L2, H1, H2, H3]
    let expected_path = vec![leaf2, h1, h2, h3];
    let expected_proof = vec![
        leaf1,
        H::zero_bytes()[1],
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    merkle_tree.append(&leaf2).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(1, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L3, H2, H3, H4]
    let expected_path = vec![leaf3, h2, h3, h4];
    let expected_proof = vec![
        H::zero_bytes()[0],
        h1,
        H::zero_bytes()[2],
        H::zero_bytes()[3],
    ];

    merkle_tree.append(&leaf3).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(2, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L4, H2, H3, H4]
    let expected_path = vec![leaf4, h2, h3, h4];
    let expected_proof = vec![leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(3, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L1, H1, H3, H4]
    let expected_path = vec![new_leaf1, h1, h3, h4];
    let expected_proof = vec![leaf2, h2, H::zero_bytes()[2], H::zero_bytes()[3]];

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(0, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L2, H1, H3, H4]
    let expected_path = vec![new_leaf2, h1, h3, h4];
    let expected_proof = vec![new_leaf1, h2, H::zero_bytes()[2], H::zero_bytes()[3]];

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(1, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L3, H2, H3, H4]
    let expected_path = vec![new_leaf3, h2, h3, h4];
    let expected_proof = vec![leaf4, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(2, false).unwrap(),
        expected_path
    );
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
    // The Merkle path is:
    // [L4, H2, H3, H4]
    let expected_path = vec![new_leaf4, h2, h3, h4];
    let expected_proof = vec![new_leaf3, h1, H::zero_bytes()[2], H::zero_bytes()[3]];

    assert_eq!(merkle_tree.root(), expected_root);
    assert_eq!(
        merkle_tree.get_path_of_leaf(3, false).unwrap(),
        expected_path
    );
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

#[test]
fn test_sequence_number() {
    let mut merkle_tree = MerkleTree::<Poseidon>::new(4, 0);
    assert_eq!(merkle_tree.sequence_number, 0);

    let leaf1 = Poseidon::hash(&[1u8; 32]).unwrap();
    merkle_tree.append(&leaf1).unwrap();
    assert_eq!(merkle_tree.sequence_number, 1);

    let leaf2 = Poseidon::hash(&[2u8; 32]).unwrap();
    merkle_tree.update(&leaf2, 0).unwrap();
    assert_eq!(merkle_tree.sequence_number, 2);
}

#[test]
fn test_get_proof_by_indices_for_existent_or_non_existent_leaves() {
    let mut merkle_tree = MerkleTree::<Poseidon>::new(4, 0);

    let indices = [0];
    let proof = merkle_tree.get_proof_by_indices(&indices);
    assert_eq!(proof.len(), 1);
    assert_eq!(proof[0].len(), 4);

    for (level, zero_byte) in ZERO_BYTES.iter().enumerate().take(4) {
        assert_eq!(proof[0][level], *zero_byte);
    }

    let mut leaf_1 = [0u8; 32];
    leaf_1[31] = 1;
    let mut leaf_2 = [0u8; 32];
    leaf_2[31] = 2;

    merkle_tree.append(&leaf_1).unwrap();
    merkle_tree.append(&leaf_2).unwrap();

    // Test proofs for existing leaves
    let indices = [0];
    let proof = merkle_tree.get_proof_by_indices(&indices);
    assert_eq!(proof.len(), 1);
    assert_eq!(proof[0].len(), 4);

    // Test proofs for non-existent leaf (index 3)
    let indices = [3];
    let proof = merkle_tree.get_proof_by_indices(&indices);
    assert_eq!(proof.len(), 1);
    assert_eq!(proof[0].len(), 4);

    // Test multiple indices at once
    let indices = [0, 1, 2, 3];
    let proof = merkle_tree.get_proof_by_indices(&indices);
    assert_eq!(proof.len(), 4);
    for p in proof.iter() {
        assert_eq!(p.len(), 4);
    }
}
