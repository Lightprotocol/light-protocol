use std::{
    alloc::{self, handle_alloc_error, Layout},
    cmp::Ordering,
    marker::Send,
    mem,
    ptr::NonNull,
};

use light_hasher::{bigint::bigint_to_be_bytes_array, HasherError};
use num_bigint::{BigUint, ToBigUint};
use num_traits::{FromBytes, ToPrimitive};
use thiserror::Error;

pub mod zero_copy;

pub const ITERATIONS: usize = 20;

#[derive(Debug, Error, PartialEq)]
pub enum HashSetError {
    #[error("The hash set is full, cannot add any new elements")]
    Full,
    #[error("The provided element is already in the hash set")]
    ElementAlreadyExists,
    #[error("The provided element doesn't exist in the hash set")]
    ElementDoesNotExist,
    #[error("Could not convert the index from/to usize")]
    UsizeConv,
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Invalid buffer size, expected {0}, got {1}")]
    BufferSize(usize, usize),
    #[error("HasherError: big integer conversion error")]
    Hasher(#[from] HasherError),
}

impl From<HashSetError> for u32 {
    fn from(e: HashSetError) -> u32 {
        match e {
            HashSetError::Full => 9001,
            HashSetError::ElementAlreadyExists => 9002,
            HashSetError::ElementDoesNotExist => 9003,
            HashSetError::UsizeConv => 9004,
            HashSetError::IntegerOverflow => 9005,
            HashSetError::BufferSize(_, _) => 9006,
            HashSetError::Hasher(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<HashSetError> for solana_program_error::ProgramError {
    fn from(e: HashSetError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HashSetCell {
    pub value: [u8; 32],
    pub sequence_number: Option<usize>,
}

unsafe impl Send for HashSet {}

impl HashSetCell {
    /// Returns the value as a byte array.
    pub fn value_bytes(&self) -> [u8; 32] {
        self.value
    }

    /// Returns the value as a big number.
    pub fn value_biguint(&self) -> BigUint {
        BigUint::from_bytes_be(self.value.as_slice())
    }

    /// Returns the associated sequence number.
    pub fn sequence_number(&self) -> Option<usize> {
        self.sequence_number
    }

    /// Checks whether the value is marked with a sequence number.
    pub fn is_marked(&self) -> bool {
        self.sequence_number.is_some()
    }

    /// Checks whether the value is valid according to the provided
    /// `current_sequence_number` (which usually should be a sequence number
    /// associated with the Merkle tree).
    ///
    /// The value is valid if:
    ///
    /// * It was not annotated with sequence number.
    /// * Its sequence number is lower than the provided `sequence_number`.
    ///
    /// The value is invalid if it's lower or equal to the provided
    /// `sequence_number`.
    pub fn is_valid(&self, current_sequence_number: usize) -> bool {
        match self.sequence_number {
            Some(sequence_number) => match sequence_number.cmp(&current_sequence_number) {
                Ordering::Less | Ordering::Equal => false,
                Ordering::Greater => true,
            },
            None => true,
        }
    }
}

#[derive(Debug)]
pub struct HashSet {
    /// Capacity of the buckets.
    capacity: usize,
    /// Difference of sequence numbers, after which the given element can be
    /// replaced by an another one (with a sequence number higher than the
    /// threshold).
    pub sequence_threshold: usize,

    /// An array of buckets. It has a size equal to the expected number of
    /// elements.
    buckets: NonNull<Option<HashSetCell>>,
}

unsafe impl Send for HashSetCell {}

impl HashSet {
    /// Size of the struct **without** dynamically sized fields.
    pub fn non_dyn_fields_size() -> usize {
        // capacity
        mem::size_of::<usize>()
        // sequence_threshold
        + mem::size_of::<usize>()
    }

    /// Size which needs to be allocated on Solana account to fit the hash set.
    pub fn size_in_account(capacity_values: usize) -> usize {
        let dyn_fields_size = Self::non_dyn_fields_size();

        let buckets_size_unaligned = mem::size_of::<Option<HashSetCell>>() * capacity_values;
        // Make sure that alignment of `values` matches the alignment of `usize`.
        let buckets_size = buckets_size_unaligned + mem::align_of::<usize>()
            - (buckets_size_unaligned % mem::align_of::<usize>());

        dyn_fields_size + buckets_size
    }

    // Create a new hash set with the given capacity
    pub fn new(capacity_values: usize, sequence_threshold: usize) -> Result<Self, HashSetError> {
        // SAFETY: It's just a regular allocation.
        let layout = Layout::array::<Option<HashSetCell>>(capacity_values).unwrap();
        let values_ptr = unsafe { alloc::alloc(layout) as *mut Option<HashSetCell> };
        if values_ptr.is_null() {
            handle_alloc_error(layout);
        }
        let values = NonNull::new(values_ptr).unwrap();
        for i in 0..capacity_values {
            unsafe {
                std::ptr::write(values_ptr.add(i), None);
            }
        }

        Ok(HashSet {
            sequence_threshold,
            capacity: capacity_values,
            buckets: values,
        })
    }

    /// Creates a copy of `HashSet` from the given byte slice.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in the SDK code, to convert
    /// fetched Solana accounts to actual hash sets. Creating a copy is the
    /// safest way of conversion in async Rust.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the alignment and that the slice
    /// provides actual actual data of the hash set is the caller's
    /// responsibility.
    pub unsafe fn from_bytes_copy(bytes: &mut [u8]) -> Result<Self, HashSetError> {
        if bytes.len() < Self::non_dyn_fields_size() {
            return Err(HashSetError::BufferSize(
                Self::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        let capacity = usize::from_le_bytes(bytes[0..8].try_into().unwrap());
        let sequence_threshold = usize::from_le_bytes(bytes[8..16].try_into().unwrap());
        let expected_size = Self::size_in_account(capacity);
        if bytes.len() != expected_size {
            return Err(HashSetError::BufferSize(expected_size, bytes.len()));
        }

        let buckets_layout = Layout::array::<Option<HashSetCell>>(capacity).unwrap();
        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
        let buckets_dst_ptr = unsafe { alloc::alloc(buckets_layout) as *mut Option<HashSetCell> };
        if buckets_dst_ptr.is_null() {
            handle_alloc_error(buckets_layout);
        }
        let buckets = NonNull::new(buckets_dst_ptr).unwrap();
        for i in 0..capacity {
            std::ptr::write(buckets_dst_ptr.add(i), None);
        }

        let offset = Self::non_dyn_fields_size() + mem::size_of::<usize>();
        let buckets_src_ptr = bytes.as_ptr().add(offset) as *const Option<HashSetCell>;
        std::ptr::copy(buckets_src_ptr, buckets_dst_ptr, capacity);

        Ok(Self {
            capacity,
            sequence_threshold,
            buckets,
        })
    }

    fn probe_index(&self, value: &BigUint, iteration: usize) -> usize {
        // Increase stepsize over the capacity of the hash set.
        let iteration = iteration + self.capacity / 10;
        let probe_index = (value
            + iteration.to_biguint().unwrap() * iteration.to_biguint().unwrap())
            % self.capacity.to_biguint().unwrap();
        probe_index.to_usize().unwrap()
    }

    /// Returns a reference to a bucket under the given `index`. Does not check
    /// the validity.
    pub fn get_bucket(&self, index: usize) -> Option<&Option<HashSetCell>> {
        if index >= self.capacity {
            return None;
        }
        let bucket = unsafe { &*self.buckets.as_ptr().add(index) };
        Some(bucket)
    }

    /// Returns a mutable reference to a bucket under the given `index`. Does
    /// not check the validity.
    pub fn get_bucket_mut(&mut self, index: usize) -> Option<&mut Option<HashSetCell>> {
        if index >= self.capacity {
            return None;
        }
        let bucket = unsafe { &mut *self.buckets.as_ptr().add(index) };
        Some(bucket)
    }

    /// Returns a reference to an unmarked bucket under the given index. If the
    /// bucket is marked, returns `None`.
    pub fn get_unmarked_bucket(&self, index: usize) -> Option<&Option<HashSetCell>> {
        let bucket = self.get_bucket(index);
        let is_unmarked = match bucket {
            Some(Some(bucket)) => !bucket.is_marked(),
            Some(None) => false,
            None => false,
        };
        if is_unmarked {
            bucket
        } else {
            None
        }
    }

    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    fn insert_into_occupied_cell(
        &mut self,
        value_index: usize,
        value: &BigUint,
        current_sequence_number: usize,
    ) -> Result<bool, HashSetError> {
        // PANICS: We trust the bounds of `value_index` here.
        let bucket = self.get_bucket_mut(value_index).unwrap();

        match bucket {
            // The cell in the value array is already taken.
            Some(bucket) => {
                // We can overwrite that cell only if the element
                // is expired - when the difference between its
                // sequence number and provided sequence number is
                // greater than the threshold.
                if let Some(element_sequence_number) = bucket.sequence_number {
                    if current_sequence_number >= element_sequence_number {
                        *bucket = HashSetCell {
                            value: bigint_to_be_bytes_array(value)?,
                            sequence_number: None,
                        };
                        return Ok(true);
                    }
                }
                // Otherwise, we need to prevent having multiple valid
                // elements with the same value.
                if &BigUint::from_be_bytes(bucket.value.as_slice()) == value {
                    return Err(HashSetError::ElementAlreadyExists);
                }
            }
            // Panics: If there is a hash set cell pointing to a `None` value,
            // it means we really screwed up in the implementation...
            // That should never happen.
            None => unreachable!(),
        }
        Ok(false)
    }

    /// Inserts a value into the hash set, with `self.capacity_values` attempts.
    ///
    /// Every attempt uses quadratic probing to find an empty cell or a cell
    /// which can be overwritten.
    ///
    /// `current sequence_number` is used to check whether existing values can
    /// be overwritten.
    pub fn insert(
        &mut self,
        value: &BigUint,
        current_sequence_number: usize,
    ) -> Result<usize, HashSetError> {
        let index_bucket = self.find_element_iter(value, current_sequence_number, 0, ITERATIONS)?;
        let (index, is_new) = match index_bucket {
            Some(index) => index,
            None => {
                return Err(HashSetError::Full);
            }
        };

        match is_new {
            // The visited hash set cell points to a value in the array.
            false => {
                if self.insert_into_occupied_cell(index, value, current_sequence_number)? {
                    return Ok(index);
                }
            }
            true => {
                // PANICS: We trust the bounds of `index`.
                let bucket = self.get_bucket_mut(index).unwrap();

                *bucket = Some(HashSetCell {
                    value: bigint_to_be_bytes_array(value)?,
                    sequence_number: None,
                });
                return Ok(index);
            }
        }
        Err(HashSetError::Full)
    }

    /// Finds an index of the provided `value` inside `buckets`.
    ///
    /// Uses the optional `current_sequence_number` arguments for checking the
    /// validity of the element.
    pub fn find_element_index(
        &self,
        value: &BigUint,
        current_sequence_number: Option<usize>,
    ) -> Result<Option<usize>, HashSetError> {
        for i in 0..ITERATIONS {
            let probe_index = self.probe_index(value, i);
            // PANICS: `probe_index()` ensures the bounds.
            let bucket = self.get_bucket(probe_index).unwrap();
            match bucket {
                Some(bucket) => {
                    if &bucket.value_biguint() == value {
                        match current_sequence_number {
                            // If the caller provided `current_sequence_number`,
                            // check the validity of the bucket.
                            Some(current_sequence_number) => {
                                if bucket.is_valid(current_sequence_number) {
                                    return Ok(Some(probe_index));
                                }
                                continue;
                            }
                            None => return Ok(Some(probe_index)),
                        }
                    }
                    continue;
                }
                // If we found an empty bucket, it means that there is no
                // chance of our element existing in the hash set.
                None => {
                    return Ok(None);
                }
            }
        }

        Ok(None)
    }

    pub fn find_element(
        &self,
        value: &BigUint,
        current_sequence_number: Option<usize>,
    ) -> Result<Option<(&HashSetCell, usize)>, HashSetError> {
        let index = self.find_element_index(value, current_sequence_number)?;
        match index {
            Some(index) => {
                let bucket = self.get_bucket(index).unwrap();
                match bucket {
                    Some(bucket) => Ok(Some((bucket, index))),
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    pub fn find_element_mut(
        &mut self,
        value: &BigUint,
        current_sequence_number: Option<usize>,
    ) -> Result<Option<(&mut HashSetCell, usize)>, HashSetError> {
        let index = self.find_element_index(value, current_sequence_number)?;
        match index {
            Some(index) => {
                let bucket = self.get_bucket_mut(index).unwrap();
                match bucket {
                    Some(bucket) => Ok(Some((bucket, index))),
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// find_element_iter iterates over a fixed range of elements
    /// in the hash set.
    /// We always have to iterate over the whole range
    /// to make sure that the value is not in the hash-set.
    /// Returns the position of the first free value.
    pub fn find_element_iter(
        &mut self,
        value: &BigUint,
        current_sequence_number: usize,
        start_iter: usize,
        num_iterations: usize,
    ) -> Result<Option<(usize, bool)>, HashSetError> {
        let mut first_free_element: Option<(usize, bool)> = None;
        for i in start_iter..start_iter + num_iterations {
            let probe_index = self.probe_index(value, i);
            let bucket = self.get_bucket(probe_index).unwrap();

            match bucket {
                Some(bucket) => {
                    let is_valid = bucket.is_valid(current_sequence_number);
                    if first_free_element.is_none() && !is_valid {
                        first_free_element = Some((probe_index, false));
                    }
                    if is_valid && &bucket.value_biguint() == value {
                        return Err(HashSetError::ElementAlreadyExists);
                    } else {
                        continue;
                    }
                }
                None => {
                    // A previous bucket could have been freed already even
                    // though the whole hash set has not been used yet.
                    if first_free_element.is_none() {
                        first_free_element = Some((probe_index, true));
                    }
                    // Since we encountered an empty bucket we know for sure
                    // that the element is not in a bucket with higher probe
                    // index.
                    break;
                }
            }
        }
        Ok(first_free_element)
    }

    /// Returns a first available element.
    pub fn first(
        &self,
        current_sequence_number: usize,
    ) -> Result<Option<&HashSetCell>, HashSetError> {
        for i in 0..self.capacity {
            // PANICS: The loop ensures the bounds.
            let bucket = self.get_bucket(i).unwrap();
            if let Some(bucket) = bucket {
                if bucket.is_valid(current_sequence_number) {
                    return Ok(Some(bucket));
                }
            }
        }

        Ok(None)
    }

    /// Returns a first available element that does not have a sequence number.
    pub fn first_no_seq(&self) -> Result<Option<(HashSetCell, u16)>, HashSetError> {
        for i in 0..self.capacity {
            // PANICS: The loop ensures the bounds.
            let bucket = self.get_bucket(i).unwrap();

            if let Some(bucket) = bucket {
                if bucket.sequence_number.is_none() {
                    return Ok(Some((*bucket, i as u16)));
                }
            }
        }

        Ok(None)
    }

    /// Checks if the hash set contains a value.
    pub fn contains(
        &self,
        value: &BigUint,
        sequence_number: Option<usize>,
    ) -> Result<bool, HashSetError> {
        let element = self.find_element(value, sequence_number)?;
        Ok(element.is_some())
    }

    /// Marks the given element with a given sequence number.
    pub fn mark_with_sequence_number(
        &mut self,
        index: usize,
        sequence_number: usize,
    ) -> Result<(), HashSetError> {
        let sequence_threshold = self.sequence_threshold;
        let element = self
            .get_bucket_mut(index)
            .ok_or(HashSetError::ElementDoesNotExist)?;

        match element {
            Some(element) => {
                element.sequence_number = Some(sequence_number + sequence_threshold);
                Ok(())
            }
            None => Err(HashSetError::ElementDoesNotExist),
        }
    }

    /// Returns an iterator over elements.
    pub fn iter(&self) -> HashSetIterator<'_> {
        HashSetIterator {
            hash_set: self,
            current: 0,
        }
    }
}

impl Drop for HashSet {
    fn drop(&mut self) {
        // SAFETY: As long as `next_value_index`, `capacity_indices` and
        // `capacity_values` are correct, this deallocaion is safe.
        unsafe {
            let layout = Layout::array::<Option<HashSetCell>>(self.capacity).unwrap();
            alloc::dealloc(self.buckets.as_ptr() as *mut u8, layout);
        }
    }
}

impl PartialEq for HashSet {
    fn eq(&self, other: &Self) -> bool {
        self.capacity.eq(&other.capacity)
            && self.sequence_threshold.eq(&other.sequence_threshold)
            && self.iter().eq(other.iter())
    }
}

pub struct HashSetIterator<'a> {
    hash_set: &'a HashSet,
    current: usize,
}

impl<'a> Iterator for HashSetIterator<'a> {
    type Item = (usize, &'a HashSetCell);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.hash_set.get_capacity() {
            let element_index = self.current;
            self.current += 1;

            if let Some(Some(cur_element)) = self.hash_set.get_bucket(element_index) {
                return Some((element_index, cur_element));
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::zero_copy::HashSetZeroCopy;

    #[test]
    fn test_is_valid() {
        let mut rng = thread_rng();

        let cell = HashSetCell {
            value: [0u8; 32],
            sequence_number: None,
        };
        // It should be always valid, no matter the sequence number.
        assert!(cell.is_valid(0));
        for _ in 0..100 {
            let seq: usize = rng.gen();
            assert!(cell.is_valid(seq));
        }

        let cell = HashSetCell {
            value: [0u8; 32],
            sequence_number: Some(2400),
        };
        // Sequence numbers up to 2400 should succeed.
        for i in 0..2400 {
            assert!(cell.is_valid(i));
        }
        for i in 2400..10000 {
            assert!(!cell.is_valid(i));
        }
    }

    /// Manual test cases. A simple check whether basic properties of the hash
    /// set work.
    #[test]
    fn test_hash_set_manual() {
        let mut hs = HashSet::new(256, 4).unwrap();

        // Insert an element and immediately mark it with a sequence number.
        // An equivalent to a single insertion in Light Protocol
        let element_1_1 = 1.to_biguint().unwrap();
        let index_1_1 = hs.insert(&element_1_1, 0).unwrap();
        hs.mark_with_sequence_number(index_1_1, 1).unwrap();

        // Check if element exists in the set.
        assert!(hs.contains(&element_1_1, Some(1)).unwrap());
        // Try inserting the same element, even though we didn't reach the
        // threshold.
        assert!(matches!(
            hs.insert(&element_1_1, 1),
            Err(HashSetError::ElementAlreadyExists)
        ));

        // Insert multiple elements and mark them with one sequence number.
        // An equivalent to a batched insertion in Light Protocol.

        let element_2_3 = 3.to_biguint().unwrap();
        let element_2_6 = 6.to_biguint().unwrap();
        let element_2_8 = 8.to_biguint().unwrap();
        let element_2_9 = 9.to_biguint().unwrap();
        let index_2_3 = hs.insert(&element_2_3, 1).unwrap();
        let index_2_6 = hs.insert(&element_2_6, 1).unwrap();
        let index_2_8 = hs.insert(&element_2_8, 1).unwrap();
        let index_2_9 = hs.insert(&element_2_9, 1).unwrap();
        assert!(hs.contains(&element_2_3, Some(2)).unwrap());
        assert!(hs.contains(&element_2_6, Some(2)).unwrap());
        assert!(hs.contains(&element_2_8, Some(2)).unwrap());
        assert!(hs.contains(&element_2_9, Some(2)).unwrap());
        hs.mark_with_sequence_number(index_2_3, 2).unwrap();
        hs.mark_with_sequence_number(index_2_6, 2).unwrap();
        hs.mark_with_sequence_number(index_2_8, 2).unwrap();
        hs.mark_with_sequence_number(index_2_9, 2).unwrap();
        assert!(matches!(
            hs.insert(&element_2_3, 2),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_6, 2),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_8, 2),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_9, 2),
            Err(HashSetError::ElementAlreadyExists)
        ));

        let element_3_11 = 11.to_biguint().unwrap();
        let element_3_13 = 13.to_biguint().unwrap();
        let element_3_21 = 21.to_biguint().unwrap();
        let element_3_29 = 29.to_biguint().unwrap();
        let index_3_11 = hs.insert(&element_3_11, 2).unwrap();
        let index_3_13 = hs.insert(&element_3_13, 2).unwrap();
        let index_3_21 = hs.insert(&element_3_21, 2).unwrap();
        let index_3_29 = hs.insert(&element_3_29, 2).unwrap();
        assert!(hs.contains(&element_3_11, Some(3)).unwrap());
        assert!(hs.contains(&element_3_13, Some(3)).unwrap());
        assert!(hs.contains(&element_3_21, Some(3)).unwrap());
        assert!(hs.contains(&element_3_29, Some(3)).unwrap());
        hs.mark_with_sequence_number(index_3_11, 3).unwrap();
        hs.mark_with_sequence_number(index_3_13, 3).unwrap();
        hs.mark_with_sequence_number(index_3_21, 3).unwrap();
        hs.mark_with_sequence_number(index_3_29, 3).unwrap();
        assert!(matches!(
            hs.insert(&element_3_11, 3),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_3_13, 3),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_3_21, 3),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_3_29, 3),
            Err(HashSetError::ElementAlreadyExists)
        ));

        let element_4_93 = 93.to_biguint().unwrap();
        let element_4_65 = 64.to_biguint().unwrap();
        let element_4_72 = 72.to_biguint().unwrap();
        let element_4_15 = 15.to_biguint().unwrap();
        let index_4_93 = hs.insert(&element_4_93, 3).unwrap();
        let index_4_65 = hs.insert(&element_4_65, 3).unwrap();
        let index_4_72 = hs.insert(&element_4_72, 3).unwrap();
        let index_4_15 = hs.insert(&element_4_15, 3).unwrap();
        assert!(hs.contains(&element_4_93, Some(4)).unwrap());
        assert!(hs.contains(&element_4_65, Some(4)).unwrap());
        assert!(hs.contains(&element_4_72, Some(4)).unwrap());
        assert!(hs.contains(&element_4_15, Some(4)).unwrap());
        hs.mark_with_sequence_number(index_4_93, 4).unwrap();
        hs.mark_with_sequence_number(index_4_65, 4).unwrap();
        hs.mark_with_sequence_number(index_4_72, 4).unwrap();
        hs.mark_with_sequence_number(index_4_15, 4).unwrap();

        // Try inserting the same elements we inserted before.
        //
        // Ones with the sequence number difference lower or equal to the
        // sequence threshold (4) will fail.
        //
        // Ones with the higher dif will succeed.
        assert!(matches!(
            hs.insert(&element_1_1, 4),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_3, 5),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_6, 5),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_8, 5),
            Err(HashSetError::ElementAlreadyExists)
        ));
        assert!(matches!(
            hs.insert(&element_2_9, 5),
            Err(HashSetError::ElementAlreadyExists)
        ));
        hs.insert(&element_1_1, 5).unwrap();
        hs.insert(&element_2_3, 6).unwrap();
        hs.insert(&element_2_6, 6).unwrap();
        hs.insert(&element_2_8, 6).unwrap();
        hs.insert(&element_2_9, 6).unwrap();
    }

    /// Test cases with random prime field elements.
    #[test]
    fn test_hash_set_random() {
        let mut hs = HashSet::new(6857, 2400).unwrap();

        // The hash set should be empty.
        assert_eq!(hs.first(0).unwrap(), None);
        let mut rng = thread_rng();
        let mut seq = 0;
        let nullifiers: [BigUint; 10000] =
            std::array::from_fn(|_| BigUint::from(Fr::rand(&mut rng)));
        for nf_chunk in nullifiers.chunks(2400) {
            for nullifier in nf_chunk.iter() {
                assert!(!hs.contains(nullifier, Some(seq)).unwrap());
                let index = hs.insert(nullifier, seq).unwrap();
                assert!(hs.contains(nullifier, Some(seq)).unwrap());

                let nullifier_bytes = bigint_to_be_bytes_array(nullifier).unwrap();

                let element = *hs.find_element(nullifier, Some(seq)).unwrap().unwrap().0;
                assert_eq!(
                    element,
                    HashSetCell {
                        value: bigint_to_be_bytes_array(nullifier).unwrap(),
                        sequence_number: None,
                    }
                );
                assert_eq!(element.value_bytes(), nullifier_bytes);
                assert_eq!(&element.value_biguint(), nullifier);
                assert_eq!(element.sequence_number(), None);
                assert!(!element.is_marked());
                assert!(element.is_valid(seq));

                hs.mark_with_sequence_number(index, seq).unwrap();
                let element = *hs.find_element(nullifier, Some(seq)).unwrap().unwrap().0;

                assert_eq!(
                    element,
                    HashSetCell {
                        value: nullifier_bytes,
                        sequence_number: Some(2400 + seq)
                    }
                );
                assert_eq!(element.value_bytes(), nullifier_bytes);
                assert_eq!(&element.value_biguint(), nullifier);
                assert_eq!(element.sequence_number(), Some(2400 + seq));
                assert!(element.is_marked());
                assert!(element.is_valid(seq));

                // Trying to insert the same nullifier, before reaching the
                // sequence threshold, should fail.
                assert!(matches!(
                    hs.insert(nullifier, seq + 2399),
                    Err(HashSetError::ElementAlreadyExists),
                ));
                seq += 1;
            }
            seq += 2400;
        }
    }

    fn hash_set_from_bytes_copy<
        const CAPACITY: usize,
        const SEQUENCE_THRESHOLD: usize,
        const OPERATIONS: usize,
    >() {
        let mut hs_1 = HashSet::new(CAPACITY, SEQUENCE_THRESHOLD).unwrap();

        let mut rng = thread_rng();

        // Create a buffer with random bytes.
        let mut bytes = vec![0u8; HashSet::size_in_account(CAPACITY)];
        rng.fill(bytes.as_mut_slice());

        // Initialize a hash set on top of a byte slice.
        {
            let mut hs_2 = unsafe {
                HashSetZeroCopy::from_bytes_zero_copy_init(&mut bytes, CAPACITY, SEQUENCE_THRESHOLD)
                    .unwrap()
            };

            for seq in 0..OPERATIONS {
                let value = BigUint::from(Fr::rand(&mut rng));
                hs_1.insert(&value, seq).unwrap();
                hs_2.insert(&value, seq).unwrap();
            }

            assert_eq!(hs_1, *hs_2);
        }

        // Create a copy on top of a byte slice.
        {
            let hs_2 = unsafe { HashSet::from_bytes_copy(&mut bytes).unwrap() };
            assert_eq!(hs_1, hs_2);
        }
    }

    #[test]
    fn test_hash_set_from_bytes_copy_6857_2400_3600() {
        hash_set_from_bytes_copy::<6857, 2400, 3600>()
    }

    #[test]
    fn test_hash_set_from_bytes_copy_9601_2400_5000() {
        hash_set_from_bytes_copy::<9601, 2400, 5000>()
    }

    fn hash_set_full<const CAPACITY: usize, const SEQUENCE_THRESHOLD: usize>() {
        for _ in 0..100 {
            let mut hs = HashSet::new(CAPACITY, SEQUENCE_THRESHOLD).unwrap();

            let mut rng = rand::thread_rng();

            // Insert as many values as possible. The important point is to
            // encounter the `HashSetError::Full` at some point
            for i in 0..CAPACITY {
                let value = BigUint::from(Fr::rand(&mut rng));
                match hs.insert(&value, 0) {
                    Ok(index) => hs.mark_with_sequence_number(index, 0).unwrap(),
                    Err(e) => {
                        assert!(matches!(e, HashSetError::Full));
                        println!("initial insertions: {i}: failed, stopping");
                        break;
                    }
                }
            }

            // Keep inserting. It should mostly fail, although there might be
            // also some successful insertions - there might be values which
            // will end up in unused buckets.
            for i in 0..1000 {
                let value = BigUint::from(Fr::rand(&mut rng));
                let res = hs.insert(&value, 0);
                if res.is_err() {
                    assert!(matches!(res, Err(HashSetError::Full)));
                } else {
                    println!("secondary insertions: {i}: apparent success with value: {value:?}");
                }
            }

            // Try again with defined sequence numbers, but still too small to
            // vacate any cell.
            for i in 0..1000 {
                let value = BigUint::from(Fr::rand(&mut rng));
                // Sequence numbers lower than the threshold should not vacate
                // any cell.
                let sequence_number = rng.gen_range(0..hs.sequence_threshold);
                let res = hs.insert(&value, sequence_number);
                if res.is_err() {
                    assert!(matches!(res, Err(HashSetError::Full)));
                } else {
                    println!("tertiary insertions: {i}: surprising success with value: {value:?}");
                }
            }

            // Use sequence numbers which are going to vacate cells. All
            // insertions should be successful now.
            for i in 0..CAPACITY {
                let value = BigUint::from(Fr::rand(&mut rng));
                if let Err(e) = hs.insert(&value, SEQUENCE_THRESHOLD + i) {
                    assert!(matches!(e, HashSetError::Full));
                    println!("insertions after fillup: {i}: failed, stopping");
                    break;
                }
            }
        }
    }

    #[test]
    fn test_hash_set_full_6857_2400() {
        hash_set_full::<6857, 2400>()
    }

    #[test]
    fn test_hash_set_full_9601_2400() {
        hash_set_full::<9601, 2400>()
    }

    #[test]
    fn test_hash_set_element_does_not_exist() {
        let mut hs = HashSet::new(4800, 2400).unwrap();

        let mut rng = thread_rng();

        for _ in 0..1000 {
            let index = rng.gen_range(0..4800);

            // Assert `ElementDoesNotExist` error.
            let res = hs.mark_with_sequence_number(index, 0);
            assert!(matches!(res, Err(HashSetError::ElementDoesNotExist)));
        }

        for _ in 0..1000 {
            // After actually appending the value, the same operation should be
            // possible
            let value = BigUint::from(Fr::rand(&mut rng));
            let index = hs.insert(&value, 0).unwrap();
            hs.mark_with_sequence_number(index, 1).unwrap();
        }
    }

    #[test]
    fn test_hash_set_iter_manual() {
        let mut hs = HashSet::new(6857, 2400).unwrap();

        let nullifier_1 = 945635_u32.to_biguint().unwrap();
        let nullifier_2 = 3546656654734254353455_u128.to_biguint().unwrap();
        let nullifier_3 = 543543656564_u64.to_biguint().unwrap();
        let nullifier_4 = 43_u8.to_biguint().unwrap();
        let nullifier_5 = 0_u8.to_biguint().unwrap();
        let nullifier_6 = 65423_u32.to_biguint().unwrap();
        let nullifier_7 = 745654665_u32.to_biguint().unwrap();
        let nullifier_8 = 97664353453465354645645465_u128.to_biguint().unwrap();
        let nullifier_9 = 453565465464565635475_u128.to_biguint().unwrap();
        let nullifier_10 = 543645654645_u64.to_biguint().unwrap();

        hs.insert(&nullifier_1, 0).unwrap();
        hs.insert(&nullifier_2, 0).unwrap();
        hs.insert(&nullifier_3, 0).unwrap();
        hs.insert(&nullifier_4, 0).unwrap();
        hs.insert(&nullifier_5, 0).unwrap();
        hs.insert(&nullifier_6, 0).unwrap();
        hs.insert(&nullifier_7, 0).unwrap();
        hs.insert(&nullifier_8, 0).unwrap();
        hs.insert(&nullifier_9, 0).unwrap();
        hs.insert(&nullifier_10, 0).unwrap();

        let inserted_nullifiers = hs
            .iter()
            .map(|(_, nullifier)| nullifier.value_biguint())
            .collect::<Vec<_>>();
        assert_eq!(inserted_nullifiers.len(), 10);
        assert_eq!(inserted_nullifiers[0], nullifier_7);
        assert_eq!(inserted_nullifiers[1], nullifier_3);
        assert_eq!(inserted_nullifiers[2], nullifier_10);
        assert_eq!(inserted_nullifiers[3], nullifier_1);
        assert_eq!(inserted_nullifiers[4], nullifier_8);
        assert_eq!(inserted_nullifiers[5], nullifier_5);
        assert_eq!(inserted_nullifiers[6], nullifier_4);
        assert_eq!(inserted_nullifiers[7], nullifier_2);
        assert_eq!(inserted_nullifiers[8], nullifier_9);
        assert_eq!(inserted_nullifiers[9], nullifier_6);
    }

    fn hash_set_iter_random<
        const INSERTIONS: usize,
        const CAPACITY: usize,
        const SEQUENCE_THRESHOLD: usize,
    >() {
        let mut hs = HashSet::new(CAPACITY, SEQUENCE_THRESHOLD).unwrap();
        let mut rng = thread_rng();

        let nullifiers: [BigUint; INSERTIONS] =
            std::array::from_fn(|_| BigUint::from(Fr::rand(&mut rng)));

        for nullifier in nullifiers.iter() {
            hs.insert(nullifier, 0).unwrap();
        }

        let mut sorted_nullifiers = nullifiers.iter().collect::<Vec<_>>();
        let mut inserted_nullifiers = hs
            .iter()
            .map(|(_, nullifier)| nullifier.value_biguint())
            .collect::<Vec<_>>();
        sorted_nullifiers.sort();
        inserted_nullifiers.sort();

        let inserted_nullifiers = inserted_nullifiers.iter().collect::<Vec<&BigUint>>();
        assert_eq!(inserted_nullifiers.len(), INSERTIONS);
        assert_eq!(sorted_nullifiers.as_slice(), inserted_nullifiers.as_slice());
    }

    #[test]
    fn test_hash_set_iter_random_6857_2400() {
        hash_set_iter_random::<3500, 6857, 2400>()
    }

    #[test]
    fn test_hash_set_iter_random_9601_2400() {
        hash_set_iter_random::<5000, 9601, 2400>()
    }

    #[test]
    fn test_hash_set_get_bucket() {
        let mut hs = HashSet::new(6857, 2400).unwrap();

        for i in 0..3600 {
            let bn_i = i.to_biguint().unwrap();
            hs.insert(&bn_i, i).unwrap();
        }
        let mut unused_indices = vec![true; 6857];
        for i in 0..3600 {
            let bn_i = i.to_biguint().unwrap();
            let i = hs.find_element_index(&bn_i, None).unwrap().unwrap();
            let element = hs.get_bucket(i).unwrap().unwrap();
            assert_eq!(element.value_biguint(), bn_i);
            unused_indices[i] = false;
        }
        // Unused cells within the capacity should be `Some(None)`.
        for i in unused_indices.iter().enumerate() {
            if *i.1 {
                assert!(hs.get_bucket(i.0).unwrap().is_none());
            }
        }
        // Cells over the capacity should be `None`.
        for i in 6857..10_000 {
            assert!(hs.get_bucket(i).is_none());
        }
    }

    #[test]
    fn test_hash_set_get_bucket_mut() {
        let mut hs = HashSet::new(6857, 2400).unwrap();

        for i in 0..3600 {
            let bn_i = i.to_biguint().unwrap();
            hs.insert(&bn_i, i).unwrap();
        }
        let mut unused_indices = vec![false; 6857];

        for i in 0..3600 {
            let bn_i = i.to_biguint().unwrap();
            let i = hs.find_element_index(&bn_i, None).unwrap().unwrap();

            let element = hs.get_bucket_mut(i).unwrap();
            assert_eq!(element.unwrap().value_biguint(), bn_i);
            unused_indices[i] = true;

            // "Nullify" the element.
            *element = Some(HashSetCell {
                value: [0_u8; 32],
                sequence_number: None,
            });
        }

        for (i, is_used) in unused_indices.iter().enumerate() {
            if *is_used {
                let element = hs.get_bucket_mut(i).unwrap().unwrap();
                assert_eq!(element.value_bytes(), [0_u8; 32]);
            }
        }
        // Unused cells within the capacity should be `Some(None)`.
        for (i, is_used) in unused_indices.iter().enumerate() {
            if !*is_used {
                assert!(hs.get_bucket_mut(i).unwrap().is_none());
            }
        }
        // Cells over the capacity should be `None`.
        for i in 6857..10_000 {
            assert!(hs.get_bucket_mut(i).is_none());
        }
    }

    #[test]
    fn test_hash_set_get_unmarked_bucket() {
        let mut hs = HashSet::new(6857, 2400).unwrap();

        // Insert incremental elements, so they end up being in the same
        // sequence in the hash set.
        (0..3600).for_each(|i| {
            let bn_i = i.to_biguint().unwrap();
            hs.insert(&bn_i, i).unwrap();
        });

        for i in 0..3600 {
            let i = hs
                .find_element_index(&i.to_biguint().unwrap(), None)
                .unwrap()
                .unwrap();
            let element = hs.get_unmarked_bucket(i);
            assert!(element.is_some());
        }

        // Mark the elements.
        for i in 0..3600 {
            let index = hs
                .find_element_index(&i.to_biguint().unwrap(), None)
                .unwrap()
                .unwrap();
            hs.mark_with_sequence_number(index, i).unwrap();
        }

        for i in 0..3600 {
            let i = hs
                .find_element_index(&i.to_biguint().unwrap(), None)
                .unwrap()
                .unwrap();
            let element = hs.get_unmarked_bucket(i);
            assert!(element.is_none());
        }
    }

    #[test]
    fn test_hash_set_first_no_seq() {
        let mut hs = HashSet::new(6857, 2400).unwrap();

        // Insert incremental elements, so they end up being in the same
        // sequence in the hash set.
        for i in 0..3600 {
            let bn_i = i.to_biguint().unwrap();
            hs.insert(&bn_i, i).unwrap();

            let element = hs.first_no_seq().unwrap().unwrap();
            assert_eq!(element.0.value_biguint(), 0.to_biguint().unwrap());
        }
    }
}
