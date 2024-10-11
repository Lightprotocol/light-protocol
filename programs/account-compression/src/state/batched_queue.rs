use crate::{batch::Batch, errors::AccountCompressionErrorCode, QueueMetadata, QueueType};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{
    BoundedVec, BoundedVecMetadata, CyclicBoundedVec, CyclicBoundedVecMetadata,
};
use light_utils::offset::zero_copy::write_at;
use light_utils::offset::zero_copy::{read_array_like_ptr_at, read_ptr_at};
use std::mem::ManuallyDrop;
use std::ops::Sub;

// TODO: implement update that verifies multiple proofs
// TODO: implement mock circuit logic as well to sanity check
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
}
#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(AnchorDeserialize, Debug, Default, PartialEq)]
pub struct BatchedQueue {
    pub num_batches: u64,
    pub batch_size: u64,
    /// Next index of associated Merkle tree.
    /// Is used to derive compressed account hashes.
    /// Is not used in Input queue.
    pub next_index: u64,
    pub currently_processing_batch_index: u64,
    pub next_full_batch_index: u64,
    /// Index of last batch used to update in the Merkle tree.
    pub last_mt_updated_batch: u64,
    pub bloom_filter_capacity: u64,
}

pub fn queue_account_size(account: &BatchedQueue, queue_type: u64) -> Result<usize> {
    println!("queue_account_size: {:?}", account);
    println!("queue_type: {:?}", queue_type);
    let (num_value_vec, num_bloom_filter_stores) = account.get_size_parameters(queue_type)?;

    let account_size = if queue_type != QueueType::Output as u64 {
        0
    } else {
        std::mem::size_of::<BatchedQueueAccount>()
    };
    println!("queue_account_size: account size {:?}", account_size);
    let batches_size = (std::mem::size_of::<BoundedVecMetadata>() + std::mem::size_of::<Batch>())
        * account.num_batches as usize;
    let value_vecs_size = (std::mem::size_of::<BoundedVecMetadata>()
        + 32 * account.batch_size as usize)
        * num_value_vec;
    println!("value_vecs_size: {:?}", value_vecs_size);
    let bloom_filter_stores_size = (std::mem::size_of::<BoundedVecMetadata>()
        + account.bloom_filter_capacity as usize)
        * num_bloom_filter_stores;
    let size = account_size + batches_size + value_vecs_size + bloom_filter_stores_size;
    Ok(size)
}
impl BatchedQueueAccount {
    pub fn get_size_parameters(&self) -> Result<(usize, usize)> {
        self.queue.get_size_parameters(self.metadata.queue_type)
    }
    pub fn init(&mut self, meta_data: QueueMetadata, num_batches: u64, batch_size: u64) {
        self.metadata = meta_data;
        self.queue.init(num_batches, batch_size);
    }
}

impl BatchedQueue {
    pub fn init(&mut self, num_batches: u64, batch_size: u64) {
        self.num_batches = num_batches;
        self.batch_size = batch_size;
    }

    pub fn get_size_parameters(&self, queue_type: u64) -> Result<(usize, usize)> {
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
            } else if queue_type == QueueType::Output as u64 {
                0
            } else {
                return err!(AccountCompressionErrorCode::InvalidQueueType);
            };
        Ok((num_value_stores, num_stores))
    }
}

#[derive(Debug)]
pub struct ZeroCopyBatchedQueueAccount<'a> {
    pub account: &'a mut BatchedQueueAccount,
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloomfilter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
}

impl<'a> ZeroCopyBatchedQueueAccount<'a> {
    // TODO: add discriminator check
    // TODO: add from_account_info,  and from_account_loader
    pub fn from_account(
        account: &'a mut BatchedQueueAccount,
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedQueueAccount<'a>> {
        let (batches, value_vecs, bloomfilter_stores) = queue_from_account(account, account_data)?;
        Ok(ZeroCopyBatchedQueueAccount {
            account,
            batches,
            value_vecs,
            bloomfilter_stores,
        })
    }

    pub fn init_from_account(
        account: &'a mut BatchedQueueAccount,
        account_data: &mut [u8],
        num_iters: u64,
        bloomfilter_capacity: u64,
    ) -> Result<ZeroCopyBatchedQueueAccount<'a>> {
        let (batches, value_vecs, bloomfilter_stores) = init_queue_from_account(
            &account.queue,
            account.metadata.queue_type,
            account_data,
            num_iters,
            bloomfilter_capacity,
            &mut 0,
        )?;
        Ok(ZeroCopyBatchedQueueAccount {
            account,
            batches,
            value_vecs,
            bloomfilter_stores,
        })
    }

    pub fn insert_into_current_batch(&mut self, value: &[u8; 32]) -> Result<()> {
        insert_into_current_batch(
            self.account.metadata.queue_type,
            &mut self.account.queue,
            &mut self.batches,
            &mut self.value_vecs,
            &mut self.bloomfilter_stores,
            value,
        )
    }
    pub fn get_next_full_batch(&mut self) -> Result<&mut Batch> {
        // println!(
        //     "next_full_batch_index: {:?}",
        //     self.account.next_full_batch_index
        // );
        // println!("batches.len(): {:?}", self.batches.len());
        // let batches_len = self.batches.len();
        // let batch = self
        //     .batches
        //     .get_mut(self.account.next_full_batch_index as usize)
        //     .unwrap();
        // if batch.is_ready_to_update_tree() {
        //     self.account.next_full_batch_index += 1;
        //     self.account.next_full_batch_index %= batches_len as u64;
        //     Ok(batch)
        // } else {
        //     println!("batch id: {:?}", batch.id);
        //     err!(AccountCompressionErrorCode::BatchNotReady)
        // }
        queue_get_next_full_batch(&mut self.account.queue, &mut self.batches)
    }
}

pub fn queue_get_next_full_batch<'a>(
    account: &'a mut BatchedQueue,
    batches: &'a mut ManuallyDrop<BoundedVec<Batch>>,
) -> Result<&'a mut Batch> {
    println!("next_full_batch_index: {:?}", account.next_full_batch_index);
    println!("batches.len(): {:?}", account.num_batches);
    let batches_len = account.num_batches;
    let batch = batches
        .get_mut(account.next_full_batch_index as usize)
        .unwrap();
    if batch.is_ready_to_update_tree() {
        account.next_full_batch_index += 1;
        account.next_full_batch_index %= batches_len as u64;
        Ok(batch)
    } else {
        println!("batch id: {:?}", batch.id);
        err!(AccountCompressionErrorCode::BatchNotReady)
    }
}
pub fn insert_into_current_batch<'a>(
    queue_type: u64,
    account: &'a mut BatchedQueue,
    batches: &mut ManuallyDrop<BoundedVec<Batch>>,
    value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    bloomfilter_stores: &mut Vec<ManuallyDrop<BoundedVec<u8>>>,
    value: &[u8; 32],
) -> Result<()> {
    let len = batches.len();
    let mut inserted = false;

    if account.batch_size == batches[account.currently_processing_batch_index as usize].num_inserted
    {
        println!("bump currently_processing_batch_index");
        account.currently_processing_batch_index += 1;
        account.currently_processing_batch_index %= len as u64;
    }
    // insertion mode
    // Try to insert into the current queue.
    // In case the current queue fails, try to insert into the next queue.
    // Check every queue.
    for index in account.currently_processing_batch_index
        ..(len as u64 + account.currently_processing_batch_index)
    {
        let index = index as usize % len;
        println!("index: {:?}", index);

        let mut bloomfilter_stores = bloomfilter_stores.get_mut(index);
        let mut value_store = value_vecs.get_mut(index);
        let current_batch = batches.get_mut(index).unwrap();
        let queue_type = QueueType::from(queue_type);
        // let is_full = account.batch_size == current_batch.num_inserted;
        let (can_be_filled, wipe_batch) = current_batch.can_be_filled();

        // TODO: implement more efficient bloom filter wipe this will not work onchain
        if wipe_batch {
            println!(
                "wipe bloom filter is some {:?}",
                bloomfilter_stores.is_some()
            );
            if let Some(blomfilter_stores) = bloomfilter_stores.as_mut() {
                println!("wiping bloom filter");
                (*blomfilter_stores)
                    .as_mut_slice()
                    .iter_mut()
                    .for_each(|x| *x = 0);
            }
            println!("wipe value store is some {:?}", value_store.is_some());

            if let Some(value_store) = value_store.as_mut() {
                println!("wiping value store");
                (*value_store).clear();
            }
        }
        println!(
            "insert into current batch {:?} index: {:?} can_be_filled: {:?} inserted: {:?}",
            queue_type, index, can_be_filled, inserted
        );
        // TODO: remove unwraps
        if !inserted && can_be_filled {
            println!("store value");
            let insert_result = match queue_type {
                QueueType::Address => current_batch.insert_and_store(
                    value,
                    bloomfilter_stores.unwrap().as_mut_slice(),
                    value_store.unwrap(),
                ),
                QueueType::Input => {
                    current_batch.insert(value, bloomfilter_stores.unwrap().as_mut_slice())
                }

                QueueType::Output => current_batch.store_and_hash(value, value_store.unwrap()),

                _ => err!(AccountCompressionErrorCode::InvalidQueueType),
            };
            match insert_result {
                Ok(_) => {
                    // For the output queue we only need to insert. For address
                    // and input queues we need to prove non-inclusion as well
                    // hence check every bloomfilter.
                    if QueueType::Output == queue_type {
                        return Ok(());
                    }
                    inserted = true;
                }
                Err(error) => {
                    println!("wipe batch {:?}", wipe_batch);
                    println!("batch 0 {:?}", batches[0]);
                    println!("batch 1 {:?}", batches[1]);
                    return Err(error);
                }
            }
        } else if bloomfilter_stores.is_some() {
            println!("check non inclusion");
            current_batch.check_non_inclusion(value, bloomfilter_stores.unwrap().as_mut_slice())?;
        }
    }

    if !inserted {
        println!("batch 0 {:?}", batches[0]);
        // println!("batch 0 {:?}", batches[0].can_be_filled());
        println!("batch 1 {:?}", batches[1]);
        // println!("batch 1 {:?}", batches[1].can_be_filled());
        println!("Both batches are not ready to insert");
        return err!(AccountCompressionErrorCode::BatchInsertFailed);
    }
    Ok(())
}

pub fn queue_from_account(
    account: &BatchedQueueAccount,
    account_data: &mut [u8],
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
)> {
    let (num_value_stores, num_stores) = account.get_size_parameters()?;
    let mut start_offset = std::mem::size_of::<BatchedQueueAccount>();
    let batches = deserialize_bounded_vec(account_data, &mut start_offset);
    let value_vecs = deserialize_bounded_vecs(num_value_stores, account_data, &mut start_offset);
    let bloomfilter_stores = deserialize_bounded_vecs(num_stores, account_data, &mut start_offset);

    Ok((batches, value_vecs, bloomfilter_stores))
}

pub fn batched_queue_from_account(
    account: &BatchedQueue,
    account_data: &mut [u8],
    queue_type: u64,
    start_offset: &mut usize,
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
)> {
    let (num_value_stores, num_stores) = account.get_size_parameters(queue_type)?;
    if queue_type == QueueType::Output as u64 {
        println!(
            "batched_queue_from_account: is output queue start_offset: {:?}",
            start_offset
        );
        *start_offset += std::mem::size_of::<BatchedQueueAccount>();
    }
    let batches = deserialize_bounded_vec(account_data, start_offset);
    let value_vecs = deserialize_bounded_vecs(num_value_stores, account_data, start_offset);
    let bloomfilter_stores = deserialize_bounded_vecs(num_stores, account_data, start_offset);

    Ok((batches, value_vecs, bloomfilter_stores))
}

pub fn init_queue_from_account(
    account: &BatchedQueue,
    queue_type: u64,
    account_data: &mut [u8],
    num_iters: u64,
    bloomfilter_capacity: u64,
    start_offset: &mut usize,
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
)> {
    if account_data.len() - *start_offset != queue_account_size(&account, queue_type)? {
        println!("account_data.len() {:?}", account_data.len());
        println!(
            "queue_account_size {:?}",
            queue_account_size(&account, queue_type)?
        );
        return err!(AccountCompressionErrorCode::SizeMismatch);
    }
    let (num_value_stores, num_stores) = account.get_size_parameters(queue_type)?;
    println!("num_value_stores: {:?}", num_value_stores);
    println!("account data len {:?}", account_data.len());
    if queue_type == QueueType::Output as u64 {
        *start_offset += std::mem::size_of::<BatchedQueueAccount>();
    }
    let mut batches = init_bounded_vec(
        account.num_batches as usize,
        account_data,
        start_offset,
        false,
    );
    for i in 0..account.num_batches {
        batches
            .push(Batch {
                id: i as u8,
                bloomfilter_store_id: i as u8,
                value_store_id: i as u8,
                num_iters,
                bloomfilter_capacity,
                user_hash_chain: [0; 32],
                prover_hash_chain: [0; 32],
                num_inserted: 0,
                value_capacity: account.batch_size,
                is_inserted: false,
            })
            .map_err(ProgramError::from)?;
    }

    // TODO: reset value vecs after sequence number has expired
    let value_vecs = init_bounded_vecs(
        num_value_stores,
        account.batch_size as usize,
        account_data,
        start_offset,
        false,
    );

    let bloomfilter_stores = init_bounded_vecs(
        num_stores,
        account.bloom_filter_capacity as usize / 8,
        account_data,
        start_offset,
        true,
    );

    Ok((batches, value_vecs, bloomfilter_stores))
}

pub fn deserialize_bounded_vec<T: Clone>(
    account_data: &mut [u8],
    start_offset: &mut usize,
) -> ManuallyDrop<BoundedVec<T>> {
    let metadata = unsafe { read_ptr_at(account_data, start_offset) };
    unsafe {
        ManuallyDrop::new(BoundedVec::from_raw_parts(
            metadata,
            read_array_like_ptr_at(account_data, start_offset, (*metadata).capacity()),
        ))
    }
}

pub fn deserialize_cyclic_bounded_vec<T: Clone>(
    account_data: &mut [u8],
    start_offset: &mut usize,
) -> ManuallyDrop<CyclicBoundedVec<T>> {
    let metadata = unsafe { read_ptr_at(account_data, start_offset) };
    unsafe {
        ManuallyDrop::new(CyclicBoundedVec::from_raw_parts(
            metadata,
            read_array_like_ptr_at(account_data, start_offset, (*metadata).capacity()),
        ))
    }
}

pub fn deserialize_bounded_vecs<T: Clone>(
    num_batches: usize,
    account_data: &mut [u8],
    start_offset: &mut usize,
) -> Vec<ManuallyDrop<BoundedVec<T>>> {
    let mut value_vecs = Vec::with_capacity(num_batches);
    for _ in 0..num_batches {
        let vec = deserialize_bounded_vec(account_data, start_offset);
        value_vecs.push(vec);
    }
    value_vecs
}

pub fn init_bounded_vec<T: Clone>(
    capacity: usize,
    account_data: &mut [u8],
    start_offset: &mut usize,
    with_len: bool,
) -> ManuallyDrop<BoundedVec<T>> {
    let meta: BoundedVecMetadata = if with_len {
        BoundedVecMetadata::new_with_length(capacity, capacity)
    } else {
        BoundedVecMetadata::new(capacity)
    };
    write_at::<BoundedVecMetadata>(account_data, meta.to_le_bytes().as_slice(), start_offset);
    let meta: *mut BoundedVecMetadata = unsafe {
        read_ptr_at(
            &*account_data,
            &mut start_offset.sub(std::mem::size_of::<BoundedVecMetadata>()),
        )
    };
    unsafe {
        ManuallyDrop::new(BoundedVec::from_raw_parts(
            meta,
            read_array_like_ptr_at(&*account_data, start_offset, capacity),
        ))
    }
}

pub fn init_bounded_vecs<T: Clone>(
    num_batches: usize,
    capacity: usize,
    account_data: &mut [u8],
    start_offset: &mut usize,
    with_len: bool,
) -> Vec<ManuallyDrop<BoundedVec<T>>> {
    let mut value_vecs = Vec::with_capacity(num_batches);
    for _ in 0..num_batches {
        let vec = init_bounded_vec(capacity, account_data, start_offset, with_len);
        value_vecs.push(vec);
    }
    value_vecs
}

pub fn init_bounded_cyclic_vec<T: Clone>(
    capacity: usize,
    account_data: &mut [u8],
    start_offset: &mut usize,
    with_len: bool,
) -> ManuallyDrop<CyclicBoundedVec<T>> {
    let meta: CyclicBoundedVecMetadata = if with_len {
        CyclicBoundedVecMetadata::new_with_length(capacity, capacity)
    } else {
        CyclicBoundedVecMetadata::new(capacity)
    };
    write_at::<CyclicBoundedVecMetadata>(account_data, meta.to_le_bytes().as_slice(), start_offset);
    let meta: *mut CyclicBoundedVecMetadata = unsafe {
        read_ptr_at(
            &*account_data,
            &mut start_offset.sub(std::mem::size_of::<CyclicBoundedVecMetadata>()),
        )
    };
    unsafe {
        ManuallyDrop::new(CyclicBoundedVec::from_raw_parts(
            meta,
            read_array_like_ptr_at(&*account_data, start_offset, capacity),
        ))
    }
}
pub fn get_output_queue_account_size_default() -> usize {
    let account = BatchedQueueAccount {
        metadata: QueueMetadata::default(),
        queue: BatchedQueue {
            currently_processing_batch_index: 0,
            num_batches: 2,
            batch_size: 5000,
            bloom_filter_capacity: 0,
            next_index: 0,
            ..Default::default()
        },
    };
    queue_account_size(&account.queue, QueueType::Output as u64).unwrap()
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
            queue: BatchedQueue {
                batch_size: batch_size as u64,
                num_batches: num_batches as u64,
                currently_processing_batch_index: 0,
                next_index: 0,
                next_full_batch_index: 0,
                last_mt_updated_batch: 0,
                bloom_filter_capacity,
            },
        };
        let account_data: Vec<u8> =
            vec![0; queue_account_size(&account.queue, account.metadata.queue_type).unwrap()];
        (account, account_data)
    }

    fn assert_queue_zero_copy_inited(
        batch_size: usize,
        num_batches: usize,
        zero_copy_account: &ZeroCopyBatchedQueueAccount,
        account: &BatchedQueueAccount,
    ) {
        assert_eq!(zero_copy_account.account, account, "metadata mismatch");
        assert_eq!(
            zero_copy_account.batches.len(),
            num_batches,
            "batches mismatch"
        );
        if account.metadata.queue_type == QueueType::Input as u64 {
            assert_eq!(zero_copy_account.value_vecs.len(), 0, "value_vecs mismatch");
            assert_eq!(
                zero_copy_account.value_vecs.capacity(),
                0,
                "value_vecs mismatch"
            );
        } else {
            assert_eq!(
                zero_copy_account.value_vecs.capacity(),
                num_batches,
                "value_vecs mismatch"
            );
            assert_eq!(
                zero_copy_account.value_vecs.len(),
                num_batches,
                "value_vecs mismatch"
            );
        }
        if account.metadata.queue_type == QueueType::Output as u64 {
            assert_eq!(
                zero_copy_account.bloomfilter_stores.capacity(),
                0,
                "bloomfilter_stores mismatch"
            );
        } else {
            assert_eq!(
                zero_copy_account.bloomfilter_stores.capacity(),
                num_batches,
                "bloomfilter_stores mismatch"
            );
            assert_eq!(
                zero_copy_account.bloomfilter_stores.len(),
                num_batches,
                "bloomfilter_stores mismatch"
            );
        }

        for vec in zero_copy_account.bloomfilter_stores.iter() {
            assert_eq!(
                vec.capacity() * 8,
                account.queue.bloom_filter_capacity as usize,
                "bloom_filter_capacity mismatch"
            );
            assert_eq!(
                vec.len() * 8,
                account.queue.bloom_filter_capacity as usize,
                "bloom_filter_capacity mismatch"
            );
        }

        for vec in zero_copy_account.value_vecs.iter() {
            assert_eq!(vec.capacity(), batch_size, "batch_size mismatch");
            assert_eq!(vec.len(), 0, "batch_size mismatch");
        }
    }

    #[test]
    fn test_unpack_output_queue_account() {
        let batch_size = 100;
        // 1 batch in progress, 1 batch ready to be processed
        let num_batches = 2;
        let bloomfilter_capacity = 20_000 * 8;
        let bloomfilter_num_iters = 3;
        for queue_type in vec![QueueType::Input, QueueType::Output, QueueType::Address] {
            let (mut account, mut account_data) = get_test_account_and_account_data(
                batch_size,
                num_batches,
                queue_type,
                bloomfilter_capacity,
            );
            let ref_account = account.clone();
            let zero_copy_account = ZeroCopyBatchedQueueAccount::init_from_account(
                &mut account,
                &mut account_data,
                bloomfilter_num_iters,
                bloomfilter_capacity,
            )
            .unwrap();

            assert_queue_zero_copy_inited(
                batch_size as usize,
                num_batches as usize,
                &zero_copy_account,
                &ref_account,
            );
            let mut zero_copy_account =
                ZeroCopyBatchedQueueAccount::from_account(&mut account, &mut account_data).unwrap();
            assert_queue_zero_copy_inited(
                batch_size as usize,
                num_batches as usize,
                &zero_copy_account,
                &ref_account,
            );
            println!("zero_copy_account: {:?}", zero_copy_account);
            let value = [1u8; 32];
            println!("queue_type: {:?}", queue_type);
            zero_copy_account.insert_into_current_batch(&value).unwrap();
            // assert!(zero_copy_account.insert_into_current_batch(&value).is_ok());
            if queue_type != QueueType::Output {
                assert!(zero_copy_account.insert_into_current_batch(&value).is_err());
            }
            // TODO: add full assert
        }
    }
}
