use std::{cmp::Ordering, marker::PhantomData};

use ark_ff::{BigInteger, BigInteger256};
use borsh::{BorshDeserialize, BorshSerialize};
use light_concurrent_merkle_tree::light_hasher::Hasher;
use light_utils::bigint::{be_bytes_to_bigint, bigint_to_be_bytes};
use num_bigint::{BigInt, Sign};
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::errors::IndexedMerkleTreeError;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct RawIndexingElement<I, const N: usize>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
{
    pub index: I,
    pub value: [u8; N],
    pub next_index: I,
}

#[derive(Clone, Debug, Default)]
pub struct IndexingElement<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    pub index: I,
    pub value: B,
    pub next_index: I,
}

impl<I> TryFrom<RawIndexingElement<I, 32>> for IndexingElement<I, BigInteger256>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    type Error = ();

    fn try_from(element: RawIndexingElement<I, 32>) -> Result<Self, Self::Error> {
        let value = be_bytes_to_bigint(&element.value).map_err(|_| ())?;
        Ok(Self {
            index: element.index,
            value,
            next_index: element.next_index,
        })
    }
}

impl<I> TryFrom<IndexingElement<I, BigInteger256>> for RawIndexingElement<I, 32>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    type Error = ();

    fn try_from(element: IndexingElement<I, BigInteger256>) -> Result<Self, Self::Error> {
        let value = bigint_to_be_bytes(&element.value).map_err(|_| ())?;
        Ok(Self {
            index: element.index,
            value,
            next_index: element.next_index,
        })
    }
}

impl<I, B> PartialEq for IndexingElement<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<I, B> Eq for IndexingElement<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
}

impl<I, B> PartialOrd for IndexingElement<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I, B> Ord for IndexingElement<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<I, B> IndexingElement<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    pub fn index(&self) -> usize {
        self.index.into()
    }

    pub fn next_index(&self) -> usize {
        self.next_index.into()
    }

    pub fn hash<H>(&self, next_value: &B) -> Result<[u8; 32], IndexedMerkleTreeError>
    where
        H: Hasher,
    {
        let hash = H::hashv(&[
            self.value.to_bytes_be().as_ref(),
            self.next_index.to_be_bytes().as_ref(),
            next_value.to_bytes_be().as_ref(),
        ])?;

        let hashBN = BigInt::from_bytes_be(Sign::Plus, &hash);

        println!(
            "hash={:?}, value={:?}, next_index={:?}, next_value={:?}",
            hashBN,
            self.value.to_bytes_be(),
            self.next_index.to_be_bytes(),
            next_value
        );
        Ok(hash)
    }
}

pub struct IndexingElementBundle<I, B>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    pub new_low_element: IndexingElement<I, B>,
    pub new_element: IndexingElement<I, B>,
    pub new_element_next_value: B,
}

pub struct IndexingArray<H, I, B, const ELEMENTS: usize>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    pub elements: [IndexingElement<I, B>; ELEMENTS],
    pub current_node_index: I,
    pub highest_element_index: I,

    _hasher: PhantomData<H>,
}

impl<H, I, B, const ELEMENTS: usize> Default for IndexingArray<H, I, B, ELEMENTS>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    fn default() -> Self {
        Self {
            elements: std::array::from_fn(|_| IndexingElement {
                index: I::zero(),
                value: B::from(0_u32),
                next_index: I::zero(),
            }),
            current_node_index: I::zero(),
            highest_element_index: I::zero(),
            _hasher: PhantomData,
        }
    }
}

impl<H, I, B, const ELEMENTS: usize> IndexingArray<H, I, B, ELEMENTS>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    pub fn get(&self, index: usize) -> Option<&IndexingElement<I, B>> {
        self.elements.get(index)
    }

    pub fn len(&self) -> usize {
        self.current_node_index.into()
    }

    pub fn is_empty(&self) -> bool {
        self.current_node_index == I::zero()
    }

    pub fn iter(&self) -> IndexingArrayIter<H, I, B, ELEMENTS> {
        IndexingArrayIter {
            indexing_array: self,
            front: 0,
            back: self.current_node_index.into(),
        }
    }

    pub fn find_element(&self, value: &B) -> Option<&IndexingElement<I, B>> {
        self.elements[..self.len() + 1]
            .iter()
            .find(|&node| node.value == *value)
    }

    /// Returns the index of the low element for the given `value`, which
    /// **should not** be the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_index(&self, value: &B) -> Result<I, IndexedMerkleTreeError> {
        // Try to find element whose next element is higher than the provided
        // value.
        for (i, node) in self.elements[..self.len() + 1].iter().enumerate() {
            if self.elements[node.next_index()].value > *value && node.value < *value {
                return i
                    .try_into()
                    .map_err(|_| IndexedMerkleTreeError::IntegerOverflow);
            }
        }
        // If no such element was found, it means that our value is going to be
        // the greatest in the array. This means that the currently greatest
        // element is going to be the low element of our value.
        Ok(self.highest_element_index)
    }

    /// Returns the:
    ///
    /// * Low element for the given value.
    /// * Next value for that low element.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element(
        &self,
        value: &B,
    ) -> Result<(IndexingElement<I, B>, B), IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element_index(value)?;
        let low_element = self.elements[usize::from(low_element_index)].clone();
        Ok((
            low_element.clone(),
            self.elements[low_element.next_index()].value,
        ))
    }

    /// Returns the index of the low element for the given `value`, which
    /// **should** be the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_index_for_existing_element(
        &self,
        value: &B,
    ) -> Result<Option<I>, IndexedMerkleTreeError> {
        for (i, node) in self.elements[..self.len() + 1].iter().enumerate() {
            if self.elements[usize::from(node.next_index)].value == *value {
                let i = i
                    .try_into()
                    .map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?;
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    /// Returns the hash of the given element. That hash consists of:
    ///
    /// * The value of the given element.
    /// * The `next_index` of the given element.
    /// * The value of the element pointed by `next_index`.
    pub fn hash_element(&self, index: I) -> Result<[u8; 32], IndexedMerkleTreeError> {
        let element = self
            .elements
            .get(usize::from(index))
            .ok_or(IndexedMerkleTreeError::IndexHigherThanMax)?;
        let next_element = self
            .elements
            .get(usize::from(element.next_index))
            .ok_or(IndexedMerkleTreeError::IndexHigherThanMax)?;
        let hash = H::hashv(&[
            element.value.to_bytes_le().as_ref(),
            element.next_index.to_le_bytes().as_ref(),
            next_element.value.to_bytes_le().as_ref(),
        ])?;
        Ok(hash)
    }

    /// Returns an updated low element and a new element, created based on the
    /// provided `low_element_index` and `value`.
    pub fn new_element_with_low_element_index(
        &self,
        low_element_index: I,
        value: B,
    ) -> Result<IndexingElementBundle<I, B>, IndexedMerkleTreeError> {
        let mut new_low_element = self.elements[usize::from(low_element_index)].clone();

        let new_element_index = self
            .current_node_index
            .checked_add(&I::one())
            .ok_or(IndexedMerkleTreeError::IntegerOverflow)?;
        let new_element = IndexingElement {
            index: new_element_index,
            value,
            next_index: new_low_element.next_index,
        };

        new_low_element.next_index = new_element_index;

        let new_element_next_value = self.elements[usize::from(new_element.next_index)].value;

        Ok(IndexingElementBundle {
            new_low_element,
            new_element,
            new_element_next_value,
        })
    }

    pub fn new_element(
        &self,
        value: B,
    ) -> Result<IndexingElementBundle<I, B>, IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element_index(&value)?;
        let element = self.new_element_with_low_element_index(low_element_index, value)?;

        Ok(element)
    }

    /// Appends the given `value` to the indexing array.
    pub fn append_with_low_element_index(
        &mut self,
        low_element_index: I,
        value: B,
    ) -> Result<IndexingElementBundle<I, B>, IndexedMerkleTreeError> {
        let old_low_element = &self.elements[usize::from(low_element_index)];

        // Check that the `value` belongs to the range of `old_low_element`.
        if old_low_element.next_index == I::zero() {
            // In this case, the `old_low_element` is the greatest element.
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if value <= old_low_element.value {
                return Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement);
            }
        } else {
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if value <= old_low_element.value {
                return Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement);
            }
            // The value of `new_element` needs to be lower than the value of
            // next element pointed by `old_low_element`.
            if value >= self.elements[usize::from(old_low_element.next_index)].value {
                return Err(IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement);
            }
        }

        // Create new node.
        let new_element_bundle =
            self.new_element_with_low_element_index(low_element_index, value)?;

        // If the old low element wasn't pointing to any element, it means that:
        //
        // * It used to be the highest element.
        // * Our new element, which we are appending, is going the be the
        //   highest element.
        //
        // Therefore, we need to save the new element index as the highest
        // index.
        if old_low_element.next_index == I::zero() {
            self.highest_element_index = new_element_bundle.new_element.index;
        }

        // Insert new node.
        self.current_node_index = new_element_bundle.new_element.index;
        self.elements[self.len()] = new_element_bundle.new_element.clone();

        // Update low element.
        self.elements[usize::from(low_element_index)] = new_element_bundle.new_low_element.clone();

        Ok(new_element_bundle)
    }

    pub fn append(
        &mut self,
        value: B,
    ) -> Result<IndexingElementBundle<I, B>, IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element_index(&value)?;
        self.append_with_low_element_index(low_element_index, value)
    }

    pub fn lowest(&self) -> Option<IndexingElement<I, B>> {
        if self.current_node_index < I::one() {
            None
        } else {
            self.elements.get(1).cloned()
        }
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
        low_element_index: I,
        index: I,
    ) -> Result<Option<IndexingElement<I, B>>, IndexedMerkleTreeError> {
        if index > self.current_node_index {
            // Index out of bounds.
            return Ok(None);
        }

        // Save the element to be removed.
        let removed_element = self.elements[usize::from(index)].clone();

        // Update the lower element - point to the node which the currently
        // removed element is pointing to.
        self.elements[usize::from(low_element_index)].next_index = removed_element.next_index;

        let mut new_highest_element_index = I::zero();
        for i in 0..usize::from(self.current_node_index) {
            // Shift elements, which are on the right from the removed element,
            // to the left.
            if i >= usize::from(index) {
                self.elements[i] = self.elements[i
                    .checked_add(1_usize)
                    .ok_or(IndexedMerkleTreeError::IntegerOverflow)?]
                .clone();
                self.elements[i].index = self.elements[i]
                    .index
                    .checked_sub(&I::one())
                    .ok_or(IndexedMerkleTreeError::IntegerOverflow)?;
            }
            // If the `next_index` is greater than the index of the removed
            // element, decrement it. Elements on the right from the removed
            // element are going to be shifted left.
            if self.elements[i].next_index >= index {
                self.elements[i].next_index = self.elements[i]
                    .next_index
                    .checked_sub(&I::one())
                    .ok_or(IndexedMerkleTreeError::IntegerOverflow)?;
            }

            if self.elements[i].value > self.elements[usize::from(new_highest_element_index)].value
            {
                new_highest_element_index = i
                    .try_into()
                    .map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?;
            }
        }

        // Update current_node_index
        self.current_node_index = self
            .current_node_index
            .checked_sub(&I::one())
            .ok_or(IndexedMerkleTreeError::IntegerOverflow)?;
        // Update highest_element_index
        self.highest_element_index = new_highest_element_index;

        Ok(Some(removed_element))
    }

    /// Returns and removes the element from the given index.
    ///
    /// It also performs necessary updates of the remaning elements, to
    /// preserve the integrity of the array. It searches for the low element
    /// and updates it, to point to a new next element instead of the one
    pub fn dequeue_at(
        &mut self,
        index: I,
    ) -> Result<Option<IndexingElement<I, B>>, IndexedMerkleTreeError> {
        match self.elements.get(usize::from(index)) {
            Some(node) => {
                let low_element_index = self
                    .find_low_element_index_for_existing_element(&node.value)?
                    .ok_or(IndexedMerkleTreeError::LowElementNotFound)?;
                self.dequeue_at_with_low_element_index(low_element_index, index)
            }
            None => Ok(None),
        }
    }
}

pub struct IndexingArrayIter<'a, H, I, B, const MAX_ELEMENTS: usize>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    indexing_array: &'a IndexingArray<H, I, B, MAX_ELEMENTS>,
    front: usize,
    back: usize,
}

impl<'a, H, I, B, const MAX_ELEMENTS: usize> Iterator
    for IndexingArrayIter<'a, H, I, B, MAX_ELEMENTS>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    type Item = &'a IndexingElement<I, B>;

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

impl<'a, H, I, B, const MAX_ELEMENTS: usize> DoubleEndedIterator
    for IndexingArrayIter<'a, H, I, B, MAX_ELEMENTS>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
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
    use ark_ff::BigInteger256;
    use light_concurrent_merkle_tree::light_hasher::Poseidon;

    use super::*;

    /// Tests the insertion of elements to the indexing array.
    #[test]
    fn test_append() {
        // The initial state of the array looks like:
        //
        // ```
        // value      = [0] [0] [0] [0] [0] [0] [0] [0]
        // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
        // ```
        let mut indexing_array: IndexingArray<Poseidon, usize, BigInteger256, 8> =
            IndexingArray::default();

        let nullifier1 = BigInteger256::from(30_u32);
        indexing_array.append(nullifier1).unwrap();

        // After adding a new value 30, it should look like:
        //
        // ```
        // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
        // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
        // ```
        //
        // Because:
        //
        // * Low element is the first node, with index 0 and value 0. There is
        //   no node with value greater as 30, so we found it as a one pointing to
        //   node 0 (which will always have value 0).
        // * The new nullifier is inserted in index 1.
        // * `next_*` fields of the low nullifier are updated to point to the new
        //   nullifier.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 1,
            },
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 0,
            }
        );

        let nullifier2 = BigInteger256::from(10_u32);
        indexing_array.append(nullifier2).unwrap();

        // After adding an another value 10, it should look like:
        //
        // ```
        // value      = [ 0] [30] [10] [0] [0] [0] [0] [0]
        // next_index = [ 2] [ 0] [ 1] [0] [0] [0] [0] [0]
        // ```
        //
        // Because:
        //
        // * Low nullifier is still the node 0, but this time for differen reason -
        //   its `next_index` 2 contains value 30, whish is greater than 10.
        // * The new nullifier is inserted as node 2.
        // * Low nullifier is pointing to the index 1. We assign the 1st nullifier
        //   as the next nullifier of our new nullifier. Therefore, our new nullifier
        //   looks like: `[value = 10, next_index = 1]`.
        // * Low nullifier is updated to point to the new nullifier. Therefore,
        //   after update it looks like: `[value = 0, next_index = 2]`.
        // * The previously inserted nullifier, the node 1, remains unchanged.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 0,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexingElement {
                index: 2,
                value: BigInteger256::from(10_u32),
                next_index: 1,
            }
        );

        let nullifier3 = BigInteger256::from(20_u32);
        indexing_array.append(nullifier3).unwrap();

        // After adding an another value 20, it should look like:
        //
        // ```
        // value      = [ 0] [30] [10] [20] [0] [0] [0] [0]
        // next_index = [ 2] [ 0] [ 3] [ 1] [0] [0] [0] [0]
        // ```
        //
        // Because:
        // * Low nullifier is the node 2.
        // * The new nullifier is inserted as node 3.
        // * Low nullifier is pointing to the node 2. We assign the 1st nullifier
        //   as the next nullifier of our new nullifier. Therefore, our new
        //   nullifier looks like:
        // * Low nullifier is updated to point to the new nullifier. Therefore,
        //   after update it looks like: `[value = 10, next_index = 3]`.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 0,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexingElement {
                index: 2,
                value: BigInteger256::from(10_u32),
                next_index: 3,
            }
        );
        assert_eq!(
            indexing_array.elements[3],
            IndexingElement {
                index: 3,
                value: BigInteger256::from(20_u32),
                next_index: 1,
            }
        );

        let nullifier4 = BigInteger256::from(50_u32);
        indexing_array.append(nullifier4).unwrap();

        // After adding an another value 50, it should look like:
        //
        // ```
        // value      = [ 0]  [30] [10] [20] [50] [0] [0] [0]
        // next_index = [ 2]  [ 4] [ 3] [ 1] [0 ] [0] [0] [0]
        // ```
        //
        // Because:
        //
        // * Low nullifier is the node 1 - there is no node with value greater
        //   than 50, so we found it as a one having 0 as the `next_value`.
        // * The new nullifier is inserted as node 4.
        // * Low nullifier is not pointing to any node. So our new nullifier
        //   is not going to point to any other node either. Therefore, the new
        //   nullifier looks like: `[value = 50, next_index = 0]`.
        // * Low nullifier is updated to point to the new nullifier. Therefore,
        //   after update it looks like: `[value = 30, next_index = 4]`.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 4,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexingElement {
                index: 2,
                value: BigInteger256::from(10_u32),
                next_index: 3,
            }
        );
        assert_eq!(
            indexing_array.elements[3],
            IndexingElement {
                index: 3,
                value: BigInteger256::from(20_u32),
                next_index: 1,
            }
        );
        assert_eq!(
            indexing_array.elements[4],
            IndexingElement {
                index: 4,
                value: BigInteger256::from(50_u32),
                next_index: 0,
            }
        );
    }

    #[test]
    fn test_append_with_low_element_index() {
        // The initial state of the array looks like:
        //
        // ```
        // value      = [0] [0] [0] [0] [0] [0] [0] [0]
        // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
        // ```
        let mut indexing_array: IndexingArray<Poseidon, usize, BigInteger256, 8> =
            IndexingArray::default();

        let low_element_index = 0;
        let nullifier1 = BigInteger256::from(30_u32);
        indexing_array
            .append_with_low_element_index(low_element_index, nullifier1)
            .unwrap();

        // After adding a new value 30, it should look like:
        //
        // ```
        // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
        // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
        // ```
        //
        // Because:
        //
        // * Low element is the first node, with index 0 and value 0. There is
        //   no node with value greater as 30, so we found it as a one pointing to
        //   node 0 (which will always have value 0).
        // * The new nullifier is inserted in index 1.
        // * `next_*` fields of the low nullifier are updated to point to the new
        //   nullifier.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 1,
            },
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 0,
            }
        );

        let low_element_index = 0;
        let nullifier2 = BigInteger256::from(10_u32);
        indexing_array
            .append_with_low_element_index(low_element_index, nullifier2)
            .unwrap();

        // After adding an another value 10, it should look like:
        //
        // ```
        // value      = [ 0] [30] [10] [0] [0] [0] [0] [0]
        // next_index = [ 2] [ 0] [ 1] [0] [0] [0] [0] [0]
        // ```
        //
        // Because:
        //
        // * Low nullifier is still the node 0, but this time for differen reason -
        //   its `next_index` 2 contains value 30, whish is greater than 10.
        // * The new nullifier is inserted as node 2.
        // * Low nullifier is pointing to the index 1. We assign the 1st nullifier
        //   as the next nullifier of our new nullifier. Therefore, our new nullifier
        //   looks like: `[value = 10, next_index = 1]`.
        // * Low nullifier is updated to point to the new nullifier. Therefore,
        //   after update it looks like: `[value = 0, next_index = 2]`.
        // * The previously inserted nullifier, the node 1, remains unchanged.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 0,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexingElement {
                index: 2,
                value: BigInteger256::from(10_u32),
                next_index: 1,
            }
        );

        let low_element_index = 2;
        let nullifier3 = BigInteger256::from(20_u32);
        indexing_array
            .append_with_low_element_index(low_element_index, nullifier3)
            .unwrap();

        // After adding an another value 20, it should look like:
        //
        // ```
        // value      = [ 0] [30] [10] [20] [0] [0] [0] [0]
        // next_index = [ 2] [ 0] [ 3] [ 1] [0] [0] [0] [0]
        // ```
        //
        // Because:
        // * Low nullifier is the node 2.
        // * The new nullifier is inserted as node 3.
        // * Low nullifier is pointing to the node 2. We assign the 1st nullifier
        //   as the next nullifier of our new nullifier. Therefore, our new
        //   nullifier looks like:
        // * Low nullifier is updated to point to the new nullifier. Therefore,
        //   after update it looks like: `[value = 10, next_index = 3]`.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 0,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexingElement {
                index: 2,
                value: BigInteger256::from(10_u32),
                next_index: 3,
            }
        );
        assert_eq!(
            indexing_array.elements[3],
            IndexingElement {
                index: 3,
                value: BigInteger256::from(20_u32),
                next_index: 1,
            }
        );

        let low_element_index = 1;
        let nullifier4 = BigInteger256::from(50_u32);
        indexing_array
            .append_with_low_element_index(low_element_index, nullifier4)
            .unwrap();

        // After adding an another value 50, it should look like:
        //
        // ```
        // value      = [ 0]  [30] [10] [20] [50] [0] [0] [0]
        // next_index = [ 2]  [ 4] [ 3] [ 1] [0 ] [0] [0] [0]
        // ```
        //
        // Because:
        //
        // * Low nullifier is the node 1 - there is no node with value greater
        //   than 50, so we found it as a one having 0 as the `next_value`.
        // * The new nullifier is inserted as node 4.
        // * Low nullifier is not pointing to any node. So our new nullifier
        //   is not going to point to any other node either. Therefore, the new
        //   nullifier looks like: `[value = 50, next_index = 0]`.
        // * Low nullifier is updated to point to the new nullifier. Therefore,
        //   after update it looks like: `[value = 30, next_index = 4]`.
        assert_eq!(
            indexing_array.elements[0],
            IndexingElement {
                index: 0,
                value: BigInteger256::zero(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexingElement {
                index: 1,
                value: BigInteger256::from(30_u32),
                next_index: 4,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexingElement {
                index: 2,
                value: BigInteger256::from(10_u32),
                next_index: 3,
            }
        );
        assert_eq!(
            indexing_array.elements[3],
            IndexingElement {
                index: 3,
                value: BigInteger256::from(20_u32),
                next_index: 1,
            }
        );
        assert_eq!(
            indexing_array.elements[4],
            IndexingElement {
                index: 4,
                value: BigInteger256::from(50_u32),
                next_index: 0,
            }
        );
    }

    /// Tries to violate the integrity of the array by pointing to invalid low
    /// nullifiers. Tests whether the range check works correctly and disallows
    /// the invalid appends from happening.
    #[test]
    fn test_append_with_low_element_index_invalid() {
        // The initial state of the array looks like:
        //
        // ```
        // value      = [0] [0] [0] [0] [0] [0] [0] [0]
        // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
        // ```
        let mut indexing_array: IndexingArray<Poseidon, usize, BigInteger256, 8> =
            IndexingArray::default();

        // Append nullifier 30. The low nullifier is at index 0. The array
        // should look like:
        //
        // ```
        // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
        // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
        // ```
        let low_element_index = 0;
        let nullifier1 = BigInteger256::from(30_u32);
        indexing_array
            .append_with_low_element_index(low_element_index, nullifier1)
            .unwrap();

        // Try appending nullifier 20, while pointing to index 1 as low
        // nullifier.
        // Therefore, the new element is lower than the supposed low element.
        let low_element_index = 1;
        let nullifier2 = BigInteger256::from(20_u32);
        assert!(matches!(
            indexing_array.append_with_low_element_index(low_element_index, nullifier2),
            Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement)
        ));

        // Try appending nullifier 50, while pointing to index 0 as low
        // nullifier.
        // Therefore, the new element is greater than next element.
        let low_element_index = 0;
        let nullifier2 = BigInteger256::from(50_u32);
        assert!(matches!(
            indexing_array.append_with_low_element_index(low_element_index, nullifier2),
            Err(IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement),
        ));

        // Append nullifier 50 correctly, with 0 as low nullifier. The array
        // should look like:
        //
        // ```
        // value      = [ 0] [30] [50] [0] [0] [0] [0] [0]
        // next_index = [ 1] [ 2] [ 0] [0] [0] [0] [0] [0]
        // ```
        let low_element_index = 1;
        let nullifier2 = BigInteger256::from(50_u32);
        indexing_array
            .append_with_low_element_index(low_element_index, nullifier2)
            .unwrap();

        // Try appending nullifier 40, while pointint to index 2 (value 50) as
        // low nullifier.
        // Therefore, the pointed low element is greater than the new element.
        let low_element_index = 2;
        let nullifier3 = BigInteger256::from(40_u32);
        assert!(matches!(
            indexing_array.append_with_low_element_index(low_element_index, nullifier3),
            Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement)
        ));
    }
}
