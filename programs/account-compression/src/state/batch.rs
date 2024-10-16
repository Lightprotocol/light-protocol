use crate::errors::AccountCompressionErrorCode;
use anchor_lang::prelude::*;
use light_bloom_filter::BloomFilter;
use light_bounded_vec::BoundedVec;
use light_hasher::{Hasher, Poseidon};

#[repr(u64)]
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum BatchState {
    /// Batch can be filled with values.
    CanBeFilled,
    /// Batch has been inserted into the tree.
    Inserted,
    /// Batch is ready to be inserted into the tree. Possibly it is already
    /// partially inserted into the tree.
    ReadyToUpdateTree,
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
    // TODO: don't zero out roots completely but just overwrite one non-zero
    // byte to zero
    pub root_index: u32,
}

impl Batch {
    pub fn new(
        num_iters: u64,
        bloom_filter_capacity: u64,
        batch_size: u64,
        zkp_batch_size: u64,
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
        }
    }

    pub fn get_state(&self) -> BatchState {
        self.state
    }

    /// fill -> ready -> inserted -> fill
    pub fn advance_state_to_can_be_filled(&mut self) -> Result<()> {
        if self.state == BatchState::Inserted {
            self.state = BatchState::CanBeFilled;
        } else {
            msg!(
                "Batch is in incorrect state {} expected Inserted 3",
                self.state as u64
            );
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }
        Ok(())
    }

    /// fill -> ready -> inserted -> fill
    pub fn advance_state_to_inserted(&mut self) -> Result<()> {
        if self.state == BatchState::ReadyToUpdateTree {
            self.state = BatchState::Inserted;
        } else {
            msg!(
                "Batch is in incorrect state {} expected ReadyToUpdateTree 2",
                self.state as u64
            );
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }
        Ok(())
    }

    /// fill -> ready -> inserted -> fill
    pub fn advance_state_to_ready_to_update_tree(&mut self) -> Result<()> {
        if self.state == BatchState::CanBeFilled {
            self.state = BatchState::ReadyToUpdateTree;
        } else {
            msg!(
                "Batch is in incorrect state {} expected ReadyToUpdateTree 2",
                self.state as u64
            );
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }
        Ok(())
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

    pub fn store_value(
        &mut self,
        value: &[u8; 32],
        value_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        if self.state != BatchState::CanBeFilled {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }
        value_store.push(*value).map_err(ProgramError::from)?;

        if self.num_inserted == self.zkp_batch_size || self.num_inserted == 0 {
            self.num_inserted = 0;
        }
        self.finalize_insert()
    }

    /// Inserts into the bloom filter and hashes the value.
    /// (used by input/nullifier queue)
    pub fn insert(
        &mut self,
        bloom_filter_value: &[u8; 32],
        hashchain_value: &[u8; 32],
        store: &mut [u8],
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloom_filter_capacity, store)
                .map_err(ProgramError::from)?;
        bloom_filter
            .insert(bloom_filter_value)
            .map_err(ProgramError::from)?;
        self.add_to_hash_chain(hashchain_value, hashchain_store)?;
        self.finalize_insert()
    }

    pub fn add_to_hash_chain(
        &mut self,
        value: &[u8; 32],
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        if self.num_inserted == self.zkp_batch_size || self.num_inserted == 0 {
            hashchain_store.push(*value).map_err(ProgramError::from)?;
            self.num_inserted = 0;
        } else if let Some(last_hashchain) = hashchain_store.last() {
            let hashchain =
                Poseidon::hashv(&[last_hashchain, value.as_slice()]).map_err(ProgramError::from)?;
            *hashchain_store.last_mut().unwrap() = hashchain;
        }
        Ok(())
    }

    pub fn finalize_insert(&mut self) -> Result<()> {
        self.num_inserted += 1;
        if self.num_inserted == self.zkp_batch_size {
            self.current_zkp_batch_index += 1;
        }
        if self.get_num_zkp_batches() == self.current_zkp_batch_index {
            self.advance_state_to_ready_to_update_tree()?;
            self.num_inserted = 0;
        }
        Ok(())
    }

    /// Inserts into the bloom filter and hashes the value.
    /// (used by nullifier queue)
    pub fn check_non_inclusion(&mut self, value: &[u8; 32], store: &mut [u8]) -> Result<()> {
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloom_filter_capacity, store)
                .map_err(ProgramError::from)?;
        if bloom_filter.contains(value) {
            msg!("Value already exists in the bloom filter.");
            return err!(AccountCompressionErrorCode::BatchInsertFailed);
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
    ) -> Result<()> {
        if self.state != BatchState::ReadyToUpdateTree {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }
        let num_zkp_batches = self.get_num_zkp_batches();

        self.num_inserted_zkps += 1;

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

    /// Check if the value is inserted in the merkle tree.
    pub fn value_is_inserted_in_merkle_tree(&self, value_index: u64) -> bool {
        let last_inserted_index = self.get_current_zkp_batch_index() * self.zkp_batch_size;
        value_index >= last_inserted_index
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn get_test_batch() -> Batch {
        Batch::new(3, 160_000, 500, 100)
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
                assert_eq!(batch.get_state(), BatchState::ReadyToUpdateTree);
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

        let mut ref_batch = get_test_batch();
        for i in 0..batch.batch_size {
            ref_batch.num_inserted %= ref_batch.zkp_batch_size;

            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            assert!(batch.store_value(&value, &mut value_store).is_ok());
            ref_batch.num_inserted += 1;
            if ref_batch.num_inserted == ref_batch.zkp_batch_size {
                ref_batch.current_zkp_batch_index += 1;
            }
            if ref_batch.current_zkp_batch_index == ref_batch.get_num_zkp_batches() {
                ref_batch.state = BatchState::ReadyToUpdateTree;
                ref_batch.num_inserted = 0;
            }
            assert_eq!(batch, ref_batch);
            assert_eq!(*value_store.get(i as usize).unwrap(), value);
        }
        let result = batch.store_value(&[1u8; 32], &mut value_store);
        assert_eq!(
            result.unwrap_err(),
            AccountCompressionErrorCode::BatchNotReady.into()
        );
        assert_eq!(batch.get_state(), BatchState::ReadyToUpdateTree);
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
                ref_batch.state = BatchState::ReadyToUpdateTree;
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
        assert!(batch.finalize_insert().is_ok());
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
        assert!(batch.finalize_insert().is_ok());

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
        batch.advance_state_to_ready_to_update_tree().unwrap();
        assert_eq!(batch.get_state(), BatchState::ReadyToUpdateTree);
        batch.advance_state_to_inserted().unwrap();
        assert_eq!(batch.get_state(), BatchState::Inserted);
    }
}
