use std::{fmt, marker::PhantomData, mem, ptr::NonNull};

use num_bigint::{BigUint, ToBigUint};
use num_traits::{Bounded, CheckedAdd, CheckedSub, ToPrimitive, Unsigned};

use crate::{HashMap, HashMapCell, HashMapError, HashMapKey};

/// A `HashSet` wrapper which can be instantiated from Solana account bytes
/// without copying them.
#[derive(Debug)]
pub struct HashMapZeroCopy<'a, I, K, V>
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
    K: HashMapKey,
{
    pub hash_map: mem::ManuallyDrop<HashMap<I, K, V>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, I, K, V> HashMapZeroCopy<'a, I, K, V>
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
    K: HashMapKey + Clone + PartialEq,
    V: Clone + PartialEq,
{
    // TODO(vadorovsky): Add a non-mut method: `from_bytes_zero_copy`.

    /// Casts a byte slice into `HashSet`.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs, where memory
    /// constraints are tight and we want to make sure no data is copied.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the alignment and that the slice
    /// provides actual data of the hash set is the caller's responsibility.
    ///
    /// Calling it in async context (or anyhwere where the underlying data can
    /// be moved in the memory) is certainly going to cause undefined behavior.
    pub unsafe fn from_bytes_zero_copy_mut(bytes: &'a mut [u8]) -> Result<Self, HashMapError> {
        if bytes.len() < HashMap::<I, K, V>::non_dyn_fields_size() {
            return Err(HashMapError::BufferSize(
                HashMap::<I, K, V>::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        let capacity_indices = usize::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let capacity_values = usize::from_ne_bytes(bytes[8..16].try_into().unwrap());

        let next_value_index = bytes.as_mut_ptr().add(16) as *mut usize;

        let offset = HashMap::<I, K, V>::non_dyn_fields_size() + mem::size_of::<usize>();
        let indices_size_unaligned = mem::size_of::<Option<I>>() * capacity_indices;
        // Make sure that alignment of `indices` matches the alignment of `usize`.
        let indices_size = indices_size_unaligned + mem::align_of::<usize>()
            - (indices_size_unaligned % mem::align_of::<usize>());
        let indices = NonNull::new(bytes.as_mut_ptr().add(offset) as *mut Option<I>).unwrap();

        let offset = offset + indices_size;
        let values =
            NonNull::new(bytes.as_mut_ptr().add(offset) as *mut Option<HashMapCell<K, V>>).unwrap();

        Ok(Self {
            hash_map: mem::ManuallyDrop::new(HashMap {
                capacity_indices,
                capacity_values,
                next_value_index,
                indices,
                values,
            }),
            _marker: PhantomData,
        })
    }

    /// Casts a byte slice into `HashSet` and then initializes it.
    ///
    /// * `bytes` is casted into a reference of `HashSet` and used as
    ///   storage for the struct.
    /// * `capacity_indices` indicates the size of the indices table. It should
    ///   already include a desired load factor and be greater than the expected
    ///   number of elements to avoid filling the set too early and avoid
    ///   creating clusters.
    /// * `capacity_values` indicates the size of the values array. It should be
    ///   equal to the number of expected elements, without load factor.
    /// * `sequence_threshold` indicates a difference of sequence numbers which
    ///   make elements of the has set expired. Expiration means that they can
    ///   be replaced during insertion of new elements with sequence numbers
    ///   higher by at least a threshold.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs to initialize
    /// a new account which is supposed to store the hash set.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the alignment and that the slice has
    /// a correct size, which is able to fit the hash set, is the caller's
    /// responsibility.
    ///
    /// Calling it in async context (or anywhere where the underlying data can
    /// be moved in memory) is certainly going to cause undefined behavior.
    pub unsafe fn from_bytes_zero_copy_init(
        bytes: &'a mut [u8],
        capacity_indices: usize,
        capacity_values: usize,
    ) -> Result<Self, HashMapError> {
        if bytes.len() < HashMap::<I, K, V>::non_dyn_fields_size() {
            return Err(HashMapError::BufferSize(
                HashMap::<I, K, V>::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        bytes[0..8].copy_from_slice(&capacity_indices.to_ne_bytes());
        bytes[8..16].copy_from_slice(&capacity_values.to_ne_bytes());
        bytes[16..24].copy_from_slice(&0_usize.to_ne_bytes());

        let hash_map = Self::from_bytes_zero_copy_mut(bytes)?;

        for i in 0..capacity_indices {
            std::ptr::write(hash_map.hash_map.indices.as_ptr().add(i), None);
        }
        for i in 0..capacity_values {
            std::ptr::write(hash_map.hash_map.values.as_ptr().add(i), None);
        }

        Ok(hash_map)
    }

    pub fn insert(&mut self, key: &K, value: &V) -> Result<(), HashMapError> {
        self.hash_map.insert(key, value)
    }

    pub fn get(&mut self, key: &K) -> Result<Option<&V>, HashMapError> {
        self.hash_map.get(key)
    }

    pub fn by_value_index(&self, value_index: usize) -> Option<&mut HashMapCell<K, V>> {
        self.hash_map.by_value_index(value_index)
    }
}
