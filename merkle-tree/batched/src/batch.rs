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
