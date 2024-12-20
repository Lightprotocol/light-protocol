use crate::errors::BatchedMerkleTreeError;
use light_bloom_filter::BloomFilter;
use light_bounded_vec::BoundedVec;
use light_hasher::{Hasher, Poseidon};
use solana_program::msg;

#[repr(u64)]
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum BatchState {
    /// Batch can be filled with values.
    CanBeFilled,
    /// Batch has been inserted into the tree.
    Inserted,
    /// Batch is full, and insertion is in progress.
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    /// Number of inserted elements in the zkp batch.
    num_inserted: u64,
    state: BatchState,
    current_zkp_batch_index: u64,
    num_inserted_zkps: u64,
    /// Number of iterations for the bloom_filter.
    pub num_iters: u64,
    /// Theoretical capacity of the bloom_filter. We want to make it much larger
    /// than batch_size to avoid false positives.
    pub bloom_filter_capacity: u64,
    pub batch_size: u64,
    pub zkp_batch_size: u64,
    /// Sequence number when it is save to clear the batch without advancing to
    /// the saved root index.
    pub sequence_number: u64,
    pub root_index: u32,
    pub start_index: u64,
    /// Placeholder for forester to signal that the bloom filter is wiped
    /// already.
    pub bloom_filter_is_wiped: bool,
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
            state: BatchState::CanBeFilled,
            zkp_batch_size,
            current_zkp_batch_index: 0,
            num_inserted_zkps: 0,
            sequence_number: 0,
            root_index: 0,
            start_index,
            bloom_filter_is_wiped: false,
        }
    }

    pub fn get_state(&self) -> BatchState {
        self.state
    }

    /// fill -> full -> inserted -> fill
    pub fn advance_state_to_can_be_filled(&mut self) -> Result<(), BatchedMerkleTreeError> {
        if self.state == BatchState::Inserted {
            self.state = BatchState::CanBeFilled;
        } else {
            msg!(
                "Batch is in incorrect state {} expected Inserted 3",
                self.state as u64
            );
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        Ok(())
    }

    /// fill -> full -> inserted -> fill
    pub fn advance_state_to_inserted(&mut self) -> Result<(), BatchedMerkleTreeError> {
        if self.state == BatchState::Full {
            self.state = BatchState::Inserted;
        } else {
            msg!(
                "Batch is in incorrect state {} expected ReadyToUpdateTree 2",
                self.state as u64
            );
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        Ok(())
    }

    /// fill -> full -> inserted -> fill
    pub fn advance_state_to_full(&mut self) -> Result<(), BatchedMerkleTreeError> {
        if self.state == BatchState::CanBeFilled {
            self.state = BatchState::Full;
        } else {
            msg!(
                "Batch is in incorrect state {} expected ReadyToUpdateTree 2",
                self.state as u64
            );
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        Ok(())
    }

    pub fn get_first_ready_zkp_batch(&self) -> Result<u64, BatchedMerkleTreeError> {
        if self.state == BatchState::Inserted {
            Err(BatchedMerkleTreeError::BatchAlreadyInserted)
        } else if self.current_zkp_batch_index > self.num_inserted_zkps {
            Ok(self.num_inserted_zkps)
        } else {
            Err(BatchedMerkleTreeError::BatchNotReady)
        }
    }

    pub fn get_num_inserted(&self) -> u64 {
        self.num_inserted
    }

    pub fn get_current_zkp_batch_index(&self) -> u64 {
        self.current_zkp_batch_index
    }

    pub fn get_num_inserted_zkps(&self) -> u64 {
        self.num_inserted_zkps
    }

    pub fn get_num_inserted_elements(&self) -> u64 {
        self.num_inserted_zkps * self.zkp_batch_size + self.num_inserted
    }

    pub fn store_value(
        &mut self,
        value: &[u8; 32],
        value_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        if self.state != BatchState::CanBeFilled {
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        value_store.push(*value)?;
        Ok(())
    }

    pub fn store_and_hash_value(
        &mut self,
        value: &[u8; 32],
        value_store: &mut BoundedVec<[u8; 32]>,
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.store_value(value, value_store)?;
        self.add_to_hash_chain(value, hashchain_store)
    }

    /// Inserts into the bloom filter and hashes the value.
    /// (used by input/nullifier queue)
    pub fn insert(
        &mut self,
        bloom_filter_value: &[u8; 32],
        hashchain_value: &[u8; 32],
        store: &mut [u8],
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloom_filter_capacity, store)?;
        bloom_filter.insert(bloom_filter_value)?;
        self.add_to_hash_chain(hashchain_value, hashchain_store)
    }

    pub fn add_to_hash_chain(
        &mut self,
        value: &[u8; 32],
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), BatchedMerkleTreeError> {
        if self.num_inserted == self.zkp_batch_size || self.num_inserted == 0 {
            hashchain_store.push(*value)?;
            self.num_inserted = 0;
        } else if let Some(last_hashchain) = hashchain_store.last() {
            let hashchain = Poseidon::hashv(&[last_hashchain, value.as_slice()])?;
            *hashchain_store.last_mut().unwrap() = hashchain;
        }

        self.num_inserted += 1;
        if self.num_inserted == self.zkp_batch_size {
            self.current_zkp_batch_index += 1;
        }

        if self.get_num_zkp_batches() == self.current_zkp_batch_index {
            self.advance_state_to_full()?;
            self.num_inserted = 0;
        }

        Ok(())
    }

    /// Inserts into the bloom filter and hashes the value.
    /// (used by nullifier queue)
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

    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }

    pub fn mark_as_inserted_in_merkle_tree(
        &mut self,
        sequence_number: u64,
        root_index: u32,
        root_history_length: u32,
    ) -> Result<(), BatchedMerkleTreeError> {
        // Check that batch is ready.
        self.get_first_ready_zkp_batch()?;

        let num_zkp_batches = self.get_num_zkp_batches();

        self.num_inserted_zkps += 1;
        msg!(
            "Marking batch as inserted in the merkle tree. num_inserted_zkps: {}",
            self.num_inserted_zkps
        );
        msg!("num_zkp_batches: {}", num_zkp_batches);
        // Batch has been successfully inserted into the tree.
        if self.num_inserted_zkps == num_zkp_batches {
            self.current_zkp_batch_index = 0;
            self.state = BatchState::Inserted;
            self.num_inserted_zkps = 0;
            // Saving sequence number and root index for the batch.
            // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
            // if not advance current root index to root index
            self.sequence_number = sequence_number + root_history_length as u64;
            self.root_index = root_index;
        }

        Ok(())
    }

    pub fn get_hashchain_store_len(&self) -> usize {
        self.batch_size as usize / self.zkp_batch_size as usize
    }

    pub fn value_is_inserted_in_batch(
        &self,
        leaf_index: u64,
    ) -> Result<bool, BatchedMerkleTreeError> {
        let max_batch_leaf_index =
            self.get_num_zkp_batches() * self.zkp_batch_size + self.start_index;
        let min_batch_leaf_index = self.start_index;
        Ok(leaf_index < max_batch_leaf_index && leaf_index >= min_batch_leaf_index)
    }

    pub fn get_value_index_in_batch(&self, leaf_index: u64) -> Result<u64, BatchedMerkleTreeError> {
        leaf_index
            .checked_sub(self.start_index)
            .ok_or(BatchedMerkleTreeError::LeafIndexNotInBatch)
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
            sequence_number += i as u64;
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
        ref_batch.state = BatchState::Inserted;
        ref_batch.root_index = root_index;
        ref_batch.sequence_number = sequence_number + root_history_length as u64;
        assert_eq!(batch, ref_batch);
    }

    #[test]
    fn test_store_value() {
        let mut batch = get_test_batch();

        let mut value_store = BoundedVec::with_capacity(batch.batch_size as usize);
        let mut hashchain_store = BoundedVec::with_capacity(batch.get_hashchain_store_len());

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
            }
            if ref_batch.current_zkp_batch_index == ref_batch.get_num_zkp_batches() {
                ref_batch.state = BatchState::Full;
                ref_batch.num_inserted = 0;
            }
            assert_eq!(batch, ref_batch);
            assert_eq!(*value_store.get(i as usize).unwrap(), value);
        }
        let result = batch.store_and_hash_value(&[1u8; 32], &mut value_store, &mut hashchain_store);
        assert_eq!(
            result.unwrap_err(),
            BatchedMerkleTreeError::BatchNotReady.into()
        );
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
        let mut store = vec![0u8; 20_000];
        let hashchain_store_len = batch.get_hashchain_store_len();
        let mut hashchain_store: BoundedVec<[u8; 32]> =
            BoundedVec::with_capacity(hashchain_store_len);

        let mut ref_batch = get_test_batch();
        for i in 0..batch.batch_size {
            ref_batch.num_inserted %= ref_batch.zkp_batch_size;
            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            let ref_hash_chain = if i % batch.zkp_batch_size == 0 {
                value
            } else {
                Poseidon::hashv(&[hashchain_store.last().unwrap(), &value]).unwrap()
            };
            assert!(batch
                .insert(&value, &value, &mut store, &mut hashchain_store)
                .is_ok());
            let mut bloom_filter = BloomFilter {
                num_iters: batch.num_iters as usize,
                capacity: batch.bloom_filter_capacity,
                store: &mut store,
            };
            assert!(bloom_filter.contains(&value));
            batch.check_non_inclusion(&value, &mut store).unwrap_err();

            ref_batch.num_inserted += 1;
            assert_eq!(*hashchain_store.last().unwrap(), ref_hash_chain);
            if ref_batch.num_inserted == ref_batch.zkp_batch_size {
                ref_batch.current_zkp_batch_index += 1;
            }
            if i == batch.batch_size - 1 {
                ref_batch.state = BatchState::Full;
                ref_batch.num_inserted = 0;
            }
            assert_eq!(batch, ref_batch);
        }
        test_mark_as_inserted(batch);
    }

    #[test]
    fn test_add_to_hash_chain() {
        let mut batch = get_test_batch();
        let hashchain_store_len = batch.get_hashchain_store_len();
        let mut hashchain_store: BoundedVec<[u8; 32]> =
            BoundedVec::with_capacity(hashchain_store_len);
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
        let mut batch = get_test_batch();

        let value = [1u8; 32];
        let mut store = vec![0u8; 20_000];
        let hashchain_store_len = batch.get_hashchain_store_len();
        let mut hashchain_store: BoundedVec<[u8; 32]> =
            BoundedVec::with_capacity(hashchain_store_len);

        assert!(batch.check_non_inclusion(&value, &mut store).is_ok());
        let ref_batch = get_test_batch();
        assert_eq!(batch, ref_batch);
        batch
            .insert(&value, &value, &mut store, &mut hashchain_store)
            .unwrap();
        assert!(batch.check_non_inclusion(&value, &mut store).is_err());
    }

    #[test]
    fn test_getters() {
        let mut batch = get_test_batch();
        assert_eq!(batch.get_num_zkp_batches(), 5);
        assert_eq!(batch.get_hashchain_store_len(), 5);
        assert_eq!(batch.get_state(), BatchState::CanBeFilled);
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
        assert_eq!(
            batch
                .value_is_inserted_in_batch(lowest_eligible_value - 1)
                .unwrap(),
            false
        );
        // 2. Functional test lowest value in eligble range
        assert_eq!(
            batch
                .value_is_inserted_in_batch(lowest_eligible_value)
                .unwrap(),
            true
        );
        // 3. Functional test highest value in eligble range
        assert_eq!(
            batch
                .value_is_inserted_in_batch(highest_eligible_value)
                .unwrap(),
            true
        );
        // 4. Failing test eligble range + 1
        assert_eq!(
            batch
                .value_is_inserted_in_batch(highest_eligible_value + 1)
                .unwrap(),
            false
        );
    }

    /// 1. Failing: empty batch
    /// 2. Functional: if zkp batch size is full else failing
    /// 3. Failing: batch is completely inserted
    #[test]
    fn test_can_insert_batch() {
        let mut batch = get_test_batch();
        assert_eq!(
            batch.get_first_ready_zkp_batch(),
            Err(BatchedMerkleTreeError::BatchNotReady.into())
        );
        let mut bounded_vec = BoundedVec::with_capacity(batch.batch_size as usize);
        let mut value_store = BoundedVec::with_capacity(batch.batch_size as usize);

        for i in 0..batch.batch_size + 10 {
            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            if i < batch.batch_size {
                batch
                    .store_and_hash_value(&value, &mut value_store, &mut bounded_vec)
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
                    Err(BatchedMerkleTreeError::BatchAlreadyInserted.into())
                );
            } else {
                assert_eq!(
                    batch.get_first_ready_zkp_batch(),
                    Err(BatchedMerkleTreeError::BatchNotReady.into())
                );
            }
        }
    }
}
