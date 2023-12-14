use light_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::{Hasher, Poseidon, Sha256};

fn append<H>()
where
    H: Hasher,
{
    let mut merkle_tree = ConcurrentMerkleTree::<H, 8, 32>::default();

    merkle_tree.init();

    let leaf1 = [1u8; 32];

    // The hash of our new leaf (`[1u8; 32]`) and its sibling (a zero value).
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

    // The hash of `h2` and its sibling (a subtree represented by `Z[2]`).
    //
    //          H3
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let h3 = H::hashv(&[&h2, &H::zero_bytes()[2]]).unwrap();

    // The hash of `h3` and its sibling (a subtree represented by `Z[3]`).
    //
    //             H4
    //           /    \
    //          H3   Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[3]]).unwrap();

    // The hash of `h4` and its sibling,
    //
    //                H5
    //              /    \
    //             H4   Z[4]
    //           /    \
    //          H3   Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let h5 = H::hashv(&[&h4, &H::zero_bytes()[4]]).unwrap();

    // The hash of `h5` and its sibling.
    //
    //                   H6
    //                 /    \
    //                H5   Z[5]
    //              /    \
    //             H4   Z[4]
    //           /    \
    //          H3   Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let h6 = H::hashv(&[&h5, &H::zero_bytes()[5]]).unwrap();

    // The hash of `h6` and its sibling.
    //
    //                      H7
    //                    /    \
    //                   H6   Z[6]
    //                 /    \
    //                H5   Z[5]
    //              /    \
    //             H4   Z[4]
    //           /    \
    //          H3   Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]
    let h7 = H::hashv(&[&h6, &H::zero_bytes()[6]]).unwrap();

    // Finally, we computed enough hashes (8) to compute the root of our
    // tree (with height 8).
    //
    //                          R
    //                       /     \
    //                      H7    Z[7]
    //                    /    \
    //                   H6   Z[6]
    //                 /    \
    //                H5   Z[5]
    //              /    \
    //             H4   Z[4]
    //           /    \
    //          H3   Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //    H1   Z[1]
    //  /    \
    // L1   Z[0]

    let expected_root = H::hashv(&[&h7, &H::zero_bytes()[7]]).unwrap();

    merkle_tree.append(leaf1).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Appending the 2nd leaf should result in recomputing the root due to the
    // change of the `h1`, which now is a hash of the two non-zero leafs. So
    // when computing all hashes up to the root, we are still going to use
    // zero bytes from 1 to 8.
    //
    // The other subtrees still remain the same.
    //
    //                          R
    //                       /     \
    //                      H7    Z[7]
    //                    /    \
    //                   H6   Z[6]
    //                 /    \
    //                H5   Z[5]
    //              /    \
    //             H4   Z[4]
    //           /    \
    //          H3   Z[3]
    //        /    \
    //       H2   Z[2]
    //     /    \
    //   H1    Z[1]
    //  /  \
    // L1  L2
    let leaf2 = [2u8; 32];

    let h1 = H::hashv(&[&leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&h1, &H::zero_bytes()[1]]).unwrap();
    let h3 = H::hashv(&[&h2, &H::zero_bytes()[2]]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[3]]).unwrap();
    let h5 = H::hashv(&[&h4, &H::zero_bytes()[4]]).unwrap();
    let h6 = H::hashv(&[&h5, &H::zero_bytes()[5]]).unwrap();
    let h7 = H::hashv(&[&h6, &H::zero_bytes()[6]]).unwrap();
    let expected_root = H::hashv(&[&h7, &H::zero_bytes()[7]]).unwrap();

    merkle_tree.append(leaf2).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // Appending the 3rd leaf alters the next subtree on the right.
    // Instead of using Z[1], we will end up with the hash of the new leaf and
    // Z[0].
    //
    // The other subtrees still remain the same.
    //
    //                              R
    //                           /     \
    //                          H8    Z[7]
    //                        /    \
    //                      H7    Z[6]
    //                    /    \
    //                  H6   Z[5]
    //                /    \
    //              H5   Z[4]
    //            /    \
    //           H4   Z[3]
    //         /    \
    //       H3   Z[2]
    //     /    \
    //   H1      H2
    //  /  \    /  \
    // L1  L2  L3  Z[0]
    let leaf3 = [3u8; 32];

    let h1 = H::hashv(&[&leaf1, &leaf2]).unwrap();
    let h2 = H::hashv(&[&leaf3, &H::zero_bytes()[0]]).unwrap();
    let h3 = H::hashv(&[&h1, &h2]).unwrap();
    let h4 = H::hashv(&[&h3, &H::zero_bytes()[2]]).unwrap();
    let h5 = H::hashv(&[&h4, &H::zero_bytes()[3]]).unwrap();
    let h6 = H::hashv(&[&h5, &H::zero_bytes()[4]]).unwrap();
    let h7 = H::hashv(&[&h6, &H::zero_bytes()[5]]).unwrap();
    let h8 = H::hashv(&[&h7, &H::zero_bytes()[6]]).unwrap();
    let expected_root = H::hashv(&[&h8, &H::zero_bytes()[7]]).unwrap();

    merkle_tree.append(leaf3).unwrap();

    assert_eq!(merkle_tree.root(), expected_root);

    // OK, that's enough of manual tests.
}

#[test]
fn test_append_sha256() {
    append::<Sha256>()
}

#[test]
fn test_append_poseidon() {
    append::<Poseidon>()
}
