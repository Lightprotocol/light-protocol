use anchor_lang::prelude::*;
use light_bloom_filter::BloomFilter;
use light_bounded_vec::{BoundedVec, BoundedVecError};
use light_hasher::{Hasher, Poseidon};

use crate::errors::AccountCompressionErrorCode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    pub id: u8,
    pub bloomfilter_store_id: u8,
    pub value_store_id: u8,
    pub num_iters: u64,
    pub bloomfilter_capacity: u64,
    pub value_capacity: u64,
    // TODO: make private
    pub num_inserted: u64,
    pub user_hash_chain: [u8; 32],
    /// To enable update of the batch in multiple proofs the prover hash chain
    /// is used to save intermediate state.
    pub prover_hash_chain: [u8; 32],
    pub is_inserted: bool,
}

impl Batch {
    /// Batch has been marked as ready to update the tree.
    pub fn is_ready_to_update_tree(&self) -> bool {
        println!("Num inserted: {:?}", self.num_inserted);
        println!("Value capacity: {:?}", self.value_capacity);
        println!("Is inserted: {:?}", self.is_inserted);
        self.num_inserted == self.value_capacity && !self.is_inserted
    }

    /// It is possible to add values to the batch:
    /// 1. If the batch is not ready to update the tree.
    /// 2. If the sequence number is greater than the current sequence number.
    pub fn can_be_filled(&mut self) -> (bool, bool) {
        let can_be_filled = !self.is_ready_to_update_tree();
        let wipe_bloomfilter = if self.is_inserted && self.num_inserted == self.value_capacity {
            // self.is_inserted = false;
            self.num_inserted = 0;
            true
        } else {
            false
        };
        (can_be_filled, wipe_bloomfilter)
    }

    /// Inserts values into the bloom filter, stores value in values array and hashes the value.
    /// (Used by Address Queue)
    pub fn insert_and_store(
        &mut self,
        value: &[u8; 32],
        store: &mut [u8],
        value_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        self.insert(value, store)?;
        self.store_value(value, value_store)
    }

    /// Used directly by output queue.
    pub fn store_and_hash(
        &mut self,
        value: &[u8; 32],
        value_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        match self.store_value(value, value_store) {
            Ok(_) => self.add_to_hash_chain(value),
            Err(err) => {
                if ProgramError::from(err) == BoundedVecError::Full.into() {
                    return err!(AccountCompressionErrorCode::BloomFilterFull);
                } else {
                    return err!(AccountCompressionErrorCode::BatchInsertFailed);
                }
            }
        }?;
        Ok(())
    }

    pub fn store_value(
        &mut self,
        value: &[u8; 32],
        value_store: &mut BoundedVec<[u8; 32]>,
    ) -> Result<()> {
        value_store.push(*value).map_err(ProgramError::from)?;

        Ok(())
    }

    /// Inserts into the bloom filter and hashes the value.
    /// (used by input/nullifier queue)
    pub fn insert(&mut self, value: &[u8; 32], store: &mut [u8]) -> Result<()> {
        println!("Inserting value: {:?}", value);
        println!("Num iters: {:?}", self.num_iters);
        println!("Capacity: {:?}", self.bloomfilter_capacity);
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloomfilter_capacity, store)
                .map_err(ProgramError::from)?;
        bloom_filter.insert(value).map_err(ProgramError::from)?;
        self.add_to_hash_chain(value)?;

        Ok(())
    }

    /// Adds a value to the hash chain so that it can be used in the batch
    /// update zkp.
    pub fn add_to_hash_chain(&mut self, value: &[u8; 32]) -> Result<()> {
        self.user_hash_chain =
            Poseidon::hashv(&[self.user_hash_chain.as_slice(), value.as_slice()])
                .map_err(ProgramError::from)?;
        self.num_inserted += 1;
        println!("num inserted: {:?}", self.num_inserted);
        if self.num_inserted == self.value_capacity {
            self.is_inserted = false;
        }
        Ok(())
    }

    pub fn get_num_inserted(&self) -> u64 {
        self.num_inserted
    }

    /// Inserts into the bloom filter and hashes the value.
    /// (used by nullifier queue)
    pub fn check_non_inclusion(&mut self, value: &[u8; 32], store: &mut [u8]) -> Result<()> {
        let mut bloom_filter =
            BloomFilter::new(self.num_iters as usize, self.bloomfilter_capacity, store)
                .map_err(ProgramError::from)?;
        if bloom_filter.contains(value) {
            msg!("Value already exists in the bloom filter.");
            return err!(AccountCompressionErrorCode::BatchInsertFailed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_batch() -> Batch {
        Batch {
            id: 1,
            bloomfilter_store_id: 1,
            value_store_id: 1,
            num_iters: 3,
            bloomfilter_capacity: 160_000,
            num_inserted: 0,
            user_hash_chain: [0u8; 32],
            prover_hash_chain: [0u8; 32],
            value_capacity: 500,
            is_inserted: true,
        }
    }

    #[test]
    fn test_is_ready_to_update_tree() {
        let mut batch = get_test_batch();
        assert!(!batch.is_ready_to_update_tree());
        batch.num_inserted = batch.value_capacity;
        batch.is_inserted = false;
        assert!(batch.is_ready_to_update_tree());
    }

    // #[test]
    // fn test_can_be_filled() {
    //     let mut batch = get_test_batch();
    //     assert!(batch.can_be_filled(0));
    //     batch.mark_with_sequence_number(1, 5);
    //     assert_eq!(batch.sequence_number, 6);
    //     assert!(batch.can_be_filled(6));
    //     assert!(!batch.can_be_filled(4));
    // }

    #[test]
    fn test_insert_and_store() {
        // Behavior Address queue
        let mut batch = get_test_batch();
        let mut store = vec![0u8; 20_000];
        let mut value_store = BoundedVec::with_capacity(batch.value_capacity as usize);

        let mut ref_batch = get_test_batch();
        for i in 0..batch.value_capacity {
            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            let ref_hash_chain = Poseidon::hashv(&[&batch.user_hash_chain, &value]).unwrap();
            assert!(batch
                .insert_and_store(&value, &mut store, &mut value_store)
                .is_ok());
            let mut bloomfilter = BloomFilter {
                num_iters: batch.num_iters as usize,
                capacity: batch.bloomfilter_capacity,
                store: &mut store,
            };
            assert!(bloomfilter.contains(&value));
            ref_batch.num_inserted += 1;
            ref_batch.user_hash_chain = ref_hash_chain;
            if i == batch.value_capacity - 1 {
                ref_batch.is_inserted = false;
            }
            assert_eq!(batch, ref_batch);
            assert_eq!(*value_store.get(i as usize).unwrap(), value);
        }
    }

    #[test]
    fn test_store_and_hash() {
        // Behavior Output queue
        let mut batch = get_test_batch();
        let mut value_store = BoundedVec::with_capacity(batch.value_capacity as usize);

        let mut ref_batch = get_test_batch();
        for i in 0..batch.value_capacity {
            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            let ref_hash_chain = Poseidon::hashv(&[&batch.user_hash_chain, &value]).unwrap();
            assert!(batch.store_and_hash(&value, &mut value_store).is_ok());

            ref_batch.num_inserted += 1;
            ref_batch.user_hash_chain = ref_hash_chain;
            if i == batch.value_capacity - 1 {
                ref_batch.is_inserted = false;
            }
            assert_eq!(batch, ref_batch);
            assert_eq!(*value_store.get(i as usize).unwrap(), value);
        }
        assert!(batch.is_ready_to_update_tree());
        let value = [0u8; 32];
        assert!(matches!(
            batch.store_and_hash(&value, &mut value_store),
            Err(error) if error ==  AccountCompressionErrorCode::BloomFilterFull.into()
        ));
    }

    #[test]
    fn test_store_value() {
        let mut batch = get_test_batch();

        let value = [1u8; 32];
        let mut value_store = BoundedVec::with_capacity(batch.value_capacity as usize);

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

        let mut ref_batch = get_test_batch();
        for i in 0..batch.value_capacity {
            let mut value = [0u8; 32];
            value[24..].copy_from_slice(&i.to_be_bytes());
            let ref_hash_chain = Poseidon::hashv(&[&batch.user_hash_chain, &value]).unwrap();
            assert!(batch.insert(&value, &mut store).is_ok());
            let mut bloomfilter = BloomFilter {
                num_iters: batch.num_iters as usize,
                capacity: batch.bloomfilter_capacity,
                store: &mut store,
            };
            assert!(bloomfilter.contains(&value));
            ref_batch.num_inserted += 1;
            ref_batch.user_hash_chain = ref_hash_chain;
            if i == batch.value_capacity - 1 {
                ref_batch.is_inserted = false;
            }
            assert_eq!(batch, ref_batch);
        }
    }

    #[test]
    fn test_add_to_hash_chain() {
        let mut batch = get_test_batch();

        let value = [1u8; 32];

        assert!(batch.add_to_hash_chain(&value).is_ok());
        let mut ref_batch = get_test_batch();
        let user_hash_chain = Poseidon::hashv(&[&[0u8; 32], &value]).unwrap();
        ref_batch.user_hash_chain = user_hash_chain;
        ref_batch.num_inserted = 1;
        assert_eq!(batch, ref_batch);
    }

    #[test]
    fn test_check_non_inclusion() {
        let mut batch = get_test_batch();

        let value = [1u8; 32];
        let mut store = vec![0u8; 20_000];

        assert!(batch.check_non_inclusion(&value, &mut store).is_ok());
        let ref_batch = get_test_batch();
        assert_eq!(batch, ref_batch);
        batch.insert(&value, &mut store).unwrap();
        assert!(batch.check_non_inclusion(&value, &mut store).is_err());
    }
}
