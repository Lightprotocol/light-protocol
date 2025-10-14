#![no_std]

use core::ptr::read_unaligned;

use tinyvec::ArrayVec;

/// A generic tinyvec::ArrayVec backed map with O(n) lookup.
/// Maintains insertion order and tracks the last updated entry index.
pub struct ArrayMap<K, V, const N: usize>
where
    K: PartialEq + Default,
    V: Default,
{
    entries: ArrayVec<[(K, V); N]>,
    last_updated_index: Option<usize>,
}

impl<K, V, const N: usize> ArrayMap<K, V, N>
where
    K: PartialEq + Default,
    V: Default,
{
    pub fn new() -> Self {
        Self {
            entries: ArrayVec::new(),
            last_updated_index: None,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn last_updated_index(&self) -> Option<usize> {
        self.last_updated_index
    }

    pub fn get(&self, index: usize) -> Option<&(K, V)> {
        self.entries.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut (K, V)> {
        self.entries.get_mut(index)
    }

    pub fn get_u8(&self, index: u8) -> Option<&(K, V)> {
        self.get(index as usize)
    }

    pub fn get_mut_u8(&mut self, index: u8) -> Option<&mut (K, V)> {
        self.get_mut(index as usize)
    }

    pub fn get_by_key(&self, key: &K) -> Option<&V> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_mut_by_key(&mut self, key: &K) -> Option<&mut V> {
        self.entries
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn find(&self, key: &K) -> Option<(usize, &(K, V))> {
        self.entries.iter().enumerate().find(|(_, (k, _))| k == key)
    }

    pub fn find_mut(&mut self, key: &K) -> Option<(usize, &mut (K, V))> {
        self.entries
            .iter_mut()
            .enumerate()
            .find(|(_, (k, _))| k == key)
    }

    pub fn find_index(&self, key: &K) -> Option<usize> {
        self.find(key).map(|(idx, _)| idx)
    }

    pub fn set_last_updated_index<E>(&mut self, index: usize) -> Result<(), E>
    where
        E: From<ArrayMapError>,
    {
        if index < self.entries.len() {
            self.last_updated_index = Some(index);
            Ok(())
        } else {
            Err(ArrayMapError::IndexOutOfBounds.into())
        }
    }

    pub fn insert<E>(&mut self, key: K, value: V, error: E) -> Result<usize, E> {
        let new_idx = self.entries.len();
        // tinyvec's try_push returns Some(item) on failure, None on success
        if self.entries.try_push((key, value)).is_some() {
            return Err(error);
        }
        self.last_updated_index = Some(new_idx);
        Ok(new_idx)
    }
}

impl<K, V, const N: usize> Default for ArrayMap<K, V, N>
where
    K: PartialEq + Default,
    V: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

// Optimized [u8; 32] key methods (4x u64 comparison instead of 32x u8).
impl<V, const N: usize> ArrayMap<[u8; 32], V, N>
where
    V: Default,
{
    pub fn get_by_pubkey(&self, key: &[u8; 32]) -> Option<&V> {
        self.entries
            .iter()
            .find(|(k, _)| pubkey_eq(k, key))
            .map(|(_, v)| v)
    }

    pub fn get_mut_by_pubkey(&mut self, key: &[u8; 32]) -> Option<&mut V> {
        self.entries
            .iter_mut()
            .find(|(k, _)| pubkey_eq(k, key))
            .map(|(_, v)| v)
    }

    pub fn find_by_pubkey(&self, key: &[u8; 32]) -> Option<(usize, &([u8; 32], V))> {
        self.entries
            .iter()
            .enumerate()
            .find(|(_, (k, _))| pubkey_eq(k, key))
    }

    pub fn find_mut_by_pubkey(&mut self, key: &[u8; 32]) -> Option<(usize, &mut ([u8; 32], V))> {
        self.entries
            .iter_mut()
            .enumerate()
            .find(|(_, (k, _))| pubkey_eq(k, key))
    }

    pub fn find_pubkey_index(&self, key: &[u8; 32]) -> Option<usize> {
        self.find_by_pubkey(key).map(|(idx, _)| idx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayMapError {
    CapacityExceeded,
    IndexOutOfBounds,
}

impl core::fmt::Display for ArrayMapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ArrayMapError::CapacityExceeded => write!(f, "ArrayMap capacity exceeded"),
            ArrayMapError::IndexOutOfBounds => write!(f, "ArrayMap index out of bounds"),
        }
    }
}

#[inline(always)]
pub const fn pubkey_eq(p1: &[u8; 32], p2: &[u8; 32]) -> bool {
    let p1_ptr = p1.as_ptr() as *const u64;
    let p2_ptr = p2.as_ptr() as *const u64;

    unsafe {
        read_unaligned(p1_ptr) == read_unaligned(p2_ptr)
            && read_unaligned(p1_ptr.add(1)) == read_unaligned(p2_ptr.add(1))
            && read_unaligned(p1_ptr.add(2)) == read_unaligned(p2_ptr.add(2))
            && read_unaligned(p1_ptr.add(3)) == read_unaligned(p2_ptr.add(3))
    }
}
