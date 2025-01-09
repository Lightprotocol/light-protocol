use std::ops::{Deref, DerefMut};

use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_hasher::Discriminator;
use light_merkle_tree_metadata::{
    errors::MerkleTreeMetadataError,
    queue::{QueueMetadata, QueueType},
};
use light_utils::{
    account::{check_account_info_mut, check_discriminator, set_discriminator, DISCRIMINATOR_LEN},
    pubkey::Pubkey,
};
use light_zero_copy::{errors::ZeroCopyError, slice_mut::ZeroCopySliceMutU64, vec::ZeroCopyVecU64};
use solana_program::{account_info::AccountInfo, msg};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref};

use super::batch::BatchState;
use crate::{
    batch::Batch,
    batch_metadata::BatchMetadata,
    constants::{ACCOUNT_COMPRESSION_PROGRAM_ID, OUTPUT_QUEUE_TYPE, TEST_DEFAULT_BATCH_SIZE},
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
    Pod,
    Zeroable,
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
    /// Output queue requires next index to derive compressed account hashes.
    /// next_index in queue is ahead or equal to next index in the associated
    /// batched Merkle tree account.
    pub next_index: u64,
}

// TODO: make discriminators anchor conistent
impl Discriminator for BatchedQueueAccount<'_> {
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
    let batches_size =
        ZeroCopySliceMutU64::<Batch>::required_size_for_capacity(batch_metadata.num_batches);
    let value_vecs_size =
        ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(batch_metadata.batch_size as usize)
            * num_value_vec;
    // Bloomfilter capacity is in bits.
    let bloom_filter_stores_size = ZeroCopySliceMutU64::<u8>::required_size_for_capacity(
        batch_metadata.bloom_filter_capacity / 8,
    ) * num_bloom_filter_stores;
    let hashchain_store_size = ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
        batch_metadata.get_num_zkp_batches() as usize,
    ) * num_hashchain_store;
    let size = account_size
        + batches_size
        + value_vecs_size
        + bloom_filter_stores_size
        + hashchain_store_size;
    Ok(size)
}
/// Batched output queue
#[repr(C)]
#[derive(Debug)]
pub struct BatchedQueueAccount<'a> {
    metadata: Ref<&'a mut [u8], BatchedQueueMetadata>,
    pub batches: ZeroCopySliceMutU64<'a, Batch>,
    pub value_vecs: Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
    pub bloom_filter_stores: Vec<ZeroCopySliceMutU64<'a, u8>>,
    /// hashchain_store_capacity = batch_capacity / zkp_batch_size
    pub hashchain_store: Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
    marker: std::marker::PhantomData<&'a ()>,
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

impl<'a> BatchedQueueAccount<'a> {
    // TODO: remove
    pub fn get_metadata(&self) -> &BatchedQueueMetadata {
        &self.metadata
    }

    // TODO: remove
    pub fn get_metadata_mut(&mut self) -> &mut BatchedQueueMetadata {
        &mut self.metadata
    }

    pub fn output_queue_from_account_info_mut(
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        Self::from_account_info_mut::<OUTPUT_QUEUE_TYPE>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    pub fn from_account_info_mut<const QUEUE_TYPE: u64>(
        program_id: &solana_program::pubkey::Pubkey,
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        check_account_info_mut::<Self>(program_id, account_info)?;
        let account_data = &mut account_info.try_borrow_mut_data()?;
        let account_data: &'a mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(account_data.as_mut_ptr(), account_data.len())
        };

        Self::internal_from_bytes_mut::<OUTPUT_QUEUE_TYPE>(account_data)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn output_queue_from_bytes_mut(
        account_data: &'a mut [u8],
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        Self::internal_from_bytes_mut::<OUTPUT_QUEUE_TYPE>(account_data)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn from_bytes_mut<const QUEUE_TYPE: u64>(
        account_data: &'a mut [u8],
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        Self::internal_from_bytes_mut::<QUEUE_TYPE>(account_data)
    }

    fn internal_from_bytes_mut<const QUEUE_TYPE: u64>(
        account_data: &'a mut [u8],
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        let (discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        check_discriminator::<BatchedQueueAccount>(discriminator)?;
        // TODO: remove unwrap
        let (metadata, account_data) =
            Ref::<&'a mut [u8], BatchedQueueMetadata>::from_prefix(account_data).unwrap();

        if metadata.metadata.queue_type != QUEUE_TYPE {
            return Err(MerkleTreeMetadataError::InvalidQueueType.into());
        }
        let (num_value_stores, num_stores, num_hashchain_stores) =
            metadata.get_size_parameters()?;

        let (batches, value_vecs, bloom_filter_stores, hashchain_store) = output_queue_from_bytes(
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
            marker: std::marker::PhantomData,
        })
    }

    pub fn init(
        account_data: &'a mut [u8],
        metadata: QueueMetadata,
        num_batches_output_queue: u64,
        output_queue_batch_size: u64,
        output_queue_zkp_batch_size: u64,
        num_iters: u64,
        bloom_filter_capacity: u64,
    ) -> Result<BatchedQueueAccount<'a>, BatchedMerkleTreeError> {
        let account_data_len = account_data.len();
        let (discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        set_discriminator::<Self>(discriminator)?;

        let (mut account_metadata, account_data) =
            Ref::<&mut [u8], BatchedQueueMetadata>::from_prefix(account_data).unwrap();

        account_metadata.init(
            metadata,
            num_batches_output_queue,
            output_queue_batch_size,
            output_queue_zkp_batch_size,
            bloom_filter_capacity,
        )?;
        if account_data_len
            != queue_account_size(
                &account_metadata.batch_metadata,
                account_metadata.metadata.queue_type,
            )?
        {
            msg!("account_data.len() {:?}", account_data_len);
            msg!(
                "queue_account_size {:?}",
                queue_account_size(
                    &account_metadata.batch_metadata,
                    account_metadata.metadata.queue_type
                )?
            );
            return Err(ZeroCopyError::InvalidAccountSize.into());
        }

        let (batches, value_vecs, bloom_filter_stores, hashchain_store) = init_queue(
            &account_metadata.batch_metadata,
            account_metadata.metadata.queue_type,
            account_data,
            num_iters,
            bloom_filter_capacity,
            0,
        )?;
        Ok(BatchedQueueAccount {
            metadata: account_metadata,
            batches,
            value_vecs,
            bloom_filter_stores,
            hashchain_store,
            marker: std::marker::PhantomData,
        })
    }

    pub fn insert_into_current_batch(
        &mut self,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let current_index = self.next_index;

        insert_into_current_batch(
            self.metadata.metadata.queue_type,
            &mut self.metadata.batch_metadata,
            &mut self.batches,
            &mut self.value_vecs,
            self.bloom_filter_stores.as_mut_slice(),
            &mut self.hashchain_store,
            value,
            None,
            Some(current_index),
        )?;
        self.metadata.next_index += 1;

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

    pub fn leaf_index_could_exist_in_batches(
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
        let next_full_batch = self.batch_metadata.currently_processing_batch_index;
        let batch = self.batches.get(next_full_batch as usize).unwrap();
        batch.get_num_inserted() + batch.get_current_zkp_batch_index() * batch.zkp_batch_size
    }

    pub fn is_associated(&self, account: &Pubkey) -> bool {
        self.metadata.metadata.associated_merkle_tree == *account
    }

    pub fn check_is_associated(&self, account: &Pubkey) -> Result<(), BatchedMerkleTreeError> {
        if !self.is_associated(account) {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into());
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn insert_into_current_batch(
    queue_type: u64,
    account: &mut BatchMetadata,
    batches: &mut ZeroCopySliceMutU64<Batch>,
    value_vecs: &mut [ZeroCopyVecU64<[u8; 32]>],
    bloom_filter_stores: &mut [ZeroCopySliceMutU64<u8>],
    hashchain_store: &mut [ZeroCopyVecU64<[u8; 32]>],
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
                if !current_batch.bloom_filter_is_wiped() {
                    (*blomfilter_stores).iter_mut().for_each(|x| *x = 0);
                    // Saving sequence number and root index for the batch.
                    // When the batch is cleared check that sequence number is greater or equal than self.sequence_number
                    // if not advance current root index to root index
                    if current_batch.sequence_number != 0 {
                        root_index = Some(current_batch.root_index);
                        sequence_number = Some(current_batch.sequence_number);
                    }
                } else {
                    current_batch.set_bloom_filter_is_not_wiped();
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
        ZeroCopySliceMutU64<'_, Batch>,
        Vec<ZeroCopyVecU64<'_, [u8; 32]>>,
        Vec<ZeroCopySliceMutU64<'_, u8>>,
        Vec<ZeroCopyVecU64<'_, [u8; 32]>>,
    ),
    BatchedMerkleTreeError,
> {
    // let mut start_offset = BatchedQueueMetadata::LEN;
    let (batches, account_data) = ZeroCopySliceMutU64::from_bytes_at(account_data)?;
    let (value_vecs, account_data) =
        ZeroCopyVecU64::from_bytes_at_multiple(num_value_stores, account_data)?;

    let (bloom_filter_stores, account_data) =
        ZeroCopySliceMutU64::from_bytes_at_multiple(num_stores, account_data)?;
    let (hashchain_store, _) =
        ZeroCopyVecU64::from_bytes_at_multiple(num_hashchain_stores, account_data)?;
    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

#[allow(clippy::type_complexity)]
pub fn input_queue_bytes<'a>(
    account: &BatchMetadata,
    account_data: &'a mut [u8],
    queue_type: u64,
) -> Result<
    (
        ZeroCopySliceMutU64<'a, Batch>,
        Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
        Vec<ZeroCopySliceMutU64<'a, u8>>,
        Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
    ),
    BatchedMerkleTreeError,
> {
    let (num_value_stores, num_stores, hashchain_store_capacity) =
        account.get_size_parameters(queue_type)?;
    // if queue_type == QueueType::Output as u64 {
    //     *start_offset += BatchedQueueMetadata::LEN;
    // }
    let (batches, account_data) = ZeroCopySliceMutU64::from_bytes_at(account_data)?;
    let (value_vecs, account_data) =
        ZeroCopyVecU64::from_bytes_at_multiple(num_value_stores, account_data)?;
    // let mut bloom_filter_stores = Vec::with_capacity(num_stores);
    let (bloom_filter_stores, account_data) =
        ZeroCopySliceMutU64::from_bytes_at_multiple(num_stores, account_data)?;
    // for _ in 0..num_stores {

    // }
    println!(
        "account.bloom_filter_capacity {:?}",
        account.bloom_filter_capacity / 8
    );
    // let account_data = &mut account_data[*start_offset..];
    // let (bloom_filter_store, account_data) = Ref::<&'a mut [u8], [u8]>::from_prefix_with_elems(
    //     account_data,
    //     (account.bloom_filter_capacity / 8) as usize,
    // )
    // .unwrap();
    // // *start_offset += (account.bloom_filter_capacity / 8) as usize;
    // bloom_filter_stores.push(bloom_filter_store);
    // let (bloom_filter_store, account_data) = Ref::<&'a mut [u8], [u8]>::from_prefix_with_elems(
    //     account_data,
    //     (account.bloom_filter_capacity / 8) as usize,
    // )
    // .unwrap();
    // // *start_offset += (account.bloom_filter_capacity / 8) as usize;
    // bloom_filter_stores.push(bloom_filter_store);

    // // reset start_offset to 0
    // *start_offset = 0;

    let (hashchain_store, _) =
        ZeroCopyVecU64::from_bytes_at_multiple(hashchain_store_capacity, account_data)?;

    Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
}

#[allow(clippy::type_complexity)]
pub fn init_queue<'a>(
    account: &BatchMetadata,
    queue_type: u64,
    account_data: &'a mut [u8],
    num_iters: u64,
    bloom_filter_capacity: u64,
    batch_start_index: u64,
) -> Result<
    (
        ZeroCopySliceMutU64<'a, Batch>,
        Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
        Vec<ZeroCopySliceMutU64<'a, u8>>,
        Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
    ),
    BatchedMerkleTreeError,
> {
    let (num_value_stores, num_stores, num_hashchain_stores) =
        account.get_size_parameters(queue_type)?;

    // if queue_type == QueueType::Output as u64 {
    //     *start_offset += BatchedQueueMetadata::LEN;
    // }

    let (mut batches, account_data) =
        ZeroCopySliceMutU64::new_at(account.num_batches, account_data)?;

    for i in 0..account.num_batches {
        batches[i as usize] = Batch::new(
            num_iters,
            bloom_filter_capacity,
            account.batch_size,
            account.zkp_batch_size,
            account.batch_size * i + batch_start_index,
        );
    }
    let (value_vecs, account_data) =
        ZeroCopyVecU64::new_at_multiple(num_value_stores, account.batch_size, account_data)?;

    let (bloom_filter_stores, account_data) = ZeroCopySliceMutU64::new_at_multiple(
        num_stores,
        account.bloom_filter_capacity / 8,
        account_data,
    )?;
    // ZeroCopySliceMutU64::new_at_multiple(
    //     num_stores,
    //     account.bloom_filter_capacity as usize / 8,
    //     account_data,
    //     start_offset,
    // )?;
    // if num_stores == 2 {
    //     let account_data = &mut account_data[*start_offset..];
    //     let (bloom_filter_store, account_data) = Ref::<&mut [u8], [u8]>::from_prefix_with_elems(
    //         account_data,
    //         (account.bloom_filter_capacity / 8) as usize,
    //     )
    //     .unwrap();
    //     *start_offset += (account.bloom_filter_capacity / 8) as usize;
    //     bloom_filter_stores.push(bloom_filter_store);
    //     let (bloom_filter_store, account_data) = Ref::<&mut [u8], [u8]>::from_prefix_with_elems(
    //         account_data,
    //         (account.bloom_filter_capacity / 8) as usize,
    //     )
    //     .unwrap();
    //     *start_offset += (account.bloom_filter_capacity / 8) as usize;
    //     bloom_filter_stores.push(bloom_filter_store);
    //     *start_offset = 0;
    //     let hashchain_store = ZeroCopyVecU64::new_at_multiple(
    //         num_hashchain_stores,
    //         account.get_num_zkp_batches() as usize,
    //         account_data,
    //         start_offset,
    //     )?;

    //     Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
    // } else {
    //     assert_eq!(num_stores, 0);
    //     let hashchain_store = ZeroCopyVecU64::new_at_multiple(
    //         num_hashchain_stores,
    //         account.get_num_zkp_batches() as usize,
    //         account_data,
    //         start_offset,
    //     )?;

    //     Ok((batches, value_vecs, bloom_filter_stores, hashchain_store))
    // }
    let (hashchain_store, _) = ZeroCopyVecU64::new_at_multiple(
        num_hashchain_stores,
        account.get_num_zkp_batches(),
        account_data,
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
    value_vecs: &mut Vec<ZeroCopyVecU64<'_, [u8; 32]>>,
    bloom_filter_stores: &mut Vec<ZeroCopySliceMutU64<'_, u8>>,
    batches: &mut ZeroCopySliceMutU64<'_, Batch>,
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
            vec.len() * 8,
            batch_metadata.bloom_filter_capacity as usize,
            "bloom_filter_capacity mismatch"
        );
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
pub fn assert_queue_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: BatchedQueueMetadata,
    num_iters: u64,
) {
    let mut account = BatchedQueueAccount::output_queue_from_bytes_mut(account_data)
        .expect("from_bytes_unchecked_mut failed");
    let num_batches = ref_account.batch_metadata.num_batches as usize;
    let batch_metadata = account.batch_metadata;
    let queue_type = account.metadata.metadata.queue_type;
    let next_index = account.next_index;
    assert_eq!(
        account.metadata.metadata, ref_account.metadata,
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
