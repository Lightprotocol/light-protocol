use light_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::{Hasher, Poseidon, Sha256};

fn append<H>()
where
    H: Hasher,
{
    let mut merkle_tree = ConcurrentMerkleTree::<H, 4, 32>::default();
    merkle_tree.init();

    let leaf1 = [1u8; 32];

    // The hash of our new leaf (`[1u8; 32]`).
    let h1 = H::hash(&leaf1).unwrap();

    // The hash of our new hashed leaf and its sibling (a zero value).
    //
    //    H2
    //  /    \
    // H1   Z[0]
    // |
    // L1
    let h2 = H::hashv(&[&h1, &H::zero_bytes()[0]]).unwrap();

    // The hash of `h2` and its sibling (a subtree represented by `Z[1]`).
    //
    //          H3
    //      /-/    \-\
    //    H2          Z[1]
    //  /    \      /      \
    // H1   Z[0]   Z[0]   Z[0]
    // |
    // L1
    //
    // `Z[1]` represents the whole subtree on the right from `h2`. In the next
    // examples, we are just going to show empty subtrees instead of the whole
    // hierarchy.
    let h3 = H::hashv(&[&h2, &H::zero_bytes()[1]]).unwrap();

    // The hash of `h3` and its sibling (a subtree represented by `Z[2]`).
    //
    //          H4
    //        /    \
    //       H3   Z[2]
    //     /    \
    //    H2   Z[1]
    //  /    \
    // H1   Z[0]
    // |
    // L1
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();

    // The hash of `h4` and its sibling (a subtree represented by `Z[3]`),
    // which is the root.
    //
    //              R
    //           /     \
    //          H4    Z[3]
    //        /    \
    //       H3   Z[2]
    //     /    \
    //    H2   Z[1]
    //  /    \
    // H1   Z[0]
    // |
    // L1
    let expected_root = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();

    merkle_tree.append(&leaf1).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Appending the 2nd leaf should result in recomputing the root due to the
    // change of the `h1`, which now is a hash of the two non-zero leafs. So
    // when computing all hashes up to the root, we are still going to use
    // zero bytes from 1 to 8.
    //
    // The other subtrees still remain the same.
    //
    //              R
    //           /     \
    //          H5    Z[3]
    //        /    \
    //       H4   Z[2]
    //     /    \
    //   H3    Z[1]
    //  /  \
    // H1  H2
    // |    |
    // L1  L2
    let leaf2 = [2u8; 32];

    let h1 = H::hash(&leaf1).unwrap();
    let h2 = H::hash(&leaf2).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[1]]).unwrap();
    let h5 = H::hashv(&[&h4, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h5, &H::zero_bytes()[3]]).unwrap();

    merkle_tree.append(&leaf2).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Appending the 3rd leaf alters the next subtree on the right.
    // Instead of using Z[1], we will end up with the hash of the new leaf and
    // Z[0].
    //
    // The other subtrees still remain the same.
    //
    //               R
    //            /     \
    //           H7    Z[3]
    //         /    \
    //       H6    Z[2]
    //     /    \
    //   H4      H5
    //  /  \    /  \
    // H1  H2  H3  Z[0]
    // |    |  |
    // L1  L2  L3
    let leaf3 = [3u8; 32];

    let h1 = H::hash(&leaf1).unwrap();
    let h2 = H::hash(&leaf2).unwrap();
    let h3 = H::hash(&leaf3).unwrap();
    let h4 = H::hashv(&[&h1, &h2]).unwrap();
    let h5 = H::hashv(&[&h3, &H::zero_bytes()[0]]).unwrap();
    let h6 = H::hashv(&[&h4, &h5]).unwrap();
    let h7 = H::hashv(&[&h6, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h7, &H::zero_bytes()[3]]).unwrap();

    merkle_tree.append(&leaf3).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Appending the 4th leaf alters the next subtree on the right.
    // Instead of using Z[1], we will end up with the hash of the new leaf and
    // Z[0].
    //
    // The other subtrees still remain the same.
    //
    //               R
    //            /     \
    //           H8    Z[3]
    //         /    \
    //       H7    Z[2]
    //     /    \
    //   H5      H6
    //  /  \    /  \
    // H1  H2  H3  H4
    // |    |  |    |
    // L1  L2  L3  L4
    let leaf4 = [4u8; 32];

    let h1 = H::hash(&leaf1).unwrap();
    let h2 = H::hash(&leaf2).unwrap();
    let h3 = H::hash(&leaf3).unwrap();
    let h4 = H::hash(&leaf4).unwrap();
    let h5 = H::hashv(&[&h1, &h2]).unwrap();
    let h6 = H::hashv(&[&h3, &h4]).unwrap();
    let h7 = H::hashv(&[&h5, &h6]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[3]]).unwrap();

    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
}

fn replace_leaf<H>()
where
    H: Hasher,
{
    let mut merkle_tree = ConcurrentMerkleTree::<H, 4, 32>::default();
    merkle_tree.init();

    let leaf1 = [1u8; 32];
    let leaf2 = [2u8; 32];
    let leaf3 = [3u8; 32];
    let leaf4 = [4u8; 32];

    // Append 4 leaves.
    //
    //               R
    //            /     \
    //           H8    Z[3]
    //         /    \
    //       H7    Z[2]
    //     /    \
    //   H5      H6
    //  /  \    /  \
    // H1  H2  H3  H4
    // |    |  |    |
    // L1  L2  L3  L4
    let h1 = H::hash(&leaf1).unwrap();
    let h2 = H::hash(&leaf2).unwrap();
    let h3 = H::hash(&leaf3).unwrap();
    let h4 = H::hash(&leaf4).unwrap();
    let h5 = H::hashv(&[&h1, &h2]).unwrap();
    let h6 = H::hashv(&[&h3, &h4]).unwrap();
    let h7 = H::hashv(&[&h5, &h6]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[3]]).unwrap();

    merkle_tree.append(&leaf1).unwrap();
    merkle_tree.append(&leaf2).unwrap();
    merkle_tree.append(&leaf3).unwrap();
    merkle_tree.append(&leaf4).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Replace `leaf1`.
    let new_leaf1 = [9u8; 32];

    // Replacing L1 affects H1 and all parent hashes up to the root.
    //
    // Merkle proof for the replaced leaf L1 is:
    // [H2, H6, Z[2], Z[3]]
    //
    // Our Merkle tree implementation should be smart enough to fill up the
    // proof with zero bytes, so we can skip them and just define the proof as:
    // [H2, H6]
    let proof = &[h2, h6];

    merkle_tree
        .replace_leaf(merkle_tree.root(), &leaf1, &new_leaf1, 0, proof)
        .unwrap();

    let h1 = H::hash(&new_leaf1).unwrap();
    let h2 = H::hash(&leaf2).unwrap();
    let h3 = H::hash(&leaf3).unwrap();
    let h4 = H::hash(&leaf4).unwrap();
    let h5 = H::hashv(&[&h1, &h2]).unwrap();
    let h6 = H::hashv(&[&h3, &h4]).unwrap();
    let h7 = H::hashv(&[&h5, &h6]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[3]]).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Replace `leaf2`.
    let new_leaf2 = [8u8; 32];

    // Merkle proof for the replaced leaf L2 is:
    // [H1, H6]
    let proof = &[h1, h6];

    merkle_tree
        .replace_leaf(merkle_tree.root(), &leaf2, &new_leaf2, 1, proof)
        .unwrap();

    let h1 = H::hash(&new_leaf1).unwrap();
    let h2 = H::hash(&new_leaf2).unwrap();
    let h3 = H::hash(&leaf3).unwrap();
    let h4 = H::hash(&leaf4).unwrap();
    let h5 = H::hashv(&[&h1, &h2]).unwrap();
    let h6 = H::hashv(&[&h3, &h4]).unwrap();
    let h7 = H::hashv(&[&h5, &h6]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[3]]).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Replace `leaf3`.
    let new_leaf3 = [7u8; 32];

    // Merkle proof for the replaced leaf L3 is:
    // [H4, H5]
    let proof = &[h4, h5];

    merkle_tree
        .replace_leaf(merkle_tree.root(), &leaf3, &new_leaf3, 2, proof)
        .unwrap();

    let h1 = H::hash(&new_leaf1).unwrap();
    let h2 = H::hash(&new_leaf2).unwrap();
    let h3 = H::hash(&new_leaf3).unwrap();
    let h4 = H::hash(&leaf4).unwrap();
    let h5 = H::hashv(&[&h1, &h2]).unwrap();
    let h6 = H::hashv(&[&h3, &h4]).unwrap();
    let h7 = H::hashv(&[&h5, &h6]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[3]]).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Replace `leaf4`.
    let new_leaf4 = [6u8; 32];

    // Merkle proof for the replaced leaf L4 is:
    // [H3, H5]
    let proof = &[h3, h5];

    merkle_tree
        .replace_leaf(merkle_tree.root(), &leaf4, &new_leaf4, 3, proof)
        .unwrap();

    let h1 = H::hash(&new_leaf1).unwrap();
    let h2 = H::hash(&new_leaf2).unwrap();
    let h3 = H::hash(&new_leaf3).unwrap();
    let h4 = H::hash(&new_leaf4).unwrap();
    let h5 = H::hashv(&[&h1, &h2]).unwrap();
    let h6 = H::hashv(&[&h3, &h4]).unwrap();
    let h7 = H::hashv(&[&h5, &h6]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[2]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[3]]).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);
}

#[test]
fn test_append_sha256() {
    append::<Sha256>()
}

#[test]
fn test_append_poseidon() {
    append::<Poseidon>()
}

#[test]
fn test_replace_leaf_sha256() {
    replace_leaf::<Sha256>()
}

#[test]
fn test_replace_leaf_poseidon() {
    replace_leaf::<Poseidon>()
}
