use std::{cmp::Ordering, fmt::Debug, marker::PhantomData};

use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher};
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned, Zero};

use crate::{changelog::RawIndexedElement, errors::IndexedArrayError};

#[derive(Clone, Debug, Default)]
pub struct IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    pub index: I,
    pub value: BigUint,
    pub next_index: I,
}

impl<I> From<RawIndexedElement<I>> for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    fn from(value: RawIndexedElement<I>) -> Self {
        IndexedElement {
            index: value.index,
            value: BigUint::from_bytes_be(&value.value),
            next_index: value.next_index,
        }
    }
}

impl<I> PartialEq for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.index == other.index
            && self.next_index == other.next_index
    }
}

impl<I> Eq for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
}

impl<I> PartialOrd for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I> Ord for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<I> IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    pub fn index(&self) -> usize {
        self.index.into()
    }

    pub fn next_index(&self) -> usize {
        self.next_index.into()
    }

    pub fn hash<H>(&self, next_value: &BigUint) -> Result<[u8; 32], IndexedArrayError>
    where
        H: Hasher,
    {
        let hash = H::hashv(&[
            bigint_to_be_bytes_array::<32>(&self.value)?.as_ref(),
            bigint_to_be_bytes_array::<32>(next_value)?.as_ref(),
        ])?;

        Ok(hash)
    }

    pub fn update_from_raw_element(&mut self, raw_element: &RawIndexedElement<I>) {
        self.index = raw_element.index;
        self.value = BigUint::from_bytes_be(&raw_element.value);
        self.next_index = raw_element.next_index;
    }
}

#[derive(Clone, Debug)]
pub struct IndexedElementBundle<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    pub new_low_element: IndexedElement<I>,
    pub new_element: IndexedElement<I>,
    pub new_element_next_value: BigUint,
}

#[derive(Clone, Debug)]
pub struct IndexedArray<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    pub elements: Vec<IndexedElement<I>>,
    pub current_node_index: I,
    pub highest_element_index: I,
    pub highest_value: BigUint,

    _hasher: PhantomData<H>,
}

impl<H, I> Default for IndexedArray<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    fn default() -> Self {
        Self {
            elements: vec![IndexedElement {
                index: I::zero(),
                value: BigUint::zero(),
                next_index: I::zero(),
            }],
            current_node_index: I::zero(),
            highest_element_index: I::zero(),
            highest_value: BigUint::zero(),
            _hasher: PhantomData,
        }
    }
}

impl<H, I> IndexedArray<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    pub fn new(value: BigUint, next_value: BigUint) -> Self {
        Self {
            current_node_index: I::zero(),
            highest_element_index: I::zero(),
            highest_value: next_value,
            elements: vec![IndexedElement {
                index: I::zero(),
                value,
                next_index: I::zero(),
            }],
            _hasher: PhantomData,
        }
    }
    pub fn get(&self, index: usize) -> Option<&IndexedElement<I>> {
        self.elements.get(index)
    }

    pub fn len(&self) -> usize {
        self.current_node_index.into()
    }

    pub fn is_empty(&self) -> bool {
        self.current_node_index == I::zero()
    }

    pub fn iter(&self) -> IndexingArrayIter<'_, H, I> {
        IndexingArrayIter {
            indexing_array: self,
            front: 0,
            back: self.current_node_index.into(),
        }
    }

    pub fn find_element(&self, value: &BigUint) -> Option<&IndexedElement<I>> {
        self.elements[..self.len() + 1]
            .iter()
            .find(|&node| node.value == *value)
    }

    /// Returns the index of the low element for the given `value`, which is
    /// not yet the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_index_for_nonexistent(
        &self,
        value: &BigUint,
    ) -> Result<I, IndexedArrayError> {
        // Try to find element whose next element is higher than the provided
        // value.
        for (i, node) in self.elements.iter().enumerate() {
            if node.value == *value {
                return Err(IndexedArrayError::ElementAlreadyExists);
            }
            if self.elements[node.next_index()].value > *value && node.value < *value {
                return i.try_into().map_err(|_| IndexedArrayError::IntegerOverflow);
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
    /// For the given `value`, which is not yet the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_for_nonexistent(
        &self,
        value: &BigUint,
    ) -> Result<(IndexedElement<I>, BigUint), IndexedArrayError> {
        let low_element_index = self.find_low_element_index_for_nonexistent(value)?;
        let low_element = self.elements[low_element_index.into()].clone();
        let next_value = if low_element.next_index == I::zero() {
            self.highest_value.clone()
        } else {
            self.elements[low_element.next_index.into()].value.clone()
        };
        Ok((low_element.clone(), next_value))
    }

    /// Returns the index of the low element for the given `value`, which is
    /// already the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_index_for_existent(
        &self,
        value: &BigUint,
    ) -> Result<I, IndexedArrayError> {
        for (i, node) in self.elements[..self.len() + 1].iter().enumerate() {
            if self.elements[node.next_index.into()].value == *value {
                let i = i
                    .try_into()
                    .map_err(|_| IndexedArrayError::IntegerOverflow)?;
                return Ok(i);
            }
        }
        Err(IndexedArrayError::ElementDoesNotExist)
    }

    /// Returns the low element for the given `value`, which is already the
    /// part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_for_existent(
        &self,
        value: &BigUint,
    ) -> Result<IndexedElement<I>, IndexedArrayError> {
        let low_element_index = self.find_low_element_index_for_existent(value)?;
        let low_element = self.elements[low_element_index.into()].clone();
        Ok(low_element)
    }

    /// Returns the hash of the given element. That hash consists of:
    ///
    /// * The value of the given element.
    /// * The `next_index` of the given element.
    /// * The value of the element pointed by `next_index`.
    pub fn hash_element(&self, index: I) -> Result<[u8; 32], IndexedArrayError> {
        let element = self
            .elements
            .get(index.into())
            .ok_or(IndexedArrayError::IndexHigherThanMax)?;
        let next_element = self
            .elements
            .get(element.next_index.into())
            .ok_or(IndexedArrayError::IndexHigherThanMax)?;
        element.hash::<H>(&next_element.value)
    }

    /// Returns an updated low element and a new element, created based on the
    /// provided `low_element_index` and `value`.
    pub fn new_element_with_low_element_index(
        &self,
        low_element_index: I,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedArrayError> {
        let mut new_low_element = self.elements[low_element_index.into()].clone();

        let new_element_index = self
            .current_node_index
            .checked_add(&I::one())
            .ok_or(IndexedArrayError::IntegerOverflow)?;
        let new_element = IndexedElement {
            index: new_element_index,
            value: value.clone(),
            next_index: new_low_element.next_index,
        };

        new_low_element.next_index = new_element_index;

        let new_element_next_value = if new_element.next_index == I::zero() {
            self.highest_value.clone()
        } else {
            self.elements[new_element.next_index.into()].value.clone()
        };

        Ok(IndexedElementBundle {
            new_low_element,
            new_element,
            new_element_next_value,
        })
    }

    pub fn new_element(
        &self,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedArrayError> {
        let low_element_index = self.find_low_element_index_for_nonexistent(value)?;
        let element = self.new_element_with_low_element_index(low_element_index, value)?;

        Ok(element)
    }

    /// Appends the given `value` to the indexing array.
    pub fn append_with_low_element_index(
        &mut self,
        low_element_index: I,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedArrayError> {
        // TOD0: add length check, and add field to with tree height here

        let old_low_element = &self.elements[low_element_index.into()];

        // Check that the `value` belongs to the range of `old_low_element`.
        if old_low_element.next_index == I::zero() {
            // In this case, the `old_low_element` is the greatest element.
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if value <= &old_low_element.value {
                return Err(IndexedArrayError::LowElementGreaterOrEqualToNewElement);
            }
        } else {
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if value <= &old_low_element.value {
                return Err(IndexedArrayError::LowElementGreaterOrEqualToNewElement);
            }
            // The value of `new_element` needs to be lower than the value of
            // next element pointed by `old_low_element`.
            if value >= &self.elements[old_low_element.next_index.into()].value {
                return Err(IndexedArrayError::NewElementGreaterOrEqualToNextElement);
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
        self.elements.push(new_element_bundle.new_element.clone());

        // Update low element.
        self.elements[low_element_index.into()] = new_element_bundle.new_low_element.clone();

        Ok(new_element_bundle)
    }

    pub fn append(
        &mut self,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedArrayError> {
        let low_element_index = self.find_low_element_index_for_nonexistent(value)?;
        self.append_with_low_element_index(low_element_index, value)
    }

    pub fn lowest(&self) -> Option<IndexedElement<I>> {
        if self.current_node_index < I::one() {
            None
        } else {
            self.elements.get(1).cloned()
        }
    }
}

pub struct IndexingArrayIter<'a, H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    indexing_array: &'a IndexedArray<H, I>,
    front: usize,
    back: usize,
}

impl<'a, H, I> Iterator for IndexingArrayIter<'a, H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
{
    type Item = &'a IndexedElement<I>;

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

impl<H, I> DoubleEndedIterator for IndexingArrayIter<'_, H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    I: Into<usize>,
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
