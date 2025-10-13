use light_array_map::ArrayMap;

use crate::{errors::SystemProgramError, Result};

/// Extension trait for ArrayMap with tuple values (u64, u8).
/// Only increments the first element of the tuple.
pub trait TreeLeafTrackerTupleExt<K, const N: usize>
where
    K: PartialEq + Default,
{
    /// Increments the first element of the tuple for the last accessed entry.
    /// Returns the tuple value before incrementing.
    ///
    /// # Errors
    /// Returns error if no entry has been accessed (last_accessed_index is None).
    fn increment_current_tuple(&mut self) -> Result<(u64, u8)>;

    /// Gets or inserts a tuple entry, incrementing only the first element.
    /// If the key exists, returns its current value and increments the first element.
    /// If the key doesn't exist, inserts it with the given initial value,
    /// returns that value, and increments the first element for next use.
    /// Sets this entry as the last accessed entry.
    ///
    /// # Arguments
    /// * `key` - The key to look up or insert
    /// * `initial_value` - The tuple (leaf_index, account_index) to use if this is a new entry
    /// * `error` - The error to return if capacity is exceeded
    ///
    /// # Returns
    /// A tuple of ((leaf_index, account_index), is_new) where is_new indicates if this was a new entry.
    fn get_or_insert_tuple(
        &mut self,
        key: &K,
        initial_value: (u64, u8),
        error: SystemProgramError,
    ) -> Result<((u64, u8), bool)>;
}

impl<K, const N: usize> TreeLeafTrackerTupleExt<K, N> for ArrayMap<K, (u64, u8), N>
where
    K: PartialEq + Copy + Default,
{
    fn increment_current_tuple(&mut self) -> Result<(u64, u8)> {
        let idx = self
            .last_accessed_index()
            .ok_or(SystemProgramError::OutputMerkleTreeIndexOutOfBounds)?;

        let entry = self
            .get_mut(idx)
            .ok_or(SystemProgramError::OutputMerkleTreeIndexOutOfBounds)?;

        let prev = entry.1;
        entry.1 .0 += 1; // Only increment the leaf index (first element)
        Ok(prev)
    }

    fn get_or_insert_tuple(
        &mut self,
        key: &K,
        initial_value: (u64, u8),
        error: SystemProgramError,
    ) -> Result<((u64, u8), bool)> {
        // Find existing key
        if let Some((idx, entry)) = self.find_mut(key) {
            let prev = entry.1;
            entry.1 .0 += 1; // Only increment the leaf index (first element)
            self.set_last_accessed_index::<SystemProgramError>(idx)?;
            return Ok((prev, false));
        }
        // Insert new entry with first element incremented
        self.insert(*key, (initial_value.0 + 1, initial_value.1), error)?;

        Ok((initial_value, true))
    }
}
