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
    /// Number of iterations for the bloomfilter.
    pub num_iters: u64,
    /// Theoretical capacity of the bloomfilter. We want to make it much larger
    /// than batch_size to avoid false positives.
    pub bloomfilter_capacity: u64,
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
        bloomfilter_capacity: u64,
        batch_size: u64,
        zkp_batch_size: u64,
    ) -> Self {
        Batch {
            num_iters,
            bloomfilter_capacity,
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
        value_store.push(*value).map_err(ProgramError::from)?;
        if self.num_inserted == self.zkp_batch_size || self.num_inserted == 0 {
            self.num_inserted = 0;
        }
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
    /// (used by input/nullifier queue)
    pub fn insert(
        &mut self,
        bloomfilter_value: &[u8; 32],
        hashchain_value: &[u8; 32],
        store: &mut [u8],
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloomfilter_capacity, store)
                .map_err(ProgramError::from)?;
        msg!("blooom filter created");
        bloom_filter
            .insert(bloomfilter_value)
            .map_err(ProgramError::from)?;
        msg!("value inserted into bloom filter");

        self.add_to_hash_chain(hashchain_value, hashchain_store)?;
        Ok(())
    }

    pub fn add_to_hash_chain(
        &mut self,
        value: &[u8; 32],
        hashchain_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        println!(
            "add value to hashchain {}: {:?}",
            hashchain_store.len(),
            value
        );
        if self.num_inserted == self.zkp_batch_size || self.num_inserted == 0 {
            hashchain_store.push(*value).map_err(ProgramError::from)?;
            self.num_inserted = 0;
        } else if let Some(last_hashchain) = hashchain_store.last() {
            let hashchain =
                Poseidon::hashv(&[last_hashchain, value.as_slice()]).map_err(ProgramError::from)?;
            *hashchain_store.last_mut().unwrap() = hashchain;
        }

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
            BloomFilter::new(self.num_iters as usize, self.bloomfilter_capacity, store)
                .map_err(ProgramError::from)?;
        if bloom_filter.contains(value) {
            msg!("Value already exists in the bloom filter.");
            println!("value already exists in the bloom filter.");
            return err!(AccountCompressionErrorCode::BatchInsertFailed);
        }
        Ok(())
    }

    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }
    // TODO: rename
    pub fn mark_as_inserted(
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
        println!("num_zkp_batches: {}", num_zkp_batches);
        println!("num_inserted_zkps: {}", self.num_inserted_zkps);

        // Batch has been successfully inserted into the tree.
        if self.num_inserted_zkps == num_zkp_batches {
            self.current_zkp_batch_index = 0;
            self.state = BatchState::Inserted;
            msg!("Batch marked as inserted into the tree.");
            self.num_inserted_zkps = 0;
            // Saving sequence number and root index for the batch.
            // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
            // if not advance current root index to root index
            self.sequence_number = sequence_number - 1 + root_history_length as u64;
            self.root_index = root_index;
        }

        Ok(())
    }

    pub fn get_hashchain_store_len(&self) -> usize {
        self.batch_size as usize / self.zkp_batch_size as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_batch() -> Batch {
        Batch::new(3, 160_000, 500, 100)
    }

    #[test]
    fn test_store_value() {
        let mut batch = get_test_batch();

        let value = [1u8; 32];
        let mut value_store = BoundedVec::with_capacity(batch.batch_size as usize);

        assert!(batch.store_value(&value, &mut value_store).is_ok());
        let ref_batch = get_test_batch();
        assert_eq!(batch, ref_batch);
        assert_eq!(*value_store.get(0).unwrap(), value);
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
            let mut bloomfilter = BloomFilter {
                num_iters: batch.num_iters as usize,
                capacity: batch.bloomfilter_capacity,
                store: &mut store,
            };
            assert!(bloomfilter.contains(&value));
            ref_batch.num_inserted += 1;
            assert_eq!(*hashchain_store.last().unwrap(), ref_hash_chain);
            if ref_batch.num_inserted == ref_batch.zkp_batch_size {
                ref_batch.current_zkp_batch_index += 1;
                ref_batch.num_inserted = 0;
            }
            if i == batch.batch_size - 1 {
                ref_batch.state = BatchState::ReadyToUpdateTree;
            }
            assert_eq!(batch, ref_batch);
        }
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

    // TODO: test for different value and hashchain value
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
}
