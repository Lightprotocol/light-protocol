use std::{cmp::Ordering, marker::PhantomData};

use light_hasher::{errors::HasherError, Hasher};

#[derive(Copy, Clone, Debug, Default)]
pub struct IndexingElement {
    pub index: usize,
    pub value: [u8; 32],
    pub next_index: usize,
    pub next_value: [u8; 32],
}

impl PartialEq for IndexingElement {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for IndexingElement {}

impl PartialOrd for IndexingElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexingElement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl IndexingElement {
    pub fn hash<H>(&self) -> Result<[u8; 32], HasherError>
    where
        H: Hasher,
    {
        H::hashv(&[
            self.value.as_slice(),
            self.next_index.to_le_bytes().as_slice(),
            self.next_value.as_slice(),
        ])
    }
}

pub struct IndexingArray<H, const ELEMENTS: usize>
where
    H: Hasher,
{
    pub(crate) elements: [IndexingElement; ELEMENTS],
    current_node_index: usize,
    highest_element_index: usize,

    _hasher: PhantomData<H>,
}

impl<H, const ELEMENTS: usize> Default for IndexingArray<H, ELEMENTS>
where
    H: Hasher,
{
    fn default() -> Self {
        Self {
            elements: std::array::from_fn(|_| IndexingElement {
                index: 0,
                value: [0u8; 32],
                next_index: 0,
                next_value: [0u8; 32],
            }),
            current_node_index: 0,
            highest_element_index: 0,
            _hasher: PhantomData,
        }
    }
}

impl<H, const ELEMENTS: usize> IndexingArray<H, ELEMENTS>
where
    H: Hasher,
{
    pub fn get(&self, index: usize) -> Option<&IndexingElement> {
        self.elements.get(index)
    }

    pub fn len(&self) -> usize {
        self.current_node_index
    }

    pub fn empty(&self) -> bool {
        self.current_node_index == 0
    }

    pub fn iter(&self) -> IndexingArrayIter<H, ELEMENTS> {
        println!("current_node_index: {}", self.current_node_index);
        IndexingArrayIter {
            indexing_array: self,
            front: 0,
            back: self.current_node_index,
        }
    }

    pub fn find_element(&self, value: &[u8; 32]) -> Option<&IndexingElement> {
        for node in self.elements[..self.current_node_index + 1].iter() {
            if node.value == *value {
                return Some(node);
            }
        }
        None
    }

    /// Returns the index of the low element for the given `value`, which
    /// **should not** be the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_index(&self, value: &[u8; 32]) -> usize {
        // Try to find element whose next element is higher than the provided
        // value.
        for (i, node) in self.elements[..self.current_node_index + 1]
            .iter()
            .enumerate()
        {
            if node.next_value > *value {
                return i;
            }
        }
        // If no such element was found, it means that our value is going to be
        // the greatest in the array. This means that the currently greatest
        // element is going to be the low element of our value.
        self.highest_element_index
    }

    /// Returns the index of the low element for the given value.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element(&self, value: &[u8; 32]) -> IndexingElement {
        let low_element_index = self.find_low_element_index(value);
        self.elements[low_element_index]
    }

    /// Returns the index of the low element for the given `value`, which
    /// **should** be the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_index_for_existing_element(&self, value: &[u8; 32]) -> Option<usize> {
        for (i, node) in self.elements[..self.current_node_index + 1]
            .iter()
            .enumerate()
        {
            if node.next_value == *value {
                return Some(i);
            }
        }
        None
    }

    /// Returns the hash of the given element. That hash consists of:
    ///
    /// * The value of the given element.
    /// * The `next_index` of the given element.
    /// * The value of the element pointed by `next_index`.
    pub fn hash_element(&self, index: usize) -> Result<[u8; 32], HasherError> {
        let element = self
            .elements
            .get(index)
            .ok_or(HasherError::IndexHigherThanMax)?;
        let next_element = self
            .elements
            .get(element.next_index)
            .ok_or(HasherError::IndexHigherThanMax)?;
        H::hashv(&[
            element.value.as_slice(),
            element.next_index.to_le_bytes().as_slice(),
            next_element.value.as_slice(),
        ])
    }

    /// Returns an updated low element and a new element, created based on the
    /// provided `low_element_index` and `value`.
    pub fn new_element_with_low_element_index(
        &self,
        low_element_index: usize,
        value: [u8; 32],
    ) -> (IndexingElement, IndexingElement) {
        let mut low_element = self.elements[low_element_index];

        let new_element_index = self.current_node_index + 1;
        let new_element = IndexingElement {
            index: new_element_index,
            value,
            next_index: low_element.next_index,
            next_value: low_element.next_value,
        };

        low_element.next_index = new_element_index;
        low_element.next_value = value;

        (low_element, new_element)
    }

    pub fn new_element(
        &self,
        value: [u8; 32],
    ) -> Result<(IndexingElement, IndexingElement), HasherError> {
        let low_element_index = self.find_low_element_index(&value);
        let element = self.new_element_with_low_element_index(low_element_index, value);

        Ok(element)
    }

    /// Appends the given `value` to the indexing array.
    pub fn append_with_low_element_index(
        &mut self,
        low_element_index: usize,
        value: [u8; 32],
    ) -> (IndexingElement, IndexingElement) {
        let old_low_element = self.elements[low_element_index];

        // Create new node.
        let (new_low_element, new_element) =
            self.new_element_with_low_element_index(low_element_index, value);

        // If the old low element wasn't pointing to any element, it means that:
        //
        // * It used to be the highest element.
        // * Our new element, which we are appending, is going the be the
        //   highest element.
        //
        // Therefore, we need to save the new element index as the highest
        // index.
        if old_low_element.next_value == [0u8; 32] {
            self.highest_element_index = new_element.index;
        }

        // Insert new node.
        self.current_node_index = new_element.index;
        self.elements[self.current_node_index] = new_element;

        // Update low element.
        self.elements[low_element_index] = new_low_element;

        (new_low_element, new_element)
    }

    pub fn append(
        &mut self,
        value: [u8; 32],
    ) -> Result<(IndexingElement, IndexingElement), HasherError> {
        let low_element_index = self.find_low_element_index(&value);
        let node = self.append_with_low_element_index(low_element_index, value);

        Ok(node)
    }

    pub fn lowest(&self) -> Option<IndexingElement> {
        self.elements.get(1).cloned()
    }

    /// Returns and removes the element from the given index.
    ///
    /// It also performs necessary updated of the remaning elements, to
    /// preserve the integrity of the array.
    ///
    /// The low element under `low_element_index` is updated, to point to a new
    /// next element instead of the one which is removed.
    pub fn dequeue_at_with_low_element_index(
        &mut self,
        low_element_index: usize,
        index: usize,
    ) -> Option<IndexingElement> {
        if index > self.current_node_index {
            // Index out of bounds.
            return None;
        }

        // Save the element to be removed.
        let removed_element = self.elements[index];

        // Update the lower element - point to the node which the currently
        // removed element is pointing to.
        self.elements[low_element_index].next_index = removed_element.next_index;
        self.elements[low_element_index].next_value = removed_element.next_value;

        for i in 0..self.current_node_index {
            // Shift elements, which are on the right from the removed element,
            // to the left.
            if i >= index {
                self.elements[i] = self.elements[i + 1];
                self.elements[i].index -= 1;
            }
            // If the `next_index` is greater than the index of the removed
            // element, decrement it. Elements on the right from the removed
            // element are going to be shifted left.
            if self.elements[i].next_index >= index {
                self.elements[i].next_index -= 1;
            }
        }

        // Update current_node_index
        self.current_node_index -= 1;

        Some(removed_element)
    }

    /// Returns and removes the element from the given index.
    ///
    /// It also performs necessary updates of the remaning elements, to
    /// preserve the integrity of the array. It searches for the low element
    /// and updates it, to point to a new next element instead of the one
    pub fn dequeue_at(&mut self, index: usize) -> Result<Option<IndexingElement>, HasherError> {
        match self.elements.get(index) {
            Some(node) => {
                let low_element_index = self
                    .find_low_element_index_for_existing_element(&node.value)
                    .ok_or(HasherError::LowElementNotFound)?;
                Ok(self.dequeue_at_with_low_element_index(low_element_index, index))
            }
            None => Ok(None),
        }
    }

    pub fn try_extend<T: IntoIterator<Item = [u8; 32]>>(
        &mut self,
        iter: T,
    ) -> Result<(), HasherError> {
        for new_value in iter {
            self.append(new_value)?;
        }
        Ok(())
    }
}

pub struct IndexingArrayIter<'a, H, const MAX_ELEMENTS: usize>
where
    H: Hasher,
{
    indexing_array: &'a IndexingArray<H, MAX_ELEMENTS>,
    front: usize,
    back: usize,
}

impl<'a, H, const MAX_ELEMENTS: usize> Iterator for IndexingArrayIter<'a, H, MAX_ELEMENTS>
where
    H: Hasher,
{
    type Item = &'a IndexingElement;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front <= self.back {
            let result = self.indexing_array.elements.get(self.front);
            self.front += 1;
            result
        } else {
            None
        }
    }
}

impl<'a, H, const MAX_ELEMENTS: usize> DoubleEndedIterator
    for IndexingArrayIter<'a, H, MAX_ELEMENTS>
where
    H: Hasher,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.back >= self.front {
            let result = self.indexing_array.elements.get(self.back);
            self.back -= 1;
            result
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use light_hasher::Poseidon;

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
        let mut nullifier_merkle_tree: IndexingArray<Poseidon, 8> = IndexingArray::default();

        let nullifier1: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 30,
        ];
        nullifier_merkle_tree.append(nullifier1).unwrap();

        assert_eq!(
            nullifier_merkle_tree.elements[0],
            IndexingElement {
                index: 0,
                value: [0u8; 32],
                next_index: 1,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30,
                ],
            },
        );
        assert_eq!(
            nullifier_merkle_tree.elements[1],
            IndexingElement {
                index: 1,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30,
                ],
                next_index: 0,
                next_value: [0u8; 32],
            }
        );

        let nullifier2: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 10,
        ];
        nullifier_merkle_tree.append(nullifier2).unwrap();

        assert_eq!(
            nullifier_merkle_tree.elements[0],
            IndexingElement {
                index: 0,
                value: [0u8; 32],
                next_index: 2,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10,
                ],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[1],
            IndexingElement {
                index: 1,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 0,
                next_value: [0u8; 32],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[2],
            IndexingElement {
                index: 2,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 1,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ]
            }
        );

        let nullifier3: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 20,
        ];
        nullifier_merkle_tree.append(nullifier3).unwrap();

        assert_eq!(
            nullifier_merkle_tree.elements[0],
            IndexingElement {
                index: 0,
                value: [0u8; 32],
                next_index: 2,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[1],
            IndexingElement {
                index: 1,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 0,
                next_value: [0u8; 32],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[2],
            IndexingElement {
                index: 2,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[3],
            IndexingElement {
                index: 3,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ]
            }
        );

        let nullifier4: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50,
        ];
        nullifier_merkle_tree.append(nullifier4).unwrap();

        assert_eq!(
            nullifier_merkle_tree.elements[0],
            IndexingElement {
                index: 0,
                value: [0u8; 32],
                next_index: 2,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ]
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[1],
            IndexingElement {
                index: 1,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 4,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[2],
            IndexingElement {
                index: 2,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[3],
            IndexingElement {
                index: 3,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ]
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[4],
            IndexingElement {
                index: 4,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ],
                next_index: 0,
                next_value: [0u8; 32],
            }
        );
    }

    /// Tests the same case as above, but inserts all nullifiers at once.
    #[test]
    fn test_insert_all_at_once() {
        let mut nullifier_merkle_tree: IndexingArray<Poseidon, 8> = IndexingArray::default();

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
            nullifier_merkle_tree.elements[0],
            IndexingElement {
                index: 0,
                value: [0u8; 32],
                next_index: 2,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[1],
            IndexingElement {
                index: 1,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ],
                next_index: 4,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ]
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[2],
            IndexingElement {
                index: 2,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 10
                ],
                next_index: 3,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ]
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[3],
            IndexingElement {
                index: 3,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 20
                ],
                next_index: 1,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 30
                ]
            }
        );
        assert_eq!(
            nullifier_merkle_tree.elements[4],
            IndexingElement {
                index: 4,
                value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 50
                ],
                next_index: 0,
                next_value: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0
                ]
            }
        );
    }
}
