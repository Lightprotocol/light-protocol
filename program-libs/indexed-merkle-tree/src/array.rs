use std::{cmp::Ordering, fmt::Debug, marker::PhantomData};

use light_concurrent_merkle_tree::{event::RawIndexedElement, light_hasher::Hasher};
use light_hasher::bigint::bigint_to_be_bytes_array;
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned, Zero};

use crate::{errors::IndexedMerkleTreeError, HIGHEST_ADDRESS_PLUS_ONE};

#[derive(Clone, Debug, Default)]
pub struct IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    pub index: I,
    pub value: BigUint,
    pub next_index: I,
}

impl<I> From<RawIndexedElement<I>> for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
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
    usize: From<I>,
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
    usize: From<I>,
{
}

impl<I> PartialOrd for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I> Ord for IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<I> IndexedElement<I>
where
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    pub fn index(&self) -> usize {
        self.index.into()
    }

    pub fn next_index(&self) -> usize {
        self.next_index.into()
    }

    pub fn hash<H>(&self, next_value: &BigUint) -> Result<[u8; 32], IndexedMerkleTreeError>
    where
        H: Hasher,
    {
        let mut bytes = [0u8; 32];
        let len = std::mem::size_of::<I>();
        bytes[32 - len..].copy_from_slice(self.next_index.to_be_bytes().as_ref());
        let hash = H::hashv(&[
            bigint_to_be_bytes_array::<32>(&self.value)?.as_ref(),
            &bytes,
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
    usize: From<I>,
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
    usize: From<I>,
{
    pub elements: Vec<IndexedElement<I>>,
    pub current_node_index: I,
    pub highest_element_index: I,

    _hasher: PhantomData<H>,
}

impl<H, I> Default for IndexedArray<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
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
            _hasher: PhantomData,
        }
    }
}

impl<H, I> IndexedArray<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
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

    pub fn init(&mut self) -> Result<IndexedElementBundle<I>, IndexedMerkleTreeError> {
        use num_traits::Num;
        let init_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10)
            .map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?;
        self.append(&init_value)
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
    ) -> Result<I, IndexedMerkleTreeError> {
        // Try to find element whose next element is higher than the provided
        // value.
        for (i, node) in self.elements.iter().enumerate() {
            if node.value == *value {
                return Err(IndexedMerkleTreeError::ElementAlreadyExists);
            }
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
    /// For the given `value`, which is not yet the part of the array.
    ///
    /// Low element is the greatest element which still has lower value than
    /// the provided one.
    ///
    /// Low elements are used in non-membership proofs.
    pub fn find_low_element_for_nonexistent(
        &self,
        value: &BigUint,
    ) -> Result<(IndexedElement<I>, BigUint), IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element_index_for_nonexistent(value)?;
        let low_element = self.elements[usize::from(low_element_index)].clone();
        Ok((
            low_element.clone(),
            self.elements[low_element.next_index()].value.clone(),
        ))
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
    ) -> Result<I, IndexedMerkleTreeError> {
        for (i, node) in self.elements[..self.len() + 1].iter().enumerate() {
            if self.elements[usize::from(node.next_index)].value == *value {
                let i = i
                    .try_into()
                    .map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?;
                return Ok(i);
            }
        }
        Err(IndexedMerkleTreeError::ElementDoesNotExist)
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
    ) -> Result<IndexedElement<I>, IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element_index_for_existent(value)?;
        let low_element = self.elements[usize::from(low_element_index)].clone();
        Ok(low_element)
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
        element.hash::<H>(&next_element.value)
    }

    /// Returns an updated low element and a new element, created based on the
    /// provided `low_element_index` and `value`.
    pub fn new_element_with_low_element_index(
        &self,
        low_element_index: I,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedMerkleTreeError> {
        let mut new_low_element = self.elements[usize::from(low_element_index)].clone();

        let new_element_index = self
            .current_node_index
            .checked_add(&I::one())
            .ok_or(IndexedMerkleTreeError::IntegerOverflow)?;
        let new_element = IndexedElement {
            index: new_element_index,
            value: value.clone(),
            next_index: new_low_element.next_index,
        };

        new_low_element.next_index = new_element_index;

        let new_element_next_value = self.elements[usize::from(new_element.next_index)]
            .value
            .clone();

        Ok(IndexedElementBundle {
            new_low_element,
            new_element,
            new_element_next_value,
        })
    }

    pub fn new_element(
        &self,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedMerkleTreeError> {
        let low_element_index = self.find_low_element_index_for_nonexistent(value)?;
        let element = self.new_element_with_low_element_index(low_element_index, value)?;

        Ok(element)
    }

    /// Appends the given `value` to the indexing array.
    pub fn append_with_low_element_index(
        &mut self,
        low_element_index: I,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedMerkleTreeError> {
        // TOD0: add length check, and add field to with tree height here

        let old_low_element = &self.elements[usize::from(low_element_index)];

        // Check that the `value` belongs to the range of `old_low_element`.
        if old_low_element.next_index == I::zero() {
            // In this case, the `old_low_element` is the greatest element.
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if value <= &old_low_element.value {
                return Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement);
            }
        } else {
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if value <= &old_low_element.value {
                return Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement);
            }
            // The value of `new_element` needs to be lower than the value of
            // next element pointed by `old_low_element`.
            if value >= &self.elements[usize::from(old_low_element.next_index)].value {
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
        self.elements.push(new_element_bundle.new_element.clone());

        // Update low element.
        self.elements[usize::from(low_element_index)] = new_element_bundle.new_low_element.clone();

        Ok(new_element_bundle)
    }

    pub fn append(
        &mut self,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<I>, IndexedMerkleTreeError> {
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
    usize: From<I>,
{
    indexing_array: &'a IndexedArray<H, I>,
    front: usize,
    back: usize,
}

impl<'a, H, I> Iterator for IndexingArrayIter<'a, H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
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
    use light_concurrent_merkle_tree::light_hasher::Poseidon;
    use num_bigint::{RandBigInt, ToBigUint};
    use rand::thread_rng;

    use super::*;

    #[test]
    fn test_indexed_element_cmp() {
        let mut rng = thread_rng();

        for _ in 0..1000 {
            let value = rng.gen_biguint(128);
            let element_1 = IndexedElement::<u16> {
                index: 0,
                value: value.clone(),
                next_index: 1,
            };
            let element_2 = IndexedElement::<u16> {
                index: 0,
                value,
                next_index: 1,
            };
            assert_eq!(element_1, element_2);
            assert_eq!(element_2, element_1);
            assert!(matches!(element_1.cmp(&element_2), Ordering::Equal));
            assert!(matches!(element_2.cmp(&element_1), Ordering::Equal));

            let value_higher = rng.gen_biguint(128);
            if value_higher == 0.to_biguint().unwrap() {
                continue;
            }
            let value_lower = rng.gen_biguint_below(&value_higher);
            let element_lower = IndexedElement::<u16> {
                index: 0,
                value: value_lower,
                next_index: 1,
            };
            let element_higher = IndexedElement::<u16> {
                index: 1,
                value: value_higher,
                next_index: 2,
            };
            assert_ne!(element_lower, element_higher);
            assert_ne!(element_higher, element_lower);
            assert!(matches!(element_lower.cmp(&element_higher), Ordering::Less));
            assert!(matches!(
                element_higher.cmp(&element_lower),
                Ordering::Greater
            ));
            assert!(matches!(
                element_lower.partial_cmp(&element_higher),
                Some(Ordering::Less)
            ));
            assert!(matches!(
                element_higher.partial_cmp(&element_lower),
                Some(Ordering::Greater)
            ));
        }
    }

    /// Tests the insertion of elements to the indexing array.
    #[test]
    fn test_append() {
        // The initial state of the array looks like:
        //
        // ```
        // value      = [0] [0] [0] [0] [0] [0] [0] [0]
        // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
        // ```
        let mut indexed_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

        let nullifier1 = 30_u32.to_biguint().unwrap();
        let bundle1 = indexed_array.new_element(&nullifier1).unwrap();
        assert!(indexed_array.find_element(&nullifier1).is_none());
        indexed_array.append(&nullifier1).unwrap();

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
            indexed_array.find_element(&nullifier1),
            Some(&bundle1.new_element),
        );
        let expected_hash = Poseidon::hashv(&[
            bigint_to_be_bytes_array::<32>(&nullifier1)
                .unwrap()
                .as_ref(),
            0_usize.to_be_bytes().as_ref(),
            bigint_to_be_bytes_array::<32>(&(0.to_biguint().unwrap()))
                .unwrap()
                .as_ref(),
        ])
        .unwrap();
        assert_eq!(indexed_array.hash_element(1).unwrap(), expected_hash);
        assert_eq!(
            indexed_array.elements[0],
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 1,
            },
        );
        assert_eq!(
            indexed_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );
        assert_eq!(
            indexed_array.iter().collect::<Vec<_>>().as_slice(),
            &[
                &IndexedElement {
                    index: 0,
                    value: 0_u32.to_biguint().unwrap(),
                    next_index: 1,
                },
                &IndexedElement {
                    index: 1,
                    value: 30_u32.to_biguint().unwrap(),
                    next_index: 0
                }
            ]
        );

        let nullifier2 = 10_u32.to_biguint().unwrap();
        let bundle2 = indexed_array.new_element(&nullifier2).unwrap();
        assert!(indexed_array.find_element(&nullifier2).is_none());
        indexed_array.append(&nullifier2).unwrap();

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
            indexed_array.find_element(&nullifier2),
            Some(&bundle2.new_element),
        );
        let expected_hash = Poseidon::hashv(&[
            bigint_to_be_bytes_array::<32>(&nullifier2)
                .unwrap()
                .as_ref(),
            1_usize.to_be_bytes().as_ref(),
            bigint_to_be_bytes_array::<32>(&(30.to_biguint().unwrap()))
                .unwrap()
                .as_ref(),
        ])
        .unwrap();
        assert_eq!(indexed_array.hash_element(2).unwrap(), expected_hash);
        assert_eq!(
            indexed_array.elements[0],
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexed_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );
        assert_eq!(
            indexed_array.elements[2],
            IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        );
        assert_eq!(
            indexed_array.iter().collect::<Vec<_>>().as_slice(),
            &[
                &IndexedElement {
                    index: 0,
                    value: 0_u32.to_biguint().unwrap(),
                    next_index: 2,
                },
                &IndexedElement {
                    index: 1,
                    value: 30_u32.to_biguint().unwrap(),
                    next_index: 0,
                },
                &IndexedElement {
                    index: 2,
                    value: 10_u32.to_biguint().unwrap(),
                    next_index: 1,
                }
            ]
        );

        let nullifier3 = 20_u32.to_biguint().unwrap();
        let bundle3 = indexed_array.new_element(&nullifier3).unwrap();
        assert!(indexed_array.find_element(&nullifier3).is_none());
        indexed_array.append(&nullifier3).unwrap();

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
            indexed_array.find_element(&nullifier3),
            Some(&bundle3.new_element),
        );
        let expected_hash = Poseidon::hashv(&[
            bigint_to_be_bytes_array::<32>(&nullifier3)
                .unwrap()
                .as_ref(),
            1_usize.to_be_bytes().as_ref(),
            bigint_to_be_bytes_array::<32>(&(30.to_biguint().unwrap()))
                .unwrap()
                .as_ref(),
        ])
        .unwrap();
        assert_eq!(indexed_array.hash_element(3).unwrap(), expected_hash);
        assert_eq!(
            indexed_array.elements[0],
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexed_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );
        assert_eq!(
            indexed_array.elements[2],
            IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 3,
            }
        );
        assert_eq!(
            indexed_array.elements[3],
            IndexedElement {
                index: 3,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        );
        assert_eq!(
            indexed_array.iter().collect::<Vec<_>>().as_slice(),
            &[
                &IndexedElement {
                    index: 0,
                    value: 0_u32.to_biguint().unwrap(),
                    next_index: 2,
                },
                &IndexedElement {
                    index: 1,
                    value: 30_u32.to_biguint().unwrap(),
                    next_index: 0,
                },
                &IndexedElement {
                    index: 2,
                    value: 10_u32.to_biguint().unwrap(),
                    next_index: 3,
                },
                &IndexedElement {
                    index: 3,
                    value: 20_u32.to_biguint().unwrap(),
                    next_index: 1
                }
            ]
        );

        let nullifier4 = 50_u32.to_biguint().unwrap();
        let bundle4 = indexed_array.new_element(&nullifier4).unwrap();
        assert!(indexed_array.find_element(&nullifier4).is_none());
        indexed_array.append(&nullifier4).unwrap();

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
            indexed_array.find_element(&nullifier4),
            Some(&bundle4.new_element),
        );
        let expected_hash = Poseidon::hashv(&[
            bigint_to_be_bytes_array::<32>(&nullifier4)
                .unwrap()
                .as_ref(),
            0_usize.to_be_bytes().as_ref(),
            bigint_to_be_bytes_array::<32>(&(0.to_biguint().unwrap()))
                .unwrap()
                .as_ref(),
        ])
        .unwrap();
        assert_eq!(indexed_array.hash_element(4).unwrap(), expected_hash);
        assert_eq!(
            indexed_array.elements[0],
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexed_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 4,
            }
        );
        assert_eq!(
            indexed_array.elements[2],
            IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 3,
            }
        );
        assert_eq!(
            indexed_array.elements[3],
            IndexedElement {
                index: 3,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        );
        assert_eq!(
            indexed_array.elements[4],
            IndexedElement {
                index: 4,
                value: 50_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );
        assert_eq!(
            indexed_array.iter().collect::<Vec<_>>().as_slice(),
            &[
                &IndexedElement {
                    index: 0,
                    value: 0_u32.to_biguint().unwrap(),
                    next_index: 2,
                },
                &IndexedElement {
                    index: 1,
                    value: 30_u32.to_biguint().unwrap(),
                    next_index: 4,
                },
                &IndexedElement {
                    index: 2,
                    value: 10_u32.to_biguint().unwrap(),
                    next_index: 3,
                },
                &IndexedElement {
                    index: 3,
                    value: 20_u32.to_biguint().unwrap(),
                    next_index: 1,
                },
                &IndexedElement {
                    index: 4,
                    value: 50_u32.to_biguint().unwrap(),
                    next_index: 0,
                }
            ]
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
        let mut indexing_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

        let low_element_index = 0;
        let nullifier1 = 30_u32.to_biguint().unwrap();
        indexing_array
            .append_with_low_element_index(low_element_index, &nullifier1)
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
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 1,
            },
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );

        let low_element_index = 0;
        let nullifier2 = 10_u32.to_biguint().unwrap();
        indexing_array
            .append_with_low_element_index(low_element_index, &nullifier2)
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
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        );

        let low_element_index = 2;
        let nullifier3 = 20_u32.to_biguint().unwrap();
        indexing_array
            .append_with_low_element_index(low_element_index, &nullifier3)
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
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 3,
            }
        );
        assert_eq!(
            indexing_array.elements[3],
            IndexedElement {
                index: 3,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        );

        let low_element_index = 1;
        let nullifier4 = 50_u32.to_biguint().unwrap();
        indexing_array
            .append_with_low_element_index(low_element_index, &nullifier4)
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
            IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            }
        );
        assert_eq!(
            indexing_array.elements[1],
            IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 4,
            }
        );
        assert_eq!(
            indexing_array.elements[2],
            IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 3,
            }
        );
        assert_eq!(
            indexing_array.elements[3],
            IndexedElement {
                index: 3,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        );
        assert_eq!(
            indexing_array.elements[4],
            IndexedElement {
                index: 4,
                value: 50_u32.to_biguint().unwrap(),
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
        let mut indexing_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

        // Append nullifier 30. The low nullifier is at index 0. The array
        // should look like:
        //
        // ```
        // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
        // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
        // ```
        let low_element_index = 0;
        let nullifier1 = 30_u32.to_biguint().unwrap();
        indexing_array
            .append_with_low_element_index(low_element_index, &nullifier1)
            .unwrap();

        // Try appending nullifier 20, while pointing to index 1 as low
        // nullifier.
        // Therefore, the new element is lower than the supposed low element.
        let low_element_index = 1;
        let nullifier2 = 20_u32.to_biguint().unwrap();
        assert!(matches!(
            indexing_array.append_with_low_element_index(low_element_index, &nullifier2),
            Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement)
        ));

        // Try appending nullifier 50, while pointing to index 0 as low
        // nullifier.
        // Therefore, the new element is greater than next element.
        let low_element_index = 0;
        let nullifier2 = 50_u32.to_biguint().unwrap();
        assert!(matches!(
            indexing_array.append_with_low_element_index(low_element_index, &nullifier2),
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
        let nullifier2 = 50_u32.to_biguint().unwrap();
        indexing_array
            .append_with_low_element_index(low_element_index, &nullifier2)
            .unwrap();

        // Try appending nullifier 40, while pointint to index 2 (value 50) as
        // low nullifier.
        // Therefore, the pointed low element is greater than the new element.
        let low_element_index = 2;
        let nullifier3 = 40_u32.to_biguint().unwrap();
        assert!(matches!(
            indexing_array.append_with_low_element_index(low_element_index, &nullifier3),
            Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement)
        ));
    }

    /// Tests whether `find_*_for_existent` elements return `None` when a
    /// nonexistent is provided.
    #[test]
    fn test_find_low_element_for_existent_element() {
        let mut indexed_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

        // Append nullifiers 40 and 20.
        let low_element_index = 0;
        let nullifier_1 = 40_u32.to_biguint().unwrap();
        indexed_array
            .append_with_low_element_index(low_element_index, &nullifier_1)
            .unwrap();
        let low_element_index = 0;
        let nullifier_2 = 20_u32.to_biguint().unwrap();
        indexed_array
            .append_with_low_element_index(low_element_index, &nullifier_2)
            .unwrap();

        // Try finding a low element for nonexistent nullifier 30.
        let nonexistent_nullifier = 30_u32.to_biguint().unwrap();
        // `*_existent` methods should fail.
        let res = indexed_array.find_low_element_index_for_existent(&nonexistent_nullifier);
        assert!(matches!(
            res,
            Err(IndexedMerkleTreeError::ElementDoesNotExist)
        ));
        let res = indexed_array.find_low_element_for_existent(&nonexistent_nullifier);
        assert!(matches!(
            res,
            Err(IndexedMerkleTreeError::ElementDoesNotExist)
        ));
        // `*_nonexistent` methods should succeed.
        let low_element_index = indexed_array
            .find_low_element_index_for_nonexistent(&nonexistent_nullifier)
            .unwrap();
        assert_eq!(low_element_index, 2);
        let low_element = indexed_array
            .find_low_element_for_nonexistent(&nonexistent_nullifier)
            .unwrap();
        assert_eq!(
            low_element,
            (
                IndexedElement::<usize> {
                    index: 2,
                    value: 20_u32.to_biguint().unwrap(),
                    next_index: 1,
                },
                40_u32.to_biguint().unwrap(),
            )
        );

        // Try finding a low element of existent nullifier 40.
        // `_existent` methods should succeed.
        let low_element_index = indexed_array
            .find_low_element_index_for_existent(&nullifier_1)
            .unwrap();
        assert_eq!(low_element_index, 2);
        let low_element = indexed_array
            .find_low_element_for_existent(&nullifier_1)
            .unwrap();
        assert_eq!(
            low_element,
            IndexedElement::<usize> {
                index: 2,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            },
        );
        // `*_nonexistent` methods should fail.
        let res = indexed_array.find_low_element_index_for_nonexistent(&nullifier_1);
        assert!(matches!(
            res,
            Err(IndexedMerkleTreeError::ElementAlreadyExists)
        ));
        let res = indexed_array.find_low_element_for_nonexistent(&nullifier_1);
        assert!(matches!(
            res,
            Err(IndexedMerkleTreeError::ElementAlreadyExists)
        ));
    }
}
