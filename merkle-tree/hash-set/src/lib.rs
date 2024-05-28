use std::{
    alloc::{self, handle_alloc_error, Layout},
    fmt, mem,
    ptr::NonNull,
};

use light_utils::{bigint::bigint_to_be_bytes_array, UtilsError};
use num_bigint::{BigUint, ToBigUint};
use num_traits::{Bounded, CheckedAdd, CheckedSub, FromBytes, ToPrimitive, Unsigned};
use thiserror::Error;

pub mod zero_copy;

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
    #[error("Utils: big integer conversion error")]
    Utils(#[from] UtilsError),
}

#[cfg(feature = "solana")]
impl From<HashSetError> for u32 {
    fn from(e: HashSetError) -> u32 {
        match e {
            HashSetError::Full => 9001,
            HashSetError::ElementAlreadyExists => 9002,
            HashSetError::ElementDoesNotExist => 9003,
            HashSetError::UsizeConv => 9004,
            HashSetError::IntegerOverflow => 9005,
            HashSetError::BufferSize(_, _) => 9006,
            HashSetError::Utils(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<HashSetError> for solana_program::program_error::ProgramError {
    fn from(e: HashSetError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct HashSetCell<I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
{
    value: [u8; 32],
    sequence_number: Option<usize>,
    probing_distance: I,
}

impl<I> HashSetCell<I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
{
    pub fn value_bytes(&self) -> [u8; 32] {
        self.value
    }

    pub fn value_biguint(&self) -> BigUint {
        BigUint::from_bytes_be(self.value.as_slice())
    }

    pub fn sequence_number(&self) -> Option<usize> {
        self.sequence_number
    }
}

#[derive(Debug)]
pub struct HashSet<I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
{
    /// Capacity of `values`, which is equal to the expected number of elements.
    pub capacity_values: usize,
    /// Difference of sequence numbers, after which the given element can be
    /// replaced by an another one (with a sequence number higher than the
    /// threshold).
    pub sequence_threshold: usize,

    /// Index of the next vacant cell in the value array.
    pub next_value_index: *mut usize,

    /// An array of values. It has a size equal to the expected number of
    /// elements.
    values: NonNull<Option<HashSetCell>>,
}

impl<I> HashSet<I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    u64: TryFrom<I>,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
{
    /// Size of the struct **without** dynamically sized fields.
    pub fn non_dyn_fields_size() -> usize {
        // capacity_values
        mem::size_of::<usize>()
        // sequence_threshold
        + mem::size_of::<usize>()
    }

    /// Size which needs to be allocated on Solana account to fit the hash set.
    pub fn size_in_account(capacity_values: usize) -> Result<usize, HashSetError> {
        let dyn_fields_size = Self::non_dyn_fields_size();
        let next_value_index_size = mem::size_of::<usize>();

        let values_size_unaligned = mem::size_of::<Option<HashSetCell>>() * capacity_values;
        // Make sure that alignment of `values` matches the alignment of `usize`.
        let values_size = values_size_unaligned + mem::align_of::<usize>()
            - (values_size_unaligned % mem::align_of::<usize>());

        Ok(dyn_fields_size + next_value_index_size + values_size)
    }

    // Create a new hash set with the given capacity
    pub fn new(capacity_values: usize, sequence_threshold: usize) -> Result<Self, HashSetError> {
        let layout = Layout::new::<usize>();
        // SAFETY: Allocating a primitive type. There is no chance of
        // misalignment.
        let next_value_index = unsafe { alloc::alloc(layout) as *mut usize };
        if next_value_index.is_null() {
            handle_alloc_error(layout);
        }
        unsafe {
            *next_value_index = 0;
        }

        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
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
            next_value_index,
            sequence_threshold,
            capacity_values,
            values,
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

        let capacity_values = usize::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let sequence_threshold = usize::from_ne_bytes(bytes[8..16].try_into().unwrap());

        let expected_size = Self::size_in_account(capacity_values)?;
        if bytes.len() != expected_size {
            return Err(HashSetError::BufferSize(expected_size, bytes.len()));
        }

        let next_value_index_layout = Layout::new::<usize>();
        let next_value_index = unsafe { alloc::alloc(next_value_index_layout) as *mut usize };
        if next_value_index.is_null() {
            handle_alloc_error(next_value_index_layout);
        }
        unsafe {
            *next_value_index = usize::from_ne_bytes(bytes[24..32].try_into().unwrap());
        }

        let values_layout = Layout::array::<Option<HashSetCell<I>>>(capacity_values).unwrap();
        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
        let values_dst_ptr = unsafe { alloc::alloc(values_layout) as *mut Option<HashSetCell<I>> };
        if values_dst_ptr.is_null() {
            handle_alloc_error(values_layout);
        }
        let values = NonNull::new(values_dst_ptr).unwrap();
        for i in 0..capacity_values {
            std::ptr::write(values_dst_ptr.add(i), None);
        }

        let offset = Self::non_dyn_fields_size() + mem::size_of::<usize>();
        let values_src_ptr = bytes.as_ptr().add(offset) as *const Option<HashSetCell<I>>;
        std::ptr::copy(values_src_ptr, values_dst_ptr, capacity_values);

        Ok(Self {
            capacity_values,
            next_value_index,
            sequence_threshold,
            values,
        })
    }

    fn get_bucket(&self, index: usize) -> &Option<HashSetCell<I>> {
        if index >= self.capacity_values {
            panic!("Index out of bounds");
        }
        // SAFETY: We checked the bounds above.
        unsafe { self.values.as_ptr().add(index) }
    }

    fn get_bucket_mut(&self, index: usize) -> &mut Option<HashSetCell<I>> {
        if index >= self.capacity_values {
            panic!("Index out of bounds");
        }
        // SAFETY: We checked the bounds above.
        unsafe { self.values.as_ptr().add(index) }
    }

    fn index_for_value(&self, value: &BigUint) -> usize {
        // PANICS: `capacity_values` always fits in `BigUint`.
        let index = value % self.capacity_values.to_biguint().unwrap();
        // PANICS: The modulus above makes sure that `index` fits in `usize`.
        index.to_usize().unwrap()
    }

    pub fn insert(
        &mut self,
        value: &BigUint,
        mut current_sequence_number: usize,
    ) -> Result<(), HashSetError> {
        let mut index = self.index_for_value(value);
        let mut distance = I::zero();

        let mut value = value.clone();

        let capacity_values =
            I::try_from(self.capacity_values).map_err(|_| HashSetError::IntegerOverflow)?;
        while distance < capacity_values {
            let cell = self.get_bucket_mut(index);

            match cell {
                Some(cell) if cell.probing_distance < distance => {
                    // Swap logic for Robin Hood: displace the resident if it has a smaller probing distance
                    let temp = *cell;
                    *cell = HashSetCell {
                        value: bigint_to_be_bytes_array(&value)?,
                        sequence_number: Some(current_sequence_number),
                        probing_distance: distance,
                    };
                    value = temp.value_biguint();
                    current_sequence_number = temp.sequence_number.unwrap_or_default();
                    distance = temp.probing_distance;
                }
                None => {
                    // Insert new value
                    *cell = Some(HashSetCell {
                        value: bigint_to_be_bytes_array(&value)?,
                        sequence_number: Some(current_sequence_number),
                        probing_distance: distance,
                    });
                    return Ok(());
                }
                _ => {}
            }
            index = (index + 1) % self.capacity_values;
            distance = distance
                .checked_add(&I::one())
                .ok_or(HashSetError::IntegerOverflow)?;
        }

        Err(HashSetError::Full)
    }

    // pub fn find_element(
    //     &self,
    //     value: &BigUint,
    //     current_sequence_number: Option<usize>,
    // ) -> Result<Option<(&mut HashSetCell, I)>, HashSetError> {
    //     for i in 0..self.capacity_values {
    //         let probe_index = (value.clone() + i.to_biguint().unwrap() * i.to_biguint().unwrap())
    //             % self.capacity_values.to_biguint().unwrap();
    //         let probe_index = probe_index.to_usize().unwrap();
    //         let index_bucket = unsafe { &*self.indices.as_ptr().add(probe_index) };

    //         match index_bucket {
    //             Some(value_index) => {
    //                 let value_bucket = self.by_value_index(
    //                     usize::try_from(*value_index).map_err(|_| HashSetError::UsizeConv)?,
    //                     current_sequence_number,
    //                 );

    //                 if let Some(value_bucket) = value_bucket {
    //                     if &value_bucket.value_biguint() == value {
    //                         return Ok(Some((value_bucket, *value_index)));
    //                     }
    //                 }
    //             }
    //             None => {
    //                 return Ok(None);
    //             }
    //         }
    //     }

    //     Ok(None)
    // }

    pub fn find_element(
        &self,
        value: &BigUint,
        current_sequence_number: Option<usize>,
    ) -> Result<Option<&mut HashSetCell<I>>, HashSetError> {
        let index = self.index_for_value(value);
        let mut probe_index = index;

        for distance in 0..self.capacity_values {
            let cell = self.get_bucket_mut(probe_index);

            match cell {
                Some(cell) if &cell.value_biguint() == value => {
                    // If a sequence number is given and it matches the conditions, return the cell
                    if let Some(seq_num) = current_sequence_number {
                        if let Some(cell_seq_num) = cell.sequence_number {
                            if cell_seq_num <= seq_num + self.sequence_threshold {
                                return Ok(Some(cell));
                            }
                        }
                    } else {
                        // If no specific sequence number condition must be met
                        return Ok(Some(cell));
                    }
                }
                None => {
                    // If we find an empty slot, stop the search as no further items will be found
                    return Ok(None);
                }
                _ => {}
            }

            // Update the probe index using quadratic probing formula or another probing sequence
            probe_index = (hash_index + distance * distance) % self.capacity_values;
        }

        Ok(None) // If no element is found after full probing
    }

    /// find_element_iter iterates over a fixed range of elements
    /// in the hash set.
    /// We always have to iterate over the whole range
    /// to make sure that the value is not in the hash-set.
    /// Returns the position of the first free value.
    pub fn find_element_iter(
        &self,
        value: &BigUint,
        current_sequence_number: usize,
        start_iter: usize,
        num_iterations: usize,
    ) -> Result<Option<(usize, bool)>, HashSetError> {
        let mut first_free_element: Option<(usize, bool)> = None;
        for i in start_iter..num_iterations {
            let probe_index = (value.clone() + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % self.capacity_values.to_biguint().unwrap();
            let probe_index = probe_index.to_usize().unwrap();
            let index_bucket = unsafe { &*self.indices.as_ptr().add(probe_index) };

            match index_bucket {
                Some(value_index) => {
                    let value_bucket = self.by_value_index(
                        usize::try_from(*value_index).map_err(|_| HashSetError::UsizeConv)?,
                        Some(current_sequence_number),
                    );

                    if let Some(value_bucket) = value_bucket {
                        if first_free_element.is_none()
                            && value_bucket.sequence_number.is_some()
                            && current_sequence_number >= value_bucket.sequence_number.unwrap()
                        {
                            first_free_element = Some((probe_index, false));
                        }
                        if &value_bucket.value_biguint() == value {
                            return Err(HashSetError::ElementAlreadyExists);
                        }
                    }
                }
                None => {
                    if first_free_element.is_none() {
                        first_free_element = Some((probe_index, true));
                        // Since we encountered an empty element we know for sure
                        // that the element is not in the hash set.
                        break;
                    }
                }
            }
        }
        Ok(first_free_element)
    }

    /// Returns a first available element.
    pub fn first(
        &self,
        current_sequence_number: usize,
    ) -> Result<Option<&mut HashSetCell>, HashSetError> {
        for i in 0..self.capacity_values {
            let value_bucket = unsafe { &mut *self.values.as_ptr().add(i) };

            if let Some(value_bucket) = value_bucket {
                if let Some(element_sequence_number) = value_bucket.sequence_number {
                    if current_sequence_number < element_sequence_number {
                        return Ok(Some(value_bucket));
                    }
                } else {
                    return Ok(Some(value_bucket));
                }
            }
        }

        Ok(None)
    }

    /// Returns a first available element that does not have a sequence number.
    pub fn first_no_seq(&self) -> Result<Option<(HashSetCell, u16)>, HashSetError> {
        for i in 0..self.capacity_values {
            let value_bucket = unsafe { &mut *self.values.as_ptr().add(i) };

            if let Some(value_bucket) = value_bucket {
                if value_bucket.sequence_number.is_none() {
                    return Ok(Some((*value_bucket, i as u16)));
                }
            }
        }

        Ok(None)
    }

    pub fn by_value_index(
        &self,
        value_index: usize,
        current_sequence_number: Option<usize>,
    ) -> Option<&mut HashSetCell<I>> {
        let value_bucket = unsafe { &mut *self.values.as_ptr().add(value_index) };
        if let Some(value_bucket) = value_bucket {
            match current_sequence_number {
                Some(current_sequence_number) => {
                    // If the `current_sequence_number` was specified,
                    // search for an element with either...
                    match value_bucket.sequence_number {
                        // ...a lower sequence number...
                        Some(element_sequence_number) => {
                            if current_sequence_number > element_sequence_number {
                                return Some(value_bucket);
                            }
                        }
                        // ...or without sequence number.
                        None => return Some(value_bucket),
                    }
                    if let Some(element_sequence_number) = value_bucket.sequence_number {
                        if current_sequence_number < element_sequence_number {
                            return Some(value_bucket);
                        }
                    }
                }
                None => {
                    // If the `current_sequence_number` was not specified,
                    // search for an element without specified sequence number.
                    if value_bucket.sequence_number.is_none() {
                        return Some(value_bucket);
                    }
                }
            }
        }

        None
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
        value: &BigUint,
        sequence_number: usize,
    ) -> Result<(), HashSetError> {
        let element = self.find_element(value, None)?;

        match element {
            Some((element, _)) => {
                element.sequence_number = Some(sequence_number + self.sequence_threshold);
                Ok(())
            }
            None => Err(HashSetError::ElementDoesNotExist),
        }
    }

    pub fn iter(&self) -> HashSetIterator<I> {
        HashSetIterator {
            hash_set: self,
            current: 0,
        }
    }
}

impl<I> Drop for HashSet<I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
{
    fn drop(&mut self) {
        // SAFETY: As long as `next_value_index`, `capacity_indices` and
        // `capacity_values` are correct, this deallocaion is safe.
        unsafe {
            let layout = Layout::new::<usize>();
            alloc::dealloc(self.next_value_index as *mut u8, layout);

            let layout = Layout::array::<Option<I>>(self.capacity_indices).unwrap();
            alloc::dealloc(self.indices.as_ptr() as *mut u8, layout);

            let layout = Layout::array::<Option<HashSetCell>>(self.capacity_values).unwrap();
            alloc::dealloc(self.values.as_ptr() as *mut u8, layout);
        }
    }
}

pub struct HashSetIterator<'a, I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
{
    hash_set: &'a HashSet<I>,
    current: usize,
}

impl<'a, I> Iterator for HashSetIterator<'a, I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
{
    type Item = (usize, &'a HashSetCell);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.hash_set.capacity_values {
            let element_index = self.current;
            let element = unsafe { &*self.hash_set.values.as_ptr().add(element_index) };

            self.current += 1;
            element.as_ref().map(|element| (element_index, element))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use rand::{rngs::StdRng, thread_rng, SeedableRng};

    #[test]
    fn test_find_next_prime() {
        assert_eq!(find_next_prime(0.0), 2.0);
        assert_eq!(find_next_prime(2.0), 2.0);
        assert_eq!(find_next_prime(3.0), 3.0);
        assert_eq!(find_next_prime(4.0), 5.0);

        assert_eq!(find_next_prime(10.0), 11.0);
        assert_eq!(find_next_prime(28.0), 29.0);

        assert_eq!(find_next_prime(100.0), 101.0);
        assert_eq!(find_next_prime(1000.0), 1009.0);

        assert_eq!(find_next_prime(102.0), 103.0);
        assert_eq!(find_next_prime(105.0), 107.0);

        assert_eq!(find_next_prime(7900.0), 7901.0);
        assert_eq!(find_next_prime(7907.0), 7907.0);
    }

    #[test]
    fn test_capacity_cells() {
        assert_eq!(HashSet::<u16>::capacity_indices(256, 0.5).unwrap(), 521.0);
        assert_eq!(HashSet::<u16>::capacity_indices(4800, 0.7).unwrap(), 6857.0);
    }

    /// Manual test cases. A simple check whether basic properties of the hash
    /// set work.
    #[test]
    fn test_hash_set_manual() {
        let mut hs = HashSet::<u16>::new(521, 256, 4).unwrap();

        // Insert an element and immediately mark it with a sequence number.
        // An equivalent to a single insertion in Light Protocol
        let element_1_1 = 1.to_biguint().unwrap();
        hs.insert(&element_1_1, 0).unwrap();
        hs.mark_with_sequence_number(&element_1_1, 1).unwrap();

        // Check if element exists in the set.
        assert_eq!(hs.contains(&element_1_1, Some(1)).unwrap(), true);
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
        hs.insert(&element_2_3, 1).unwrap();
        hs.insert(&element_2_6, 1).unwrap();
        hs.insert(&element_2_8, 1).unwrap();
        hs.insert(&element_2_9, 1).unwrap();
        assert_eq!(hs.contains(&element_2_3, Some(2)).unwrap(), true);
        assert_eq!(hs.contains(&element_2_6, Some(2)).unwrap(), true);
        assert_eq!(hs.contains(&element_2_8, Some(2)).unwrap(), true);
        assert_eq!(hs.contains(&element_2_9, Some(2)).unwrap(), true);
        hs.mark_with_sequence_number(&element_2_3, 2).unwrap();
        hs.mark_with_sequence_number(&element_2_6, 2).unwrap();
        hs.mark_with_sequence_number(&element_2_8, 2).unwrap();
        hs.mark_with_sequence_number(&element_2_9, 2).unwrap();
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
        hs.insert(&element_3_11, 2).unwrap();
        hs.insert(&element_3_13, 2).unwrap();
        hs.insert(&element_3_21, 2).unwrap();
        hs.insert(&element_3_29, 2).unwrap();
        assert_eq!(hs.contains(&element_3_11, Some(3)).unwrap(), true);
        assert_eq!(hs.contains(&element_3_13, Some(3)).unwrap(), true);
        assert_eq!(hs.contains(&element_3_21, Some(3)).unwrap(), true);
        assert_eq!(hs.contains(&element_3_29, Some(3)).unwrap(), true);
        hs.mark_with_sequence_number(&element_3_11, 3).unwrap();
        hs.mark_with_sequence_number(&element_3_13, 3).unwrap();
        hs.mark_with_sequence_number(&element_3_21, 3).unwrap();
        hs.mark_with_sequence_number(&element_3_29, 3).unwrap();
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
        hs.insert(&element_4_93, 3).unwrap();
        hs.insert(&element_4_65, 3).unwrap();
        hs.insert(&element_4_72, 3).unwrap();
        hs.insert(&element_4_15, 3).unwrap();
        assert_eq!(hs.contains(&element_4_93, Some(4)).unwrap(), true);
        assert_eq!(hs.contains(&element_4_65, Some(4)).unwrap(), true);
        assert_eq!(hs.contains(&element_4_72, Some(4)).unwrap(), true);
        assert_eq!(hs.contains(&element_4_15, Some(4)).unwrap(), true);
        hs.mark_with_sequence_number(&element_4_93, 4).unwrap();
        hs.mark_with_sequence_number(&element_4_65, 4).unwrap();
        hs.mark_with_sequence_number(&element_4_72, 4).unwrap();
        hs.mark_with_sequence_number(&element_4_15, 4).unwrap();

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
        let mut hs = HashSet::<u16>::new(6857, 4800, 2400).unwrap();

        // The hash set should be empty.
        assert_eq!(hs.first(0).unwrap(), None);

        let mut rng = thread_rng();
        let mut seq = 0;
        let nullifiers: [BigUint; 24000] =
            std::array::from_fn(|_| BigUint::from(Fr::rand(&mut rng)));
        for (j, nf_chunk) in nullifiers.chunks(2400).enumerate() {
            for nullifier in nf_chunk.iter() {
                assert_eq!(hs.contains(&nullifier, Some(seq)).unwrap(), false);
                hs.insert(&nullifier, seq as usize).unwrap();

                assert_eq!(hs.contains(&nullifier, Some(seq)).unwrap(), true);
                assert_eq!(
                    hs.find_element(&nullifier, Some(seq))
                        .unwrap()
                        .unwrap()
                        .0
                        .clone(),
                    HashSetCell {
                        value: bigint_to_be_bytes_array(&nullifier).unwrap(),
                        sequence_number: None,
                    }
                );

                hs.mark_with_sequence_number(&nullifier, seq).unwrap();
                let element = hs
                    .find_element(&nullifier, Some(seq))
                    .unwrap()
                    .unwrap()
                    .0
                    .clone();

                assert_eq!(
                    element,
                    HashSetCell {
                        value: bigint_to_be_bytes_array(&nullifier).unwrap(),
                        sequence_number: Some(2400 + seq)
                    }
                );

                // Trying to insert the same nullifier, before reaching the
                // sequence threshold, should fail.
                assert!(matches!(
                    hs.insert(&nullifier, seq as usize + 2399),
                    Err(HashSetError::ElementAlreadyExists),
                ));
                seq += 1;
            }
            if j == 0 {
                for (i, element) in hs.iter() {
                    assert_eq!(element.value_biguint(), nf_chunk[i]);
                }

                // As long as we request the first element while providing sequence
                // numbers not reaching the threshold (from 0 to 2399)
                for _seq in 0..2399 {
                    assert_eq!(
                        hs.first(_seq).unwrap().unwrap().value_biguint(),
                        nf_chunk[0]
                    );
                }
            }
            seq += 2400;
        }
    }

    #[test]
    fn test_hash_set_full() {
        for _ in 0..100 {
            let mut hs = HashSet::<u16>::new(6857, 4800, 2400).unwrap();

            let mut rng = StdRng::seed_from_u64(1);
            let mut res = Ok(());
            let mut value = BigUint::from(Fr::rand(&mut rng));
            // Since capacity is 4800, it will always fail on insert 4801.
            // It might fail earlier for the hashset is probabilistic.
            for _ in 0..4801 {
                value = BigUint::from(Fr::rand(&mut rng));
                res = hs.insert(&value, 0);
            }
            // The prior assert made the test always succeed
            assert_eq!(res.unwrap_err(), HashSetError::Full);
            assert_eq!(hs.contains(&value, Some(0)).unwrap(), false);
        }
    }

    #[test]
    fn test_hash_set_full_onchain() {
        for _ in 0..1000 {
            let mut hs = HashSet::<u16>::new(6857, 600, 2400).unwrap();

            let mut rng = StdRng::seed_from_u64(1);
            // We only fill each hash set to 80% because with much more we get conflicts.
            // TODO: investigate why even a 0.1 loadfactor does not enable a full value array.
            for i in 0..500 {
                let value = BigUint::from(Fr::rand(&mut rng));
                let res = hs.insert(&value, 0);
                match res {
                    Ok(_) => {
                        assert_eq!(hs.contains(&value, Some(0)).unwrap(), true);
                    }
                    Err(HashSetError::Full) => {
                        assert_eq!(hs.contains(&value, Some(0)).unwrap(), false);
                        panic!("unexpected error {}", i);
                    }
                    _ => {
                        panic!("unexpected error");
                    }
                }
            }
        }
    }

    #[test]
    fn test_hash_set_element_does_not_exist() {
        let mut hs = HashSet::<u16>::new(6857, 4800, 2400).unwrap();

        let mut rng = thread_rng();

        for _ in 0..1000 {
            let value = BigUint::from(Fr::rand(&mut rng));

            // Assert `ElementDoesNotExist` error.
            let res = hs.mark_with_sequence_number(&value, 0);
            assert!(matches!(res, Err(HashSetError::ElementDoesNotExist)));

            // After actually appending the value, the same operation should be
            // possible
            hs.insert(&value, 0).unwrap();
            hs.mark_with_sequence_number(&value, 1).unwrap();
        }
    }
}
