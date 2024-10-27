pub mod offset;

use std::{
    alloc::{self, handle_alloc_error, Layout},
    fmt,
    mem::{self, ManuallyDrop},
    ops::{Index, IndexMut, Sub},
    ptr::{self, NonNull},
    slice::{self, Iter, IterMut, SliceIndex},
};

use memoffset::span_of;
use offset::zero_copy::{read_array_like_ptr_at, read_ptr_at, write_at};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum BoundedVecError {
    #[error("The vector is full, cannot push any new elements")]
    Full,
    #[error("Requested array of size {0}, but the vector has {1} elements")]
    ArraySize(usize, usize),
    #[error("The requested start index is out of bounds.")]
    IterFromOutOfBounds,
    #[error("Input length {0} allocates unaligned memory. Must be a multiple of 8.")]
    UnalignedMemory(usize),
    #[error("Memory allocated {0}, Memory required {0}")]
    InsufficientMemoryAllocated(usize, usize),
}

#[cfg(feature = "solana")]
impl From<BoundedVecError> for u32 {
    fn from(e: BoundedVecError) -> u32 {
        match e {
            BoundedVecError::Full => 8001,
            BoundedVecError::ArraySize(_, _) => 8002,
            BoundedVecError::IterFromOutOfBounds => 8003,
            BoundedVecError::UnalignedMemory(_) => 8004,
            BoundedVecError::InsufficientMemoryAllocated(_, _) => 8005,
        }
    }
}

#[cfg(feature = "solana")]
impl From<BoundedVecError> for solana_program::program_error::ProgramError {
    fn from(e: BoundedVecError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedVecMetadata {
    capacity: usize,
    length: usize,
}

impl BoundedVecMetadata {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            length: 0,
        }
    }

    pub fn new_with_length(capacity: usize, length: usize) -> Self {
        Self { capacity, length }
    }

    pub fn from_le_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self {
        Self {
            capacity: usize::from_le_bytes(bytes[span_of!(Self, capacity)].try_into().unwrap()),
            length: usize::from_le_bytes(bytes[span_of!(Self, length)].try_into().unwrap()),
        }
    }

    pub fn to_le_bytes(&self) -> [u8; mem::size_of::<Self>()] {
        let mut bytes = [0u8; mem::size_of::<Self>()];
        bytes[span_of!(Self, capacity)].copy_from_slice(&self.capacity.to_le_bytes());
        bytes[span_of!(Self, length)].copy_from_slice(&self.length.to_le_bytes());

        bytes
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn length(&self) -> usize {
        self.length
    }
}

/// `BoundedVec` is a custom vector implementation which forbids
/// post-initialization reallocations. The size is not known during compile
/// time (that makes it different from arrays), but can be defined only once
/// (that makes it different from [`Vec`](std::vec::Vec)).
pub struct BoundedVec<T>
where
    T: Clone,
{
    metadata: *mut BoundedVecMetadata,
    data: NonNull<T>,
}

impl<T> BoundedVec<T>
where
    T: Clone,
{
    #[inline]
    fn metadata_with_capacity(capacity: usize) -> *mut BoundedVecMetadata {
        let layout = Layout::new::<BoundedVecMetadata>();
        let metadata = unsafe { alloc::alloc(layout) as *mut BoundedVecMetadata };
        if metadata.is_null() {
            handle_alloc_error(layout);
        }
        unsafe {
            *metadata = BoundedVecMetadata {
                capacity,
                length: 0,
            };
        }

        metadata
    }

    #[inline]
    fn metadata_from(src_metadata: &BoundedVecMetadata) -> *mut BoundedVecMetadata {
        let layout = Layout::new::<BoundedVecMetadata>();
        let metadata = unsafe { alloc::alloc(layout) as *mut BoundedVecMetadata };
        if metadata.is_null() {
            handle_alloc_error(layout);
        }
        unsafe { (*metadata).clone_from(src_metadata) };

        metadata
    }

    #[inline]
    fn data_with_capacity(capacity: usize) -> NonNull<T> {
        let layout = Layout::array::<T>(capacity).unwrap();
        let data_ptr = unsafe { alloc::alloc(layout) as *mut T };
        if data_ptr.is_null() {
            handle_alloc_error(layout);
        }
        // PANICS: We ensured that the pointer is not NULL.
        NonNull::new(data_ptr).unwrap()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let metadata = Self::metadata_with_capacity(capacity);
        let data = Self::data_with_capacity(capacity);

        Self { metadata, data }
    }

    #[inline]
    pub fn clear(&mut self) {
        unsafe {
            (*self.metadata).length = 0;
        }
    }

    /// Creates a `BoundedVec<T>` with the given `metadata`.
    ///
    /// # Safety
    ///
    /// This method is unsafe, as it does not guarantee the correctness of
    /// provided parameters (other than `capacity`). The full responisibility
    /// is on the caller.
    #[inline]
    pub unsafe fn with_metadata(metadata: &BoundedVecMetadata) -> Self {
        let capacity = metadata.capacity();
        let metadata = Self::metadata_from(metadata);
        let data = Self::data_with_capacity(capacity);

        Self { metadata, data }
    }

    pub fn metadata(&self) -> &BoundedVecMetadata {
        unsafe { &*self.metadata }
    }

    pub fn from_array<const N: usize>(array: &[T; N]) -> Self {
        let mut vec = Self::with_capacity(N);
        for element in array {
            // SAFETY: We are sure that the array and the vector have equal
            // sizes, there is no chance for the error to occur.
            vec.push(element.clone()).unwrap();
        }
        vec
    }

    pub fn from_slice(slice: &[T]) -> Self {
        let mut vec = Self::with_capacity(slice.len());
        for element in slice {
            // SAFETY: We are sure that the array and the vector have equal
            // sizes, there is no chance for the error to occur.
            vec.push(element.clone()).unwrap();
        }
        vec
    }

    /// Creates `BoundedVec<T>` directly from a pointer, a capacity, and a length.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * `ptr` must have been allocated using the global allocator, such as via
    ///   the [`alloc::alloc`] function.
    /// * `T` needs to have the same alignment as what `ptr` was allocated with.
    ///   (`T` having a less strict alignment is not sufficient, the alignment really
    ///   needs to be equal to satisfy the [`dealloc`] requirement that memory must be
    ///   allocated and deallocated with the same layout.)
    /// * The size of `T` times the `capacity` (ie. the allocated size in bytes) needs
    ///   to be the same size as the pointer was allocated with. (Because similar to
    ///   alignment, [`dealloc`] must be called with the same layout `size`.)
    /// * `length` needs to be less than or equal to `capacity`.
    /// * The first `length` values must be properly initialized values of type `T`.
    /// * `capacity` needs to be the capacity that the pointer was allocated with.
    /// * The allocated size in bytes must be no larger than `isize::MAX`.
    ///   See the safety documentation of [`pointer::offset`].
    #[inline]
    pub unsafe fn from_raw_parts(metadata: *mut BoundedVecMetadata, ptr: *mut T) -> Self {
        let data = NonNull::new(ptr).unwrap();
        Self { metadata, data }
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec: Vec<i32> = Vec::with_capacity(10);
    /// vec.push(42);
    /// assert!(vec.capacity() >= 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe { (*self.metadata).capacity }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.len()) }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec = vec![1, 2];
    /// vec.push(3);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), BoundedVecError> {
        if self.len() == self.capacity() {
            return Err(BoundedVecError::Full);
        }

        unsafe { ptr::write(self.data.as_ptr().add(self.len()), value) };
        self.inc_len();

        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { (*self.metadata).length }
    }

    #[inline]
    fn inc_len(&mut self) {
        unsafe { (*self.metadata).length += 1 };
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        let cell = unsafe { &*self.data.as_ptr().add(index) };
        Some(cell)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        let cell = unsafe { &mut *self.data.as_ptr().add(index) };
        Some(cell)
    }

    /// Returns a mutable pointer to `BoundedVec`'s buffer.
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr()
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.get(self.len() - 1)
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            return None;
        }
        self.get_mut(self.len() - 1)
    }

    pub fn to_array<const N: usize>(&self) -> Result<[T; N], BoundedVecError> {
        if self.len() != N {
            return Err(BoundedVecError::ArraySize(N, self.len()));
        }
        Ok(std::array::from_fn(|i| self.get(i).unwrap().clone()))
    }

    pub fn to_vec(self) -> Vec<T> {
        self.as_slice().to_vec()
    }

    pub fn extend<U: IntoIterator<Item = T>>(&mut self, iter: U) -> Result<(), BoundedVecError> {
        for item in iter {
            self.push(item)?;
        }
        Ok(())
    }

    pub fn deserialize(
        account_data: &mut [u8],
        start_offset: &mut usize,
    ) -> Result<ManuallyDrop<BoundedVec<T>>, BoundedVecError> {
        unsafe {
            if account_data.len().saturating_sub(*start_offset)
                < std::mem::size_of::<BoundedVecMetadata>()
            {
                return Err(BoundedVecError::InsufficientMemoryAllocated(
                    account_data.len().saturating_sub(*start_offset),
                    std::mem::size_of::<BoundedVecMetadata>(),
                ));
            }
            let metadata: *mut BoundedVecMetadata = read_ptr_at(account_data, start_offset);
            if ((*metadata).capacity() * std::mem::size_of::<T>()) % 8 != 0 {
                return Err(BoundedVecError::UnalignedMemory(
                    (*metadata).capacity() * std::mem::size_of::<T>(),
                ));
            }
            let full_vector_size = std::mem::size_of::<BoundedVecMetadata>()
                + ((*metadata).capacity() * std::mem::size_of::<T>());
            if account_data.len().saturating_sub(*start_offset) < full_vector_size {
                return Err(BoundedVecError::InsufficientMemoryAllocated(
                    account_data.len().saturating_sub(*start_offset),
                    full_vector_size,
                ));
            }
            Ok(ManuallyDrop::new(BoundedVec::from_raw_parts(
                metadata,
                read_array_like_ptr_at(account_data, start_offset, (*metadata).capacity()),
            )))
        }
    }

    pub fn deserialize_multiple(
        num: usize,
        account_data: &mut [u8],
        start_offset: &mut usize,
    ) -> Result<Vec<ManuallyDrop<BoundedVec<T>>>, BoundedVecError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let vec = Self::deserialize(account_data, start_offset)?;
            value_vecs.push(vec);
        }
        Ok(value_vecs)
    }

    pub fn init(
        capacity: usize,
        account_data: &mut [u8],
        start_offset: &mut usize,
        with_len: bool,
    ) -> Result<ManuallyDrop<BoundedVec<T>>, BoundedVecError> {
        let vector_size = capacity * std::mem::size_of::<T>();
        if vector_size % 8 != 0 {
            return Err(BoundedVecError::UnalignedMemory(
                capacity * std::mem::size_of::<T>(),
            ));
        }
        let full_vector_size = vector_size + std::mem::size_of::<BoundedVecMetadata>();
        if full_vector_size > account_data.len().saturating_sub(*start_offset) {
            return Err(BoundedVecError::InsufficientMemoryAllocated(
                account_data.len().saturating_sub(*start_offset),
                full_vector_size,
            ));
        }
        let meta: BoundedVecMetadata = if with_len {
            BoundedVecMetadata::new_with_length(capacity, capacity)
        } else {
            BoundedVecMetadata::new(capacity)
        };
        write_at::<BoundedVecMetadata>(account_data, meta.to_le_bytes().as_slice(), start_offset);
        let meta: *mut BoundedVecMetadata = unsafe {
            read_ptr_at(
                &*account_data,
                &mut start_offset.sub(std::mem::size_of::<BoundedVecMetadata>()),
            )
        };

        Ok(unsafe {
            ManuallyDrop::new(BoundedVec::from_raw_parts(
                meta,
                read_array_like_ptr_at(&*account_data, start_offset, capacity),
            ))
        })
    }

    pub fn init_multiple(
        num: usize,
        capacity: usize,
        account_data: &mut [u8],
        start_offset: &mut usize,
        with_len: bool,
    ) -> Result<Vec<ManuallyDrop<BoundedVec<T>>>, BoundedVecError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let vec = Self::init(capacity, account_data, start_offset, with_len)?;
            value_vecs.push(vec);
        }
        Ok(value_vecs)
    }
}

impl<T> Clone for BoundedVec<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        // Create a new buffer with the same capacity as the original

        let layout = Layout::new::<BoundedVecMetadata>();
        let metadata = unsafe { alloc::alloc(layout) as *mut BoundedVecMetadata };
        if metadata.is_null() {
            handle_alloc_error(layout);
        }
        unsafe { *metadata = (*self.metadata).clone() };

        let layout = Layout::array::<T>(self.capacity()).unwrap();
        let data_ptr = unsafe { alloc::alloc(layout) as *mut T };
        if data_ptr.is_null() {
            handle_alloc_error(layout);
        }
        let data = NonNull::new(data_ptr).unwrap();

        // Copy elements from the original data slice to the new slice
        let new_vec = Self { metadata, data };

        // Clone each element into the new vector
        for i in 0..self.len() {
            unsafe { ptr::write(data_ptr.add(i), (*self.get(i).unwrap()).clone()) };
        }

        new_vec
    }
}

impl<T> fmt::Debug for BoundedVec<T>
where
    T: Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

impl<T> Drop for BoundedVec<T>
where
    T: Clone,
{
    fn drop(&mut self) {
        let layout = Layout::array::<T>(self.capacity()).unwrap();
        unsafe { alloc::dealloc(self.data.as_ptr() as *mut u8, layout) };

        let layout = Layout::new::<BoundedVecMetadata>();
        unsafe { alloc::dealloc(self.metadata as *mut u8, layout) };
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for BoundedVec<T>
where
    T: Clone,
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T, I> IndexMut<I> for BoundedVec<T>
where
    T: Clone,
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(index)
    }
}

impl<T> IntoIterator for BoundedVec<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = BoundedVecIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        BoundedVecIntoIterator {
            vec: self,
            current: 0,
        }
    }
}

impl<T> PartialEq for BoundedVec<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<T> Eq for BoundedVec<T> where T: Clone + Eq {}

pub struct BoundedVecIntoIterator<T>
where
    T: Clone,
{
    vec: BoundedVec<T>,
    current: usize,
}

impl<T> Iterator for BoundedVecIntoIterator<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let element = self.vec.get(self.current).map(|element| element.to_owned());
        self.current += 1;
        element
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CyclicBoundedVecMetadata {
    capacity: usize,
    length: usize,
    first_index: usize,
    last_index: usize,
}

impl CyclicBoundedVecMetadata {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            length: 0,
            first_index: 0,
            last_index: 0,
        }
    }

    pub fn new_with_length(capacity: usize, length: usize) -> Self {
        Self {
            capacity,
            length,
            first_index: 0,
            last_index: 0,
        }
    }

    pub fn new_with_indices(
        capacity: usize,
        length: usize,
        first_index: usize,
        last_index: usize,
    ) -> Self {
        Self {
            capacity,
            length,
            first_index,
            last_index,
        }
    }

    pub fn from_le_bytes(bytes: [u8; mem::size_of::<CyclicBoundedVecMetadata>()]) -> Self {
        Self {
            capacity: usize::from_le_bytes(bytes[span_of!(Self, capacity)].try_into().unwrap()),
            length: usize::from_le_bytes(bytes[span_of!(Self, length)].try_into().unwrap()),
            first_index: usize::from_le_bytes(
                bytes[span_of!(Self, first_index)].try_into().unwrap(),
            ),
            last_index: usize::from_le_bytes(bytes[span_of!(Self, last_index)].try_into().unwrap()),
        }
    }

    pub fn to_le_bytes(&self) -> [u8; mem::size_of::<Self>()] {
        let mut bytes = [0u8; mem::size_of::<Self>()];
        bytes[span_of!(Self, capacity)].copy_from_slice(&self.capacity.to_le_bytes());
        bytes[span_of!(Self, length)].copy_from_slice(&self.length.to_le_bytes());
        bytes[span_of!(Self, first_index)].copy_from_slice(&self.first_index.to_le_bytes());
        bytes[span_of!(Self, last_index)].copy_from_slice(&self.last_index.to_le_bytes());

        bytes
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn get_first_index(&self) -> usize {
        self.first_index
    }
}

/// `CyclicBoundedVec` is a wrapper around [`Vec`](std::vec::Vec) which:
///
/// * Forbids post-initialization reallocations.
/// * Starts overwriting elements from the beginning once it reaches its
///   capacity.
pub struct CyclicBoundedVec<T>
where
    T: Clone,
{
    metadata: *mut CyclicBoundedVecMetadata,
    data: NonNull<T>,
}

impl<T> CyclicBoundedVec<T>
where
    T: Clone,
{
    #[inline]
    fn metadata_with_capacity(capacity: usize) -> *mut CyclicBoundedVecMetadata {
        let layout = Layout::new::<CyclicBoundedVecMetadata>();
        let metadata = unsafe { alloc::alloc(layout) as *mut CyclicBoundedVecMetadata };
        if metadata.is_null() {
            handle_alloc_error(layout);
        }
        unsafe {
            *metadata = CyclicBoundedVecMetadata {
                capacity,
                length: 0,
                first_index: 0,
                last_index: 0,
            };
        }

        metadata
    }

    #[inline]
    fn metadata_from(src_metadata: &CyclicBoundedVecMetadata) -> *mut CyclicBoundedVecMetadata {
        let layout = Layout::new::<CyclicBoundedVecMetadata>();
        let metadata = unsafe { alloc::alloc(layout) as *mut CyclicBoundedVecMetadata };
        if metadata.is_null() {
            handle_alloc_error(layout);
        }
        unsafe { (*metadata).clone_from(src_metadata) };

        metadata
    }

    #[inline]
    fn data_with_capacity(capacity: usize) -> NonNull<T> {
        let layout = Layout::array::<T>(capacity).unwrap();
        let data_ptr = unsafe { alloc::alloc(layout) as *mut T };
        if data_ptr.is_null() {
            handle_alloc_error(layout);
        }
        // PANICS: We ensured that the pointer is not NULL.
        NonNull::new(data_ptr).unwrap()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let metadata = Self::metadata_with_capacity(capacity);
        let data = Self::data_with_capacity(capacity);

        Self { metadata, data }
    }

    /// Creates a `CyclicBoundedVec<T>` with the given `metadata`.
    ///
    /// # Safety
    ///
    /// This method is unsafe, as it does not guarantee the correctness of
    /// provided parameters (other than `capacity`). The full responisibility
    /// is on the caller.
    #[inline]
    pub unsafe fn with_metadata(metadata: &CyclicBoundedVecMetadata) -> Self {
        let capacity = metadata.capacity();
        let metadata = Self::metadata_from(metadata);
        let data = Self::data_with_capacity(capacity);

        Self { metadata, data }
    }

    pub fn metadata(&self) -> &CyclicBoundedVecMetadata {
        unsafe { &*self.metadata }
    }

    /// Creates a `CyclicBoundedVec<T>` directly from a pointer, a capacity, and a length.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * `ptr` must have been allocated using the global allocator, such as via
    ///   the [`alloc::alloc`] function.
    /// * `T` needs to have the same alignment as what `ptr` was allocated with.
    ///   (`T` having a less strict alignment is not sufficient, the alignment really
    ///   needs to be equal to satisfy the [`dealloc`] requirement that memory must be
    ///   allocated and deallocated with the same layout.)
    /// * The size of `T` times the `capacity` (ie. the allocated size in bytes) needs
    ///   to be the same size as the pointer was allocated with. (Because similar to
    ///   alignment, [`dealloc`] must be called with the same layout `size`.)
    /// * `length` needs to be less than or equal to `capacity`.
    /// * The first `length` values must be properly initialized values of type `T`.
    /// * `capacity` needs to be the capacity that the pointer was allocated with.
    /// * The allocated size in bytes must be no larger than `isize::MAX`.
    ///   See the safety documentation of [`pointer::offset`].
    #[inline]
    pub unsafe fn from_raw_parts(metadata: *mut CyclicBoundedVecMetadata, ptr: *mut T) -> Self {
        let data = NonNull::new(ptr).unwrap();
        Self { metadata, data }
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec: Vec<i32> = Vec::with_capacity(10);
    /// vec.push(42);
    /// assert!(vec.capacity() >= 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe { (*self.metadata).capacity }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec = vec![1, 2];
    /// vec.push(3);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.is_empty() {
            self.inc_len();
        } else if self.len() < self.capacity() {
            self.inc_len();
            self.inc_last_index();
        } else {
            self.inc_last_index();
            self.inc_first_index();
        }
        // SAFETY: We made sure that `last_index` doesn't exceed the capacity.
        unsafe {
            std::ptr::write(self.data.as_ptr().add(self.last_index()), value);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { (*self.metadata).length }
    }

    #[inline]
    fn inc_len(&mut self) {
        unsafe { (*self.metadata).length += 1 }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        let cell = unsafe { &*self.data.as_ptr().add(index) };
        Some(cell)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        let cell = unsafe { &mut *self.data.as_ptr().add(index) };
        Some(cell)
    }

    /// Returns a mutable pointer to `BoundedVec`'s buffer.
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr()
    }

    #[inline]
    pub fn iter(&self) -> CyclicBoundedVecIterator<'_, T> {
        CyclicBoundedVecIterator {
            vec: self,
            current: self.first_index(),
            is_finished: false,
        }
    }

    #[inline]
    pub fn iter_from(
        &self,
        start: usize,
    ) -> Result<CyclicBoundedVecIterator<'_, T>, BoundedVecError> {
        if start >= self.len() {
            return Err(BoundedVecError::IterFromOutOfBounds);
        }
        Ok(CyclicBoundedVecIterator {
            vec: self,
            current: start,
            is_finished: false,
        })
    }

    #[inline]
    pub fn first_index(&self) -> usize {
        unsafe { (*self.metadata).first_index }
    }

    #[inline]
    fn inc_first_index(&self) {
        unsafe {
            (*self.metadata).first_index = ((*self.metadata).first_index + 1) % self.capacity();
        }
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.get(self.first_index())
    }

    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.first_index())
    }

    #[inline]
    pub fn last_index(&self) -> usize {
        unsafe { (*self.metadata).last_index }
    }

    #[inline]
    fn inc_last_index(&mut self) {
        unsafe {
            (*self.metadata).last_index = ((*self.metadata).last_index + 1) % self.capacity();
        }
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.get(self.last_index())
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.last_index())
    }

    pub fn init(
        capacity: usize,
        account_data: &mut [u8],
        start_offset: &mut usize,
        with_len: bool,
    ) -> Result<ManuallyDrop<Self>, BoundedVecError> {
        let vector_size = capacity * std::mem::size_of::<T>();
        if vector_size % 8 != 0 {
            return Err(BoundedVecError::UnalignedMemory(
                capacity * std::mem::size_of::<T>(),
            ));
        }
        let full_vector_size = vector_size + std::mem::size_of::<CyclicBoundedVecMetadata>();
        if full_vector_size > account_data.len().saturating_sub(*start_offset) {
            return Err(BoundedVecError::InsufficientMemoryAllocated(
                account_data.len().saturating_sub(*start_offset),
                full_vector_size,
            ));
        }
        let meta: CyclicBoundedVecMetadata = if with_len {
            CyclicBoundedVecMetadata::new_with_length(capacity, capacity)
        } else {
            CyclicBoundedVecMetadata::new(capacity)
        };
        write_at::<CyclicBoundedVecMetadata>(
            account_data,
            meta.to_le_bytes().as_slice(),
            start_offset,
        );
        let meta: *mut CyclicBoundedVecMetadata = unsafe {
            read_ptr_at(
                &*account_data,
                &mut start_offset.sub(std::mem::size_of::<CyclicBoundedVecMetadata>()),
            )
        };
        Ok(unsafe {
            ManuallyDrop::new(CyclicBoundedVec::from_raw_parts(
                meta,
                read_array_like_ptr_at(&*account_data, start_offset, capacity),
            ))
        })
    }

    // TODO: pull ManuallyDrop into CyclicBoundedVec
    pub fn deserialize(
        account_data: &mut [u8],
        start_offset: &mut usize,
    ) -> Result<ManuallyDrop<CyclicBoundedVec<T>>, BoundedVecError> {
        unsafe {
            if account_data.len().saturating_sub(*start_offset)
                < std::mem::size_of::<CyclicBoundedVecMetadata>()
            {
                return Err(BoundedVecError::InsufficientMemoryAllocated(
                    account_data.len().saturating_sub(*start_offset),
                    std::mem::size_of::<CyclicBoundedVecMetadata>(),
                ));
            }

            let metadata: *mut CyclicBoundedVecMetadata = read_ptr_at(account_data, start_offset);
            if ((*metadata).capacity() * std::mem::size_of::<T>()) % 8 != 0 {
                return Err(BoundedVecError::UnalignedMemory(
                    (*metadata).capacity() * std::mem::size_of::<T>(),
                ));
            }

            let full_vector_size = std::mem::size_of::<BoundedVecMetadata>()
                + ((*metadata).capacity() * std::mem::size_of::<T>());
            if account_data.len().saturating_sub(*start_offset) < full_vector_size {
                return Err(BoundedVecError::InsufficientMemoryAllocated(
                    account_data.len().saturating_sub(*start_offset),
                    full_vector_size,
                ));
            }

            Ok(ManuallyDrop::new(CyclicBoundedVec::from_raw_parts(
                metadata,
                read_array_like_ptr_at(account_data, start_offset, (*metadata).capacity()),
            )))
        }
    }
}

impl<T> fmt::Debug for CyclicBoundedVec<T>
where
    T: Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

impl<T> Drop for CyclicBoundedVec<T>
where
    T: Clone,
{
    fn drop(&mut self) {
        let layout = Layout::array::<T>(self.capacity()).unwrap();
        unsafe { alloc::dealloc(self.data.as_ptr() as *mut u8, layout) };

        let layout = Layout::new::<CyclicBoundedVecMetadata>();
        unsafe { alloc::dealloc(self.metadata as *mut u8, layout) };
    }
}

impl<T> Index<usize> for CyclicBoundedVec<T>
where
    T: Clone,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<T> IndexMut<usize> for CyclicBoundedVec<T>
where
    T: Clone,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl<T> PartialEq for CyclicBoundedVec<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<T> Eq for CyclicBoundedVec<T> where T: Clone + Eq {}

pub struct CyclicBoundedVecIterator<'a, T>
where
    T: Clone,
{
    vec: &'a CyclicBoundedVec<T>,
    current: usize,
    is_finished: bool,
}

impl<'a, T> Iterator for CyclicBoundedVecIterator<'a, T>
where
    T: Clone,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.vec.capacity() == 0 || self.is_finished {
            None
        } else {
            if self.current == self.vec.last_index() {
                self.is_finished = true;
            }
            let new_current = (self.current + 1) % self.vec.capacity();
            let element = self.vec.get(self.current);
            self.current = new_current;
            element
        }
    }
}

#[cfg(test)]
mod test {
    use std::array;

    use rand::{
        distributions::{Distribution, Standard},
        thread_rng, Rng,
    };

    use super::*;

    use rand::distributions::uniform::{SampleRange, SampleUniform};

    /// Generates a random value in the given range, excluding the values provided
    /// in `exclude`.
    fn gen_range_exclude<N, R, T>(rng: &mut N, range: R, exclude: &[T]) -> T
    where
        N: Rng,
        R: Clone + SampleRange<T>,
        T: PartialEq + SampleUniform,
    {
        loop {
            // This utility is supposed to be used only in unit tests. This `clone`
            // is harmless and necessary (can't pass a reference to range, it has
            // to be moved).
            let sample = rng.gen_range(range.clone());
            if !exclude.contains(&sample) {
                return sample;
            }
        }
    }

    #[test]
    fn test_gen_range_exclude() {
        let mut rng = thread_rng();

        for n_excluded in 1..100 {
            let excluded: Vec<u64> = (0..n_excluded).map(|_| rng.gen_range(0..100)).collect();

            for _ in 0..10_000 {
                let sample = gen_range_exclude(&mut rng, 0..100, excluded.as_slice());
                for excluded in excluded.iter() {
                    assert_ne!(&sample, excluded);
                }
            }
        }
    }

    fn rand_bounded_vec<T>() -> BoundedVec<T>
    where
        T: Clone,
        Standard: Distribution<T>,
    {
        let mut rng = rand::thread_rng();

        let capacity = rng.gen_range(1..1000);
        let length = rng.gen_range(0..capacity);

        let mut bounded_vec = BoundedVec::<T>::with_capacity(capacity);
        for _ in 0..length {
            let element = rng.gen();
            bounded_vec.push(element).unwrap();
        }

        bounded_vec
    }

    #[test]
    fn test_bounded_vec_metadata_serialization() {
        let mut rng = thread_rng();

        for _ in 0..1000 {
            let capacity = rng.gen();
            let metadata = BoundedVecMetadata::new(capacity);

            assert_eq!(metadata.capacity(), capacity);
            assert_eq!(metadata.length(), 0);

            let bytes = metadata.to_le_bytes();
            let metadata_2 = BoundedVecMetadata::from_le_bytes(bytes);

            assert_eq!(metadata, metadata_2);
        }
    }

    #[test]
    fn test_bounded_vec_with_capacity() {
        for capacity in 0..1024 {
            let bounded_vec = BoundedVec::<u32>::with_capacity(capacity);

            assert_eq!(bounded_vec.capacity(), capacity);
            assert_eq!(bounded_vec.len(), 0);
        }
    }

    fn bounded_vec_from_array<const N: usize>() {
        let mut rng = thread_rng();

        let arr: [u64; N] = array::from_fn(|_| rng.gen());
        let vec = BoundedVec::from_array(&arr);

        assert_eq!(&arr, vec.as_slice());
    }

    #[test]
    fn test_bounded_vec_from_array_256() {
        bounded_vec_from_array::<256>()
    }

    #[test]
    fn test_bounded_vec_from_array_512() {
        bounded_vec_from_array::<512>()
    }

    #[test]
    fn test_bounded_vec_from_array_1024() {
        bounded_vec_from_array::<1024>()
    }

    #[test]
    fn test_bounded_vec_from_slice() {
        let mut rng = thread_rng();

        for capacity in 0..10_000 {
            let vec: Vec<u64> = (0..capacity).map(|_| rng.gen()).collect();
            let bounded_vec = BoundedVec::from_slice(&vec);

            assert_eq!(vec.as_slice(), bounded_vec.as_slice());
        }
    }

    #[test]
    fn test_bounded_vec_is_empty() {
        let mut rng = thread_rng();
        let mut vec = BoundedVec::with_capacity(1000);

        assert!(vec.is_empty());

        for _ in 0..1000 {
            let element: u64 = rng.gen();
            vec.push(element).unwrap();

            assert!(!vec.is_empty());
        }
    }

    #[test]
    fn test_bounded_vec_get() {
        let mut vec = BoundedVec::with_capacity(1000);

        for i in 0..1000 {
            assert!(vec.get(i).is_none());
            vec.push(i).unwrap();
        }

        for i in 0..1000 {
            assert_eq!(vec.get(i), Some(&i));
        }
        for i in 1000..10_000 {
            assert!(vec.get(i).is_none());
        }
    }

    #[test]
    fn test_bounded_vec_get_mut() {
        let mut vec = BoundedVec::with_capacity(1000);

        for i in 0..1000 {
            assert!(vec.get_mut(i).is_none());
            vec.push(i).unwrap();
        }

        for i in 0..1000 {
            let element = vec.get_mut(i).unwrap();
            assert_eq!(element, &i);
            *element = i * 2;
        }
        for i in 0..1000 {
            assert_eq!(vec.get_mut(i), Some(&mut (i * 2)));
        }
        for i in 1000..10_000 {
            assert!(vec.get_mut(i).is_none());
        }
    }

    #[test]
    fn test_bounded_vec_iter_mut() {
        let mut vec = BoundedVec::with_capacity(1000);

        for i in 0..1000 {
            vec.push(i).unwrap();
        }

        for (i, element) in vec.iter().enumerate() {
            assert_eq!(*element, i);
        }

        for element in vec.iter_mut() {
            *element = *element * 2;
        }

        for (i, element) in vec.iter().enumerate() {
            assert_eq!(*element, i * 2);
        }
    }

    #[test]
    fn test_bounded_vec_last() {
        let mut rng = thread_rng();
        let mut vec = BoundedVec::with_capacity(1000);

        assert!(vec.last().is_none());

        for _ in 0..1000 {
            let element: u64 = rng.gen();
            vec.push(element).unwrap();

            assert_eq!(vec.last(), Some(&element));
        }
    }

    #[test]
    fn test_bounded_vec_last_mut() {
        let mut rng = thread_rng();
        let mut vec = BoundedVec::with_capacity(1000);

        assert!(vec.last_mut().is_none());

        for _ in 0..1000 {
            let element_old: u64 = rng.gen();
            vec.push(element_old).unwrap();

            let element_ref = vec.last_mut().unwrap();
            assert_eq!(*element_ref, element_old);

            // Assign a new value.
            let element_new: u64 = rng.gen();
            *element_ref = element_new;

            // Assert that it took the effect.
            let element_ref = vec.last_mut().unwrap();
            assert_eq!(*element_ref, element_new);
        }
    }

    #[test]
    fn test_bounded_vec_to_array() {
        let vec = BoundedVec::from_array(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let arr: [u32; 16] = vec.to_array().unwrap();
        assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

        assert!(matches!(
            vec.to_array::<15>(),
            Err(BoundedVecError::ArraySize(_, _))
        ));
        assert!(matches!(
            vec.to_array::<17>(),
            Err(BoundedVecError::ArraySize(_, _))
        ));
    }

    #[test]
    fn test_bounded_vec_to_vec() {
        let mut rng = thread_rng();

        for capacity in (0..10_000).step_by(100) {
            let vec_1: Vec<u64> = (0..capacity).map(|_| rng.gen()).collect();
            let bounded_vec = BoundedVec::from_slice(&vec_1);
            let vec_2 = bounded_vec.to_vec();

            assert_eq!(vec_1.as_slice(), vec_2.as_slice());
        }
    }

    #[test]
    fn test_bounded_vec_extend() {
        let mut rng = thread_rng();

        for capacity in (1..10_000).step_by(100) {
            let length = rng.gen_range(0..capacity);

            let mut vec = BoundedVec::with_capacity(capacity);
            vec.extend(0..length).unwrap();

            assert_eq!(vec.capacity(), capacity);
            assert_eq!(vec.len(), length);
            for (element_1, element_2) in vec.iter().zip(0..length) {
                assert_eq!(*element_1, element_2);
            }
        }
    }

    #[test]
    fn test_bounded_vec_clone() {
        for _ in 0..1000 {
            let bounded_vec = rand_bounded_vec::<u32>();
            let cloned_bounded_vec = bounded_vec.clone();

            assert_eq!(bounded_vec.capacity(), cloned_bounded_vec.capacity());
            assert_eq!(bounded_vec.len(), cloned_bounded_vec.len());
            assert_eq!(bounded_vec, cloned_bounded_vec);
        }
    }

    #[test]
    fn test_bounded_vec_index() {
        let mut vec = BoundedVec::with_capacity(1000);
        for i in 0..1000 {
            vec.push(i).unwrap();
        }

        for i in 0..1000 {
            assert_eq!(vec[i], i);
        }

        for i in 0..1000 {
            vec[i] = i * 2;
        }

        for i in 0..1000 {
            assert_eq!(vec[i], i * 2);
        }
    }

    #[test]
    fn test_bounded_vec_into_iter() {
        let mut vec = BoundedVec::with_capacity(1000);
        for i in 0..1000 {
            vec.push(i).unwrap();
        }

        for (i, element) in vec.into_iter().enumerate() {
            assert_eq!(element, i);
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_metadata_serialization() {
        let mut rng = thread_rng();

        for _ in 0..1000 {
            let capacity = rng.gen();
            let metadata = CyclicBoundedVecMetadata::new(capacity);

            assert_eq!(metadata.capacity(), capacity);
            assert_eq!(metadata.length(), 0);

            let bytes = metadata.to_le_bytes();
            let metadata_2 = CyclicBoundedVecMetadata::from_le_bytes(bytes);

            assert_eq!(metadata, metadata_2);
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_with_capacity() {
        for capacity in 0..1024 {
            let cyclic_bounded_vec = CyclicBoundedVec::<u32>::with_capacity(capacity);

            assert_eq!(cyclic_bounded_vec.capacity(), capacity);
            assert_eq!(cyclic_bounded_vec.len(), 0);
            assert_eq!(cyclic_bounded_vec.first_index(), 0);
            assert_eq!(cyclic_bounded_vec.last_index(), 0);
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_is_empty() {
        let mut rng = thread_rng();
        let mut vec = CyclicBoundedVec::with_capacity(1000);

        assert!(vec.is_empty());

        for _ in 0..1000 {
            let element: u64 = rng.gen();
            vec.push(element);

            assert!(!vec.is_empty());
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_get() {
        let mut vec = CyclicBoundedVec::with_capacity(1000);

        for i in 0..1000 {
            vec.push(i);
        }

        for i in 0..1000 {
            assert_eq!(vec.get(i), Some(&i));
        }
        for i in 1000..10_000 {
            assert!(vec.get(i).is_none());
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_get_mut() {
        let mut vec = CyclicBoundedVec::with_capacity(1000);

        for i in 0..2000 {
            vec.push(i);
        }

        for i in 0..1000 {
            let element = vec.get_mut(i).unwrap();
            assert_eq!(*element, 1000 + i);
            *element = i * 2;
        }
        for i in 0..1000 {
            assert_eq!(vec.get_mut(i), Some(&mut (i * 2)));
        }
        for i in 1000..10_000 {
            assert!(vec.get_mut(i).is_none());
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_first() {
        let mut vec = CyclicBoundedVec::with_capacity(500);

        assert!(vec.first().is_none());

        for i in 0..1000 {
            vec.push(i);
            assert_eq!(vec.first(), Some(&((i as u64).saturating_sub(499))));
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_last() {
        let mut rng = thread_rng();
        let mut vec = CyclicBoundedVec::with_capacity(500);

        assert!(vec.last().is_none());

        for _ in 0..1000 {
            let element: u64 = rng.gen();
            vec.push(element);

            assert_eq!(vec.last(), Some(&element));
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_last_mut() {
        let mut rng = thread_rng();
        let mut vec = CyclicBoundedVec::with_capacity(500);

        assert!(vec.last_mut().is_none());

        for _ in 0..1000 {
            let element_old: u64 = rng.gen();
            vec.push(element_old);

            let element_ref = vec.last_mut().unwrap();
            assert_eq!(*element_ref, element_old);

            // Assign a new value.
            let element_new: u64 = rng.gen();
            *element_ref = element_new;

            // Assert that it took the effect.
            let element_ref = vec.last_mut().unwrap();
            assert_eq!(*element_ref, element_new);
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_manual() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        // Fill up the cyclic vector.
        //
        // ```
        //        ^                    $
        // index [0, 1, 2, 3, 4, 5, 6, 7]
        // value [0, 1, 2, 3, 4, 5, 6, 7]
        // ```
        //
        // * `^` - first element
        // * `$` - last element
        for i in 0..8 {
            cyclic_bounded_vec.push(i);
        }
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 7);
        assert_eq!(
            cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
            &[&0, &1, &2, &3, &4, &5, &6, &7]
        );

        // Overwrite half of values.
        //
        // ```
        //                   $  ^
        // index [0, 1,  2,  3, 4, 5, 6, 7]
        // value [8, 9, 10, 11, 4, 5, 6, 7]
        // ```
        //
        // * `^` - first element
        // * `$` - last element
        for i in 0..4 {
            cyclic_bounded_vec.push(i + 8);
        }
        assert_eq!(cyclic_bounded_vec.first_index(), 4);
        assert_eq!(cyclic_bounded_vec.last_index(), 3);
        assert_eq!(
            cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
            &[&4, &5, &6, &7, &8, &9, &10, &11]
        );

        // Overwrite even more.
        //
        // ```
        //                           $  ^
        // index [0, 1,  2,  3,  4,  5, 6, 7]
        // value [8, 9, 10, 11, 12, 13, 6, 7]
        // ```
        //
        // * `^` - first element
        // * `$` - last element
        for i in 0..2 {
            cyclic_bounded_vec.push(i + 12);
        }
        assert_eq!(cyclic_bounded_vec.first_index(), 6);
        assert_eq!(cyclic_bounded_vec.last_index(), 5);
        assert_eq!(
            cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
            &[&6, &7, &8, &9, &10, &11, &12, &13]
        );

        // Overwrite all values from the first loop.
        //
        // ```
        //        ^                          $
        // index [0, 1,  2,  3,  4,  5,  6,  7]
        // value [8, 9, 10, 11, 12, 13, 14, 15]
        // ```
        //
        // * `^` - first element
        // * `$` - last element
        for i in 0..2 {
            cyclic_bounded_vec.push(i + 14);
        }
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 7);
        assert_eq!(
            cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
            &[&8, &9, &10, &11, &12, &13, &14, &15]
        );
    }

    /// Iteration on a vector with one element.
    ///
    /// ```
    ///        ^$
    /// index [0]
    /// value [0]
    /// ```
    ///
    /// * `^` - first element
    /// * `$` - last element
    /// * `#` - visited elements
    ///
    /// Length: 1
    /// Capacity: 8
    /// First index: 0
    /// Last index: 0
    ///
    /// Start iteration from: 0
    ///
    /// Should iterate over one element.
    #[test]
    fn test_cyclic_bounded_vec_iter_one_element() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);
        cyclic_bounded_vec.push(0);

        assert_eq!(cyclic_bounded_vec.len(), 1);
        assert_eq!(cyclic_bounded_vec.capacity(), 8);
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 0);

        let elements = cyclic_bounded_vec.iter().collect::<Vec<_>>();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements.as_slice(), &[&0]);

        let elements = cyclic_bounded_vec.iter_from(0).unwrap().collect::<Vec<_>>();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements.as_slice(), &[&0]);
    }

    /// Iteration without reset in a vector which is not full.
    ///
    /// ```
    ///              #  #  #  #
    ///        ^              $
    /// index [0, 1, 2, 3, 4, 5]
    /// value [0, 1, 2, 3, 4, 5]
    /// ```
    ///
    /// * `^` - first element
    /// * `$` - last element
    /// * `#` - visited elements
    ///
    /// Length: 6
    /// Capacity: 8
    /// First index: 0
    /// Last index: 5
    ///
    /// Start iteration from: 2
    ///
    /// Should iterate over elements from 2 to 5, with 4 iterations.
    #[test]
    fn test_cyclic_bounded_vec_iter_from_without_reset_not_full_6_8_4() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..6 {
            cyclic_bounded_vec.push(i);
        }

        assert_eq!(cyclic_bounded_vec.len(), 6);
        assert_eq!(cyclic_bounded_vec.capacity(), 8);
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 5);

        let elements = cyclic_bounded_vec.iter_from(2).unwrap().collect::<Vec<_>>();
        assert_eq!(elements.len(), 4);
        assert_eq!(elements.as_slice(), &[&2, &3, &4, &5]);
    }
    /// Iteration without reset in a vector which is full.
    ///
    /// ```
    ///              #  #  #
    ///        ^           $
    /// index [0, 1, 2, 3, 4]
    /// value [0, 1, 2, 3, 4]
    /// ```
    ///
    /// * `^` - first element
    /// * `$` - last element
    /// * `#` - visited elements
    ///
    /// Length: 5
    /// Capacity: 5
    /// First index: 0
    /// Last index: 4
    ///
    /// Start iteration from: 2
    ///
    /// Should iterate over elements 2..4 - 3 iterations.
    #[test]
    fn test_cyclic_bounded_vec_iter_from_without_reset_not_full_5_5_4() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(5);

        for i in 0..5 {
            cyclic_bounded_vec.push(i);
        }

        assert_eq!(cyclic_bounded_vec.len(), 5);
        assert_eq!(cyclic_bounded_vec.capacity(), 5);
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 4);

        let elements = cyclic_bounded_vec.iter_from(2).unwrap().collect::<Vec<_>>();
        assert_eq!(elements.len(), 3);
        assert_eq!(elements.as_slice(), &[&2, &3, &4]);
    }

    /// Iteration without reset in a vector which is full.
    ///
    /// ```
    ///              #  #  #  #  #  #
    ///        ^                    $
    /// index [0, 1, 2, 3, 4, 5, 6, 7]
    /// value [0, 1, 2, 3, 4, 5, 6, 7]
    /// ```
    ///
    /// * `^` - first element
    /// * `$` - last element
    /// * `#` - visited elements
    ///
    /// Length: 8
    /// Capacity: 8
    /// First index: 0
    /// Last index: 7
    ///
    /// Start iteration from: 2
    ///
    /// Should iterate over elements 2..7 - 6 iterations.
    #[test]
    fn test_cyclic_bounded_vec_iter_from_without_reset_full_8_8_6() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..8 {
            cyclic_bounded_vec.push(i);
        }

        assert_eq!(cyclic_bounded_vec.len(), 8);
        assert_eq!(cyclic_bounded_vec.capacity(), 8);
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 7);

        let elements = cyclic_bounded_vec.iter_from(2).unwrap().collect::<Vec<_>>();
        assert_eq!(elements.len(), 6);
        assert_eq!(elements.as_slice(), &[&2, &3, &4, &5, &6, &7]);
    }

    /// Iteration with reset.
    ///
    /// Insert elements over capacity, so the vector resets and starts
    /// overwriting elements from the start - 12 elements into a vector with
    /// capacity 8.
    ///
    /// The resulting data structure looks like:
    ///
    /// ```
    ///        #  #   #   #        #  #
    ///                   $  ^
    /// index [0, 1,  2,  3, 4, 5, 6, 7]
    /// value [8, 9, 10, 11, 4, 5, 6, 7]
    /// ```
    ///
    /// * `^` - first element
    /// * `$` - last element
    /// * `#` - visited elements
    ///
    /// Length: 8
    /// Capacity: 8
    /// First: 4
    /// Last: 3
    ///
    /// Start iteration from: 6
    ///
    /// Should iterate over elements 6..7 and 8..11 - 6 iterations.
    #[test]
    fn test_cyclic_bounded_vec_iter_from_reset() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..12 {
            cyclic_bounded_vec.push(i);
        }

        assert_eq!(cyclic_bounded_vec.len(), 8);
        assert_eq!(cyclic_bounded_vec.capacity(), 8);
        assert_eq!(cyclic_bounded_vec.first_index(), 4);
        assert_eq!(cyclic_bounded_vec.last_index(), 3);

        let elements = cyclic_bounded_vec.iter_from(6).unwrap().collect::<Vec<_>>();
        assert_eq!(elements.len(), 6);
        assert_eq!(elements.as_slice(), &[&6, &7, &8, &9, &10, &11]);
    }

    #[test]
    fn test_cyclic_bounded_vec_iter_from_out_of_bounds_not_full() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..4 {
            cyclic_bounded_vec.push(i);
        }

        // Try `start` values in bounds.
        for i in 0..4 {
            let elements = cyclic_bounded_vec.iter_from(i).unwrap().collect::<Vec<_>>();
            assert_eq!(elements.len(), 4 - i);
            let expected = (i..4).collect::<Vec<_>>();
            // Just to coerce it to have references...
            let expected = expected.iter().collect::<Vec<_>>();
            assert_eq!(elements.as_slice(), expected.as_slice());
        }

        // Try `start` values out of bounds.
        for i in 4..1000 {
            let elements = cyclic_bounded_vec.iter_from(i);
            assert!(matches!(
                elements,
                Err(BoundedVecError::IterFromOutOfBounds)
            ));
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_iter_from_out_of_bounds_full() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..12 {
            cyclic_bounded_vec.push(i);
        }

        // Try different `start` values which are out of bounds.
        for start in 8..1000 {
            let elements = cyclic_bounded_vec.iter_from(start);
            assert!(matches!(
                elements,
                Err(BoundedVecError::IterFromOutOfBounds)
            ));
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_iter_from_out_of_bounds_iter_from() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..8 {
            assert!(matches!(
                cyclic_bounded_vec.iter_from(i),
                Err(BoundedVecError::IterFromOutOfBounds)
            ));
            cyclic_bounded_vec.push(i);
        }
    }

    #[test]
    fn test_cyclic_bounded_vec_overwrite() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(64);

        for i in 0..256 {
            cyclic_bounded_vec.push(i);
        }

        assert_eq!(cyclic_bounded_vec.len(), 64);
        assert_eq!(cyclic_bounded_vec.capacity(), 64);
        assert_eq!(
            cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
            &[
                &192, &193, &194, &195, &196, &197, &198, &199, &200, &201, &202, &203, &204, &205,
                &206, &207, &208, &209, &210, &211, &212, &213, &214, &215, &216, &217, &218, &219,
                &220, &221, &222, &223, &224, &225, &226, &227, &228, &229, &230, &231, &232, &233,
                &234, &235, &236, &237, &238, &239, &240, &241, &242, &243, &244, &245, &246, &247,
                &248, &249, &250, &251, &252, &253, &254, &255
            ]
        );
    }

    #[test]
    fn test_clear_pass() {
        let mut vec = BoundedVec::with_capacity(5);
        vec.push(1).unwrap();
        vec.push(2).unwrap();
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert!(vec.get(0).is_none());
        assert!(vec.get(1).is_none());
    }

    #[test]
    fn test_clear_fail() {
        let mut vec = BoundedVec::with_capacity(5);
        vec.push(1).unwrap();
        assert_eq!(vec.get(0).unwrap(), &1);
        vec.clear();
        assert_eq!(vec.get(0), None);
    }

    #[test]
    fn test_deserialize_pass() {
        let mut account_data = vec![0u8; 64];
        let mut start_offset = 0;

        // Initialize data with valid BoundedVec metadata and elements
        BoundedVec::<u64>::init(4, &mut account_data, &mut start_offset, false).unwrap();
        start_offset = 0;

        // Deserialize the BoundedVec
        let deserialized_vec = BoundedVec::<u64>::deserialize(&mut account_data, &mut start_offset)
            .expect("Failed to deserialize BoundedVec");

        assert_eq!(deserialized_vec.metadata().capacity(), 4);
        assert_eq!(deserialized_vec.metadata().length(), 0);
    }

    #[test]
    fn test_deserialize_fail_unaligned_memory() {
        let mut account_data = vec![0u8; 64];
        let mut start_offset = 0;
        BoundedVec::<u64>::init(4, &mut account_data, &mut start_offset, false).unwrap();
        let mut start_offset = 0;

        let result =
            BoundedVec::<UnalignedStruct>::deserialize(&mut account_data, &mut start_offset);
        assert!(matches!(result, Err(BoundedVecError::UnalignedMemory(_))));
    }

    #[test]
    fn test_deserialize_multiple_pass() {
        let mut account_data = vec![0u8; 128];
        let mut start_offset = 0;

        // Initialize data for multiple BoundedVecs
        BoundedVec::<u64>::init(4, &mut account_data, &mut start_offset, false).unwrap();
        BoundedVec::<u64>::init(4, &mut account_data, &mut start_offset, false).unwrap();
        start_offset = 0;

        // Deserialize multiple BoundedVecs
        let deserialized_vecs =
            BoundedVec::<u64>::deserialize_multiple(2, &mut account_data, &mut start_offset)
                .expect("Failed to deserialize multiple BoundedVecs");

        assert_eq!(deserialized_vecs.len(), 2);
    }

    #[test]
    fn test_init_pass() {
        let mut account_data = vec![0u8; 64];
        let mut start_offset = 0;

        // Initialize a BoundedVec with capacity 4
        let mut vec = BoundedVec::<u64>::init(4, &mut account_data, &mut start_offset, false)
            .expect("Failed to initialize BoundedVec");

        assert_eq!(vec.metadata().capacity(), 4);
        assert_eq!(vec.metadata().length(), 0);
        for i in 0..4 {
            assert!(vec.get(i).is_none());
            vec.push(i as u64).unwrap();
            assert_eq!(*vec.get(i).unwrap(), i as u64);
            assert!(vec.metadata().length() == i + 1);
        }
    }

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    pub struct UnalignedStruct {
        pub field: u8,
    }

    #[test]
    fn test_init_fail_unaligned_memory() {
        let mut account_data = vec![0u8; 64];
        let mut start_offset = 0;

        // Attempt to initialize a BoundedVec with unaligned capacity
        let result =
            BoundedVec::<UnalignedStruct>::init(3, &mut account_data, &mut start_offset, false);
        assert!(matches!(result, Err(BoundedVecError::UnalignedMemory(_))));
    }

    #[test]
    fn test_init_multiple_pass() {
        let mut account_data = vec![0u8; 128];
        let mut start_offset = 0;
        let mut initialized_vecs =
            BoundedVec::<u64>::init_multiple(2, 4, &mut account_data, &mut start_offset, false)
                .expect("Failed to initialize multiple BoundedVecs");

        assert_eq!(initialized_vecs.len(), 2);
        assert_eq!(initialized_vecs[0].metadata().capacity(), 4);
        assert_eq!(initialized_vecs[1].metadata().capacity(), 4);
        assert_eq!(initialized_vecs[0].metadata().length(), 0);
        assert_eq!(initialized_vecs[1].metadata().length(), 0);
        for i in 0..4 {
            for vec in initialized_vecs.iter_mut() {
                assert!(vec.get(i).is_none());
                vec.push(i as u64).unwrap();
                assert_eq!(*vec.get(i).unwrap(), i as u64);
                assert!(vec.metadata().length() == i + 1);
            }
        }
    }

    #[test]
    fn test_init_multiple_fail_unaligned_memory() {
        let mut account_data = vec![0u8; 128];
        let mut start_offset = 0;

        let result = BoundedVec::<UnalignedStruct>::init_multiple(
            2,
            3,
            &mut account_data,
            &mut start_offset,
            false,
        );
        assert!(matches!(result, Err(BoundedVecError::UnalignedMemory(_))));
    }

    #[test]
    fn test_insufficient_memory_deserialize_metadata() {
        let required_memory = mem::size_of::<BoundedVecMetadata>();
        let mut account_data = vec![0u8; required_memory - 1];
        let mut start_offset = 0;

        let result = BoundedVec::<u64>::deserialize(&mut account_data, &mut start_offset);
        assert!(matches!(
            result,
            Err(BoundedVecError::InsufficientMemoryAllocated(_, expected_memory
            )) if expected_memory == required_memory
        ));
    }

    #[test]
    fn test_insufficient_memory_deserialize_full_vector() {
        let required_memory = mem::size_of::<BoundedVecMetadata>() + 4 * mem::size_of::<u64>();
        let mut account_data = vec![0u8; required_memory];
        BoundedVec::<u64>::init(4, &mut account_data, &mut 0, false).unwrap();
        let mut account_data = account_data[0..required_memory - 1].to_vec();
        let mut start_offset = 0;

        let result = BoundedVec::<u64>::deserialize(&mut account_data, &mut start_offset);
        assert!(matches!(
            result,
            Err(BoundedVecError::InsufficientMemoryAllocated(_, expected_memory
            )) if expected_memory == required_memory
        ));
    }

    #[test]
    fn test_insufficient_memory_init_single() {
        let required_memory = mem::size_of::<BoundedVecMetadata>() + 4 * mem::size_of::<u64>();
        let mut account_data = vec![0u8; required_memory - 1];
        let mut start_offset = 0;
        let result = BoundedVec::<u64>::init(4, &mut account_data, &mut start_offset, false);
        assert!(matches!(
            result,
            Err(BoundedVecError::InsufficientMemoryAllocated(_, expected_memory
            )) if expected_memory == required_memory
        ));
    }

    #[test]
    fn test_insufficient_memory_deserialize_multiple() {
        let required_memory =
            2 * (mem::size_of::<BoundedVecMetadata>() + 3 * mem::size_of::<u64>());
        let mut account_data = vec![0u8; required_memory];
        BoundedVec::<u64>::init_multiple(2, 3, &mut account_data, &mut 0, false).unwrap();
        let mut account_data = account_data[0..required_memory - 1].to_vec();
        let mut start_offset = 0;

        let result =
            BoundedVec::<u64>::deserialize_multiple(2, &mut account_data, &mut start_offset);
        let required_memory_per_vec = required_memory / 2;
        assert!(matches!(
            result,
            Err(BoundedVecError::InsufficientMemoryAllocated(_, expected_memory
            )) if expected_memory == required_memory_per_vec
        ));
    }

    #[test]
    fn test_insufficient_memory_init_multiple() {
        let required_memory =
            2 * (mem::size_of::<BoundedVecMetadata>() + 3 * mem::size_of::<u64>());
        let mut account_data = vec![0u8; required_memory - 1];

        let result = BoundedVec::<u64>::init_multiple(2, 3, &mut account_data, &mut 0, false);
        let required_memory_per_vec = required_memory / 2;
        assert!(matches!(
            result,
            Err(BoundedVecError::InsufficientMemoryAllocated(
                _,
                expected_memory
            )) if expected_memory == required_memory_per_vec
        ));
    }
}
