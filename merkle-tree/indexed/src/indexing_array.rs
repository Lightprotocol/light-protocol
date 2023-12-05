use crate::errors::IndexedMerkleTreeError;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IndexingNode {
    pub(crate) value: [u8; 32],
    pub(crate) next_index: usize,
}

pub struct IndexingArray<const MAX_ELEMENTS: usize> {
    pub(crate) nodes: [IndexingNode; MAX_ELEMENTS],
    current_node_index: usize,
}

impl<const MAX_ELEMENTS: usize> Default for IndexingArray<MAX_ELEMENTS> {
    fn default() -> Self {
        Self {
            nodes: std::array::from_fn(|_| IndexingNode {
                value: [0u8; 32],
                next_index: 0,
            }),
            current_node_index: 0,
        }
    }
}

impl<const MAX_ELEMENTS: usize> IndexingArray<MAX_ELEMENTS> {
    pub fn append(&mut self, value: [u8; 32]) -> Result<(usize, usize), IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element(&value)?;
        let next_index = self.nodes[low_element_index].next_index;

        let value = value.to_owned().to_owned();

        let new_node = IndexingNode {
            value,
            next_index: self.nodes[low_element_index].next_index,
        };

        // Insert new node.
        self.current_node_index += 1;
        self.nodes[self.current_node_index] = new_node;

        // Update the lower nullifier - point to the new node.
        self.nodes[low_element_index].next_index = self.current_node_index;

        Ok((low_element_index, self.current_node_index))
    }

    pub fn try_extend<T: IntoIterator<Item = [u8; 32]>>(
        &mut self,
        iter: T,
    ) -> Result<(), IndexedMerkleTreeError> {
        for new_value in iter {
            self.append(new_value)?;
        }

        Ok(())
    }
}

impl<const MAX_ELEMENTS: usize> IndexingArray<MAX_ELEMENTS> {
    fn find_low_element(&mut self, new_value: &[u8; 32]) -> Result<usize, IndexedMerkleTreeError> {
        for (i, node) in self.nodes.iter().enumerate() {
            if self.nodes[node.next_index].value > *new_value {
                return Ok(i);
            }
        }
        for (i, node) in self.nodes.iter().enumerate() {
            if self.nodes[node.next_index].value == [0u8; 32] {
                return Ok(i);
            }
        }
        Err(IndexedMerkleTreeError::LowElementNotFound)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Tests the insertion of elements to the indexing array.
    ///
    /// # Initial state
    ///
    /// The initial state of the array looks like:
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
    /// * Low element is the first node, with index 0 and value 0. There is
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
    fn test_append() {
        let mut nullifier_merkle_tree: IndexingArray<8> = IndexingArray::default();

        let nullifier1: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 30,
        ];
        nullifier_merkle_tree.append(nullifier1).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            IndexingNode {
                value: [0u8; 32],
                next_index: 1,
            },
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30,
                ],
                next_index: 0,
            }
        );

        let nullifier2: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 10,
        ];
        nullifier_merkle_tree.append(nullifier2).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            IndexingNode {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 0,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 1,
            }
        );

        let nullifier3: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 20,
        ];
        nullifier_merkle_tree.append(nullifier3).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            IndexingNode {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 0,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[3],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
            }
        );

        let nullifier4: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50,
        ];
        nullifier_merkle_tree.append(nullifier4).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            IndexingNode {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 4,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[3],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[4],
            IndexingNode {
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
        let mut nullifier_merkle_tree: IndexingArray<8> = IndexingArray::default();

        let nullifiers: [[u8; 32]; 4] = [
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 30,
            ],
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 10,
            ],
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 20,
            ],
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 50,
            ],
        ];

        nullifier_merkle_tree.try_extend(nullifiers).unwrap();

        assert_eq!(
            nullifier_merkle_tree.nodes[0],
            IndexingNode {
                value: [0u8; 32],
                next_index: 2,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[1],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 4,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[2],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[3],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
            }
        );
        assert_eq!(
            nullifier_merkle_tree.nodes[4],
            IndexingNode {
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ],
                next_index: 0,
            }
        );
    }
}
