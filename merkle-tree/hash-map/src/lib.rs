use std::{
    alloc::{self, handle_alloc_error, Layout},
    fmt, mem,
    ptr::NonNull,
};

use light_utils::prime::find_next_prime;
use num_bigint::{BigUint, ToBigUint};
use num_traits::{Bounded, CheckedAdd, CheckedSub, ToPrimitive, Unsigned};
use thiserror::Error;

pub mod zero_copy;

#[derive(Debug, Error)]
pub enum HashMapError {
    #[error("The hash set is full, cannot add any new elements")]
    Full,
    #[error("The provided element is already in the hash set")]
    ElementAlreadyExists,
    #[error("The provided element doesn't exist in the hash set")]
    ElementDoesNotExist,
    #[error("The hash set is empty")]
    Empty,
    #[error("Could not convert the index from/to usize")]
    UsizeConv,
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Invalid buffer size, expected {0}, got {1}")]
    BufferSize(usize, usize),
}

#[cfg(feature = "solana")]
impl From<HashMapError> for u32 {
    fn from(e: HashMapError) -> u32 {
        match e {
            HashMapError::Full => 9001,
            HashMapError::ElementAlreadyExists => 9002,
            HashMapError::ElementDoesNotExist => 9003,
            HashMapError::Empty => 9004,
            HashMapError::UsizeConv => 9005,
            HashMapError::IntegerOverflow => 9006,
            HashMapError::BufferSize(_, _) => 9007,
        }
    }
}

#[cfg(feature = "solana")]
impl From<HashMapError> for solana_program::program_error::ProgramError {
    fn from(e: HashMapError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

pub trait HashMapKey {
    fn hash(&self) -> usize;
}

impl HashMapKey for [u8; 32] {
    fn hash(&self) -> usize {
        // PANICS: `usize::MAX` is always going to fit into `BigUint`.
        let h = BigUint::from_bytes_be(self) % usize::MAX.to_biguint().unwrap();
        // PANICS: By the modulo operation above, we guarantee that the hash
        // always fits into `usize`.
        h.to_usize().unwrap()
    }
}

impl HashMapKey for BigUint {
    fn hash(&self) -> usize {
        // PANICS: `usize::MAX` is always going to fit into `BigUint`.
        let h = self % usize::MAX.to_biguint().unwrap();
        // PANICS: By the modulo operation above, we guarantee that the hash
        // always fits into `usize`.
        h.to_usize().unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct HashMapCell<K, V>
where
    K: HashMapKey,
{
    key: K,
    value: V,
}

#[derive(Debug)]
pub struct HashMap<I, K, V>
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
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
    K: HashMapKey,
{
    /// Capacity of `indices`, which is a prime number larger than the expected
    /// number of elements and an included load factor.
    pub capacity_indices: usize,
    /// Capacity of `values`, which is equal to the expected number of elements.
    pub capacity_values: usize,

    /// Index of the next vacant cell in the value array. If it reaches the
    /// capacity of values, it gets reset to 0 and starts vacating the oldest
    /// value cells.
    pub next_value_index: *mut usize,

    /// An array of indices which maps a hash set key to the index of its
    /// value which is stored in the `values` array. It has a size greater
    /// than the expected number of elements, determined by the load factor.
    indices: NonNull<Option<I>>,
    /// An array of values. It has a size equal to the expected number of
    /// elements.
    values: NonNull<Option<HashMapCell<K, V>>>,
}

impl<I, K, V> HashMap<I, K, V>
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
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    u64: TryFrom<I>,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
    K: HashMapKey + Clone + PartialEq,
    V: Clone + PartialEq,
{
    /// Size of the struct **without** dynamically sized fields.
    pub fn non_dyn_fields_size() -> usize {
        // capacity_indices
        mem::size_of::<usize>()
        // capacity_values
        + mem::size_of::<usize>()
        // sequence_threshold
        + mem::size_of::<usize>()
    }

    /// Size which needs to be allocated on Solana account to fit the hash set.
    pub fn size_in_account(
        capacity_indices: usize,
        capacity_values: usize,
    ) -> Result<usize, HashMapError> {
        let dyn_fields_size = Self::non_dyn_fields_size();
        let next_value_index_size = mem::size_of::<usize>();

        let indices_size_unaligned = mem::size_of::<Option<I>>() * capacity_indices;
        // Make sure that alignment of `indices` matches the alignment of `usize`.
        let indices_size = indices_size_unaligned + mem::align_of::<usize>()
            - (indices_size_unaligned % mem::align_of::<usize>());

        let values_size_unaligned = mem::size_of::<Option<HashMapCell<K, V>>>() * capacity_values;
        // Make sure that alignment of `values` matches the alignment of `usize`.
        let values_size = values_size_unaligned + mem::align_of::<usize>()
            - (values_size_unaligned % mem::align_of::<usize>());

        Ok(dyn_fields_size + next_value_index_size + indices_size + values_size)
    }

    /// Returns the capacity of buckets for the desired `capacity`, while taking
    /// the load factor in account.
    pub fn capacity_indices(
        capacity_elements: usize,
        load_factor: f64,
    ) -> Result<f64, HashMapError> {
        // To treat `capacity_elements` as `f64`, we need to fit it in `u32`.
        // `u64`/`usize` can't be casted directoy to `f64`.
        let capacity_elements =
            u32::try_from(capacity_elements).map_err(|_| HashMapError::IntegerOverflow)?;
        let minimum = f64::from(capacity_elements) / load_factor;
        Ok(find_next_prime(minimum))
    }

    // Create a new hash set with the given capacity
    pub fn new(capacity_indices: usize, capacity_values: usize) -> Result<Self, HashMapError> {
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

        let layout = Layout::array::<Option<I>>(capacity_indices).unwrap();
        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
        let indices_ptr = unsafe { alloc::alloc(layout) as *mut Option<I> };
        if indices_ptr.is_null() {
            handle_alloc_error(layout);
        }
        let indices = NonNull::new(indices_ptr).unwrap();
        for i in 0..capacity_indices {
            unsafe {
                std::ptr::write(indices_ptr.add(i), None);
            }
        }

        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
        let layout = Layout::array::<Option<HashMapCell<K, V>>>(capacity_values).unwrap();
        let values_ptr = unsafe { alloc::alloc(layout) as *mut Option<HashMapCell<K, V>> };
        if values_ptr.is_null() {
            handle_alloc_error(layout);
        }
        let values = NonNull::new(values_ptr).unwrap();
        for i in 0..capacity_values {
            unsafe {
                std::ptr::write(values_ptr.add(i), None);
            }
        }

        Ok(HashMap {
            next_value_index,
            capacity_indices,
            capacity_values,
            indices,
            values,
        })
    }

    fn insert_into_occupied_cell(
        &mut self,
        key: &K,
        value_index: usize,
        value: &V,
    ) -> Result<bool, HashMapError> {
        let value_bucket = unsafe { &mut *self.values.as_ptr().add(value_index) };
        match value_bucket {
            Some(value_bucket) => {
                // We can replace the `value` of the occupied cell only if the `key`
                // matches.
                if &value_bucket.key == key {
                    value_bucket.value = (*value).clone();
                    return Ok(true);
                }
            }
            // PANICS: If there is a hash set cell pointing to a `None` value,
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
    pub fn insert(&mut self, key: &K, value: &V) -> Result<(), HashMapError> {
        for i in 0..self.capacity_values {
            let probe_index = (key.hash() + i * i) % self.capacity_values;
            let index_bucket = unsafe { &mut *self.indices.as_ptr().add(probe_index) };

            match index_bucket {
                // The visited hash set cell points to a value in the array.
                Some(value_index) => {
                    let value_index =
                        usize::try_from(*value_index).map_err(|_| HashMapError::UsizeConv)?;
                    if self.insert_into_occupied_cell(key, value_index, value)? {
                        return Ok(());
                    }
                }
                None => {
                    let value_bucket =
                        unsafe { &mut *self.values.as_ptr().add(*self.next_value_index) };
                    // SAFETY: `next_value_index` is always initialized.
                    *index_bucket = Some(
                        I::try_from(unsafe { *self.next_value_index })
                            .map_err(|_| HashMapError::IntegerOverflow)?,
                    );
                    *value_bucket = Some(HashMapCell {
                        key: (*key).clone(),
                        value: (*value).clone(),
                    });
                    // SAFETY: `next_value_index` is always initialized.
                    unsafe {
                        *self.next_value_index =
                            if *self.next_value_index < self.capacity_values - 1 {
                                *self.next_value_index + 1
                            } else {
                                0
                            };
                    }
                    return Ok(());
                }
            }
        }

        Err(HashMapError::Full)
    }

    pub fn get(&mut self, key: &K) -> Result<Option<&V>, HashMapError> {
        for i in 0..self.capacity_values {
            let probe_index = (key.hash() + i * i) % self.capacity_values;
            let index_bucket = unsafe { &*self.indices.as_ptr().add(probe_index) };

            match index_bucket {
                Some(value_index) => {
                    let value_bucket = self.by_value_index(
                        usize::try_from(*value_index).map_err(|_| HashMapError::UsizeConv)?,
                    );
                    if let Some(value_bucket) = value_bucket {
                        if &value_bucket.key == key {
                            return Ok(Some(&value_bucket.value));
                        }
                    }
                }
                None => {
                    return Ok(None);
                }
            }
        }

        Ok(None)
    }

    pub fn by_value_index(&self, value_index: usize) -> Option<&mut HashMapCell<K, V>> {
        if value_index >= self.capacity_values {
            return None;
        }
        // SAFETY: We ensured the bounds.
        let value_bucket = unsafe { &mut *self.values.as_ptr().add(value_index) };
        value_bucket.as_mut()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_capacity_cells() {
        assert_eq!(
            HashMap::<u16, [u8; 32], u32>::capacity_indices(256, 0.5).unwrap(),
            521.0
        );
        assert_eq!(
            HashMap::<u16, [u8; 32], u32>::capacity_indices(4800, 0.7).unwrap(),
            6857.0
        );
    }

    #[test]
    fn test_hash_map_manual() {
        let mut hm = HashMap::<u16, [u8; 32], u32>::new(521, 256).unwrap();

        assert_eq!(hm.get(&[1u8; 32]).unwrap(), None);

        hm.insert(&[1u8; 32], &1).unwrap();
        assert_eq!(hm.get(&[1u8; 32]).unwrap(), Some(&1));
    }
}
