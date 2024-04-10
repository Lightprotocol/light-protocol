use std::{
    alloc::{self, handle_alloc_error, Layout},
    fmt,
    marker::PhantomData,
    mem,
    ptr::NonNull,
};

use light_utils::{bigint::bigint_to_be_bytes_array, UtilsError};
use num_bigint::{BigUint, ToBigUint};
use num_traits::{Bounded, CheckedAdd, CheckedSub, FromBytes, ToPrimitive, Unsigned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HashSetError {
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
    #[error("Utils: big integer conversion error")]
    Utils(#[from] UtilsError),
}

#[cfg(feature = "solana")]
impl From<HashSetError> for u32 {
    fn from(e: HashSetError) -> u32 {
        match e {
            HashSetError::Full => 6001,
            HashSetError::ElementAlreadyExists => 6002,
            HashSetError::ElementDoesNotExist => 6003,
            HashSetError::Empty => 6004,
            HashSetError::UsizeConv => 6005,
            HashSetError::IntegerOverflow => 6006,
            HashSetError::BufferSize(_, _) => 6007,
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

pub fn find_next_prime(mut n: f64) -> f64 {
    n = n.round();

    // Handle small numbers separately
    if n <= 2.0 {
        return 2.0;
    } else if n <= 3.0 {
        return 3.0;
    }

    // All prime numbers greater than 3 are of the form 6k + 1 or 6k + 5 (or
    // 6k - 1).
    // That's because:
    //
    // 6k is divisible by 2 and 3.
    // 6k + 2 = 2(3k + 1) is divisible by 2.
    // 6k + 3 = 3(2k + 1) is divisible by 3.
    // 6k + 4 = 2(3k + 2) is divisible by 2.
    //
    // This leaves only 6k + 1 and 6k + 5 as candidates.

    // Ensure the candidate is of the form 6k - 1 or 6k + 1.
    let remainder = n % 6.0;
    if remainder != 0.0 {
        n = n + 6.0 - remainder;

        let candidate = n - 1.0;
        if is_prime(candidate) {
            return candidate;
        }
    }

    loop {
        let candidate = n + 1.0;
        if is_prime(candidate) {
            return candidate;
        }
        let candidate = n + 5.0;
        if is_prime(candidate) {
            return candidate;
        }

        n += 6.0;
    }
}

pub fn is_prime(n: f64) -> bool {
    if n <= 1.0 {
        return false;
    }
    if n <= 3.0 {
        return true;
    }
    if n % 2.0 == 0.0 || n % 3.0 == 0.0 {
        return false;
    }
    let mut i = 5.0;
    while i * i <= n {
        if n % i == 0.0 || n % (i + 2.0) == 0.0 {
            return false;
        }
        i += 6.0;
    }
    true
}

#[derive(Clone, Debug, PartialEq)]
pub struct HashSetCell {
    value: [u8; 32],
    sequence_number: Option<usize>,
}

impl HashSetCell {
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
    /// Capacity of `indices`, which is a prime number larger than the expected
    /// number of elements and an included load factor.
    capacity_indices: usize,
    /// Capacity of `values`, which is equal to the expected number of elements.
    capacity_values: usize,
    /// Difference of sequence numbers, after which the given element can be
    /// replaced by an another one (with a sequence number higher than the
    /// threshold).
    sequence_threshold: usize,

    /// Index of the next vacant cell in the value array.
    next_value_index: *mut usize,

    /// An array of indices which maps a hash set key to the index of its
    /// value which is stored in the `values` array. It has a size greater
    /// than the expected number of elements, determined by the load factor.
    indices: NonNull<Option<I>>,
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
    ) -> Result<usize, HashSetError> {
        let dyn_fields_size = Self::non_dyn_fields_size();
        let next_value_index_size = mem::size_of::<usize>();

        let indices_size_unaligned = mem::size_of::<Option<I>>() * capacity_indices;
        // Make sure that alignment of `indices` matches the alignment of `usize`.
        let indices_size = indices_size_unaligned + mem::align_of::<usize>()
            - (indices_size_unaligned % mem::align_of::<usize>());

        let values_size_unaligned = mem::size_of::<Option<HashSetCell>>() * capacity_values;
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
    ) -> Result<f64, HashSetError> {
        // To treat `capacity_elements` as `f64`, we need to fit it in `u32`.
        // `u64`/`usize` can't be casted directoy to `f64`.
        let capacity_elements =
            u32::try_from(capacity_elements).map_err(|_| HashSetError::IntegerOverflow)?;
        let minimum = f64::from(capacity_elements) / load_factor;
        Ok(find_next_prime(minimum))
    }

    // Create a new hash set with the given capacity
    pub fn new(
        capacity_indices: usize,
        capacity_values: usize,
        sequence_threshold: usize,
    ) -> Result<Self, HashSetError> {
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
            capacity_indices,
            capacity_values,
            indices,
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

        let capacity_indices = usize::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let capacity_values = usize::from_ne_bytes(bytes[8..16].try_into().unwrap());
        let sequence_threshold = usize::from_ne_bytes(bytes[16..24].try_into().unwrap());

        let expected_size = Self::size_in_account(capacity_indices, capacity_values)?;
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

        let indices_layout = Layout::array::<Option<I>>(capacity_indices).unwrap();
        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
        let indices_dst_ptr = unsafe { alloc::alloc(indices_layout) as *mut Option<I> };
        if indices_dst_ptr.is_null() {
            handle_alloc_error(indices_layout);
        }
        let indices = NonNull::new(indices_dst_ptr).unwrap();
        // Make sure that alignment of `indices` matches the alignment of `usize`.
        // This operation is adding a padding to the offset of `values` pointer.
        let indices_size = indices_layout.size() + mem::align_of::<usize>()
            - (indices_layout.size() % mem::size_of::<usize>());
        for i in 0..capacity_indices {
            std::ptr::write(indices_dst_ptr.add(i), None);
        }

        let offset = Self::non_dyn_fields_size() + mem::size_of::<usize>();
        let indices_src_ptr = bytes.as_ptr().add(offset) as *const Option<I>;
        std::ptr::copy(indices_src_ptr, indices_dst_ptr, capacity_indices);

        let values_layout = Layout::array::<Option<HashSetCell>>(capacity_values).unwrap();
        // SAFETY: `I` is always a signed integer. Creating a layout for an
        // array of integers of any size won't cause any panic.
        let values_dst_ptr = unsafe { alloc::alloc(values_layout) as *mut Option<HashSetCell> };
        if values_dst_ptr.is_null() {
            handle_alloc_error(values_layout);
        }
        let values = NonNull::new(values_dst_ptr).unwrap();
        for i in 0..capacity_values {
            std::ptr::write(values_dst_ptr.add(i), None);
        }

        let offset = offset + indices_size;
        let values_src_ptr = bytes.as_ptr().add(offset) as *const Option<HashSetCell>;
        std::ptr::copy(values_src_ptr, values_dst_ptr, capacity_values);

        Ok(Self {
            capacity_indices,
            capacity_values,
            next_value_index,
            sequence_threshold,
            indices,
            values,
        })
    }

    fn insert_into_occupied_cell(
        &mut self,
        value_index: usize,
        value: &BigUint,
        current_sequence_number: usize,
    ) -> Result<bool, HashSetError> {
        let value_bucket = unsafe { &mut *self.values.as_ptr().add(value_index) };
        match value_bucket {
            // The cell in the value array is already taken.
            Some(value_bucket) => {
                // We can overwrite that cell only if the element
                // is expired - when the difference between its
                // sequence number and provided sequence number is
                // greater than the threshold.
                if let Some(element_sequence_number) = value_bucket.sequence_number {
                    if current_sequence_number >= element_sequence_number {
                        *value_bucket = HashSetCell {
                            value: bigint_to_be_bytes_array(value)?,
                            sequence_number: None,
                        };
                        return Ok(true);
                    }
                }
                // Otherwise, we need to prevent having multiple valid
                // elements with the same value.
                if &BigUint::from_be_bytes(value_bucket.value.as_slice()) == value {
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

    /// Inserts a value into the hash set.
    pub fn insert(
        &mut self,
        value: &BigUint,
        current_sequence_number: usize,
    ) -> Result<(), HashSetError> {
        for i in 0..self.capacity_values {
            let i = I::try_from(i).map_err(|_| HashSetError::IntegerOverflow)?;
            let probe_index = (value.clone() + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % self.capacity_values.to_biguint().unwrap();
            let probe_index = probe_index.to_usize().unwrap();
            let index_bucket = unsafe { &mut *self.indices.as_ptr().add(probe_index) };

            match index_bucket {
                // The visited hash set cell points to a value in the array.
                Some(value_index) => {
                    let value_index =
                        usize::try_from(*value_index).map_err(|_| HashSetError::UsizeConv)?;
                    if self.insert_into_occupied_cell(
                        value_index,
                        value,
                        current_sequence_number,
                    )? {
                        return Ok(());
                    }
                }
                None => {
                    let value_bucket =
                        unsafe { &mut *self.values.as_ptr().add(*self.next_value_index) };
                    // SAFETY: `next_value_index` is always initialized.
                    *index_bucket = Some(
                        I::try_from(unsafe { *self.next_value_index })
                            .map_err(|_| HashSetError::IntegerOverflow)?,
                    );
                    *value_bucket = Some(HashSetCell {
                        value: bigint_to_be_bytes_array(value)?,
                        sequence_number: None,
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

        Err(HashSetError::Full)
    }

    pub fn find_element(
        &self,
        value: &BigUint,
        current_sequence_number: Option<usize>,
    ) -> Result<Option<&mut HashSetCell>, HashSetError> {
        for i in 0..self.capacity_values {
            let i = I::try_from(i).map_err(|_| HashSetError::IntegerOverflow)?;
            let probe_index = (value.clone() + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % self.capacity_values.to_biguint().unwrap();
            let probe_index = probe_index.to_usize().unwrap();
            let index_bucket = unsafe { &*self.indices.as_ptr().add(probe_index) };

            match index_bucket {
                Some(value_index) => {
                    let value_bucket = self.by_value_index(
                        usize::try_from(*value_index).map_err(|_| HashSetError::UsizeConv)?,
                        current_sequence_number,
                    );
                    if let Some(value_bucket) = value_bucket {
                        if &value_bucket.value_biguint() == value {
                            return Ok(Some(value_bucket));
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

    pub fn by_value_index(
        &self,
        value_index: usize,
        current_sequence_number: Option<usize>,
    ) -> Option<&mut HashSetCell> {
        let value_bucket = unsafe { &mut *self.values.as_ptr().add(value_index) };
        if let Some(value_bucket) = value_bucket {
            match current_sequence_number {
                Some(current_sequence_number) => {
                    // If the `current_sequence_number` was specified,
                    // search for an element with either...
                    match value_bucket.sequence_number {
                        // ...a lower sequence number...
                        Some(element_sequence_number) => {
                            if current_sequence_number < element_sequence_number {
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
    pub fn contains(&self, value: &BigUint, sequence_number: usize) -> Result<bool, HashSetError> {
        let element = self.find_element(value, Some(sequence_number))?;
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
            Some(element) => {
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
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use rand::{thread_rng, Rng};

    use super::*;

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
        assert_eq!(hs.contains(&element_1_1, 1).unwrap(), true);
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
        assert_eq!(hs.contains(&element_2_3, 2).unwrap(), true);
        assert_eq!(hs.contains(&element_2_6, 2).unwrap(), true);
        assert_eq!(hs.contains(&element_2_8, 2).unwrap(), true);
        assert_eq!(hs.contains(&element_2_9, 2).unwrap(), true);
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
        assert_eq!(hs.contains(&element_3_11, 3).unwrap(), true);
        assert_eq!(hs.contains(&element_3_13, 3).unwrap(), true);
        assert_eq!(hs.contains(&element_3_21, 3).unwrap(), true);
        assert_eq!(hs.contains(&element_3_29, 3).unwrap(), true);
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
        assert_eq!(hs.contains(&element_4_93, 4).unwrap(), true);
        assert_eq!(hs.contains(&element_4_65, 4).unwrap(), true);
        assert_eq!(hs.contains(&element_4_72, 4).unwrap(), true);
        assert_eq!(hs.contains(&element_4_15, 4).unwrap(), true);
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

        let nullifiers: [BigUint; 2400] =
            std::array::from_fn(|_| BigUint::from(Fr::rand(&mut rng)));

        for (seq, nullifier) in nullifiers.iter().enumerate() {
            assert_eq!(hs.contains(&nullifier, seq).unwrap(), false);
            hs.insert(&nullifier, seq as usize).unwrap();
            assert_eq!(hs.contains(&nullifier, seq).unwrap(), true);
            assert_eq!(
                hs.find_element(&nullifier, Some(seq)).unwrap().cloned(),
                Some(HashSetCell {
                    value: bigint_to_be_bytes_array(&nullifier).unwrap(),
                    sequence_number: None,
                })
            );

            hs.mark_with_sequence_number(&nullifier, seq).unwrap();
            assert_eq!(
                hs.find_element(&nullifier, Some(seq)).unwrap().cloned(),
                Some(HashSetCell {
                    value: bigint_to_be_bytes_array(&nullifier).unwrap(),
                    sequence_number: Some(2400 + seq)
                })
            );

            // Trying to insert the same nullifier, before reaching the
            // sequence threshold, should fail.
            assert!(matches!(
                hs.insert(&nullifier, seq as usize + 1),
                Err(HashSetError::ElementAlreadyExists),
            ));
        }

        for (i, element) in hs.iter() {
            assert_eq!(element.value_biguint(), nullifiers[i]);
        }

        // As long as we request the first element while providing sequence
        // numbers not reaching the threshold (from 0 to 2399)
        for seq in 0..2399 {
            assert_eq!(
                hs.first(seq).unwrap().unwrap().value_biguint(),
                nullifiers[0]
            );
        }
        // Once we hit the threshold, we are going to receive next elements as
        // the first ones.
        for (seq, nullifier) in nullifiers.iter().enumerate() {
            assert_eq!(
                &hs.first(2399 + seq).unwrap().unwrap().value_biguint(),
                nullifier
            );
        }

        // As we reach the sequence threshold, we should be able to override
        // the same nullifiers.
        for (seq, nullifier) in nullifiers.iter().enumerate() {
            hs.insert(&nullifier, 2400 + seq as usize).unwrap();
        }
    }

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
