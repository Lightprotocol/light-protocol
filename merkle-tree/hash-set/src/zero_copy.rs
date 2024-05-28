use std::{marker::PhantomData, mem, ptr::NonNull};

use num_bigint::BigUint;

use crate::{HashSet, HashSetCell, HashSetError, HashSetIterator};

/// A `HashSet` wrapper which can be instantiated from Solana account bytes
/// without copying them.
#[derive(Debug)]
pub struct HashSetZeroCopy<'a> {
    pub hash_set: mem::ManuallyDrop<HashSet>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> HashSetZeroCopy<'a> {
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
        if bytes.len() < HashSet::non_dyn_fields_size() {
            return Err(HashSetError::BufferSize(
                HashSet::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        let capacity_values = usize::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let sequence_threshold = usize::from_ne_bytes(bytes[8..16].try_into().unwrap());

        let offset = HashSet::non_dyn_fields_size() + mem::size_of::<usize>();

        let values_size = mem::size_of::<Option<HashSetCell>>() * capacity_values;

        let expected_size = HashSet::non_dyn_fields_size() + values_size;
        if bytes.len() < expected_size {
            return Err(HashSetError::BufferSize(expected_size, bytes.len()));
        }

        let buckets =
            NonNull::new(bytes.as_mut_ptr().add(offset) as *mut Option<HashSetCell>).unwrap();

        Ok(Self {
            hash_set: mem::ManuallyDrop::new(HashSet {
                capacity: capacity_values,
                sequence_threshold,
                buckets,
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
        capacity_values: usize,
        sequence_threshold: usize,
    ) -> Result<Self, HashSetError> {
        if bytes.len() < HashSet::non_dyn_fields_size() {
            return Err(HashSetError::BufferSize(
                HashSet::non_dyn_fields_size(),
                bytes.len(),
            ));
        }

        bytes[0..8].copy_from_slice(&capacity_values.to_ne_bytes());
        bytes[8..16].copy_from_slice(&sequence_threshold.to_ne_bytes());
        bytes[16..24].copy_from_slice(&0_usize.to_ne_bytes());

        let hash_set = Self::from_bytes_zero_copy_mut(bytes)?;

        for i in 0..capacity_values {
            std::ptr::write(hash_set.hash_set.buckets.as_ptr().add(i), None);
        }

        Ok(hash_set)
    }

    /// Returns a reference to a bucket under the given `index`. Does not check
    /// the validity.
    pub fn get_bucket(&self, index: usize) -> Option<&Option<HashSetCell>> {
        self.hash_set.get_bucket(index)
    }

    /// Returns a mutable reference to a bucket under the given `index`. Does
    /// not check the validity.
    pub fn get_bucket_mut(&mut self, index: usize) -> Option<&mut Option<HashSetCell>> {
        self.hash_set.get_bucket_mut(index)
    }

    /// Returns a reference to an unmarked bucket under the given index. If the
    /// bucket is marked, returns `None`.
    pub fn get_unmarked_bucket(&self, index: usize) -> Option<&Option<HashSetCell>> {
        self.hash_set.get_unmarked_bucket(index)
    }

    /// Inserts a value into the hash set.
    pub fn insert(&mut self, value: &BigUint, sequence_number: usize) -> Result<(), HashSetError> {
        self.hash_set.insert(value, sequence_number)
    }

    /// Returns a first available element.
    pub fn first(&self, sequence_number: usize) -> Result<Option<&HashSetCell>, HashSetError> {
        self.hash_set.first(sequence_number)
    }

    /// Check if the hash set contains a value.
    pub fn contains(&self, value: &BigUint, sequence_number: usize) -> Result<bool, HashSetError> {
        self.hash_set.contains(value, Some(sequence_number))
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

    pub fn iter(&self) -> HashSetIterator {
        self.hash_set.iter()
    }
}

impl<'a> Drop for HashSetZeroCopy<'a> {
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
        const VALUES: usize = 4800;
        const SEQUENCE_THRESHOLD: usize = 2400;

        // Create a buffer with random bytes.
        let mut bytes = vec![0u8; HashSet::size_in_account(VALUES).unwrap()];
        thread_rng().fill(bytes.as_mut_slice());

        // Create random nullifiers.
        let mut rng = thread_rng();
        let nullifiers: [BigUint; 2400] =
            std::array::from_fn(|_| BigUint::from(Fr::rand(&mut rng)));

        // Initialize a hash set on top of a byte slice.
        {
            let mut hs = unsafe {
                HashSetZeroCopy::from_bytes_zero_copy_init(
                    bytes.as_mut_slice(),
                    VALUES,
                    SEQUENCE_THRESHOLD,
                )
                .unwrap()
            };

            // Ensure that the underlying data were properly initialized.
            assert_eq!(hs.hash_set.capacity, VALUES);
            assert_eq!(hs.hash_set.sequence_threshold, SEQUENCE_THRESHOLD);
            let mut iterator = hs.iter();
            assert_eq!(iterator.next(), None);
            for i in 0..VALUES {
                assert!(unsafe { &*hs.hash_set.buckets.as_ptr().add(i) }.is_none());
            }

            for (seq, nullifier) in nullifiers.iter().enumerate() {
                hs.insert(&nullifier, seq).unwrap();
                hs.mark_with_sequence_number(&nullifier, seq).unwrap();
            }
        }

        // Read the hash set from buffers again.
        {
            let mut hs =
                unsafe { HashSetZeroCopy::from_bytes_zero_copy_mut(bytes.as_mut_slice()).unwrap() };

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
            let hs = unsafe { HashSet::from_bytes_copy(bytes.as_mut_slice()).unwrap() };

            for (seq, nullifier) in nullifiers.iter().enumerate() {
                assert_eq!(
                    hs.contains(nullifier, Some(2400 + seq as usize)).unwrap(),
                    true
                );
            }
        }
    }

    #[test]
    fn test_buffer_size_error() {
        const VALUES: usize = 4800;
        const SEQUENCE_THRESHOLD: usize = 2400;

        let mut invalid_bytes = vec![0_u8; 256];

        let res = unsafe {
            HashSetZeroCopy::from_bytes_zero_copy_init(
                invalid_bytes.as_mut_slice(),
                VALUES,
                SEQUENCE_THRESHOLD,
            )
        };
        assert!(matches!(res, Err(HashSetError::BufferSize(_, _))));
    }
}
