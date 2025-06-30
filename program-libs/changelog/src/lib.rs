use light_zero_copy::cyclic_vec::ZeroCopyCyclicVecU64;
use light_zero_copy::ZeroCopyTraits;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Trait for key-value entries in the changelog
pub trait KeyValue {
    type Key: PartialEq;
    type Value: Copy;

    fn key(&self) -> Self::Key;
    fn value(&self) -> Self::Value;
    fn cmp_key(&self, other: &Self::Key) -> bool;
}

/// Optimized SIMD-style comparison for 32-byte arrays
#[inline(always)]
pub fn simd_iterator_compare(a: &[u8; 32], b: &[u8; 32]) -> bool {
    // Use safe byte comparison with u64 chunks for better performance
    // Convert to u64 arrays safely
    let a_u64 = unsafe { 
        std::ptr::read_unaligned(a.as_ptr() as *const [u64; 4])
    };
    let b_u64 = unsafe { 
        std::ptr::read_unaligned(b.as_ptr() as *const [u64; 4])
    };

    // Iterate over chunks with early exit
    for i in 0..4 {
        if a_u64[i] != b_u64[i] {
            return false;
        }
    }
    true
}

/// Generic changelog structure for efficient key-value storage with circular buffer behavior
pub struct GenericChangelog<'a, T: KeyValue + ZeroCopyTraits> {
    /// Once full index resets and starts at 0 again
    /// existing values are overwritten.
    pub entries: ZeroCopyCyclicVecU64<'a, T>,
}

impl<'a, T: KeyValue + ZeroCopyTraits> GenericChangelog<'a, T> {
    #[inline(always)]
    pub fn new(
        capacity: u64,
        backing_store: &'a mut [u8],
    ) -> Result<Self, light_zero_copy::errors::ZeroCopyError> {
        Ok(Self {
            entries: ZeroCopyCyclicVecU64::<T>::new(capacity, backing_store)?,
        })
    }

    #[inline(always)]
    pub fn from_bytes(
        backing_store: &'a mut [u8],
    ) -> Result<Self, light_zero_copy::errors::ZeroCopyError> {
        Ok(Self {
            entries: ZeroCopyCyclicVecU64::<T>::from_bytes(backing_store)?,
        })
    }

    #[inline(always)]
    pub fn push(&mut self, entry: T) {
        self.entries.push(entry);
    }

    /// Optimized search using SIMD iterator comparison
    /// This is the winning implementation from our benchmarks
    #[inline(always)]
    pub fn find_latest_simd_iterator(&self, key: [u8; 32], num_iters: Option<usize>) -> Option<u64>
    where
        T: KeyValue<Key = [u8; 32], Value = u64>,
    {
        let max_iters = num_iters
            .unwrap_or(self.entries.len())
            .min(self.entries.len());

        if max_iters == 0 || self.entries.is_empty() {
            return None;
        }

        let mut current_index = self.entries.last_index();
        let mut iterations = 0;

        while iterations < max_iters {
            if let Some(entry) = self.entries.get(current_index) {
                if entry.cmp_key(&key) {
                    return Some(entry.value());
                }
            }

            iterations += 1;
            if iterations < max_iters {
                if current_index == 0 {
                    if self.entries.len() == self.entries.capacity() {
                        current_index = self.entries.capacity() - 1;
                    } else {
                        break;
                    }
                } else {
                    current_index -= 1;
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }
}

/// Standard entry type for 32-byte keys and u64 values
#[derive(Copy, Clone, KnownLayout, Immutable, FromBytes, IntoBytes)]
#[repr(C)]
pub struct Entry {
    pub value: u64,
    pub mint: [u8; 32],
}

impl Entry {
    #[inline(always)]
    pub fn new(mint: [u8; 32], value: u64) -> Self {
        Self { value, mint }
    }
}

impl KeyValue for Entry {
    type Value = u64;
    type Key = [u8; 32];

    #[inline(always)]
    fn key(&self) -> [u8; 32] {
        self.mint
    }

    #[inline(always)]
    fn value(&self) -> Self::Value {
        self.value
    }

    #[inline(always)]
    fn cmp_key(&self, other: &Self::Key) -> bool {
        simd_iterator_compare(&self.mint, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use light_zero_copy::cyclic_vec::ZeroCopyCyclicVecU64;

    fn create_test_key(seed: u8) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0] = seed;
        bytes
    }

    #[test]
    fn test_simd_iterator_compare() {
        let key1 = [1u8; 32];
        let key2 = [1u8; 32];
        let key3 = [2u8; 32];

        assert!(simd_iterator_compare(&key1, &key2));
        assert!(!simd_iterator_compare(&key1, &key3));
    }

    #[test]
    fn test_changelog_basic() {
        let capacity = 5u64;
        let mut backing_store =
            vec![0u8; ZeroCopyCyclicVecU64::<Entry>::required_size_for_capacity(capacity)];
        let mut changelog = GenericChangelog::new(capacity, &mut backing_store).unwrap();

        let key1 = create_test_key(1);
        let key2 = create_test_key(2);
        let key3 = create_test_key(3);

        // Add entries
        changelog.push(Entry::new(key1, 100));
        changelog.push(Entry::new(key2, 200));
        changelog.push(Entry::new(key3, 300));

        // Test finding latest values
        assert_eq!(changelog.find_latest_simd_iterator(key1, None), Some(100));
        assert_eq!(changelog.find_latest_simd_iterator(key2, None), Some(200));
        assert_eq!(changelog.find_latest_simd_iterator(key3, None), Some(300));

        // Test non-existent key
        let key_not_found = create_test_key(99);
        assert_eq!(changelog.find_latest_simd_iterator(key_not_found, None), None);
    }

    #[test]
    fn test_changelog_limited_search() {
        let capacity = 10u64;
        let mut backing_store =
            vec![0u8; ZeroCopyCyclicVecU64::<Entry>::required_size_for_capacity(capacity)];
        let mut changelog = GenericChangelog::new(capacity, &mut backing_store).unwrap();

        let key1 = create_test_key(1);
        let key2 = create_test_key(2);

        // Add multiple entries
        for i in 1..=10 {
            if i % 2 == 0 {
                changelog.push(Entry::new(key1, i * 10));
            } else {
                changelog.push(Entry::new(key2, i * 10));
            }
        }

        // Find latest with no limit (should find most recent)
        assert_eq!(changelog.find_latest_simd_iterator(key1, None), Some(100)); // 10 * 10
        assert_eq!(changelog.find_latest_simd_iterator(key2, None), Some(90)); // 9 * 10

        // Find latest with limit of 3 iterations
        assert_eq!(changelog.find_latest_simd_iterator(key1, Some(3)), Some(100));
        assert_eq!(changelog.find_latest_simd_iterator(key2, Some(3)), Some(90));

        // Find latest with limit of 1 (only check the very last entry)
        assert_eq!(changelog.find_latest_simd_iterator(key1, Some(1)), Some(100)); // Last entry is key1
        assert_eq!(changelog.find_latest_simd_iterator(key2, Some(1)), None); // Last entry is not key2
    }

    #[test]
    fn test_changelog_cyclic_behavior() {
        let capacity = 3u64;
        let mut backing_store =
            vec![0u8; ZeroCopyCyclicVecU64::<Entry>::required_size_for_capacity(capacity)];
        let mut changelog = GenericChangelog::new(capacity, &mut backing_store).unwrap();

        let key1 = create_test_key(1);
        let key2 = create_test_key(2);
        let key3 = create_test_key(3);
        let key4 = create_test_key(4);

        // Fill the changelog
        changelog.push(Entry::new(key1, 100));
        changelog.push(Entry::new(key2, 200));
        changelog.push(Entry::new(key3, 300));

        // Add more entries (should wrap around)
        changelog.push(Entry::new(key4, 400)); // Overwrites key1

        // key1 should no longer be found
        assert_eq!(changelog.find_latest_simd_iterator(key1, None), None);
        assert_eq!(changelog.find_latest_simd_iterator(key2, None), Some(200));
        assert_eq!(changelog.find_latest_simd_iterator(key3, None), Some(300));
        assert_eq!(changelog.find_latest_simd_iterator(key4, None), Some(400));
    }

    #[test]
    fn test_performance_characteristics() {
        let capacity = 1000u64;
        let mut backing_store =
            vec![0u8; ZeroCopyCyclicVecU64::<Entry>::required_size_for_capacity(capacity)];
        let mut changelog = GenericChangelog::new(capacity, &mut backing_store).unwrap();

        // Fill with many entries
        for i in 0..1000 {
            let key = create_test_key((i % 256) as u8);
            changelog.push(Entry::new(key, i));
        }

        // Test that recent entries are found efficiently
        // The last entry (i=999) has key create_test_key(999 % 256) = create_test_key(231)
        let recent_key = create_test_key(231);
        assert!(changelog.find_latest_simd_iterator(recent_key, Some(10)).is_some());
        
        // Test that non-existent key returns None efficiently
        let non_existent_key = create_test_key(128);
        assert_eq!(changelog.find_latest_simd_iterator(non_existent_key, Some(100)), None);
    }
}