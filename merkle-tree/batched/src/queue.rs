use std::mem::{size_of, ManuallyDrop};

use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_bounded_vec::{BoundedVec, BoundedVecMetadata};
use light_hasher::Discriminator;
use light_merkle_tree_metadata::{
    errors::MerkleTreeMetadataError,
    queue::{QueueMetadata, QueueType},
};
use light_utils::account::{check_account_info_mut, set_discriminator, DISCRIMINATOR_LEN};
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

use super::batch::BatchState;
use crate::{
    batch::Batch,
    batch_metadata::BatchMetadata,
    constants::{ACCOUNT_COMPRESSION_PROGRAM_ID, OUTPUT_QUEUE_TYPE, TEST_DEFAULT_BATCH_SIZE},
    errors::BatchedMerkleTreeError,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    zero_copy::{bytes_to_struct_unchecked, ZeroCopyError},
    BorshDeserialize, BorshSerialize,
};

#[repr(C)]
#[derive(
    BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Pod, Zeroable, Clone, Copy,
)]
#[aligned_sized(anchor)]
pub struct BatchedQueueMetadata {
    pub metadata: QueueMetadata,
    pub batch_metadata: BatchMetadata,
    /// Output queue requires next index to derive compressed account hashes.
    /// next_index in queue is ahead or equal to next index in the associated
    /// batched Merkle tree account.
    pub next_index: u64,
}

// TODO: make discriminators anchor conistent
impl Discriminator for BatchedQueueMetadata {
    const DISCRIMINATOR: [u8; 8] = *b"queueacc";
}

// TODO: make discriminators anchor conistent
impl Discriminator for BatchedQueueAccount {
    const DISCRIMINATOR: [u8; 8] = *b"queueacc";
}

impl BatchedQueueMetadata {
    pub fn get_size_parameters(&self) -> Result<(usize, usize, usize), MerkleTreeMetadataError> {
        self.batch_metadata
            .get_size_parameters(self.metadata.queue_type)
    }
    pub fn init(
        &mut self,
        meta_data: QueueMetadata,
        num_batches: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        bloom_filter_capacity: u64,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.metadata = meta_data;
        self.batch_metadata
            .init(num_batches, batch_size, zkp_batch_size)?;
        self.batch_metadata.bloom_filter_capacity = bloom_filter_capacity;
        Ok(())
    }
}

impl BatchMetadata {
    pub fn init(
        &mut self,
        num_batches: u64,
        batch_size: u64,
        zkp_batch_size: u64,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.num_batches = num_batches;
        self.batch_size = batch_size;
        // Check that batch size is divisible by zkp_batch_size.
        if batch_size % zkp_batch_size != 0 {
            return Err(BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize);
        }
        self.zkp_batch_size = zkp_batch_size;
        Ok(())
    }

    pub fn get_size_parameters(
        &self,
        queue_type: u64,
    ) -> Result<(usize, usize, usize), MerkleTreeMetadataError> {
        let num_batches = self.num_batches as usize;
        // Input queues don't store values
        let num_value_stores = if queue_type == QueueType::Output as u64 {
            num_batches
        } else if queue_type == QueueType::Input as u64 {
            0
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType);
        };
        // Output queues don't use bloom filters.
        let num_stores = if queue_type == QueueType::Input as u64 {
            num_batches
        } else if queue_type == QueueType::Output as u64 && self.bloom_filter_capacity == 0 {
            0
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType);
        };
        Ok((num_value_stores, num_stores, num_batches))
    }
}

pub fn queue_account_size(
    batch_metadata: &BatchMetadata,
    queue_type: u64,
) -> Result<usize, BatchedMerkleTreeError> {
    let (num_value_vec, num_bloom_filter_stores, num_hashchain_store) =
        batch_metadata.get_size_parameters(queue_type)?;
    let account_size = if queue_type != QueueType::Output as u64 {
        0
    } else {
        BatchedQueueMetadata::LEN
    };
    let bounded_vec_metadata_size = size_of::<BoundedVecMetadata>();
    let batches_size =
        bounded_vec_metadata_size + (size_of::<Batch>() * batch_metadata.num_batches as usize);
    let value_vecs_size =
        (bounded_vec_metadata_size + 32 * batch_metadata.batch_size as usize) * num_value_vec;
    // Bloomfilter capacity is in bits.
    let bloom_filter_stores_size = (bounded_vec_metadata_size
        + batch_metadata.bloom_filter_capacity as usize / 8)
        * num_bloom_filter_stores;
    let hashchain_store_size = (bounded_vec_metadata_size
        + 32 * batch_metadata.get_num_zkp_batches() as usize)
        * num_hashchain_store;
    let size = account_size
        + batches_size
        + value_vecs_size
        + bloom_filter_stores_size
        + hashchain_store_size;
    Ok(size)
}
/// Batched output queue
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BatchedQueueAccount {
    metadata: *mut BatchedQueueMetadata,
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloom_filter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
    /// hashchain_store_capacity = batch_capacity / zkp_batch_size
    pub hashchain_store: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
}

impl BatchedQueueAccount {
    pub fn get_metadata(&self) -> &BatchedQueueMetadata {
        unsafe { &*self.metadata }
    }

    pub fn get_metadata_mut(&mut self) -> &mut BatchedQueueMetadata {
        unsafe { &mut *self.metadata }
    }

    pub fn output_queue_from_account_info_mut(
        account_info: &AccountInfo<'_>,
    ) -> Result<BatchedQueueAccount, BatchedMerkleTreeError> {
        Self::from_account_info_mut::<OUTPUT_QUEUE_TYPE>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    pub fn from_account_info_mut<const QUEUE_TYPE: u64>(
        program_id: &Pubkey,
        account_info: &AccountInfo<'_>,
    ) -> Result<BatchedQueueAccount, BatchedMerkleTreeError> {
        check_account_info_mut::<Self>(program_id, account_info)?;
        let account_data = &mut account_info.try_borrow_mut_data()?;
        Self::internal_from_bytes_mut::<OUTPUT_QUEUE_TYPE>(account_data)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn output_queue_from_bytes_mut(
        account_data: &mut [u8],
    ) -> Result<BatchedQueueAccount, BatchedMerkleTreeError> {
        Self::internal_from_bytes_mut::<OUTPUT_QUEUE_TYPE>(account_data)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn from_bytes_mut<const QUEUE_TYPE: u64>(
        account_data: &mut [u8],
    ) -> Result<BatchedQueueAccount, BatchedMerkleTreeError> {
        Self::internal_from_bytes_mut::<QUEUE_TYPE>(account_data)
    }

    fn internal_from_bytes_mut<const QUEUE_TYPE: u64>(
        account_data: &mut [u8],
    ) -> Result<BatchedQueueAccount, BatchedMerkleTreeError> {
        let metadata = bytes_to_struct_unchecked::<BatchedQueueMetadata>(account_data)?;
        unsafe {
            if (*metadata).metadata.queue_type != QUEUE_TYPE {
                return Err(MerkleTreeMetadataError::InvalidQueueType.into());
            }
            let (num_value_stores, num_stores, num_hashchain_stores) =
                (*metadata).get_size_parameters()?;

            let (batches, value_vecs, bloom_filter_stores, hashchain_store) =
                output_queue_from_bytes(
                    num_value_stores,
                    num_stores,
                    num_hashchain_stores,
                    account_data,
                )?;
            Ok(BatchedQueueAccount {
                metadata,
                batches,
                value_vecs,
                bloom_filter_stores,
                hashchain_store,
            })
        }
    }

    pub fn init(
        account_data: &mut [u8],
        metadata: QueueMetadata,
        num_batches_output_queue: u64,
        output_queue_batch_size: u64,
        output_queue_zkp_batch_size: u64,
        num_iters: u64,
        bloom_filter_capacity: u64,
    ) -> Result<BatchedQueueAccount, BatchedMerkleTreeError> {
        set_discriminator::<Self>(&mut account_data[0..DISCRIMINATOR_LEN])?;
        let account_metadata = bytes_to_struct_unchecked::<BatchedQueueMetadata>(account_data)?;
        unsafe {
            (*account_metadata).init(
                metadata,
                num_batches_output_queue,
                output_queue_batch_size,
                output_queue_zkp_batch_size,
                bloom_filter_capacity,
            )?;

            let (batches, value_vecs, bloom_filter_stores, hashchain_store) = init_queue(
                &(*account_metadata).batch_metadata,
                (*account_metadata).metadata.queue_type,
                account_data,
                num_iters,
                bloom_filter_capacity,
                &mut 0,
                0,
            )?;
            Ok(BatchedQueueAccount {
                metadata: account_metadata,
                batches,
                value_vecs,
                bloom_filter_stores,
                hashchain_store,
            })
        }
    }

    pub fn insert_into_current_batch(
        &mut self,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let current_index = self.get_metadata().next_index;
        unsafe {
            insert_into_current_batch(
                (*self.metadata).metadata.queue_type,
                &mut (*self.metadata).batch_metadata,
                &mut self.batches,
                &mut self.value_vecs,
                &mut self.bloom_filter_stores,
                &mut self.hashchain_store,
                value,
                None,
                Some(current_index),
            )?;
            (*self.metadata).next_index += 1;
        }
        Ok(())
    }

    pub fn prove_inclusion_by_index(
        &mut self,
        leaf_index: u64,
        value: &[u8; 32],
    ) -> Result<bool, BatchedMerkleTreeError> {
        for (batch_index, batch) in self.batches.iter().enumerate() {
            if batch.value_is_inserted_in_batch(leaf_index)? {
                let index = batch.get_value_index_in_batch(leaf_index)?;
                let element = self.value_vecs[batch_index]
                    .get_mut(index as usize)
                    .ok_or(BatchedMerkleTreeError::InclusionProofByIndexFailed)?;

                if element == value {
                    return Ok(true);
                } else {
                    return Err(BatchedMerkleTreeError::InclusionProofByIndexFailed);
                }
            }
        }
        Ok(false)
    }

    pub fn could_exist_in_batches(
        &mut self,
        leaf_index: u64,
    ) -> Result<(), BatchedMerkleTreeError> {
        for batch in self.batches.iter() {
            let res = batch.value_is_inserted_in_batch(leaf_index)?;
            if res {
                return Ok(());
            }
        }
        Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
    }

    /// Zero out a leaf by index if it exists in the queues value vec. If
    /// checked fail if leaf is not found.
    pub fn prove_inclusion_by_index_and_zero_out_leaf(
        &mut self,
        leaf_index: u64,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        for (batch_index, batch) in self.batches.iter().enumerate() {
            if batch.value_is_inserted_in_batch(leaf_index)? {
                let index = batch.get_value_index_in_batch(leaf_index)?;
                let element = self.value_vecs[batch_index]
                    .get_mut(index as usize)
                    .ok_or(BatchedMerkleTreeError::InclusionProofByIndexFailed)?;

                if element == value {
                    *element = [0u8; 32];
                    return Ok(());
                } else {
                    return Err(BatchedMerkleTreeError::InclusionProofByIndexFailed);
                }
            }
        }
        Ok(())
    }

    pub fn get_batch_num_inserted_in_current_batch(&self) -> u64 {
        let next_full_batch = self
            .get_metadata()
            .batch_metadata
            .currently_processing_batch_index;
        let batch = self.batches.get(next_full_batch as usize).unwrap();
        batch.get_num_inserted() + batch.get_current_zkp_batch_index() * batch.zkp_batch_size
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn insert_into_current_batch(
    queue_type: u64,
    account: &mut BatchMetadata,
    batches: &mut ManuallyDrop<BoundedVec<Batch>>,
    value_vecs: &mut [ManuallyDrop<BoundedVec<[u8; 32]>>],
    bloom_filter_stores: &mut [ManuallyDrop<BoundedVec<u8>>],
    hashchain_store: &mut [ManuallyDrop<BoundedVec<[u8; 32]>>],
    value: &[u8; 32],
    leaves_hash_value: Option<&[u8; 32]>,
    current_index: Option<u64>,
) -> Result<(Option<u32>, Option<u64>), BatchedMerkleTreeError> {
    let len = batches.len();
    let mut root_index = None;
    let mut sequence_number = None;
    let currently_processing_batch_index = account.currently_processing_batch_index as usize;
    // Insert value into current batch.
    {
        let mut bloom_filter_stores = bloom_filter_stores.get_mut(currently_processing_batch_index);
        let mut value_store = value_vecs.get_mut(currently_processing_batch_index);
        let mut hashchain_store = hashchain_store.get_mut(currently_processing_batch_index);

        let current_batch = batches.get_mut(currently_processing_batch_index).unwrap();
        let mut wipe = false;
        if current_batch.get_state() == BatchState::Inserted {
            current_batch.advance_state_to_can_be_filled()?;
            if let Some(current_index) = current_index {
                current_batch.start_index = current_index;
            }
            wipe = true;
        }

        // We expect to insert into the current batch.
        if current_batch.get_state() == BatchState::Full {
            for batch in batches.iter_mut() {
                msg!("batch {:?}", batch);
            }
            return Err(BatchedMerkleTreeError::BatchNotReady);
        }
        if wipe {
            msg!("wipe");
            if let Some(blomfilter_stores) = bloom_filter_stores.as_mut() {
                if !current_batch.bloom_filter_is_wiped {
                    (*blomfilter_stores)
                        .as_mut_slice()
                        .iter_mut()
                        .for_each(|x| *x = 0);
                    // Saving sequence number and root index for the batch.
                    // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
                    // if not advance current root index to root index
                    if current_batch.sequence_number != 0 {
                        root_index = Some(current_batch.root_index);
                        sequence_number = Some(current_batch.sequence_number);
                    }
                } else {
                    current_batch.bloom_filter_is_wiped = false;
                }
                current_batch.sequence_number = 0;
            }
            if let Some(value_store) = value_store.as_mut() {
                (*value_store).clear();
            }
            if let Some(hashchain_store) = hashchain_store.as_mut() {
                (*hashchain_store).clear();
            }
        }

        let queue_type = QueueType::from(queue_type);
        match queue_type {
            QueueType::Input | QueueType::Address => current_batch.insert(
                value,
                leaves_hash_value.unwrap(),
                bloom_filter_stores.unwrap().as_mut_slice(),
                hashchain_store.as_mut().unwrap(),
            ),
            QueueType::Output => current_batch.store_and_hash_value(
                value,
                value_store.unwrap(),
                hashchain_store.unwrap(),
            ),
            _ => Err(MerkleTreeMetadataError::InvalidQueueType.into()),
        }?;
    }

    // If queue has bloom_filters check non-inclusion of value in bloom_filters of
    // other batches. (Current batch is already checked by insertion.)
    if !bloom_filter_stores.is_empty() {
        for index in currently_processing_batch_index + 1..(len + currently_processing_batch_index)
        {
            let index = index % len;
            let bloom_filter_stores = bloom_filter_stores.get_mut(index).unwrap().as_mut_slice();
            let current_batch = batches.get_mut(index).unwrap();
            current_batch.check_non_inclusion(value, bloom_filter_stores)?;
        }
    }

    if batches[account.currently_processing_batch_index as usize].get_state() == BatchState::Full {
        account.currently_processing_batch_index += 1;
        account.currently_processing_batch_index %= len as u64;
    }
    Ok((root_index, sequence_number))
}

#[allow(clippy::type_complexity)]
pub fn output_queue_from_bytes(
    num_value_stores: usize,
    num_stores: usize,
    num_hashchain_stores: usize,
    account_data: &mut [u8],
) -> Result<
    (
        ManuallyDrop<BoundedVec<Batch>>,
        Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        Vec<ManuallyDrop<BoundedVec<u8>>>,
        Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    ),
    BatchedMerkleTreeError,
> {
    let mut start_offset = BatchedQueueMetadata::LEN;
    let batches = BoundedVec::deserialize(account_data, &mut start_offset)?;
    let value_vecs =
        BoundedVec::deserialize_multiple(num_value_stores, account_data, &mut start_offset)?;
    let bloom_filter_stores =
        BoundedVec::deserialize_multiple(num_stores, account_data, &mut start_offset)?;
    let hashchain_store =
        BoundedVec::deserialize_multiple(num_hashchain_stores, account_data, &mut start_offset)?;
    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

#[allow(clippy::type_complexity)]
pub fn input_queue_bytes(
    account: &BatchMetadata,
    account_data: &mut [u8],
    queue_type: u64,
    start_offset: &mut usize,
) -> Result<
    (
        ManuallyDrop<BoundedVec<Batch>>,
        Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        Vec<ManuallyDrop<BoundedVec<u8>>>,
        Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    ),
    BatchedMerkleTreeError,
> {
    let (num_value_stores, num_stores, hashchain_store_capacity) =
        account.get_size_parameters(queue_type)?;
    if queue_type == QueueType::Output as u64 {
        *start_offset += BatchedQueueMetadata::LEN;
    }
    let batches = BoundedVec::deserialize(account_data, start_offset)?;
    let value_vecs =
        BoundedVec::deserialize_multiple(num_value_stores, account_data, start_offset)?;
    let bloom_filter_stores =
        BoundedVec::deserialize_multiple(num_stores, account_data, start_offset)?;
    let hashchain_store =
        BoundedVec::deserialize_multiple(hashchain_store_capacity, account_data, start_offset)?;

    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

#[allow(clippy::type_complexity)]
pub fn init_queue(
    account: &BatchMetadata,
    queue_type: u64,
    account_data: &mut [u8],
    num_iters: u64,
    bloom_filter_capacity: u64,
    start_offset: &mut usize,
    batch_start_index: u64,
) -> Result<
    (
        ManuallyDrop<BoundedVec<Batch>>,
        Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        Vec<ManuallyDrop<BoundedVec<u8>>>,
        Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    ),
    BatchedMerkleTreeError,
> {
    if account_data.len() - *start_offset != queue_account_size(account, queue_type)? {
        msg!("*start_offset {:?}", *start_offset);
        msg!("account_data.len() {:?}", account_data.len());
        msg!("net size {:?}", account_data.len() - *start_offset);
        msg!(
            "queue_account_size {:?}",
            queue_account_size(account, queue_type)?
        );
        return Err(ZeroCopyError::InvalidAccountSize.into());
    }
    let (num_value_stores, num_stores, num_hashchain_stores) =
        account.get_size_parameters(queue_type)?;

    if queue_type == QueueType::Output as u64 {
        *start_offset += BatchedQueueMetadata::LEN;
    }

    let mut batches = BoundedVec::init(
        account.num_batches as usize,
        account_data,
        start_offset,
        false,
    )?;

    for i in 0..account.num_batches {
        batches.push(Batch::new(
            num_iters,
            bloom_filter_capacity,
            account.batch_size,
            account.zkp_batch_size,
            account.batch_size * i + batch_start_index,
        ))?;
    }

    let value_vecs = BoundedVec::init_multiple(
        num_value_stores,
        account.batch_size as usize,
        account_data,
        start_offset,
        false,
    )?;

    let bloom_filter_stores = BoundedVec::init_multiple(
        num_stores,
        account.bloom_filter_capacity as usize / 8,
        account_data,
        start_offset,
        true,
    )?;

    let hashchain_store = BoundedVec::init_multiple(
        num_hashchain_stores,
        account.get_num_zkp_batches() as usize,
        account_data,
        start_offset,
        false,
    )?;

    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

pub fn get_output_queue_account_size_default() -> usize {
    let account = BatchedQueueMetadata {
        metadata: QueueMetadata::default(),
        next_index: 0,
        batch_metadata: BatchMetadata {
            num_batches: 2,
            batch_size: TEST_DEFAULT_BATCH_SIZE,
            zkp_batch_size: 10,
            ..Default::default()
        },
    };
    queue_account_size(&account.batch_metadata, QueueType::Output as u64).unwrap()
}

pub fn get_output_queue_account_size_from_params(
    ix_data: InitStateTreeAccountsInstructionData,
) -> usize {
    let account = BatchedQueueMetadata {
        metadata: QueueMetadata::default(),
        next_index: 0,
        batch_metadata: BatchMetadata {
            num_batches: ix_data.output_queue_num_batches,
            batch_size: ix_data.output_queue_batch_size,
            zkp_batch_size: ix_data.output_queue_zkp_batch_size,
            ..Default::default()
        },
    };
    queue_account_size(&account.batch_metadata, QueueType::Output as u64).unwrap()
}

pub fn get_output_queue_account_size(
    batch_size: u64,
    zkp_batch_size: u64,
    num_batches: u64,
) -> usize {
    let account = BatchedQueueMetadata {
        metadata: QueueMetadata::default(),
        next_index: 0,
        batch_metadata: BatchMetadata {
            num_batches,
            batch_size,
            zkp_batch_size,
            ..Default::default()
        },
    };
    queue_account_size(&account.batch_metadata, QueueType::Output as u64).unwrap()
}

#[allow(clippy::too_many_arguments)]
pub fn assert_queue_inited(
    batch_metadata: BatchMetadata,
    ref_batch_metadata: BatchMetadata,
    queue_type: u64,
    value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    bloom_filter_stores: &mut Vec<ManuallyDrop<BoundedVec<u8>>>,
    batches: &mut ManuallyDrop<BoundedVec<Batch>>,
    num_batches: usize,
    num_iters: u64,
    start_index: u64,
) {
    assert_eq!(
        batch_metadata, ref_batch_metadata,
        "batch_metadata mismatch"
    );
    assert_eq!(batches.len(), num_batches, "batches mismatch");
    for (i, batch) in batches.iter().enumerate() {
        let ref_batch = Batch::new(
            num_iters,
            ref_batch_metadata.bloom_filter_capacity,
            ref_batch_metadata.batch_size,
            ref_batch_metadata.zkp_batch_size,
            ref_batch_metadata.batch_size * i as u64 + start_index,
        );

        assert_eq!(batch, &ref_batch, "batch mismatch");
    }

    if queue_type == QueueType::Output as u64 {
        assert_eq!(value_vecs.capacity(), num_batches, "value_vecs mismatch");
        assert_eq!(value_vecs.len(), num_batches, "value_vecs mismatch");
    } else {
        assert_eq!(value_vecs.len(), 0, "value_vecs mismatch");
        assert_eq!(value_vecs.capacity(), 0, "value_vecs mismatch");
    }

    if queue_type == QueueType::Output as u64 {
        assert_eq!(
            bloom_filter_stores.capacity(),
            0,
            "bloom_filter_stores mismatch"
        );
    } else {
        assert_eq!(
            bloom_filter_stores.capacity(),
            num_batches,
            "bloom_filter_stores mismatch"
        );
        assert_eq!(
            bloom_filter_stores.len(),
            num_batches,
            "bloom_filter_stores mismatch"
        );
    }

    for vec in bloom_filter_stores {
        assert_eq!(
            vec.metadata().capacity() * 8,
            batch_metadata.bloom_filter_capacity as usize,
            "bloom_filter_capacity mismatch"
        );
    }

    for vec in value_vecs.iter() {
        assert_eq!(
            vec.metadata().capacity(),
            batch_metadata.batch_size as usize,
            "batch_size mismatch"
        );
        assert_eq!(vec.len(), 0, "batch_size mismatch");
    }
}

#[cfg(not(target_os = "solana"))]
pub fn assert_queue_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: BatchedQueueMetadata,
    num_iters: u64,
) {
    let mut account = BatchedQueueAccount::output_queue_from_bytes_mut(account_data)
        .expect("from_bytes_unchecked_mut failed");
    let num_batches = ref_account.batch_metadata.num_batches as usize;
    let batch_metadata = account.get_metadata().batch_metadata;
    let queue_type = account.get_metadata().metadata.queue_type;
    let next_index = account.get_metadata().next_index;
    assert_eq!(
        account.get_metadata().metadata,
        ref_account.metadata,
        "metadata mismatch"
    );
    assert_queue_inited(
        batch_metadata,
        ref_account.batch_metadata,
        queue_type,
        &mut account.value_vecs,
        &mut account.bloom_filter_stores,
        &mut account.batches,
        num_batches,
        num_iters,
        next_index,
    );
}
