use thiserror::Error;

#[derive(Error, Debug)]
pub enum NullifierMerkleTreeError {
    #[error("Low nullifier not found")]
    LowNullifierNotFound,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Node {
    value: [u8; 32],
    next_index: usize,
}

pub struct NullifierMerkleTree<const HEIGHT: usize, const ROOTS: usize> {
    pub nodes: [Node; HEIGHT],
    pub roots: [Node; ROOTS],
    pub current_node_index: usize,
    pub current_root_index: usize,
}

impl<const HEIGHT: usize, const ROOTS: usize> Default for NullifierMerkleTree<HEIGHT, ROOTS> {
    fn default() -> Self {
        Self {
            nodes: std::array::from_fn(|_| Node {
                value: [0u8; 32],
                next_index: 0,
            }),
            roots: std::array::from_fn(|_| Node {
                value: [0u8; 32],
                next_index: 0,
            }),
            current_node_index: 0,
            current_root_index: 0,
        }
    }
}

impl<const HEIGHT: usize, const ROOTS: usize> NullifierMerkleTree<HEIGHT, ROOTS> {
    fn find_low_nullifier(
        &mut self,
        new_value: &[u8; 32],
    ) -> Result<usize, NullifierMerkleTreeError> {
        for (i, nullifier) in self.nodes.iter().enumerate() {
            if self.nodes[nullifier.next_index].value > *new_value {
                return Ok(i);
            }
        }
        for (i, nullifier) in self.nodes.iter().enumerate() {
            if self.nodes[nullifier.next_index].value == [0u8; 32] {
                return Ok(i);
            }
        }
        Err(NullifierMerkleTreeError::LowNullifierNotFound)
    }

    pub fn insert(&mut self, nullifiers: &[&[u8; 32]]) -> Result<(), NullifierMerkleTreeError> {
        for nullifier in nullifiers {
            let low_nullifier_index = self.find_low_nullifier(nullifier)?;

            let new_value = nullifier.to_owned().to_owned();

            let new_node = Node {
                value: new_value,
                next_index: self.nodes[low_nullifier_index].next_index,
            };

            // Insert new node.
            self.current_node_index += 1;
            self.nodes[self.current_node_index] = new_node;

            // Update the lower nullifier - point to the new node.
            self.nodes[low_nullifier_index].next_index = self.current_node_index;

            // Yield lower nullifier as a root.
            self.current_root_index = (self.current_root_index + 1) % ROOTS;
            self.roots[self.current_root_index] = self.nodes[low_nullifier_index].clone();

            // Yield new node as a root.
            self.current_root_index = (self.current_root_index + 1) % ROOTS;
            self.roots[self.current_root_index] = self.nodes[self.current_node_index].clone();
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Tests the insertion of nullifiers to the Merkle Tree.
    ///
    /// # Initial state
    ///
    /// The initial state of the Merkle Tree looks like:
    ///
    /// ```
    /// value      = [0] [0] [0] [0] [0] [0] [0] [0]
    /// next_index = [0] [0] [0] [0] [0] [0] [0] [0]
    /// ```
    ///
    /// # 1st insertion
    ///
    /// After adding a new value 30, it should look like:
    ///
    /// ```
    /// value      = [ 0]  [30] [0] [0] [0] [0] [0] [0]
    /// next_index = [ 1]  [ 0] [0] [0] [0] [0] [0] [0]
    /// ```
    ///
    /// Because:
    ///
    /// * Low nullifier is the first node, with index 0 and value 0. There is
    ///   no node with value greater as 30, so we found it as a one pointing to
    ///   node 0 (which will always have value 0).
    /// * The new nullifier is inserted in index 1.
    /// * `next_*` fields of the low nullifier are updated to point to the new
    ///   nullifier.
    ///
    /// # 2nd insertion
    ///
    /// After adding an another value 10, it should look like:
    ///
    /// ```
    /// value      = [ 0]  [30] [10] [0] [0] [0] [0] [0]
    /// next_index = [ 2]  [ 0] [ 1] [0] [0] [0] [0] [0]
    /// ```
    ///
    /// Because:
    ///
    /// * Low nullifier is still the node 0, but this time for differen reason -
    ///   its `next_index` 2 contains value 30, whish is greater than 10.
    /// * The new nullifier is inserted as node 2.
    /// * Low nullifier is pointing to the index 1. We assign the 1st nullifier
    ///   as the next nullifier of our new nullifier. Therefore, our new nullifier
    ///   looks like: `[value = 10, next_index = 1]`.
    /// * Low nullifier is updated to point to the new nullifier. Therefore,
    ///   after update it looks like: `[value = 0, next_index = 2]`.
    /// * The previously inserted nullifier, the node 1, remains unchanged.
    ///
    /// # 3rd insertion
    ///
    /// After adding an another value 20, it should look like:
    ///
    /// ```
    /// value      = [ 0]  [30] [10] [20] [0] [0] [0] [0]
    /// next_index = [ 2]  [ 0] [ 3] [ 1] [0] [0] [0] [0]
    /// ```
    ///
    /// Because:
    /// * Low nullifier is the node 2.
    /// * The new nullifier is inserted as node 3.
    /// * Low nullifier is pointing to the node 2. We assign the 1st nullifier
    ///   as the next nullifier of our new nullifier. Therefore, our new
    ///   nullifier looks like:
    /// * Low nullifier is updated to point to the new nullifier. Therefore,
    ///   after update it looks like: `[value = 10, next_index = 3]`.
    ///
    /// # 4th insertion
    ///
    /// After adding an another value 50, it should look like:
    ///
    /// ```
    /// value      = [ 0]  [30] [10] [20] [50] [0] [0] [0]
    /// next_index = [ 2]  [ 4] [ 3] [ 1] [0 ] [0] [0] [0]
    /// ```
    ///
    /// Because:
    ///
    /// * Low nullifier is the node 1 - there is no node with value greater
    ///   than 50, so we found it as a one having 0 as the `next_value`.
    /// * The new nullifier is inserted as node 4.
    /// * Low nullifier is not pointing to any node. So our new nullifier
    ///   is not going to point to any other node either. Therefore, the new
    ///   nullifier looks like: `[value = 50, next_index = 0]`.
    /// * Low nullifier is updated to point to the new nullifier. Therefore,
    ///   after update it looks like: `[value = 30, next_index = 4]`.
    #[test]
    fn test_insert_one_by_one() {
        let mut nullifier_merkle_tree: NullifierMerkleTree<8, 8> = NullifierMerkleTree::default();

        let nullifier1: &[&[u8; 32]] = &[&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 30,
        ]];
        nullifier_merkle_tree.insert(&nullifier1).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            Node {
                value: [0u8; 32],
                next_index: 1,
            },
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30,
                ],
                next_index: 0,
            }
        );

        let nullifier2: &[&[u8; 32]] = &[&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 10,
        ]];
        nullifier_merkle_tree.insert(&nullifier2).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            Node {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 0,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 1,
            }
        );

        let nullifier3: &[&[u8; 32]] = &[&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 20,
        ]];
        nullifier_merkle_tree.insert(nullifier3).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            Node {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 0,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[3],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
            }
        );

        let nullifier4: &[&[u8; 32]] = &[&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50,
        ]];
        nullifier_merkle_tree.insert(nullifier4).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            Node {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 4,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[3],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[4],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ],
                next_index: 0,
            }
        );
    }

    /// Tests the same case as above, but inserts all nullifiers at once.
    #[test]
    fn test_insert_all_at_once() {
        let mut nullifier_merkle_tree: NullifierMerkleTree<8, 8> = NullifierMerkleTree::default();

        let nullifiers: &[&[u8; 32]] = &[
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 30,
            ],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 10,
            ],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 20,
            ],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 50,
            ],
        ];

        nullifier_merkle_tree.insert(&nullifiers).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            Node {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 4,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[3],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[4],
            Node {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ],
                next_index: 0,
            }
        );
    }
}
