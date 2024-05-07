use std::{
    fmt,
    ops::{Index, IndexMut},
    slice::{Iter, IterMut, SliceIndex},
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
            BoundedVecError::Full => 8001,
            BoundedVecError::ArraySize(_, _) => 8002,
        }
    }
}

#[cfg(feature = "solana")]
impl From<BoundedVecError> for solana_program::program_error::ProgramError {
    fn from(e: BoundedVecError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

/// `BoundedVec` is a custom vector implementation which forbids
/// post-initialization reallocations.
///
/// The purpose is an ability to set an initial limit, but:
///
/// * Still be able to define the limit on runtime, not on compile time.
/// * Allocate the memory on heap (not on stack, like arrays).
///
/// `Vec` is still used as the underlying data structure, `BoundedVec` exposes
/// only the methods which don't trigger reallocations.
#[derive(Clone)]
pub struct BoundedVec<T>(Vec<T>)
where
    T: Clone;

impl<T> BoundedVec<T>
where
    T: Clone,
{
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
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
        let vec = Vec::from_raw_parts(ptr, length, capacity);
        Self(vec)
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
        self.0.capacity()
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
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
        if self.0.len() == self.0.capacity() {
            return Err(BoundedVecError::Full);
        }
        self.0.push(value);
        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.0.get_mut(index)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.0.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.0.iter_mut()
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.0.last()
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.0.last_mut()
    }

    pub fn to_array<const N: usize>(&self) -> Result<[T; N], BoundedVecError> {
        if self.len() != N {
            return Err(BoundedVecError::ArraySize(N, self.len()));
        }
        Ok(std::array::from_fn(|i| self.0[i].clone()))
    }

    pub fn to_vec(self) -> Vec<T> {
        self.0
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
        self.0.fmt(f)
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
        self.0.index(index)
    }
}

impl<T, I> IndexMut<I> for BoundedVec<T>
where
    T: Clone,
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}

impl<T> PartialEq for BoundedVec<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T> Eq for BoundedVec<T> where T: Clone + Eq {}

/// `CyclicBoundedVec` is a wrapper around [`Vec`](std::vec::Vec) which:
///
/// * Forbids post-initialization reallocations.
/// * Starts overwriting elements from the beginning once it reaches its
///   capacity.
#[derive(Clone)]
pub struct CyclicBoundedVec<T>
where
    T: Clone,
{
    first_index: usize,
    last_index: usize,
    data: Vec<T>,
}

impl<T> CyclicBoundedVec<T>
where
    T: Clone,
{
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let data = Vec::with_capacity(capacity);
        Self {
            first_index: 0,
            last_index: 0,
            data,
        }
    }

    pub fn from_array<const N: usize>(array: &[T; N]) -> Self {
        let mut vec = Self::with_capacity(N);
        for element in array {
            // SAFETY: We are sure that the array and the vector have equal
            // sizes, there is no chance for the error to occur.
            vec.push(element.clone());
        }
        vec
    }

    pub fn from_slice(slice: &[T]) -> Self {
        let mut vec = Self::with_capacity(slice.len());
        for element in slice {
            // SAFETY: We are sure that the array and the vector have equal
            // sizes, there is no chance for the error to occur.
            vec.push(element.clone());
        }
        vec
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
        length: usize,
        capacity: usize,
        first_index: usize,
        last_index: usize,
    ) -> Self {
        let data = Vec::from_raw_parts(ptr, length, capacity);
        Self {
            first_index,
            last_index,
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
        self.data.capacity()
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
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
        if self.len() < self.capacity() {
            if !self.is_empty() {
                self.last_index += 1;
            }

            self.data.push(value);
        } else {
            self.last_index = (self.last_index + 1) % self.capacity();
            self.first_index = (self.first_index + 1) % self.capacity();

            // PANICS: We made sure that `self.newest` doesn't exceed the capacity.
            self.data[self.last_index] = value;
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    #[inline]
    pub fn iter(&self) -> CyclicBoundedVecIterator<'_, T> {
        CyclicBoundedVecIterator {
            vec: self,
            current: self.first_index,
            is_finished: false,
        }
    }

    #[inline]
    pub fn iter_from(&self, start: usize) -> CyclicBoundedVecIterator<'_, T> {
        CyclicBoundedVecIterator {
            vec: self,
            current: start,
            is_finished: false,
        }
    }

    #[inline]
    pub fn first_index(&self) -> usize {
        self.first_index
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.data.get(self.first_index)
    }

    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.data.get_mut(self.first_index)
    }

    #[inline]
    pub fn last_index(&self) -> usize {
        self.last_index
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.data.get(self.last_index)
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.data.get_mut(self.last_index)
    }
}

impl<T> fmt::Debug for CyclicBoundedVec<T>
where
    T: Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.iter().collect::<Vec<_>>().as_slice().fmt(f)
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
        self.data.index(index)
    }
}

impl<T, I> IndexMut<I> for CyclicBoundedVec<T>
where
    T: Clone,
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.data.index_mut(index)
    }
}

impl<T> PartialEq for CyclicBoundedVec<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
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
        if self.is_finished {
            None
        } else {
            if self.current == self.vec.last_index {
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
    use super::*;

    fn bounded_vec_full<const CAPACITY: usize>() {
        let mut bounded_vec = BoundedVec::with_capacity(CAPACITY);

        // Append up to capaciity.
        for i in 0..CAPACITY {
            bounded_vec.push(i).unwrap();
        }
        // Try pushing over capacity - should result in an error.
        for i in 0..CAPACITY {
            let res = bounded_vec.push(i);
            assert!(matches!(res, Err(BoundedVecError::Full)));
        }
    }

    #[test]
    fn test_bounded_vec_full_8() {
        bounded_vec_full::<8>()
    }

    #[test]
    fn test_bounded_vec_full_16() {
        bounded_vec_full::<16>()
    }

    #[test]
    fn test_bounded_vec_full_32() {
        bounded_vec_full::<32>()
    }

    #[test]
    fn test_bounded_vec_full_64() {
        bounded_vec_full::<64>()
    }

    #[test]
    fn test_bounded_vec_full_128() {
        bounded_vec_full::<128>()
    }

    #[test]
    fn test_bounded_vec_to_array() {
        let bounded_vec = BoundedVec::from_array(&[1u8; 32]);

        assert!(bounded_vec.to_array::<32>().is_ok());

        assert!(matches!(
            bounded_vec.to_array::<31>(),
            Err(BoundedVecError::ArraySize(_, _))
        ));
        assert!(matches!(
            bounded_vec.to_array::<33>(),
            Err(BoundedVecError::ArraySize(_, _))
        ));
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
        assert_eq!(cyclic_bounded_vec.first_index, 0);
        assert_eq!(cyclic_bounded_vec.last_index, 7);
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
        assert_eq!(cyclic_bounded_vec.first_index, 4);
        assert_eq!(cyclic_bounded_vec.last_index, 3);
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
        assert_eq!(cyclic_bounded_vec.first_index, 6);
        assert_eq!(cyclic_bounded_vec.last_index, 5);
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
        assert_eq!(cyclic_bounded_vec.first_index, 0);
        assert_eq!(cyclic_bounded_vec.last_index, 7);
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
        assert_eq!(cyclic_bounded_vec.first_index, 0);
        assert_eq!(cyclic_bounded_vec.last_index, 0);

        let elements = cyclic_bounded_vec.iter().collect::<Vec<_>>();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements.as_slice(), &[&0]);

        let elements = cyclic_bounded_vec.iter_from(0).collect::<Vec<_>>();
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
        assert_eq!(cyclic_bounded_vec.first_index, 0);
        assert_eq!(cyclic_bounded_vec.last_index, 5);

        let elements = cyclic_bounded_vec.iter_from(2).collect::<Vec<_>>();
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
        assert_eq!(cyclic_bounded_vec.first_index, 0);
        assert_eq!(cyclic_bounded_vec.last_index, 4);

        let elements = cyclic_bounded_vec.iter_from(2).collect::<Vec<_>>();
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
        assert_eq!(cyclic_bounded_vec.first_index, 0);
        assert_eq!(cyclic_bounded_vec.last_index, 7);

        let elements = cyclic_bounded_vec.iter_from(2).collect::<Vec<_>>();
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
    fn test_cyclic_bounded_vec_iter_reset() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..12 {
            cyclic_bounded_vec.push(i);
        }

        assert_eq!(cyclic_bounded_vec.len(), 8);
        assert_eq!(cyclic_bounded_vec.capacity(), 8);
        assert_eq!(cyclic_bounded_vec.first_index, 4);
        assert_eq!(cyclic_bounded_vec.last_index, 3);

        let elements = cyclic_bounded_vec.iter_from(6).collect::<Vec<_>>();
        assert_eq!(elements.len(), 6);
        assert_eq!(elements.as_slice(), &[&6, &7, &8, &9, &10, &11]);
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
            cyclic_bounded_vec[..],
            [
                192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207,
                208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223,
                224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
                240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255
            ][..]
        );
    }

    /// Test formatting of a cycled vector.
    ///
    /// Insert elements over capacity, so the vector resets and starts
    /// overwriting elements from the start - 12 elements into a vector with
    /// capacity 8.
    ///
    /// The resulting data structure looks like:
    ///
    /// ```
    ///                   $  ^
    /// index [0, 1,  2,  3, 4, 5, 6, 7]
    /// value [8, 9, 10, 11, 4, 5, 6, 7]
    /// ```
    ///
    /// * `^` - first element
    /// * `$` - last element
    ///
    /// The debug format of that structure should look like:
    ///
    /// ```
    /// [4, 5, 6, 7, 8, 9, 10, 11]
    /// ```
    #[test]
    fn test_cyclic_bounded_vec_format() {
        let mut cyclic_bounded_vec = CyclicBoundedVec::with_capacity(8);

        for i in 0..12 {
            cyclic_bounded_vec.push(i);
        }

        let f = format!("{cyclic_bounded_vec:?}");
        assert_eq!(f, "[4, 5, 6, 7, 8, 9, 10, 11]");
    }
}
