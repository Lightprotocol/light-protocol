use std::{fmt, marker::PhantomData, mem, ptr::NonNull};

use num_bigint::{BigUint, ToBigUint};
use num_traits::{Bounded, CheckedAdd, CheckedSub, Unsigned};

use crate::{HashSet, HashSetCell, HashSetError, HashSetIterator};

/// A `HashSet` wrapper which can be instantiated from Solana account bytes
/// without copying them.
#[derive(Debug)]
pub struct HashSetZeroCopy<'a, I>
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
    pub hash_set: mem::ManuallyDrop<HashSet<I>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, I> HashSetZeroCopy<'a, I>
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
    pub unsafe fn from_bytes_zero_copy_mut(bytes: &'a mut [u8]) -> Result<Self, HashSetError> {
        if bytes.len() < HashSet::<I>::non_dyn_fields_size() {
            return Err(HashSetError::BufferSize(
                HashSet::<I>::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        let capacity_indices = usize::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let capacity_values = usize::from_ne_bytes(bytes[8..16].try_into().unwrap());
        let sequence_threshold = usize::from_ne_bytes(bytes[16..24].try_into().unwrap());

        let next_value_index = bytes.as_mut_ptr().add(24) as *mut usize;

        let offset = HashSet::non_dyn_fields_size() + mem::size_of::<usize>();
        let indices_size_unaligned = mem::size_of::<Option<I>>() * capacity_indices;
        // Make sure that alignment of `indices` matches the alignment of `usize`.
        let indices_size = indices_size_unaligned + mem::align_of::<usize>()
            - (indices_size_unaligned % mem::align_of::<usize>());
        let indices = NonNull::new(bytes.as_mut_ptr().add(offset) as *mut Option<I>).unwrap();

        let offset = offset + indices_size;
        let values =
            NonNull::new(bytes.as_mut_ptr().add(offset) as *mut Option<HashSetCell>).unwrap();

        Ok(Self {
            hash_set: mem::ManuallyDrop::new(HashSet {
                capacity_indices,
                capacity_values,
                next_value_index,
                sequence_threshold,
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
        sequence_threshold: usize,
    ) -> Result<Self, HashSetError> {
        if bytes.len() < HashSet::<I>::non_dyn_fields_size() {
            return Err(HashSetError::BufferSize(
                HashSet::<I>::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        bytes[0..8].copy_from_slice(&capacity_indices.to_ne_bytes());
        bytes[8..16].copy_from_slice(&capacity_values.to_ne_bytes());
        bytes[16..24].copy_from_slice(&sequence_threshold.to_ne_bytes());
        bytes[24..32].copy_from_slice(&0_usize.to_ne_bytes());

        let hash_set = Self::from_bytes_zero_copy_mut(bytes)?;

        for i in 0..capacity_indices {
            std::ptr::write(hash_set.hash_set.indices.as_ptr().add(i), None);
        }
        for i in 0..capacity_values {
            std::ptr::write(hash_set.hash_set.values.as_ptr().add(i), None);
        }

        Ok(hash_set)
    }

    /// Inserts a value into the hash set.
    pub fn insert(&mut self, value: &BigUint, sequence_number: usize) -> Result<(), HashSetError> {
        self.hash_set.insert(value, sequence_number)
    }

    /// Returns a first available element.
    pub fn first(&self, sequence_number: usize) -> Result<Option<&mut HashSetCell>, HashSetError> {
        self.hash_set.first(sequence_number)
    }

    pub fn by_value_index(
        &self,
        value_index: usize,
        current_sequence_number: Option<usize>,
    ) -> Option<&mut HashSetCell> {
        self.hash_set
            .by_value_index(value_index, current_sequence_number)
    }

    /// Check if the hash set contains a value.
    pub fn contains(&self, value: &BigUint, sequence_number: usize) -> Result<bool, HashSetError> {
        self.hash_set.contains(value, sequence_number)
    }

    /// Marks the given element with a given sequence number.
    pub fn mark_with_sequence_number(
        &mut self,
        value: &BigUint,
        sequence_number: usize,
    ) -> Result<(), HashSetError> {
        self.hash_set
            .mark_with_sequence_number(value, sequence_number)
    }

    pub fn iter(&self) -> HashSetIterator<I> {
        self.hash_set.iter()
    }
}

impl<'a, I> Drop for HashSetZeroCopy<'a, I>
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
        // SAFETY: Don't do anything here! Why?
        //
        // * Primitive fields of `HashSet` implement `Copy`, therefore `drop()`
        //   has no effect on them - Rust drops them when they go out of scope.
        // * Don't drop the dynamic fields (`indices` and `values`). In
        //   `HashSetZeroCopy`, they are backed by buffers provided by the
        //   caller. These buffers are going to be eventually deallocated.
        //   Performing an another `drop()` here would result double `free()`
        //   which would result in aborting the program (either with `SIGABRT`
        //   or `SIGSEGV`).
    }
}

#[cfg(test)]
mod test {
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn test_load_from_bytes() {
        const INDICES: usize = 6857;
        const VALUES: usize = 4800;
        const SEQUENCE_THRESHOLD: usize = 2400;

        // Create a buffer with random bytes.
        let mut bytes = vec![0u8; HashSet::<u16>::size_in_account(INDICES, VALUES).unwrap()];
        thread_rng().fill(bytes.as_mut_slice());

        // Create random nullifiers.
        let mut rng = thread_rng();
        let nullifiers: [BigUint; 2400] =
            std::array::from_fn(|_| BigUint::from(Fr::rand(&mut rng)));

        // Initialize a hash set on top of a byte slice.
        {
            let mut hs = unsafe {
                HashSetZeroCopy::<u16>::from_bytes_zero_copy_init(
                    bytes.as_mut_slice(),
                    INDICES,
                    VALUES,
                    SEQUENCE_THRESHOLD,
                )
                .unwrap()
            };

            // Ensure that the underlying data were properly initialized.
            assert_eq!(hs.hash_set.capacity_indices, INDICES);
            assert_eq!(hs.hash_set.capacity_values, VALUES);
            assert_eq!(hs.hash_set.sequence_threshold, SEQUENCE_THRESHOLD);
            assert_eq!(unsafe { *hs.hash_set.next_value_index }, 0);
            let mut iterator = hs.iter();
            assert_eq!(iterator.next(), None);
            for i in 0..INDICES {
                assert!(unsafe { &*hs.hash_set.indices.as_ptr().add(i) }.is_none());
            }
            for i in 0..VALUES {
                assert!(unsafe { &*hs.hash_set.values.as_ptr().add(i) }.is_none());
            }

            for (seq, nullifier) in nullifiers.iter().enumerate() {
                hs.insert(&nullifier, seq).unwrap();
                hs.mark_with_sequence_number(&nullifier, seq).unwrap();
            }
        }

        // Read the hash set from buffers again.
        {
            let mut hs = unsafe {
                HashSetZeroCopy::<u16>::from_bytes_zero_copy_mut(bytes.as_mut_slice()).unwrap()
            };

            for (seq, nullifier) in nullifiers.iter().enumerate() {
                assert_eq!(hs.contains(nullifier, seq).unwrap(), true);
            }

            for (seq, nullifier) in nullifiers.iter().enumerate() {
                hs.insert(&nullifier, 2400 + seq as usize).unwrap();
            }
            drop(hs);
        }

        // Make a copy of hash set from the same buffers.
        {
            let hs = unsafe { HashSet::<u16>::from_bytes_copy(bytes.as_mut_slice()).unwrap() };

            for (seq, nullifier) in nullifiers.iter().enumerate() {
                assert_eq!(hs.contains(nullifier, 2400 + seq as usize).unwrap(), true);
            }
        }
    }
}
