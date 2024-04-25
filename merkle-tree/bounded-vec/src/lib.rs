use std::{
    alloc::{self, handle_alloc_error, Layout},
    fmt, mem,
    ops::{Index, IndexMut},
    ptr::NonNull,
    slice::{self, SliceIndex},
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BoundedVecError {
    #[error("The vector is full, cannot push any new elements")]
    Full,
    #[error("Requested array of size {0}, but the vector has {1} elements")]
    ArraySize(usize, usize),
}

#[cfg(feature = "solana")]
impl From<BoundedVecError> for u32 {
    fn from(e: BoundedVecError) -> u32 {
        match e {
            BoundedVecError::Full => 5001,
            BoundedVecError::ArraySize(_, _) => 5002,
        }
    }
}

#[cfg(feature = "solana")]
impl From<BoundedVecError> for solana_program::program_error::ProgramError {
    fn from(e: BoundedVecError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

/// `BoundedVec` is a custom vector implementation which:
///
/// * Forbids post-initialization reallocations. The size is not known during
///   compile time (that makes it different from arrays), but can be defined
///   only once (that makes it different from [`Vec`](std::vec::Vec)).
/// * Can store only Plain Old Data ([`Pod`](bytemuck::Pod)). It cannot nest
///   any other dynamically sized types.
pub struct BoundedVec<T>
where
    T: Clone,
{
    capacity: usize,
    length: usize,
    data: NonNull<T>,
}

impl<T> BoundedVec<T>
where
    T: Clone,
{
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let size = mem::size_of::<T>() * capacity;
        let align = mem::align_of::<T>();
        // SAFETY: `size` is a multiplication of `capacity`, therefore the
        // layout is guaranteed to be aligned.
        let layout = unsafe { Layout::from_size_align_unchecked(size, align) };

        // SAFETY: As long as the provided type is correct, this global
        // allocator call should be correct too.
        //
        // We are handling the null pointer case gracefully.
        let ptr = unsafe { alloc::alloc(layout) as *mut T };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        // PANICS: Should not panic as long as the layout is correct.
        let data = NonNull::new(ptr).unwrap();

        Self {
            capacity,
            length: 0,
            data,
        }
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

    /// Creates a `BoundedVec<T>` directly from a pointer, a capacity, and a length.
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
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Self {
        let data = NonNull::new_unchecked(ptr);
        Self {
            capacity,
            length,
            data,
        }
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
        self.capacity
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The `from_raw_parts` method is safe to use here because:
        // * The pointer `self.data` is guaranteed to be non-null and
        //   correctly aligned, since it is managed by the `BoundedVec` and
        //   initialized in `with_capacity`.
        // * `self.length` elements have been properly initialized (or none if
        //   `length` is 0), so it is safe to create a slice up to `self.length`.
        // * `self.length` is guaranteed to be <= `self.capacity`, hence
        //   within the allocated memory.
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.length) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: The `from_raw_parts` method is safe to use here because:
        // * The pointer `self.data` is guaranteed to be non-null and
        //   correctly aligned, since it is managed by the `BoundedVec` and
        //   initialized in `with_capacity`.
        // * `self.length` elements have been properly initialized (or none if
        //   `length` is 0), so it is safe to create a slice up to `self.length`.
        // * `self.length` is guaranteed to be <= `self.capacity`, hence
        //   within the allocated memory.
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.length) }
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
        if self.length == self.capacity {
            return Err(BoundedVecError::Full);
        }

        unsafe { *self.data.as_ptr().add(self.length) = value };
        self.length += 1;

        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.length {
            let element = unsafe { &*self.data.as_ptr().add(index) };
            Some(element)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.length {
            let element = unsafe { &mut *self.data.as_ptr().add(index) };
            Some(element)
        } else {
            None
        }
    }

    #[inline]
    pub fn iter(&self) -> BoundedVecIterator<T> {
        BoundedVecIterator {
            bounded_vec: self,
            current: 0,
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> BoundedVecIteratorMut<T> {
        BoundedVecIteratorMut {
            bounded_vec: self,
            current: 0,
        }
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        if self.length < 1 {
            return None;
        }
        self.get(self.length - 1)
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.length < 1 {
            return None;
        }
        self.get_mut(self.length - 1)
    }

    pub fn to_array<const N: usize>(self) -> Result<[T; N], BoundedVecError> {
        if self.len() != N {
            return Err(BoundedVecError::ArraySize(N, self.len()));
        }
        // SAFETY: We ensure the bounds of this array cast.
        Ok(std::array::from_fn(|i| unsafe {
            (*self.data.as_ptr().add(i)).clone()
        }))
    }

    pub fn to_vec(self) -> Vec<T> {
        unsafe { Vec::from_raw_parts(self.data.as_ptr(), self.length, self.capacity) }
    }

    pub fn extend<U: IntoIterator<Item = T>>(&mut self, iter: U) -> Result<(), BoundedVecError> {
        for item in iter {
            self.push(item)?;
        }
        Ok(())
    }
}

impl<T> fmt::Debug for BoundedVec<T>
where
    T: Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slice = unsafe { slice::from_raw_parts(self.data.as_ptr(), self.length) };
        write!(f, "{:?}", slice)
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
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(index)
    }
}

impl<T> PartialEq for BoundedVec<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.length == other.length {
            for i in 0..self.length {
                // SAFETY: We ensure the bounds of both vectors.
                let element_1 = unsafe { &*self.data.as_ptr().add(i) };
                let element_2 = unsafe { &*other.data.as_ptr().add(i) };
                if element_1 != element_2 {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}

impl<T> Eq for BoundedVec<T> where T: Clone + Eq {}

pub struct BoundedVecIterator<'a, T>
where
    T: Clone,
{
    bounded_vec: &'a BoundedVec<T>,
    current: usize,
}

impl<'a, T> Iterator for BoundedVecIterator<'a, T>
where
    T: Clone + Eq,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.bounded_vec.length {
            let element = unsafe { &*self.bounded_vec.data.as_ptr().add(self.current) };
            self.current += 1;
            Some(element)
        } else {
            None
        }
    }
}

pub struct BoundedVecIteratorMut<'a, T>
where
    T: Clone,
{
    bounded_vec: &'a BoundedVec<T>,
    current: usize,
}

impl<'a, T> Iterator for BoundedVecIteratorMut<'a, T>
where
    T: Clone + Eq,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.bounded_vec.length {
            let element = unsafe { &mut *self.bounded_vec.data.as_ptr().add(self.current) };
            self.current += 1;
            Some(element)
        } else {
            None
        }
    }
}

/// `CyclicBoundedVec` is a wrapper around [`Vec`](std::vec::Vec) which:
///
/// * Forbids post-initialization reallocations.
/// * Starts overwriting elements from the beginning once it reaches its
///   capacity.
#[derive(Debug)]
pub struct CyclicBoundedVec<T>
where
    T: Clone,
{
    capacity: usize,
    length: usize,
    next_index: usize,
    data: NonNull<T>,
}

impl<T> CyclicBoundedVec<T>
where
    T: Clone,
{
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let size = mem::size_of::<T>() * capacity;
        let align = mem::align_of::<T>();
        // SAFETY: `size` is a multiplication of `capacity`, therefore the
        // layout is guaranteed to be aligned.
        let layout = unsafe { Layout::from_size_align_unchecked(size, align) };

        // SAFETY: As long as the provided type is correct, this global
        // allocator call should be correct too.
        //
        // We are handling the null pointer case gracefully.
        let ptr = unsafe { alloc::alloc(layout) as *mut T };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        // PANICS: Should not panic as long as the layout is correct.
        let data = NonNull::new(ptr).unwrap();

        Self {
            capacity,
            length: 0,
            next_index: 0,
            data,
        }
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
    pub unsafe fn from_raw_parts(
        ptr: *mut T,
        next_index: usize,
        length: usize,
        capacity: usize,
    ) -> Self {
        let data = NonNull::new_unchecked(ptr);
        Self {
            capacity,
            length,
            next_index,
            data,
        }
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
        self.capacity
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The `from_raw_parts` method is safe to use here because:
        // * The pointer `self.data` is guaranteed to be non-null and
        //   correctly aligned, since it is managed by the `BoundedVec` and
        //   initialized in `with_capacity`.
        // * `self.length` elements have been properly initialized (or none if
        //   `length` is 0), so it is safe to create a slice up to `self.length`.
        // * `self.length` is guaranteed to be <= `self.capacity`, hence
        //   within the allocated memory.
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.length) }
    }

    #[inline]
    pub fn as_mut_slice(&self) -> &mut [T] {
        // SAFETY: The `from_raw_parts` method is safe to use here because:
        // * The pointer `self.data` is guaranteed to be non-null and
        //   correctly aligned, since it is managed by the `BoundedVec` and
        //   initialized in `with_capacity`.
        // * `self.length` elements have been properly initialized (or none if
        //   `length` is 0), so it is safe to create a slice up to `self.length`.
        // * `self.length` is guaranteed to be <= `self.capacity`, hence
        //   within the allocated memory.
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.length) }
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
        if self.len() < self.capacity() {
            self.length += 1;
        } else if self.next_index == self.capacity() {
            self.next_index = 0;
        }
        unsafe { *self.data.as_ptr().add(self.next_index) = value };
        self.next_index += 1;

        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.length {
            let element = unsafe { &*self.data.as_ptr().add(index) };
            Some(element)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.length {
            let element = unsafe { &mut *self.data.as_ptr().add(index) };
            Some(element)
        } else {
            None
        }
    }

    #[inline]
    pub fn iter(&self) -> CyclicBoundedVecIterator<T> {
        CyclicBoundedVecIterator {
            cyclic_bounded_vec: self,
            current: 0,
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> CyclicBoundedVecIteratorMut<T> {
        CyclicBoundedVecIteratorMut {
            cyclic_bounded_vec: self,
            current: 0,
        }
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        if self.len() < self.capacity() {
            if self.is_empty() {
                return None;
            }
            self.get(self.length - 1)
        } else if self.next_index == 0 {
            self.get(self.capacity - 1)
        } else {
            self.get(self.next_index - 1)
        }
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.len() < self.capacity() {
            if self.is_empty() {
                return None;
            }
            self.get_mut(self.length - 1)
        } else if self.next_index == 0 {
            self.get_mut(self.capacity - 1)
        } else {
            self.get_mut(self.next_index - 1)
        }
    }
}

impl<T, I> Index<I> for CyclicBoundedVec<T>
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

impl<T, I> IndexMut<I> for CyclicBoundedVec<T>
where
    T: Clone,
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(index)
    }
}

impl<T> PartialEq for CyclicBoundedVec<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.length == other.length {
            for i in 0..self.length {
                // SAFETY: We ensure the bounds of both vectors.
                let element_1 = unsafe { &*self.data.as_ptr().add(i) };
                let element_2 = unsafe { &*other.data.as_ptr().add(i) };
                if element_1 != element_2 {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}

impl<T> Eq for CyclicBoundedVec<T> where T: Clone + Eq {}

pub struct CyclicBoundedVecIterator<'a, T>
where
    T: Clone,
{
    cyclic_bounded_vec: &'a CyclicBoundedVec<T>,
    current: usize,
}

impl<'a, T> Iterator for CyclicBoundedVecIterator<'a, T>
where
    T: Clone + Eq,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.cyclic_bounded_vec.length {
            let element = unsafe { &*self.cyclic_bounded_vec.data.as_ptr().add(self.current) };
            self.current += 1;
            Some(element)
        } else {
            None
        }
    }
}

pub struct CyclicBoundedVecIteratorMut<'a, T>
where
    T: Clone,
{
    cyclic_bounded_vec: &'a mut CyclicBoundedVec<T>,
    current: usize,
}

impl<'a, T> Iterator for CyclicBoundedVecIteratorMut<'a, T>
where
    T: Clone + Eq,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.cyclic_bounded_vec.length {
            let element = unsafe { &mut *self.cyclic_bounded_vec.data.as_ptr().add(self.current) };
            self.current += 1;
            Some(element)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cyclic_bounded_vec_correct_values() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..8 {
            cyclic_bounded_vec.push(i).unwrap();
        }
        assert_eq!(cyclic_bounded_vec.as_slice(), &[0, 1, 2, 3, 4, 5, 6, 7]);

        for i in 0..4 {
            cyclic_bounded_vec.push(i + 5).unwrap();
        }
        assert_eq!(cyclic_bounded_vec.as_slice(), &[5, 6, 7, 8, 4, 5, 6, 7]);
    }

    #[test]
    fn test_cyclic_bounded_vec_override() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(64);

        for i in 0..256 {
            cyclic_bounded_vec.push(i).unwrap();
        }

        assert_eq!(cyclic_bounded_vec.len(), 64);
        assert_eq!(cyclic_bounded_vec.capacity(), 64);
        assert_eq!(
            cyclic_bounded_vec.as_slice(),
            &[
                192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207,
                208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223,
                224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
                240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255
            ][..]
        );
    }
}
