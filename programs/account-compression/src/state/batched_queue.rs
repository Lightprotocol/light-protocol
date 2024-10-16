use crate::{batch::Batch, errors::AccountCompressionErrorCode, QueueMetadata, QueueType};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{BoundedVec, BoundedVecMetadata};
use light_utils::fee::compute_rollover_fee;
use std::mem::ManuallyDrop;
use std::u64;

use super::batch::BatchState;
use super::{AccessMetadata, RolloverMetadata};

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
    /// Output queue requires next index to derive compressed account hashes.
    /// next_index in queue is ahead or equal to next index in the associated
    /// batched Merkle tree account.
    pub next_index: u64,
}

impl BatchedQueueAccount {
    pub fn get_output_queue_default(
        owner: Pubkey,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        rollover_threshold: Option<u64>,
        index: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        additional_bytes: u64,
        rent: u64,
        associated_merkle_tree: Pubkey,
    ) -> Self {
        let height = 26;
        let rollover_fee = match rollover_threshold {
            Some(rollover_threshold) => {
                let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)
                    .map_err(ProgramError::from)
                    .unwrap();
                rollover_fee
            }
            None => 0,
        };
        let metadata = QueueMetadata {
            next_queue: Pubkey::default(),
            access_metadata: AccessMetadata {
                owner,
                program_owner: program_owner.unwrap_or_default(),
                forester: forester.unwrap_or_default(),
            },
            rollover_metadata: RolloverMetadata {
                close_threshold: u64::MAX,
                index,
                rolledover_slot: u64::MAX,
                rollover_threshold: rollover_threshold.unwrap_or(u64::MAX),
                rollover_fee,
                network_fee: 5000,
                additional_bytes,
            },
            queue_type: QueueType::Output as u64,
            associated_merkle_tree,
        };
        let queue = BatchedQueue::get_output_queue_default(batch_size, zkp_batch_size);
        BatchedQueueAccount {
            metadata,
            queue,
            next_index: 0,
        }
    }
    pub fn get_output_queue(
        owner: Pubkey,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        rollover_threshold: Option<u64>,
        index: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        additional_bytes: u64,
        rent: u64,
        associated_merkle_tree: Pubkey,
        network_fee: u64,
    ) -> Self {
        let height = 26;
        let rollover_fee = match rollover_threshold {
            Some(rollover_threshold) => {
                let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)
                    .map_err(ProgramError::from)
                    .unwrap();
                rollover_fee
            }
            None => 0,
        };
        let metadata = QueueMetadata {
            next_queue: Pubkey::default(),
            access_metadata: AccessMetadata {
                owner,
                program_owner: program_owner.unwrap_or_default(),
                forester: forester.unwrap_or_default(),
            },
            rollover_metadata: RolloverMetadata {
                close_threshold: u64::MAX,
                index,
                rolledover_slot: u64::MAX,
                rollover_threshold: rollover_threshold.unwrap_or(u64::MAX),
                rollover_fee,
                network_fee,
                additional_bytes,
            },
            queue_type: QueueType::Output as u64,
            associated_merkle_tree,
        };
        let queue = BatchedQueue::get_output_queue_default(batch_size, zkp_batch_size);
        BatchedQueueAccount {
            metadata,
            queue,
            next_index: 0,
        }
    }
}

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(AnchorDeserialize, Debug, Default, PartialEq)]
pub struct BatchedQueue {
    pub num_batches: u64,
    pub batch_size: u64,
    pub zkp_batch_size: u64,
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

impl BatchedQueue {
    pub fn get_num_zkp_batches(&self) -> u64 {
        println!("batch_size: {:?}", self.batch_size);
        println!("zkp_batch_size: {:?}", self.zkp_batch_size);
        self.batch_size / self.zkp_batch_size
    }
    pub fn get_output_queue_default(batch_size: u64, zkp_batch_size: u64) -> Self {
        BatchedQueue {
            num_batches: 2,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_index: 0,
            next_full_batch_index: 0,
            last_mt_updated_batch: 0,
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
            next_index: 0,
            next_full_batch_index: 0,
            last_mt_updated_batch: 0,
            bloom_filter_capacity,
        }
    }
}

pub fn queue_account_size(account: &BatchedQueue, queue_type: u64) -> Result<usize> {
    let (num_value_vec, num_bloom_filter_stores, num_hashchain_store) =
        account.get_size_parameters(queue_type)?;
    println!("account {:?}", account);
    let account_size = if queue_type != QueueType::Output as u64 {
        0
    } else {
        std::mem::size_of::<BatchedQueueAccount>()
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
    println!("hashchain_store_size: {:?}", hashchain_store_size);
    let size = account_size
        + batches_size
        + value_vecs_size
        + bloom_filter_stores_size
        + hashchain_store_size;
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
    ) {
        self.metadata = meta_data;
        self.queue.init(num_batches, batch_size, zkp_batch_size);
    }
}

impl BatchedQueue {
    pub fn init(&mut self, num_batches: u64, batch_size: u64, zkp_batch_size: u64) {
        self.num_batches = num_batches;
        self.batch_size = batch_size;
        self.zkp_batch_size = zkp_batch_size;
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
            } else if queue_type == QueueType::Output as u64 {
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

#[derive(Debug)]
pub struct ZeroCopyBatchedQueueAccount<'a> {
    pub account: &'a mut BatchedQueueAccount,
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloomfilter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
    /// hashchain_store_capacity = batch_capacity / zkp_batch_size
    pub hashchain_store: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
}

impl<'a> ZeroCopyBatchedQueueAccount<'a> {
    // TODO: add discriminator check
    // TODO: add from_account_info,  and from_account_loader
    pub fn from_account(
        account: &'a mut BatchedQueueAccount,
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedQueueAccount<'a>> {
        let (batches, value_vecs, bloomfilter_stores, hashchain_store) =
            queue_from_account(account, account_data)?;
        Ok(ZeroCopyBatchedQueueAccount {
            account,
            batches,
            value_vecs,
            bloomfilter_stores,
            hashchain_store,
        })
    }

    pub fn init_from_account(
        account: &'a mut BatchedQueueAccount,
        account_data: &mut [u8],
        num_iters: u64,
        bloomfilter_capacity: u64,
    ) -> Result<ZeroCopyBatchedQueueAccount<'a>> {
        let (batches, value_vecs, bloomfilter_stores, hashchain_store) = init_queue_from_account(
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
            hashchain_store,
        })
    }

    pub fn insert_into_current_batch(&mut self, value: &[u8; 32]) -> Result<()> {
        insert_into_current_batch(
            self.account.metadata.queue_type,
            &mut self.account.queue,
            &mut self.batches,
            &mut self.value_vecs,
            &mut self.bloomfilter_stores,
            &mut self.hashchain_store,
            value,
        )?;
        self.account.next_index += 1;
        Ok(())
    }

    pub fn get_next_full_batch(&mut self) -> Result<(&mut Batch, u8)> {
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
) -> Result<(&'a mut Batch, u8)> {
    println!("next_full_batch_index: {:?}", account.next_full_batch_index);
    println!("batches.len(): {:?}", account.num_batches);
    println!("batch: {:?}", account.next_full_batch_index);
    for batch in batches.iter() {
        println!("batch {:?}", batch);
    }
    let batches_len = account.num_batches;
    let batch = batches
        .get_mut(account.next_full_batch_index as usize)
        .unwrap();
    println!("next_full_batch_index: {:?}", account.next_full_batch_index);
    if batch.get_state() == BatchState::ReadyToUpdateTree {
        let batch_index = account.next_full_batch_index as u8;
        let num_zkp_batches = account.get_num_zkp_batches();
        // TODO: unify logic with mark_as_inserted
        if batch.get_num_inserted_zkps() == num_zkp_batches - 1 {
            println!("increment next_full_batch_index");
            account.next_full_batch_index += 1;
            account.next_full_batch_index %= batches_len as u64;
        }
        Ok((batch, batch_index))
    } else {
        err!(AccountCompressionErrorCode::BatchNotReady)
    }
}

pub fn insert_into_current_batch<'a>(
    queue_type: u64,
    account: &'a mut BatchedQueue,
    batches: &mut ManuallyDrop<BoundedVec<Batch>>,
    value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    bloomfilter_stores: &mut Vec<ManuallyDrop<BoundedVec<u8>>>,
    hashchain_store: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    value: &[u8; 32],
) -> Result<(Option<u32>, Option<u64>)> {
    let len = batches.len();
    let mut inserted = false;
    let mut root_index = None;
    let mut sequence_number = None;
    println!("len {:?}", len);
    println!(
        "account.currently_processing_batch_index {:?}",
        account.currently_processing_batch_index
    );
    // insertion mode
    // Try to insert into the current queue.
    // In case the current queue fails, try to insert into the next queue.
    // Check every queue.
    for index in account.currently_processing_batch_index
        ..(len as u64 + account.currently_processing_batch_index)
    {
        let index = index as usize % len;

        let mut bloomfilter_stores = bloomfilter_stores.get_mut(index);
        let mut value_store = value_vecs.get_mut(index);
        let mut hashchain_store = hashchain_store.get_mut(index);

        let current_batch = batches.get_mut(index).unwrap();
        println!("index {:?}", index);
        println!("current batch {:?}", current_batch);
        if current_batch.get_state() == BatchState::Inserted
            && index == account.currently_processing_batch_index as usize
        {
            current_batch.advance_state_to_can_be_filled()?;
        }
        if index == account.currently_processing_batch_index as usize
            && current_batch.get_state() == BatchState::ReadyToUpdateTree
        {
            for batch in batches.iter_mut() {
                println!("batch {:?}", batch);
            }
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        let queue_type = QueueType::from(queue_type);
        println!(
            "pre current_batch_index {:?}",
            account.currently_processing_batch_index
        );
        println!("get num inserted {:?}", current_batch.get_num_inserted());
        // TODO: implement more efficient bloom filter wipe this will not work onchain
        if current_batch.get_num_inserted() == 0 {
            println!("clearing");
            if let Some(blomfilter_stores) = bloomfilter_stores.as_mut() {
                println!("clearing bloomfilter store");
                (*blomfilter_stores)
                    .as_mut_slice()
                    .iter_mut()
                    .for_each(|x| *x = 0);
                // Saving sequence number and root index for the batch.
                // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
                // if not advance current root index to root index
                if current_batch.sequence_number != 0 {
                    current_batch.sequence_number = 0;
                    if root_index.is_none() && sequence_number.is_none() {
                        root_index = Some(current_batch.root_index);
                        sequence_number = Some(current_batch.sequence_number);
                    } else {
                        panic!("root_index is already set this is a bug.");
                    }
                }
            }
            if let Some(value_store) = value_store.as_mut() {
                println!("clearing value store");
                (*value_store).clear();
            }
            if let Some(hashchain_store) = hashchain_store.as_mut() {
                println!("clearing hashchain store");
                (*hashchain_store).clear();
            }
        }
        // TODO: remove unwraps
        if !inserted && current_batch.get_state() == BatchState::CanBeFilled {
            let insert_result = match queue_type {
                QueueType::Address => current_batch.insert_and_store(
                    value,
                    bloomfilter_stores.unwrap().as_mut_slice(),
                    value_store.unwrap(),
                    hashchain_store.unwrap(),
                ),
                QueueType::Input => current_batch.insert(
                    value,
                    bloomfilter_stores.unwrap().as_mut_slice(),
                    hashchain_store.unwrap(),
                ),
                QueueType::Output => current_batch.store_and_hash(
                    value,
                    value_store.unwrap(),
                    hashchain_store.unwrap(),
                ),

                _ => err!(AccountCompressionErrorCode::InvalidQueueType),
            };
            match insert_result {
                Ok(_) => {
                    inserted = true;
                    // For the output queue we only need to insert. For address
                    // and input queues we need to prove non-inclusion as well
                    // hence check every bloomfilter.
                    if QueueType::Output == queue_type {
                        break;
                    }
                }
                Err(error) => {
                    msg!("batch 0 {:?}", batches[0]);
                    msg!("batch 1 {:?}", batches[1]);
                    msg!("insertion failed {:?}", error);
                    return Err(error);
                }
            }
        } else if bloomfilter_stores.is_some() {
            current_batch.check_non_inclusion(value, bloomfilter_stores.unwrap().as_mut_slice())?;
        }
    }

    if !inserted {
        msg!("batch 0 {:?}", batches[0]);
        msg!("batch 1 {:?}", batches[1]);
        msg!("Both batches are not ready to insert");
        return err!(AccountCompressionErrorCode::BatchInsertFailed);
    }
    println!(
        "batches[account.currently_processing_batch_index as usize].get_num_inserted() {:?}",
        batches[account.currently_processing_batch_index as usize].get_num_inserted()
    );
    println!(
        "post current_batch_index {:?}",
        account.currently_processing_batch_index
    );
    if batches[account.currently_processing_batch_index as usize].get_state()
        == BatchState::ReadyToUpdateTree
    {
        println!("batch is full icrementing currently_processing_batch_index");
        account.currently_processing_batch_index += 1;
        account.currently_processing_batch_index %= len as u64;
    }
    Ok((root_index, sequence_number))
}

pub fn queue_from_account(
    account: &BatchedQueueAccount,
    account_data: &mut [u8],
) -> Result<(
    ManuallyDrop<BoundedVec<Batch>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    Vec<ManuallyDrop<BoundedVec<u8>>>,
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
)> {
    let (num_value_stores, num_stores, num_hashchain_stores) = account.get_size_parameters()?;
    let mut start_offset = std::mem::size_of::<BatchedQueueAccount>();
    let batches = BoundedVec::deserialize(account_data, &mut start_offset);
    let value_vecs =
        BoundedVec::deserialize_multiple(num_value_stores, account_data, &mut start_offset);
    let bloomfilter_stores =
        BoundedVec::deserialize_multiple(num_stores, account_data, &mut start_offset);
    let hashchain_store =
        BoundedVec::deserialize_multiple(num_hashchain_stores, account_data, &mut start_offset);
    Ok((batches, value_vecs, bloomfilter_stores, hashchain_store))
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
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
)> {
    let (num_value_stores, num_stores, hashchain_store_capacity) =
        account.get_size_parameters(queue_type)?;
    if queue_type == QueueType::Output as u64 {
        println!(
            "batched_queue_from_account: is output queue start_offset: {:?}",
            start_offset
        );
        *start_offset += std::mem::size_of::<BatchedQueueAccount>();
    }
    let batches = BoundedVec::deserialize(account_data, start_offset);
    let value_vecs = BoundedVec::deserialize_multiple(num_value_stores, account_data, start_offset);
    let bloomfilter_stores =
        BoundedVec::deserialize_multiple(num_stores, account_data, start_offset);
    let hashchain_store =
        BoundedVec::deserialize_multiple(hashchain_store_capacity, account_data, start_offset);

    Ok((batches, value_vecs, bloomfilter_stores, hashchain_store))
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
    Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
)> {
    if account_data.len() - *start_offset != queue_account_size(&account, queue_type)? {
        println!("*start_offset {:?}", *start_offset);
        println!("account_data.len() {:?}", account_data.len());
        println!("net size {:?}", account_data.len() - *start_offset);
        println!(
            "queue_account_size {:?}",
            queue_account_size(&account, queue_type)?
        );
        return err!(AccountCompressionErrorCode::SizeMismatch);
    }
    let (num_value_stores, num_stores, hashchain_store_capacity) =
        account.get_size_parameters(queue_type)?;
    if queue_type == QueueType::Output as u64 {
        *start_offset += std::mem::size_of::<BatchedQueueAccount>();
    }

    let mut batches = BoundedVec::init(
        account.num_batches as usize,
        account_data,
        start_offset,
        false,
    )
    .map_err(ProgramError::from)?;

    for _ in 0..account.num_batches {
        batches
            .push(Batch::new(
                num_iters,
                bloomfilter_capacity,
                account.batch_size,
                account.zkp_batch_size,
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

    let bloomfilter_stores = BoundedVec::init_multiple(
        num_stores,
        account.bloom_filter_capacity as usize / 8,
        account_data,
        start_offset,
        true,
    )
    .map_err(ProgramError::from)?;
    let hashchain_store = BoundedVec::init_multiple(
        hashchain_store_capacity,
        account.get_num_zkp_batches() as usize,
        account_data,
        start_offset,
        false,
    )
    .map_err(ProgramError::from)?;

    Ok((batches, value_vecs, bloomfilter_stores, hashchain_store))
}

pub fn get_output_queue_account_size_default() -> usize {
    let account = BatchedQueueAccount {
        metadata: QueueMetadata::default(),
        next_index: 0,
        queue: BatchedQueue {
            num_batches: 2,
            batch_size: 5000,
            zkp_batch_size: 100,
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
                next_index: 0,
                next_full_batch_index: 0,
                last_mt_updated_batch: 0,
                bloom_filter_capacity,
                zkp_batch_size: 100,
            },
        };
        let account_data: Vec<u8> =
            vec![0; queue_account_size(&account.queue, account.metadata.queue_type).unwrap()];
        (account, account_data)
    }

    pub fn assert_queue_zero_copy_inited(
        account: &mut BatchedQueueAccount,
        account_data: &mut [u8],
        ref_account: BatchedQueueAccount,
        num_iters: u64,
    ) {
        let num_batches = ref_account.queue.num_batches as usize;
        let queue = account.queue.clone();
        let queue_type = account.metadata.queue_type as u64;

        let mut zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_account(account, account_data)
                .expect("from_account failed");
        assert_eq!(
            zero_copy_account.account.metadata, ref_account.metadata,
            "metadata mismatch"
        );
        assert_queue_inited(
            queue,
            ref_account.queue,
            queue_type,
            &mut zero_copy_account.value_vecs,
            &mut zero_copy_account.bloomfilter_stores,
            &mut zero_copy_account.batches,
            num_batches,
            num_iters,
        );
    }

    pub fn assert_queue_inited(
        queue: BatchedQueue,
        ref_queue: BatchedQueue,
        queue_type: u64,
        value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        bloomfilter_stores: &mut Vec<ManuallyDrop<BoundedVec<u8>>>,
        batches: &mut ManuallyDrop<BoundedVec<Batch>>,
        num_batches: usize,
        num_iters: u64,
    ) {
        assert_eq!(queue, ref_queue, "queue mismatch");
        assert_eq!(batches.len(), num_batches, "batches mismatch");
        for batch in batches.iter() {
            let ref_batch = Batch::new(
                num_iters,
                ref_queue.bloom_filter_capacity,
                ref_queue.batch_size,
                ref_queue.zkp_batch_size,
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
                bloomfilter_stores.capacity(),
                0,
                "bloomfilter_stores mismatch"
            );
        } else {
            assert_eq!(
                bloomfilter_stores.capacity(),
                num_batches,
                "bloomfilter_stores mismatch"
            );
            assert_eq!(
                bloomfilter_stores.len(),
                num_batches,
                "bloomfilter_stores mismatch"
            );
        }

        for vec in bloomfilter_stores.iter() {
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

    #[test]
    fn test_output_queue_account() {
        let batch_size = 100;
        // 1 batch in progress, 1 batch ready to be processed
        let num_batches = 2;
        let bloomfilter_capacity = 20_000 * 8;
        let bloomfilter_num_iters = 3;
        for queue_type in vec![QueueType::Output] {
            let (mut account, mut account_data) = get_test_account_and_account_data(
                batch_size,
                num_batches,
                queue_type,
                bloomfilter_capacity,
            );
            let ref_account = account.clone();
            ZeroCopyBatchedQueueAccount::init_from_account(
                &mut account,
                &mut account_data,
                bloomfilter_num_iters,
                bloomfilter_capacity,
            )
            .unwrap();

            assert_queue_zero_copy_inited(
                &mut account,
                &mut account_data,
                ref_account,
                bloomfilter_num_iters,
            );
            let mut zero_copy_account =
                ZeroCopyBatchedQueueAccount::from_account(&mut account, &mut account_data).unwrap();
            println!("zero_copy_account     {:?}", zero_copy_account);
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
