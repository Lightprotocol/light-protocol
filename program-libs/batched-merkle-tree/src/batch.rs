use light_bloom_filter::BloomFilter;
use light_hasher::{Hasher, Poseidon};
use light_zero_copy::{slice_mut::ZeroCopySliceMutU64, vec::ZeroCopyVecU64};
use solana_program::msg;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::errors::BatchedMerkleTreeError;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
#[repr(u64)]
pub enum BatchState {
    /// Batch can be filled with values.
    Fill,
    /// Batch has been inserted into the tree.
    Inserted,
    /// Batch is full, and insertion is in progress.
    Full,
}

impl From<u64> for BatchState {
    fn from(value: u64) -> Self {
        match value {
            0 => BatchState::Fill,
            1 => BatchState::Inserted,
            2 => BatchState::Full,
            _ => panic!("Invalid BatchState value"),
        }
    }
}

impl From<BatchState> for u64 {
    fn from(val: BatchState) -> Self {
        val as u64
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, KnownLayout, Immutable, IntoBytes, FromBytes)]
pub struct Batch {
    /// Number of inserted elements in the zkp batch.
    num_inserted: u64,
    state: u64,
    current_zkp_batch_index: u64,
    num_inserted_zkps: u64,
    /// Number of iterations for the bloom_filter.
    pub num_iters: u64,
    /// Theoretical capacity of the bloom_filter. We want to make it much larger
    /// than batch_size to avoid false positives.
    pub bloom_filter_capacity: u64,
    /// Number of elements in a batch.
    pub batch_size: u64,
    /// Number of elements in a zkp batch.
    /// A batch consists out of one or more zkp batches.
    pub zkp_batch_size: u64,
    /// Sequence number when it is save to clear the batch without advancing to
    /// the saved root index.
    pub sequence_number: u64,
    /// Start leaf index of the first
    pub start_index: u64,
    pub root_index: u32,
    bloom_filter_is_zeroed: u8,
    _padding: [u8; 3],
}

impl Batch {
    pub fn new(
        num_iters: u64,
        bloom_filter_capacity: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        start_index: u64,
    ) -> Self {
        Batch {
            num_iters,
            bloom_filter_capacity,
            batch_size,
            num_inserted: 0,
            state: BatchState::Fill.into(),
            zkp_batch_size,
            current_zkp_batch_index: 0,
            num_inserted_zkps: 0,
            sequence_number: 0,
            root_index: 0,
            start_index,
            bloom_filter_is_zeroed: 0,
            _padding: [0u8; 3],
        }
    }

    pub fn get_state(&self) -> BatchState {
        self.state.into()
    }

    pub fn bloom_filter_is_zeroed(&self) -> bool {
        self.bloom_filter_is_zeroed == 1
    }

    pub fn set_bloom_filter_to_zeroed(&mut self) {
        self.bloom_filter_is_zeroed = 1;
    }

    pub fn set_bloom_filter_to_not_zeroed(&mut self) {
        self.bloom_filter_is_zeroed = 0;
    }

    /// fill -> full -> inserted -> fill
    pub fn advance_state_to_fill(&mut self) -> Result<(), BatchedMerkleTreeError> {
        if self.get_state() == BatchState::Inserted {
            self.state = BatchState::Fill.into();
        } else {
            msg!(
                "Batch is in incorrect state {} expected Inserted 3",
                self.state
            );
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        Ok(())
    }

    /// fill -> full -> inserted -> fill
    pub fn advance_state_to_inserted(&mut self) -> Result<(), BatchedMerkleTreeError> {
        if self.get_state() == BatchState::Full {
            self.state = BatchState::Inserted.into();
        } else {
            msg!(
                "Batch is in incorrect state {} expected ReadyToUpdateTree 2",
                self.state
            );
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        Ok(())
    }

    /// fill -> full -> inserted -> fill
    pub fn advance_state_to_full(&mut self) -> Result<(), BatchedMerkleTreeError> {
        if self.get_state() == BatchState::Fill {
            self.state = BatchState::Full.into();
        } else {
            msg!(
                "Batch is in incorrect state {} expected ReadyToUpdateTree 2",
                self.state
            );
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        Ok(())
    }

    pub fn get_first_ready_zkp_batch(&self) -> Result<u64, BatchedMerkleTreeError> {
        if self.get_state() == BatchState::Inserted {
            Err(BatchedMerkleTreeError::BatchAlreadyInserted)
        } else if self.batch_is_ready_to_insert() {
            Ok(self.num_inserted_zkps)
        } else {
            Err(BatchedMerkleTreeError::BatchNotReady)
        }
    }

    pub fn batch_is_ready_to_insert(&self) -> bool {
        self.current_zkp_batch_index > self.num_inserted_zkps
    }

    /// Returns the number of zkp batch updates
    /// that are ready to be inserted into the tree.
    pub fn num_ready_zkp_updates(&self) -> u64 {
        self.current_zkp_batch_index
            .saturating_sub(self.num_inserted_zkps)
    }

    /// Returns the number of inserted elements
    /// in the current zkp batch.
    pub fn get_num_inserted(&self) -> u64 {
        self.num_inserted
    }

    pub fn get_current_zkp_batch_index(&self) -> u64 {
        self.current_zkp_batch_index
    }

    pub fn get_num_inserted_zkps(&self) -> u64 {
        self.num_inserted_zkps
    }

    /// Returns the number of inserted elements in the batch.
    pub fn get_num_inserted_elements(&self) -> u64 {
        self.num_inserted_zkps * self.zkp_batch_size + self.num_inserted
    }

    /// Returns the number of zkp batches in the batch.
    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }

    pub fn get_hashchain_store_len(&self) -> usize {
        self.batch_size as usize / self.zkp_batch_size as usize
    }

    /// Returns the index of a value by leaf index in the value store,
    /// provided it could exist in the batch.
    pub fn get_value_index_in_batch(&self, leaf_index: u64) -> Result<u64, BatchedMerkleTreeError> {
        self.leaf_index_could_exist_in_batch(leaf_index)?;
        leaf_index
            .checked_sub(self.start_index)
            .ok_or(BatchedMerkleTreeError::LeafIndexNotInBatch)
    }

    pub fn store_value(
        &mut self,
        value: &[u8; 32],
        value_store: &mut ZeroCopyVecU64<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        value_store.push(*value)?;
        Ok(())
    }

    /// Stores the value in a value store,
    /// and adds the value to the current hash chain.
    pub fn store_and_hash_value(
        &mut self,
        value: &[u8; 32],
        value_store: &mut ZeroCopyVecU64<[u8; 32]>,
        hashchain_store: &mut ZeroCopyVecU64<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.add_to_hash_chain(value, hashchain_store)?;
        self.store_value(value, value_store)
    }

    /// Inserts into the bloom filter and
    /// add value a the current hash chain.
    /// (used by nullifier & address queues)
    pub fn insert(
        &mut self,
        bloom_filter_value: &[u8; 32],
        hashchain_value: &[u8; 32],
        bloom_filter_stores: &mut [ZeroCopySliceMutU64<u8>],
        hashchain_store: &mut ZeroCopyVecU64<[u8; 32]>,
        bloom_filter_index: usize,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.add_to_hash_chain(hashchain_value, hashchain_store)?;

        for (i, bloom_filter) in bloom_filter_stores.iter_mut().enumerate() {
            if i == bloom_filter_index {
                let mut bloom_filter = BloomFilter::new(
                    self.num_iters as usize,
                    self.bloom_filter_capacity,
                    bloom_filter.as_mut_slice(),
                )?;
                bloom_filter.insert(bloom_filter_value)?;
            } else {
                self.check_non_inclusion(bloom_filter_value, bloom_filter.as_mut_slice())?;
            }
        }
        Ok(())
    }

    pub fn add_to_hash_chain(
        &mut self,
        value: &[u8; 32],
        hashchain_store: &mut ZeroCopyVecU64<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        if self.get_state() != BatchState::Fill {
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        let start_new_hash_chain = self.num_inserted == 0;
        if start_new_hash_chain {
            hashchain_store.push(*value)?;
        } else if let Some(last_hashchain) = hashchain_store.last() {
            let hashchain = Poseidon::hashv(&[last_hashchain, value.as_slice()])?;
            *hashchain_store.last_mut().unwrap() = hashchain;
        } else {
            // This state should never be reached.
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        self.num_inserted += 1;

        let zkp_batch_is_full = self.num_inserted == self.zkp_batch_size;
        if zkp_batch_is_full {
            self.current_zkp_batch_index += 1;
            self.num_inserted = 0;

            let batch_is_full = self.current_zkp_batch_index == self.get_num_zkp_batches();
            if batch_is_full {
                self.advance_state_to_full()?;
            }
        }

        Ok(())
    }

    /// Checks that value is not in the bloom filter.
    pub fn check_non_inclusion(
        &self,
        value: &[u8; 32],
        store: &mut [u8],
    ) -> Result<(), BatchedMerkleTreeError> {
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloom_filter_capacity, store)?;
        if bloom_filter.contains(value) {
            #[cfg(target_os = "solana")]
            msg!("Value already exists in the bloom filter.");
            return Err(BatchedMerkleTreeError::BatchInsertFailed);
        }
        Ok(())
    }

    /// Marks the batch as inserted in the merkle tree.
    /// 1. Checks that the batch is ready.
    /// 2. increments the number of inserted zkps.
    /// 3. If all zkps are inserted, sets the state to inserted.
    /// 4. Returns the updated state of the batch.
    pub fn mark_as_inserted_in_merkle_tree(
        &mut self,
        sequence_number: u64,
        root_index: u32,
        root_history_length: u32,
    ) -> Result<BatchState, BatchedMerkleTreeError> {
        // 1. Check that batch is ready.
        self.get_first_ready_zkp_batch()?;

        let num_zkp_batches = self.get_num_zkp_batches();

        // 2. increments the number of inserted zkps.
        self.num_inserted_zkps += 1;
        println!("num_inserted_zkps: {}", self.num_inserted_zkps);
        // 3. If all zkps are inserted, sets the state to inserted.
        let batch_is_completly_inserted = self.num_inserted_zkps == num_zkp_batches;
        if batch_is_completly_inserted {
            println!("Setting state to inserted");
            self.num_inserted_zkps = 0;
            self.current_zkp_batch_index = 0;
            self.advance_state_to_inserted()?;
            // Saving sequence number and root index for the batch.
            // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
            // if not advance current root index to root index
            self.sequence_number = sequence_number + root_history_length as u64;
            self.root_index = root_index;
        }

        Ok(self.get_state())
    }

    /// Returns true if value of leaf index could exist in batch.
    /// `True` doesn't mean that the value exists in the batch,
    /// just that it is plausible. The value might already be spent
    /// or never inserted in case an invalid index was provided.
    pub fn leaf_index_could_exist_in_batch(
        &self,
        leaf_index: u64,
    ) -> Result<bool, BatchedMerkleTreeError> {
        let max_batch_leaf_index =
            self.get_num_zkp_batches() * self.zkp_batch_size + self.start_index;
        let min_batch_leaf_index = self.start_index;
        Ok(leaf_index < max_batch_leaf_index && leaf_index >= min_batch_leaf_index)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn get_test_batch() -> Batch {
        Batch::new(3, 160_000, 500, 100, 0)
    }

    /// simulate zkp batch insertion
    fn test_mark_as_inserted(mut batch: Batch) {
        let mut sequence_number = 10;
        let mut root_index = 20;
        let root_history_length = 23;
        for i in 0..batch.get_num_zkp_batches() {
            sequence_number += i;
            root_index += i as u32;
            batch
                .mark_as_inserted_in_merkle_tree(sequence_number, root_index, root_history_length)
                .unwrap();
            if i != batch.get_num_zkp_batches() - 1 {
                assert_eq!(batch.get_state(), BatchState::Full);
                assert_eq!(batch.get_num_inserted(), 0);
                assert_eq!(batch.get_current_zkp_batch_index(), 5);
                assert_eq!(batch.get_num_inserted_zkps(), i + 1);
            } else {
                assert_eq!(batch.get_state(), BatchState::Inserted);
                assert_eq!(batch.get_num_inserted(), 0);
                assert_eq!(batch.get_current_zkp_batch_index(), 0);
                assert_eq!(batch.get_num_inserted_zkps(), 0);
            }
        }
        assert_eq!(batch.get_state(), BatchState::Inserted);
        assert_eq!(batch.get_num_inserted(), 0);
        let mut ref_batch = get_test_batch();
        ref_batch.state = BatchState::Inserted.into();
        ref_batch.root_index = root_index;
        ref_batch.sequence_number = sequence_number + root_history_length as u64;
        assert_eq!(batch, ref_batch);
    }

    #[test]
    fn test_store_value() {
        let mut batch = get_test_batch();

        let mut value_store_bytes =
            vec![0u8; ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(batch.batch_size)];
        let mut value_store =
            ZeroCopyVecU64::new(batch.batch_size, &mut value_store_bytes).unwrap();
        let mut hashchain_store_bytes = vec![
            0u8;
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
                batch.get_hashchain_store_len() as u64
            )
        ];
        let mut hashchain_store = ZeroCopyVecU64::new(
            batch.get_hashchain_store_len() as u64,
            hashchain_store_bytes.as_mut_slice(),
        )
        .unwrap();

        let mut ref_batch = get_test_batch();
        for i in 0..batch.batch_size {
            ref_batch.num_inserted %= ref_batch.zkp_batch_size;

            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            assert!(batch
                .store_and_hash_value(&value, &mut value_store, &mut hashchain_store)
                .is_ok());
            ref_batch.num_inserted += 1;
            if ref_batch.num_inserted == ref_batch.zkp_batch_size {
                ref_batch.current_zkp_batch_index += 1;
                ref_batch.num_inserted = 0;
            }
            if ref_batch.current_zkp_batch_index == ref_batch.get_num_zkp_batches() {
                ref_batch.state = BatchState::Full.into();
                ref_batch.num_inserted = 0;
            }
            assert_eq!(batch, ref_batch);
            assert_eq!(*value_store.get(i as usize).unwrap(), value);
        }
        let result = batch.store_and_hash_value(&[1u8; 32], &mut value_store, &mut hashchain_store);
        assert_eq!(result.unwrap_err(), BatchedMerkleTreeError::BatchNotReady);
        assert_eq!(batch.get_state(), BatchState::Full);
        assert_eq!(batch.get_num_inserted(), 0);
        assert_eq!(batch.get_current_zkp_batch_index(), 5);
        assert_eq!(batch.get_num_zkp_batches(), 5);
        assert_eq!(batch.get_num_inserted_zkps(), 0);

        test_mark_as_inserted(batch);
    }

    #[test]
    fn test_insert() {
        // Behavior Input queue
        let mut batch = get_test_batch();
        let mut stores = vec![vec![0u8; 20_008]; 2];
        let mut bloom_filter_stores = stores
            .iter_mut()
            .map(|store| ZeroCopySliceMutU64::new(20_000, store).unwrap())
            .collect::<Vec<_>>();
        let mut hashchain_store_bytes = vec![
            0u8;
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
                batch.get_hashchain_store_len() as u64
            )
        ];
        ZeroCopyVecU64::<[u8; 32]>::new(
            batch.get_hashchain_store_len() as u64,
            hashchain_store_bytes.as_mut_slice(),
        )
        .unwrap();

        let mut ref_batch = get_test_batch();
        for processing_index in 0..=1 {
            for i in 0..(batch.batch_size / 2) {
                let i = i + (batch.batch_size / 2) * (processing_index as u64);

                ref_batch.num_inserted %= ref_batch.zkp_batch_size;
                let mut hashchain_store =
                    ZeroCopyVecU64::<[u8; 32]>::from_bytes(hashchain_store_bytes.as_mut_slice())
                        .unwrap();

                let mut value = [0u8; 32];
                value[24..].copy_from_slice(&i.to_be_bytes());
                let ref_hash_chain = if i % batch.zkp_batch_size == 0 {
                    value
                } else {
                    Poseidon::hashv(&[hashchain_store.last().unwrap(), &value]).unwrap()
                };
                let result = batch.insert(
                    &value,
                    &value,
                    bloom_filter_stores.as_mut_slice(),
                    &mut hashchain_store,
                    processing_index,
                );
                // First insert should succeed
                assert!(result.is_ok(), "Failed result: {:?}", result);
                assert_eq!(*hashchain_store.last().unwrap(), ref_hash_chain);

                {
                    let mut cloned_hashchain_store = hashchain_store_bytes.clone();
                    let mut hashchain_store = ZeroCopyVecU64::<[u8; 32]>::from_bytes(
                        cloned_hashchain_store.as_mut_slice(),
                    )
                    .unwrap();
                    let mut batch = batch;
                    // Reinsert should fail
                    assert!(batch
                        .insert(
                            &value,
                            &value,
                            bloom_filter_stores.as_mut_slice(),
                            &mut hashchain_store,
                            processing_index
                        )
                        .is_err());
                }
                let mut bloom_filter = BloomFilter {
                    num_iters: batch.num_iters as usize,
                    capacity: batch.bloom_filter_capacity,
                    store: bloom_filter_stores[processing_index].as_mut_slice(),
                };
                assert!(bloom_filter.contains(&value));
                let other_index = if processing_index == 0 { 1 } else { 0 };
                batch
                    .check_non_inclusion(&value, bloom_filter_stores[other_index].as_mut_slice())
                    .unwrap();
                batch
                    .check_non_inclusion(
                        &value,
                        bloom_filter_stores[processing_index].as_mut_slice(),
                    )
                    .unwrap_err();

                ref_batch.num_inserted += 1;
                if ref_batch.num_inserted == ref_batch.zkp_batch_size {
                    ref_batch.current_zkp_batch_index += 1;
                    ref_batch.num_inserted = 0;
                }
                if i == batch.batch_size - 1 {
                    ref_batch.state = BatchState::Full.into();
                    ref_batch.num_inserted = 0;
                }
                assert_eq!(batch, ref_batch);
            }
        }
        test_mark_as_inserted(batch);
    }

    #[test]
    fn test_add_to_hash_chain() {
        let mut batch = get_test_batch();
        let mut hashchain_store_bytes = vec![
            0u8;
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
                batch.get_hashchain_store_len() as u64
            )
        ];
        let mut hashchain_store = ZeroCopyVecU64::<[u8; 32]>::new(
            batch.get_hashchain_store_len() as u64,
            hashchain_store_bytes.as_mut_slice(),
        )
        .unwrap();
        let value = [1u8; 32];

        assert!(batch
            .add_to_hash_chain(&value, &mut hashchain_store)
            .is_ok());
        let mut ref_batch = get_test_batch();
        let user_hash_chain = value;
        ref_batch.num_inserted = 1;
        assert_eq!(batch, ref_batch);
        assert_eq!(hashchain_store[0], user_hash_chain);
        let value = [2u8; 32];
        let ref_hash_chain = Poseidon::hashv(&[&user_hash_chain, &value]).unwrap();
        assert!(batch
            .add_to_hash_chain(&value, &mut hashchain_store)
            .is_ok());

        ref_batch.num_inserted = 2;
        assert_eq!(batch, ref_batch);
        assert_eq!(hashchain_store[0], ref_hash_chain);
    }

    #[test]
    fn test_check_non_inclusion() {
        for processing_index in 0..=1 {
            let mut batch = get_test_batch();

            let value = [1u8; 32];
            let mut stores = vec![vec![0u8; 20_008]; 2];
            let mut bloom_filter_stores = stores
                .iter_mut()
                .map(|store| ZeroCopySliceMutU64::new(20_000, store).unwrap())
                .collect::<Vec<_>>();
            let mut hashchain_store_bytes = vec![
            0u8;
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
                batch.get_hashchain_store_len() as u64
            )
        ];
            let mut hashchain_store = ZeroCopyVecU64::<[u8; 32]>::new(
                batch.get_hashchain_store_len() as u64,
                hashchain_store_bytes.as_mut_slice(),
            )
            .unwrap();

            assert!(batch
                .check_non_inclusion(&value, bloom_filter_stores[processing_index].as_mut_slice())
                .is_ok());
            let ref_batch = get_test_batch();
            assert_eq!(batch, ref_batch);
            batch
                .insert(
                    &value,
                    &value,
                    bloom_filter_stores.as_mut_slice(),
                    &mut hashchain_store,
                    processing_index,
                )
                .unwrap();
            assert!(batch
                .check_non_inclusion(&value, bloom_filter_stores[processing_index].as_mut_slice())
                .is_err());

            let other_index = if processing_index == 0 { 1 } else { 0 };
            assert!(batch
                .check_non_inclusion(&value, bloom_filter_stores[other_index].as_mut_slice())
                .is_ok());
        }
    }

    #[test]
    fn test_getters() {
        let mut batch = get_test_batch();
        assert_eq!(batch.get_num_zkp_batches(), 5);
        assert_eq!(batch.get_hashchain_store_len(), 5);
        assert_eq!(batch.get_state(), BatchState::Fill);
        assert_eq!(batch.get_num_inserted(), 0);
        assert_eq!(batch.get_current_zkp_batch_index(), 0);
        assert_eq!(batch.get_num_inserted_zkps(), 0);
        batch.advance_state_to_full().unwrap();
        assert_eq!(batch.get_state(), BatchState::Full);
        batch.advance_state_to_inserted().unwrap();
        assert_eq!(batch.get_state(), BatchState::Inserted);
    }

    /// Tests:
    /// 1. Failing test lowest value in eligble range - 1
    /// 2. Functional test lowest value in eligble range
    /// 3. Functional test highest value in eligble range
    /// 4. Failing test eligble range + 1
    #[test]
    fn test_value_is_inserted_in_batch() {
        let mut batch = get_test_batch();
        batch.advance_state_to_full().unwrap();
        batch.advance_state_to_inserted().unwrap();
        batch.start_index = 1;
        let lowest_eligible_value = batch.start_index;
        let highest_eligible_value =
            batch.start_index + batch.get_num_zkp_batches() * batch.zkp_batch_size - 1;
        // 1. Failing test lowest value in eligble range - 1
        assert!(!batch
            .leaf_index_could_exist_in_batch(lowest_eligible_value - 1)
            .unwrap());
        // 2. Functional test lowest value in eligble range
        assert!(batch
            .leaf_index_could_exist_in_batch(lowest_eligible_value)
            .unwrap());
        // 3. Functional test highest value in eligble range
        assert!(batch
            .leaf_index_could_exist_in_batch(highest_eligible_value)
            .unwrap());
        // 4. Failing test eligble range + 1
        assert!(!batch
            .leaf_index_could_exist_in_batch(highest_eligible_value + 1)
            .unwrap());
    }

    /// 1. Failing: empty batch
    /// 2. Functional: if zkp batch size is full else failing
    /// 3. Failing: batch is completely inserted
    #[test]
    fn test_can_insert_batch() {
        let mut batch = get_test_batch();
        assert_eq!(
            batch.get_first_ready_zkp_batch(),
            Err(BatchedMerkleTreeError::BatchNotReady)
        );
        let mut value_store_bytes =
            vec![0u8; ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(batch.batch_size)];
        let mut value_store =
            ZeroCopyVecU64::<[u8; 32]>::new(batch.batch_size, &mut value_store_bytes).unwrap();
        let mut hashchain_store_bytes = vec![
            0u8;
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
                batch.get_hashchain_store_len() as u64
            )
        ];
        let mut hashchain_store = ZeroCopyVecU64::<[u8; 32]>::new(
            batch.get_hashchain_store_len() as u64,
            hashchain_store_bytes.as_mut_slice(),
        )
        .unwrap();

        for i in 0..batch.batch_size + 10 {
            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            if i < batch.batch_size {
                batch
                    .store_and_hash_value(&value, &mut value_store, &mut hashchain_store)
                    .unwrap();
            }
            if (i + 1) % batch.zkp_batch_size == 0 && i != 0 {
                assert_eq!(
                    batch.get_first_ready_zkp_batch().unwrap(),
                    i / batch.zkp_batch_size
                );
                batch.mark_as_inserted_in_merkle_tree(0, 0, 0).unwrap();
            } else if i >= batch.batch_size {
                assert_eq!(
                    batch.get_first_ready_zkp_batch(),
                    Err(BatchedMerkleTreeError::BatchAlreadyInserted)
                );
            } else {
                assert_eq!(
                    batch.get_first_ready_zkp_batch(),
                    Err(BatchedMerkleTreeError::BatchNotReady)
                );
            }
        }
    }
}
