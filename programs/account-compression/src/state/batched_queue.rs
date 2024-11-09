use crate::utils::constants::{DEFAULT_BATCH_SIZE, DISCRIMINATOR_LENGTH};
use crate::{batch::Batch, errors::AccountCompressionErrorCode, QueueMetadata, QueueType};
use crate::{bytes_to_struct_checked, InitStateTreeAccountsInstructionData};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{BoundedVec, BoundedVecMetadata};
use std::mem::ManuallyDrop;

use super::batch::BatchState;

// TODO: implement update that verifies multiple proofs
/// Memory layout:
/// 1. QueueMetadata
/// 2. num_batches: u64
/// 3. hash_chain hash bounded vec
/// 3. for num_batches every 33 bytes is a bloom filter
/// 3. (output queue) rest of account is bounded vec
///
/// One Batch account contains multiple batches.
#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(AnchorDeserialize, Debug, Default, PartialEq)]
pub struct BatchedQueueAccount {
    pub metadata: QueueMetadata,
    pub queue: BatchedQueue,
    /// Output queue requires next index to derive compressed account hashes.
    /// next_index in queue is ahead or equal to next index in the associated
    /// batched Merkle tree account.
    pub next_index: u64,
}

#[account(zero_copy)]
#[derive(AnchorDeserialize, Debug, Default, PartialEq)]
pub struct BatchedQueue {
    pub num_batches: u64,
    pub batch_size: u64,
    pub zkp_batch_size: u64,
    pub currently_processing_batch_index: u64,
    pub next_full_batch_index: u64,
    pub bloom_filter_capacity: u64,
}

impl BatchedQueue {
    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }

    pub fn get_output_queue_default(batch_size: u64, zkp_batch_size: u64) -> Self {
        BatchedQueue {
            num_batches: 2,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity: 0,
        }
    }

    pub fn get_input_queue_default(
        batch_size: u64,
        bloom_filter_capacity: u64,
        zkp_batch_size: u64,
    ) -> Self {
        BatchedQueue {
            num_batches: 4,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity,
        }
    }
}

pub fn queue_account_size(account: &BatchedQueue, queue_type: u64) -> Result<usize> {
    let (num_value_vec, num_bloom_filter_stores, num_hashchain_store) =
        account.get_size_parameters(queue_type)?;
    let account_size = if queue_type != QueueType::Output as u64 {
        0
    } else {
        std::mem::size_of::<BatchedQueueAccount>() + DISCRIMINATOR_LENGTH
    };

    let batches_size = (std::mem::size_of::<BoundedVecMetadata>() + std::mem::size_of::<Batch>())
        * account.num_batches as usize;
    let value_vecs_size = (std::mem::size_of::<BoundedVecMetadata>()
        + 32 * account.batch_size as usize)
        * num_value_vec;

    let bloom_filter_stores_size = (std::mem::size_of::<BoundedVecMetadata>()
        + account.bloom_filter_capacity as usize)
        * num_bloom_filter_stores;
    let hashchain_store_size = (std::mem::size_of::<BoundedVecMetadata>()
        + 32 * account.get_num_zkp_batches() as usize)
        * num_hashchain_store;
    let size = account_size
        + batches_size
        + value_vecs_size
        + bloom_filter_stores_size
        + hashchain_store_size
        + DISCRIMINATOR_LENGTH;
    Ok(size)
}

impl BatchedQueueAccount {
    pub fn get_size_parameters(&self) -> Result<(usize, usize, usize)> {
        self.queue.get_size_parameters(self.metadata.queue_type)
    }
    pub fn init(
        &mut self,
        meta_data: QueueMetadata,
        num_batches: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        bloom_filter_capacity: u64,
    ) -> Result<()> {
        self.metadata = meta_data;
        self.queue.init(num_batches, batch_size, zkp_batch_size)?;
        self.queue.bloom_filter_capacity = bloom_filter_capacity;
        Ok(())
    }
}

impl BatchedQueue {
    pub fn init(&mut self, num_batches: u64, batch_size: u64, zkp_batch_size: u64) -> Result<()> {
        self.num_batches = num_batches;
        self.batch_size = batch_size;
        // TODO: check that batch size is divisible by zkp_batch_size
        if batch_size % zkp_batch_size != 0 {
            return err!(AccountCompressionErrorCode::BatchSizeNotDivisibleByZkpBatchSize);
        }
        self.zkp_batch_size = zkp_batch_size;
        Ok(())
    }

    pub fn get_size_parameters(&self, queue_type: u64) -> Result<(usize, usize, usize)> {
        let num_batches = self.num_batches as usize;
        // Input queues don't store values
        let num_value_stores =
            if queue_type == QueueType::Output as u64 || queue_type == QueueType::Address as u64 {
                num_batches
            } else if queue_type == QueueType::Input as u64 {
                0
            } else {
                return err!(AccountCompressionErrorCode::InvalidQueueType);
            };
        // Output queues don't use bloom filters.
        let num_stores =
            if queue_type == QueueType::Input as u64 || queue_type == QueueType::Address as u64 {
                num_batches
            } else if queue_type == QueueType::Output as u64 && self.bloom_filter_capacity == 0 {
                0
            } else {
                return err!(AccountCompressionErrorCode::InvalidQueueType);
            };
        Ok((
            num_value_stores,
            num_stores,
            self.get_num_zkp_batches() as usize,
        ))
    }
}

/// Batched output queue
#[derive(Debug, Clone)]
pub struct ZeroCopyBatchedQueueAccount {
    account: *mut BatchedQueueAccount,
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloom_filter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
    /// hashchain_store_capacity = batch_capacity / zkp_batch_size
    pub hashchain_store: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
}

impl ZeroCopyBatchedQueueAccount {
    pub fn get_account(&self) -> &BatchedQueueAccount {
        unsafe { &*self.account }
    }

    pub fn get_account_mut(&mut self) -> &mut BatchedQueueAccount {
        unsafe { &mut *self.account }
    }

    pub fn from_bytes_mut(account_data: &mut [u8]) -> Result<ZeroCopyBatchedQueueAccount> {
        let account = bytes_to_struct_checked::<BatchedQueueAccount, false>(account_data)?;
        unsafe {
            let (num_value_stores, num_stores, num_hashchain_stores) =
                (*account).get_size_parameters()?;

            let (batches, value_vecs, bloom_filter_stores, hashchain_store) = queue_from_bytes(
                num_value_stores,
                num_stores,
                num_hashchain_stores,
                account_data,
            )?;
            Ok(ZeroCopyBatchedQueueAccount {
                account,
                batches,
                value_vecs,
                bloom_filter_stores,
                hashchain_store,
            })
        }
    }

    pub fn init(
        metadata: QueueMetadata,
        num_batches_output_queue: u64,
        output_queue_batch_size: u64,
        output_queue_zkp_batch_size: u64,
        account_data: &mut [u8],
        num_iters: u64,
        bloom_filter_capacity: u64,
    ) -> Result<ZeroCopyBatchedQueueAccount> {
        let account = bytes_to_struct_checked::<BatchedQueueAccount, true>(account_data)?;
        unsafe {
            (*account).init(
                metadata,
                num_batches_output_queue,
                output_queue_batch_size,
                output_queue_zkp_batch_size,
                bloom_filter_capacity,
            )?;

            let (batches, value_vecs, bloom_filter_stores, hashchain_store) = init_queue(
                &(*account).queue,
                (*account).metadata.queue_type,
                account_data,
                num_iters,
                bloom_filter_capacity,
                &mut 0,
            )?;
            Ok(ZeroCopyBatchedQueueAccount {
                account,
                batches,
                value_vecs,
                bloom_filter_stores,
                hashchain_store,
            })
        }
    }

    pub fn insert_into_current_batch(&mut self, value: &[u8; 32]) -> Result<()> {
        let current_index = self.get_account().next_index;
        unsafe {
            insert_into_current_batch(
                (*self.account).metadata.queue_type,
                &mut (*self.account).queue,
                &mut self.batches,
                &mut self.value_vecs,
                &mut self.bloom_filter_stores,
                &mut self.hashchain_store,
                value,
                None,
                Some(current_index),
            )?;
            (*self.account).next_index += 1;
        }
        Ok(())
    }
}

#[allow(clippy::ptr_arg)]
#[allow(clippy::type_complexity)]
pub fn insert_into_current_batch(
    queue_type: u64,
    account: &mut BatchedQueue,
    batches: &mut ManuallyDrop<BoundedVec<Batch>>,
    value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    bloom_filter_stores: &mut Vec<ManuallyDrop<BoundedVec<u8>>>,
    hashchain_store: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    value: &[u8; 32],
    leaves_hash_value: Option<&[u8; 32]>,
    current_index: Option<u64>,
) -> Result<(Option<u32>, Option<u64>)> {
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
        if current_batch.get_state() == BatchState::ReadyToUpdateTree {
            for batch in batches.iter_mut() {
                msg!("batch {:?}", batch);
            }
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        if wipe {
            if let Some(blomfilter_stores) = bloom_filter_stores.as_mut() {
                // TODO: consider to move it to forester because this is
                // expensive
                (*blomfilter_stores)
                    .as_mut_slice()
                    .iter_mut()
                    .for_each(|x| *x = 0);
                // Saving sequence number and root index for the batch.
                // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
                // if not advance current root index to root index
                if current_batch.sequence_number != 0 {
                    if root_index.is_none() && sequence_number.is_none() {
                        root_index = Some(current_batch.root_index);
                        sequence_number = Some(current_batch.sequence_number);
                        current_batch.sequence_number = 0;
                    } else {
                        unreachable!("root_index is already set this is a bug.");
                    }
                }
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
            // QueueType::Address => current_batch.insert_and_store(
            //     value,
            //     bloom_filter_stores.unwrap().as_mut_slice(),
            //     value_store.unwrap(),
            //     hashchain_store.unwrap(),
            // ),
            QueueType::Input => current_batch.insert(
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
            _ => err!(AccountCompressionErrorCode::InvalidQueueType),
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

    if batches[account.currently_processing_batch_index as usize].get_state()
        == BatchState::ReadyToUpdateTree
    {
        account.currently_processing_batch_index += 1;
        account.currently_processing_batch_index %= len as u64;
    }
    Ok((root_index, sequence_number))
}

#[allow(clippy::type_complexity)]
pub fn queue_from_bytes(
    num_value_stores: usize,
    num_stores: usize,
    num_hashchain_stores: usize,
    account_data: &mut [u8],
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
)> {
    let mut start_offset = std::mem::size_of::<BatchedQueueAccount>() + DISCRIMINATOR_LENGTH;
    let batches =
        BoundedVec::deserialize(account_data, &mut start_offset).map_err(ProgramError::from)?;
    let value_vecs =
        BoundedVec::deserialize_multiple(num_value_stores, account_data, &mut start_offset)
            .map_err(ProgramError::from)?;
    let bloom_filter_stores =
        BoundedVec::deserialize_multiple(num_stores, account_data, &mut start_offset)
            .map_err(ProgramError::from)?;
    let hashchain_store =
        BoundedVec::deserialize_multiple(num_hashchain_stores, account_data, &mut start_offset)
            .map_err(ProgramError::from)?;
    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

#[allow(clippy::type_complexity)]
#[allow(clippy::ptr_arg)]
pub fn batched_queue_bytes(
    account: &BatchedQueue,
    account_data: &mut [u8],
    queue_type: u64,
    start_offset: &mut usize,
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
)> {
    let (num_value_stores, num_stores, hashchain_store_capacity) =
        account.get_size_parameters(queue_type)?;
    if queue_type == QueueType::Output as u64 {
        *start_offset += std::mem::size_of::<BatchedQueueAccount>() + DISCRIMINATOR_LENGTH;
    }
    let batches =
        BoundedVec::deserialize(account_data, start_offset).map_err(ProgramError::from)?;
    let value_vecs = BoundedVec::deserialize_multiple(num_value_stores, account_data, start_offset)
        .map_err(ProgramError::from)?;
    let bloom_filter_stores =
        BoundedVec::deserialize_multiple(num_stores, account_data, start_offset)
            .map_err(ProgramError::from)?;
    let hashchain_store =
        BoundedVec::deserialize_multiple(hashchain_store_capacity, account_data, start_offset)
            .map_err(ProgramError::from)?;

    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

#[allow(clippy::type_complexity)]
pub fn init_queue(
    account: &BatchedQueue,
    queue_type: u64,
    account_data: &mut [u8],
    num_iters: u64,
    bloom_filter_capacity: u64,
    start_offset: &mut usize,
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
)> {
    if account_data.len() - *start_offset != queue_account_size(account, queue_type)? {
        msg!("*start_offset {:?}", *start_offset);
        msg!("account_data.len() {:?}", account_data.len());
        msg!("net size {:?}", account_data.len() - *start_offset);
        msg!(
            "queue_account_size {:?}",
            queue_account_size(account, queue_type)?
        );
        return err!(AccountCompressionErrorCode::SizeMismatch);
    }
    let (num_value_stores, num_stores, hashchain_store_capacity) =
        account.get_size_parameters(queue_type)?;

    if queue_type == QueueType::Output as u64 {
        *start_offset += std::mem::size_of::<BatchedQueueAccount>() + DISCRIMINATOR_LENGTH;
    }

    let mut batches = BoundedVec::init(
        account.num_batches as usize,
        account_data,
        start_offset,
        false,
    )
    .map_err(ProgramError::from)?;

    for i in 0..account.num_batches {
        batches
            .push(Batch::new(
                num_iters,
                bloom_filter_capacity,
                account.batch_size,
                account.zkp_batch_size,
                account.batch_size * i,
            ))
            .map_err(ProgramError::from)?;
    }

    let value_vecs = BoundedVec::init_multiple(
        num_value_stores,
        account.batch_size as usize,
        account_data,
        start_offset,
        false,
    )
    .map_err(ProgramError::from)?;

    let bloom_filter_stores = BoundedVec::init_multiple(
        num_stores,
        account.bloom_filter_capacity as usize / 8,
        account_data,
        start_offset,
        true,
    )
    .map_err(ProgramError::from)?;

    let hashchain_store = BoundedVec::init_multiple(
        account.num_batches as usize,
        hashchain_store_capacity,
        account_data,
        start_offset,
        false,
    )
    .map_err(ProgramError::from)?;

    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

pub fn get_output_queue_account_size_default() -> usize {
    let account = BatchedQueueAccount {
        metadata: QueueMetadata::default(),
        next_index: 0,
        queue: BatchedQueue {
            num_batches: 2,
            batch_size: DEFAULT_BATCH_SIZE,
            zkp_batch_size: 10,
            ..Default::default()
        },
    };
    queue_account_size(&account.queue, QueueType::Output as u64).unwrap()
}

pub fn get_output_queue_account_size_from_params(
    ix_data: InitStateTreeAccountsInstructionData,
) -> usize {
    let account = BatchedQueueAccount {
        metadata: QueueMetadata::default(),
        next_index: 0,
        queue: BatchedQueue {
            num_batches: 2,
            batch_size: ix_data.output_queue_batch_size,
            zkp_batch_size: ix_data.output_queue_zkp_batch_size,
            ..Default::default()
        },
    };
    queue_account_size(&account.queue, QueueType::Output as u64).unwrap()
}

pub fn get_output_queue_account_size(batch_size: u64, zkp_batch_size: u64) -> usize {
    let account = BatchedQueueAccount {
        metadata: QueueMetadata::default(),
        next_index: 0,
        queue: BatchedQueue {
            num_batches: 2,
            batch_size,
            zkp_batch_size,
            ..Default::default()
        },
    };
    queue_account_size(&account.queue, QueueType::Output as u64).unwrap()
}

pub fn assert_queue_inited(
    queue: BatchedQueue,
    ref_queue: BatchedQueue,
    queue_type: u64,
    value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    bloom_filter_stores: &mut Vec<ManuallyDrop<BoundedVec<u8>>>,
    batches: &mut ManuallyDrop<BoundedVec<Batch>>,
    num_batches: usize,
    num_iters: u64,
) {
    assert_eq!(queue, ref_queue, "queue mismatch");
    assert_eq!(batches.len(), num_batches, "batches mismatch");
    for (i, batch) in batches.iter().enumerate() {
        let ref_batch = Batch::new(
            num_iters,
            ref_queue.bloom_filter_capacity,
            ref_queue.batch_size,
            ref_queue.zkp_batch_size,
            ref_queue.batch_size * i as u64,
        );

        assert_eq!(batch, &ref_batch, "batch mismatch");
    }
    if queue_type == QueueType::Input as u64 {
        assert_eq!(value_vecs.len(), 0, "value_vecs mismatch");
        assert_eq!(value_vecs.capacity(), 0, "value_vecs mismatch");
    } else {
        assert_eq!(value_vecs.capacity(), num_batches, "value_vecs mismatch");
        assert_eq!(value_vecs.len(), num_batches, "value_vecs mismatch");
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

    for vec in bloom_filter_stores.iter() {
        assert_eq!(
            vec.capacity() * 8,
            queue.bloom_filter_capacity as usize,
            "bloom_filter_capacity mismatch"
        );
    }

    for vec in value_vecs.iter() {
        assert_eq!(
            vec.capacity(),
            queue.batch_size as usize,
            "batch_size mismatch"
        );
        assert_eq!(vec.len(), 0, "batch_size mismatch");
    }
}

pub fn assert_queue_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: BatchedQueueAccount,
    num_iters: u64,
) {
    let mut zero_copy_account =
        ZeroCopyBatchedQueueAccount::from_bytes_mut(account_data).expect("from_bytes_mut failed");
    let num_batches = ref_account.queue.num_batches as usize;
    let queue = zero_copy_account.get_account().queue;
    let queue_type = zero_copy_account.get_account().metadata.queue_type;
    assert_eq!(
        zero_copy_account.get_account().metadata,
        ref_account.metadata,
        "metadata mismatch"
    );
    assert_queue_inited(
        queue,
        ref_account.queue,
        queue_type,
        &mut zero_copy_account.value_vecs,
        &mut zero_copy_account.bloom_filter_stores,
        &mut zero_copy_account.batches,
        num_batches,
        num_iters,
    );
}

#[cfg(test)]
pub mod tests {

    use crate::{AccessMetadata, RolloverMetadata};

    use super::*;

    pub fn get_test_account_and_account_data(
        batch_size: u64,
        num_batches: u64,
        queue_type: QueueType,
        bloom_filter_capacity: u64,
    ) -> (BatchedQueueAccount, Vec<u8>) {
        let metadata = QueueMetadata {
            next_queue: Pubkey::new_unique(),
            access_metadata: AccessMetadata::default(),
            rollover_metadata: RolloverMetadata::default(),
            queue_type: queue_type as u64,
            associated_merkle_tree: Pubkey::new_unique(),
        };

        let account = BatchedQueueAccount {
            metadata: metadata.clone(),
            next_index: 0,
            queue: BatchedQueue {
                batch_size: batch_size as u64,
                num_batches: num_batches as u64,
                currently_processing_batch_index: 0,
                next_full_batch_index: 0,
                bloom_filter_capacity,
                zkp_batch_size: 10,
            },
        };
        let account_data: Vec<u8> =
            vec![0; queue_account_size(&account.queue, account.metadata.queue_type).unwrap()];
        (account, account_data)
    }

    #[test]
    fn test_output_queue_account() {
        let batch_size = 100;
        // 1 batch in progress, 1 batch ready to be processed
        let num_batches = 2;
        let bloom_filter_capacity = 0;
        let bloom_filter_num_iters = 0;
        for queue_type in vec![QueueType::Output] {
            let (ref_account, mut account_data) = get_test_account_and_account_data(
                batch_size,
                num_batches,
                queue_type,
                bloom_filter_capacity,
            );
            ZeroCopyBatchedQueueAccount::init(
                ref_account.metadata,
                num_batches,
                batch_size,
                10,
                &mut account_data,
                bloom_filter_num_iters,
                bloom_filter_capacity,
            )
            .unwrap();

            assert_queue_zero_copy_inited(&mut account_data, ref_account, bloom_filter_num_iters);
            let mut zero_copy_account =
                ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut account_data).unwrap();
            let value = [1u8; 32];
            zero_copy_account.insert_into_current_batch(&value).unwrap();
            // assert!(zero_copy_account.insert_into_current_batch(&value).is_ok());
            if queue_type != QueueType::Output {
                assert!(zero_copy_account.insert_into_current_batch(&value).is_err());
            }
            // TODO: add full assert
        }
    }
}
