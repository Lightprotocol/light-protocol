use std::ops::{Deref, DerefMut};

use aligned_sized::aligned_sized;
use light_hasher::Discriminator;
use light_merkle_tree_metadata::{
    errors::MerkleTreeMetadataError,
    queue::{QueueMetadata, QueueType},
};
use light_utils::{
    account::{check_account_info, set_discriminator, DISCRIMINATOR_LEN},
    hash_to_bn254_field_size_be,
    pubkey::Pubkey,
};
use light_zero_copy::{errors::ZeroCopyError, vec::ZeroCopyVecU64};
use solana_program::{account_info::AccountInfo, msg};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref};

use super::batch::BatchState;
use crate::{
    batch::Batch,
    batch_metadata::BatchMetadata,
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, NUM_BATCHES, OUTPUT_QUEUE_TYPE, TEST_DEFAULT_BATCH_SIZE,
    },
    errors::BatchedMerkleTreeError,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    BorshDeserialize, BorshSerialize,
};

#[repr(C)]
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    Copy,
    KnownLayout,
    Immutable,
    FromBytes,
    IntoBytes,
)]
#[aligned_sized(anchor)]
pub struct BatchedQueueMetadata {
    pub metadata: QueueMetadata,
    pub batch_metadata: BatchMetadata,
    /// Maximum number of leaves that can fit in the tree, calculated as 2^height.
    /// For example, a tree with height 3 can hold up to 8 leaves.
    pub tree_capacity: u64,
    pub hashed_merkle_tree_pubkey: [u8; 32],
    pub hashed_queue_pubkey: [u8; 32],
}

impl BatchedQueueMetadata {
    pub fn init(
        &mut self,
        meta_data: QueueMetadata,
        batch_size: u64,
        zkp_batch_size: u64,
        bloom_filter_capacity: u64,
        num_iters: u64,
        queue_pubkey: &Pubkey,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.metadata = meta_data;
        self.batch_metadata.init(batch_size, zkp_batch_size)?;
        self.batch_metadata.bloom_filter_capacity = bloom_filter_capacity;
        for (i, batches) in self.batch_metadata.batches.iter_mut().enumerate() {
            *batches = Batch::new(
                num_iters,
                bloom_filter_capacity,
                batch_size,
                zkp_batch_size,
                batch_size * (i as u64),
            );
        }
        println!(
            "merkle tree pubkey key: {:?}",
            meta_data.associated_merkle_tree
        );
        self.hashed_merkle_tree_pubkey =
            hash_to_bn254_field_size_be(&meta_data.associated_merkle_tree.to_bytes())
                .unwrap()
                .0;
        println!("hashed mt {:?}", self.hashed_merkle_tree_pubkey);
        println!("queue_pubkey pubkey key: {:?}", queue_pubkey);
        self.hashed_queue_pubkey = hash_to_bn254_field_size_be(&queue_pubkey.to_bytes())
            .unwrap()
            .0;
        println!("hashed queue {:?}", self.hashed_queue_pubkey);
        Ok(())
    }
}

/// Batched queue zero copy account.
/// Used for output queues in light protocol.
/// Output queues store compressed account hashes,
/// to be appended to a batched Merkle tree
/// in batches with a zero-knowledge proof (zkp),
/// ie. it stores hashes and commits these to hash chains.
/// Each hash chain is used as public input for
/// a batch append zkp.
///
/// An output queue is configured with:
/// 1. N batches
/// 2. N value vecs (one for each batch)
///     value vec length = batch size
/// 3. N hashchain stores (one for each batch)
///     hashchain store length = batch size /zkp batch size
///
/// Default config:
/// - 2 batches
/// - 50,000 batch size
/// - 500 zkp batch size
///
/// Initialization:
/// - an output queue is initialized
///     in combination with a state Merkle tree
/// - `init_batched_state_merkle_tree_from_account_info`
///
/// For deserialization use:
/// - in program:   `output_from_account_info`
/// - in client:    `output_from_bytes`
///
/// To insert a value use:
/// - `insert_into_current_batch`
///
/// A compressed account can be spent or read
/// while in the output queue.
///
/// To spend use:
/// - check_leaf_index_could_exist_in_batches in combination with
///     `prove_inclusion_by_index_and_zero_out_leaf`
///
/// To read use:
/// - `prove_inclusion_by_index`
#[derive(Debug, PartialEq)]
pub struct BatchedQueueAccount<'a> {
    pubkey: Pubkey,
    metadata: Ref<&'a mut [u8], BatchedQueueMetadata>,
    pub value_vecs: [ZeroCopyVecU64<'a, [u8; 32]>; 2],
    pub hash_chain_stores: [ZeroCopyVecU64<'a, [u8; 32]>; 2],
}

impl Discriminator for BatchedQueueAccount<'_> {
    const DISCRIMINATOR: [u8; 8] = *b"queueacc";
}

impl<'a> BatchedQueueAccount<'a> {
    /// Deserialize an output BatchedQueueAccount from account info.
    /// Should be used in solana programs.
    /// Checks that:
    /// 1. the program owner is the light account compression program,
    /// 2. discriminator,
    /// 3. queue type is output queue type.
    pub fn output_from_account_info(
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        Self::from_account_info::<OUTPUT_QUEUE_TYPE>(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_info)
    }

    /// Deserialize a BatchedQueueAccount from account info.
    /// Should be used in solana programs.
    /// Checks the program owner, discriminator and queue type.
    fn from_account_info<const QUEUE_TYPE: u64>(
        program_id: &solana_program::pubkey::Pubkey,
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        check_account_info::<Self>(program_id, account_info)?;
        let account_data = &mut account_info.try_borrow_mut_data()?;
        // Necessary to convince the borrow checker.
        let account_data: &'a mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(account_data.as_mut_ptr(), account_data.len())
        };
        Self::from_bytes::<OUTPUT_QUEUE_TYPE>(account_data, (*account_info.key).into())
    }

    /// Deserialize a BatchedQueueAccount from bytes.
    /// Should only be used in client.
    /// Checks the discriminator and queue type.
    #[cfg(not(target_os = "solana"))]
    pub fn output_from_bytes(
        account_data: &'a mut [u8],
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        light_utils::account::check_discriminator::<BatchedQueueAccount>(
            &account_data[..DISCRIMINATOR_LEN],
        )?;
        Self::from_bytes::<OUTPUT_QUEUE_TYPE>(account_data, Pubkey::default())
    }

    fn from_bytes<const QUEUE_TYPE: u64>(
        account_data: &'a mut [u8],
        pubkey: Pubkey,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        let (_discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        let (metadata, account_data) =
            Ref::<&'a mut [u8], BatchedQueueMetadata>::from_prefix(account_data)
                .map_err(ZeroCopyError::from)?;

        if metadata.metadata.queue_type != QUEUE_TYPE {
            return Err(MerkleTreeMetadataError::InvalidQueueType.into());
        }

        let (value_vec1, account_data) = ZeroCopyVecU64::from_bytes_at(account_data)?;
        let (value_vec2, account_data) = ZeroCopyVecU64::from_bytes_at(account_data)?;

        let (hashchain_store1, account_data) = ZeroCopyVecU64::from_bytes_at(account_data)?;
        let (hashchain_store2, _account_data) = ZeroCopyVecU64::from_bytes_at(account_data)?;

        Ok(BatchedQueueAccount {
            pubkey,
            metadata,
            value_vecs: [value_vec1, value_vec2],
            hash_chain_stores: [hashchain_store1, hashchain_store2],
        })
    }

    pub fn init(
        account_data: &'a mut [u8],
        metadata: QueueMetadata,
        output_queue_batch_size: u64,
        output_queue_zkp_batch_size: u64,
        num_iters: u64,
        bloom_filter_capacity: u64,
        pubkey: Pubkey,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        let account_data_len = account_data.len();
        let (discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        set_discriminator::<Self>(discriminator)?;

        let (mut account_metadata, account_data) =
            Ref::<&mut [u8], BatchedQueueMetadata>::from_prefix(account_data)
                .map_err(ZeroCopyError::from)?;

        account_metadata.init(
            metadata,
            output_queue_batch_size,
            output_queue_zkp_batch_size,
            bloom_filter_capacity,
            num_iters,
            &pubkey,
        )?;

        if account_data_len
            != account_metadata
                .batch_metadata
                .queue_account_size(account_metadata.metadata.queue_type)?
        {
            msg!("account_data.len() {:?}", account_data_len);
            msg!(
                "queue_account_size {:?}",
                account_metadata
                    .batch_metadata
                    .queue_account_size(account_metadata.metadata.queue_type)?
            );
            return Err(ZeroCopyError::Size.into());
        }

        let value_vec_capacity = account_metadata.batch_metadata.batch_size;
        let hash_chain_capacity = account_metadata.batch_metadata.get_num_zkp_batches();
        let (value_vecs_1, account_data) =
            ZeroCopyVecU64::new_at(value_vec_capacity, account_data)?;
        let (value_vecs_2, account_data) =
            ZeroCopyVecU64::new_at(value_vec_capacity, account_data)?;
        let (vec_1, account_data) = ZeroCopyVecU64::new_at(hash_chain_capacity, account_data)?;
        let (vec_2, _) = ZeroCopyVecU64::new_at(hash_chain_capacity, account_data)?;
        Ok(BatchedQueueAccount {
            pubkey,
            metadata: account_metadata,
            value_vecs: [value_vecs_1, value_vecs_2],
            hash_chain_stores: [vec_1, vec_2],
        })
    }

    /// Insert a value into the current batch
    /// of this output queue account.
    /// 1. insert value into a value vec and hash chain store.
    /// 2. Increment next_index.
    pub fn insert_into_current_batch(
        &mut self,
        hash_chain_value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let current_index = self.batch_metadata.next_index;

        insert_into_current_batch(
            self.metadata.metadata.queue_type,
            &mut self.metadata.batch_metadata,
            &mut self.value_vecs,
            &mut [],
            &mut self.hash_chain_stores,
            hash_chain_value,
            None,
            Some(current_index),
        )?;
        self.metadata.batch_metadata.next_index += 1;

        Ok(())
    }

    /// Proves inclusion of leaf index if it exists in one of the batches.
    /// 1. Iterate over all batches
    /// 2. Check if leaf index could exist in the batch.
    ///     2.1 If yes, check whether value at index is equal to hash_chain_value.
    ///         Throw error if not.
    /// 3. Return true if leaf index exists in one of the batches.
    ///
    /// Note, this method does not fail but returns `false`
    ///     if the leaf index is out of range for any batch.
    pub fn prove_inclusion_by_index(
        &mut self,
        leaf_index: u64,
        hash_chain_value: &[u8; 32],
    ) -> Result<bool, BatchedMerkleTreeError> {
        for (batch_index, batch) in self.batch_metadata.batches.iter().enumerate() {
            if batch.leaf_index_exists(leaf_index) {
                let index = batch.get_value_index_in_batch(leaf_index)?;
                let element = self.value_vecs[batch_index]
                    .get(index as usize)
                    .ok_or(BatchedMerkleTreeError::InclusionProofByIndexFailed)?;

                if *element == *hash_chain_value {
                    return Ok(true);
                } else {
                    return Err(BatchedMerkleTreeError::InclusionProofByIndexFailed);
                }
            }
        }
        Ok(false)
    }

    /// Zero out a leaf by index if it exists in the queues hash_chain_value vec. If
    /// checked fail if leaf is not found.
    pub fn prove_inclusion_by_index_and_zero_out_leaf(
        &mut self,
        leaf_index: u64,
        hash_chain_value: &[u8; 32],
        prove_by_index: bool,
    ) -> Result<(), BatchedMerkleTreeError> {
        for (batch_index, batch) in self.batch_metadata.batches.iter().enumerate() {
            if batch.leaf_index_exists(leaf_index) {
                let index = batch.get_value_index_in_batch(leaf_index)?;
                let element = self.value_vecs[batch_index]
                    .get_mut(index as usize)
                    .ok_or(BatchedMerkleTreeError::InclusionProofByIndexFailed)?;

                if element == hash_chain_value {
                    *element = [0u8; 32];
                    return Ok(());
                } else {
                    return Err(BatchedMerkleTreeError::InclusionProofByIndexFailed);
                }
            }
        }
        // Always check and zero out an existing value.
        // If no value is found and a check is not enforced return ok.
        if prove_by_index {
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        } else {
            Ok(())
        }
    }

    pub fn get_metadata(&self) -> &BatchedQueueMetadata {
        &self.metadata
    }

    pub fn get_metadata_mut(&mut self) -> &mut BatchedQueueMetadata {
        &mut self.metadata
    }

    /// Returns the number of elements inserted in the current batch.
    /// If current batch state is inserted, returns 0.
    pub fn get_num_inserted_in_current_batch(&self) -> u64 {
        let current_batch = self.batch_metadata.currently_processing_batch_index as usize;
        self.batch_metadata.batches[current_batch].get_num_inserted_elements()
    }

    /// Returns true if the pubkey is the associated Merkle tree of the queue.
    pub fn is_associated(&self, pubkey: &Pubkey) -> bool {
        self.metadata.metadata.associated_merkle_tree == *pubkey
    }

    /// Check if the pubkey is the associated Merkle tree of the queue.
    pub fn check_is_associated(&self, pubkey: &Pubkey) -> Result<(), BatchedMerkleTreeError> {
        if !self.is_associated(pubkey) {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into());
        }
        Ok(())
    }

    /// Returns true if the tree is full.
    pub fn tree_is_full(&self) -> bool {
        self.tree_capacity == self.batch_metadata.next_index
    }

    /// Check if the tree is full.
    pub fn check_tree_is_full(&self) -> Result<(), BatchedMerkleTreeError> {
        if self.tree_is_full() {
            return Err(BatchedMerkleTreeError::TreeIsFull);
        }
        Ok(())
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }
}

impl Deref for BatchedQueueAccount<'_> {
    type Target = BatchedQueueMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl DerefMut for BatchedQueueAccount<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

/// Insert a value into the current batch.
/// - Input&address queues: Insert into bloom filter & hash chain.
/// - Output queue: Insert into value vec & hash chain.
///
/// Steps:
/// 1. Check if the current batch is ready.
///     1.1. If the current batch is inserted, clear the batch.
/// 2. Insert value into the current batch.
/// 3. If batch is full, increment currently_processing_batch_index.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(crate) fn insert_into_current_batch(
    queue_type: u64,
    batch_metadata: &mut BatchMetadata,
    value_vecs: &mut [ZeroCopyVecU64<[u8; 32]>],
    bloom_filter_stores: &mut [&mut [u8]],
    hash_chain_stores: &mut [ZeroCopyVecU64<[u8; 32]>],
    hash_chain_value: &[u8; 32],
    bloom_filter_value: Option<&[u8; 32]>,
    current_index: Option<u64>,
) -> Result<(), BatchedMerkleTreeError> {
    let batch_index = batch_metadata.currently_processing_batch_index as usize;
    let mut value_store = value_vecs.get_mut(batch_index);
    let mut hash_chain_stores = hash_chain_stores.get_mut(batch_index);
    let current_batch = batch_metadata.get_current_batch_mut();
    // 1. Check that the current batch is ready.
    //      1.1. If the current batch is inserted, clear the batch.
    {
        let clear_batch = current_batch.get_state() == BatchState::Inserted;
        if current_batch.get_state() == BatchState::Fill {
            // Do nothing, checking most often case first.
        } else if clear_batch {
            // Clear the batch if it is inserted.

            // If a batch contains a bloom filter it must be zeroed by a forester.
            if !bloom_filter_stores.is_empty() && !current_batch.bloom_filter_is_zeroed() {
                return Err(BatchedMerkleTreeError::BloomFilterNotZeroed);
            }
            if let Some(value_store) = value_store.as_mut() {
                (*value_store).clear();
            }
            if let Some(hash_chain_stores) = hash_chain_stores.as_mut() {
                (*hash_chain_stores).clear();
            }
            // Advance the state to fill and reset the number of inserted elements.
            // If Some(current_index) set it as start index.
            // Reset, sequence number, root index, bloom filter zeroed, num_inserted_zkps.
            current_batch.advance_state_to_fill(current_index)?;
        } else {
            // We expect to insert into the current batch.
            for batch in batch_metadata.batches.iter() {
                msg!("batch {:?}", batch);
            }
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
    }

    // 2. Insert value into the current batch.
    let queue_type = QueueType::from(queue_type);
    match queue_type {
        QueueType::BatchedInput | QueueType::BatchedAddress => current_batch.insert(
            bloom_filter_value.unwrap(),
            hash_chain_value,
            bloom_filter_stores,
            hash_chain_stores.as_mut().unwrap(),
            batch_index,
        ),
        QueueType::BatchedOutput => current_batch.store_and_hash_value(
            hash_chain_value,
            value_store.unwrap(),
            hash_chain_stores.unwrap(),
        ),
        _ => Err(MerkleTreeMetadataError::InvalidQueueType.into()),
    }?;

    // 3. If batch is full, increment currently_processing_batch_index.
    batch_metadata.increment_currently_processing_batch_index_if_full();

    Ok(())
}

#[inline(always)]
pub(crate) fn deserialize_bloom_filter_stores(
    bloom_filter_capacity: usize,
    account_data: &mut [u8],
) -> ([&mut [u8]; 2], &mut [u8]) {
    let (slice_1, account_data) = account_data.split_at_mut(bloom_filter_capacity);
    let (slice_2, account_data) = account_data.split_at_mut(bloom_filter_capacity);
    ([slice_1, slice_2], account_data)
}

pub fn get_output_queue_account_size_default() -> usize {
    let batch_metadata = BatchedQueueMetadata {
        metadata: QueueMetadata::default(),
        batch_metadata: BatchMetadata {
            num_batches: NUM_BATCHES as u64,
            batch_size: TEST_DEFAULT_BATCH_SIZE,
            zkp_batch_size: 10,
            ..Default::default()
        },
        ..Default::default()
    };
    batch_metadata
        .batch_metadata
        .queue_account_size(QueueType::BatchedOutput as u64)
        .unwrap()
}

pub fn get_output_queue_account_size_from_params(
    ix_data: InitStateTreeAccountsInstructionData,
) -> usize {
    let metadata = BatchedQueueMetadata {
        metadata: QueueMetadata::default(),
        batch_metadata: BatchMetadata {
            num_batches: NUM_BATCHES as u64,
            batch_size: ix_data.output_queue_batch_size,
            zkp_batch_size: ix_data.output_queue_zkp_batch_size,
            ..Default::default()
        },
        ..Default::default()
    };
    metadata
        .batch_metadata
        .queue_account_size(QueueType::BatchedOutput as u64)
        .unwrap()
}

pub fn get_output_queue_account_size(batch_size: u64, zkp_batch_size: u64) -> usize {
    let metadata = BatchedQueueMetadata {
        metadata: QueueMetadata::default(),
        batch_metadata: BatchMetadata {
            num_batches: NUM_BATCHES as u64,
            batch_size,
            zkp_batch_size,
            ..Default::default()
        },
        ..Default::default()
    };
    metadata
        .batch_metadata
        .queue_account_size(QueueType::BatchedOutput as u64)
        .unwrap()
}

#[cfg(not(target_os = "solana"))]
#[allow(clippy::too_many_arguments)]
pub fn assert_queue_inited(
    batch_metadata: BatchMetadata,
    ref_batch_metadata: BatchMetadata,
    queue_type: u64,
    value_vecs: &mut [ZeroCopyVecU64<'_, [u8; 32]>],
) {
    assert_eq!(
        batch_metadata, ref_batch_metadata,
        "batch_metadata mismatch"
    );

    if queue_type == QueueType::BatchedOutput as u64 {
        assert_eq!(value_vecs.len(), NUM_BATCHES, "value_vecs mismatch");
    } else {
        assert_eq!(value_vecs.len(), 0, "value_vecs mismatch");
    }
    for vec in value_vecs.iter() {
        assert_eq!(
            vec.capacity(),
            batch_metadata.batch_size as usize,
            "batch_size mismatch"
        );
        assert_eq!(vec.len(), 0, "batch_size mismatch");
    }
}

#[cfg(not(target_os = "solana"))]
pub fn assert_queue_zero_copy_inited(account_data: &mut [u8], ref_account: BatchedQueueMetadata) {
    let mut account = BatchedQueueAccount::output_from_bytes(account_data)
        .expect("from_bytes_unchecked_mut failed");
    let batch_metadata = account.batch_metadata;
    let queue_type = account.metadata.metadata.queue_type;
    assert_eq!(
        account.metadata.metadata, ref_account.metadata,
        "metadata mismatch"
    );
    assert_queue_inited(
        batch_metadata,
        ref_account.batch_metadata,
        queue_type,
        &mut account.value_vecs,
    );
}

#[test]
fn test_from_bytes_invalid_tree_type() {
    let mut account_data = vec![0u8; get_output_queue_account_size_default()];
    let account = BatchedQueueAccount::from_bytes::<6>(&mut account_data, Pubkey::default());
    assert_eq!(
        account.unwrap_err(),
        MerkleTreeMetadataError::InvalidQueueType.into()
    );
}

#[test]
fn test_batched_queue_metadata_init() {
    let mut metadata = BatchedQueueMetadata::default();
    let mt_pubkey = Pubkey::new_unique();
    let queue_metadata = QueueMetadata {
        associated_merkle_tree: mt_pubkey,
        ..Default::default()
    };
    let batch_size = 4;
    let zkp_batch_size = 2;
    let bloom_filter_capacity = 10;
    let num_iters = 5;
    let queue_pubkey = Pubkey::new_unique();

    let result = metadata.init(
        queue_metadata,
        batch_size,
        zkp_batch_size,
        bloom_filter_capacity,
        num_iters,
        &queue_pubkey,
    );

    assert!(result.is_ok());
    assert_eq!(metadata.metadata, queue_metadata);
    assert_eq!(
        metadata.batch_metadata.bloom_filter_capacity,
        bloom_filter_capacity
    );
    for (i, batch) in metadata.batch_metadata.batches.iter().enumerate() {
        assert_eq!(batch.num_iters, num_iters);
        assert_eq!(batch.bloom_filter_capacity, bloom_filter_capacity);
        assert_eq!(batch.batch_size, batch_size);
        assert_eq!(batch.zkp_batch_size, zkp_batch_size);
        assert_eq!(batch.start_index, batch_size * (i as u64));
    }
    let hashed_merkle_tree_pubkey = hash_to_bn254_field_size_be(&mt_pubkey.to_bytes())
        .unwrap()
        .0;
    let hashed_queue_pubkey = hash_to_bn254_field_size_be(&queue_pubkey.to_bytes())
        .unwrap()
        .0;
    assert_eq!(
        metadata.hashed_merkle_tree_pubkey,
        hashed_merkle_tree_pubkey
    );
    assert_eq!(metadata.hashed_queue_pubkey, hashed_queue_pubkey);
}

#[test]
fn test_check_is_associated() {
    let mut account_data = vec![0u8; 984];
    let mut queue_metadata = QueueMetadata::default();
    let associated_merkle_tree = Pubkey::new_unique();
    queue_metadata.associated_merkle_tree = associated_merkle_tree;
    queue_metadata.queue_type = QueueType::BatchedOutput as u64;
    let batch_size = 4;
    let zkp_batch_size = 2;
    let bloom_filter_capacity = 0;
    let num_iters = 0;
    let account = BatchedQueueAccount::init(
        &mut account_data,
        queue_metadata,
        batch_size,
        zkp_batch_size,
        num_iters,
        bloom_filter_capacity,
        Pubkey::new_unique(),
    )
    .unwrap();
    // 1. Functional
    {
        account
            .check_is_associated(&associated_merkle_tree)
            .unwrap();
        assert!(account.is_associated(&associated_merkle_tree));
    }
    // 2. Failing
    {
        let other_merkle_tree = Pubkey::new_unique();
        assert_eq!(
            account.check_is_associated(&other_merkle_tree),
            Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into())
        );
        assert!(!account.is_associated(&other_merkle_tree));
    }
}
