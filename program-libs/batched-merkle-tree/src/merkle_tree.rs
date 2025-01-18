use std::ops::{Deref, DerefMut};

use light_hasher::{Discriminator, Hasher, Poseidon};
use light_merkle_tree_metadata::{
    errors::MerkleTreeMetadataError,
    merkle_tree::{MerkleTreeMetadata, TreeType},
    queue::QueueType,
};
use light_utils::{
    account::{check_account_info, set_discriminator, DISCRIMINATOR_LEN},
    hashchain::create_hash_chain_from_array,
    pubkey::Pubkey,
};
use light_verifier::{
    verify_batch_address_update, verify_batch_append_with_proofs, verify_batch_update,
    CompressedProof,
};
use light_zero_copy::{
    cyclic_vec::ZeroCopyCyclicVecU64, errors::ZeroCopyError, slice_mut::ZeroCopySliceMutU64,
    vec::ZeroCopyVecU64,
};
use solana_program::{account_info::AccountInfo, msg};
use zerocopy::Ref;

use super::{
    batch::Batch,
    queue::{init_queue, input_queue_from_bytes, insert_into_current_batch},
};
use crate::{
    batch::BatchState,
    batch_metadata::BatchMetadata,
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, ADDRESS_TREE_INIT_ROOT_40, BATCHED_ADDRESS_TREE_TYPE,
        BATCHED_STATE_TREE_TYPE,
    },
    errors::BatchedMerkleTreeError,
    event::{BatchAddressAppendEvent, BatchAppendEvent, BatchNullifyEvent},
    merkle_tree_metadata::BatchedMerkleTreeMetadata,
    queue::BatchedQueueAccount,
    BorshDeserialize, BorshSerialize,
};

/// Public inputs:
/// 1. old root (last root in root history)
/// 2. new root (send to chain)
/// 3. leaf hash chain (in hashchain store)
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataBatchNullifyInputs {
    pub new_root: [u8; 32],
    pub compressed_proof: CompressedProof,
}

/// Public inputs:
/// 1. old root (last root in root history)
/// 2. new root (send to chain)
/// 3. leaf hash chain (in hashchain store)
/// 4. next index (get from metadata)
pub type InstructionDataAddressAppendInputs = InstructionDataBatchNullifyInputs;

/// Public inputs:
/// 1. old root (last root in root history)
/// 2. new root (send to chain)
/// 3. leaf hash chain (in hashchain store)
/// 4. start index (get from batch)
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataBatchAppendInputs {
    pub new_root: [u8; 32],
    pub compressed_proof: CompressedProof,
}

/// Batched Merkle tree zero copy account.
/// The account is used to batched state
/// and address Merkle trees, plus the input and address queues,
/// in the  Light Protocol account compression program.
///
/// Tree roots can be used in zk proofs
/// outside of Light Protocol programs.
///
/// To access a tree root by index use:
/// - get_state_root_by_index
/// - get_address_root_by_index
#[derive(Debug, PartialEq)]
pub struct BatchedMerkleTreeAccount<'a> {
    metadata: Ref<&'a mut [u8], BatchedMerkleTreeMetadata>,
    pub root_history: ZeroCopyCyclicVecU64<'a, [u8; 32]>,
    pub batches: ZeroCopySliceMutU64<'a, Batch>,
    pub value_vecs: Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
    pub bloom_filter_stores: Vec<ZeroCopySliceMutU64<'a, u8>>,
    pub hashchain_store: Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
}

impl Discriminator for BatchedMerkleTreeAccount<'_> {
    const DISCRIMINATOR: [u8; 8] = *b"BatchMta";
}

impl<'a> BatchedMerkleTreeAccount<'a> {
    /// Checks state Merkle tree account and returns the root.
    pub fn get_state_root_by_index(
        account_info: &AccountInfo<'a>,
        index: usize,
    ) -> Result<[u8; 32], BatchedMerkleTreeError> {
        let tree = Self::state_from_account_info(account_info)?;
        Ok(*tree
            .get_root_by_index(index)
            .ok_or(BatchedMerkleTreeError::InvalidIndex)?)
    }

    /// Checks address Merkle tree account and returns the root.
    pub fn get_address_root_by_index(
        account_info: &AccountInfo<'a>,
        index: usize,
    ) -> Result<[u8; 32], BatchedMerkleTreeError> {
        let tree = Self::address_from_account_info(account_info)?;
        Ok(*tree
            .get_root_by_index(index)
            .ok_or(BatchedMerkleTreeError::InvalidIndex)?)
    }

    /// Deserialize a batched state Merkle tree from account info.
    /// Should be used in solana programs.
    /// Checks that:
    /// 1. the program owner is the light account compression program,
    /// 2. discriminator,
    /// 3. tree type is batched state tree type.
    pub fn state_from_account_info(
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_account_info::<BATCHED_STATE_TREE_TYPE>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    /// Deserialize a state BatchedMerkleTreeAccount from bytes.
    /// Should only be used in client.
    /// Checks the discriminator and tree type.
    #[cfg(not(target_os = "solana"))]
    pub fn state_from_bytes(
        account_data: &'a mut [u8],
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        use light_utils::account::check_discriminator;
        check_discriminator::<Self>(&account_data[0..DISCRIMINATOR_LEN])?;
        Self::from_bytes::<BATCHED_STATE_TREE_TYPE>(account_data)
    }

    /// Deserialize a batched address Merkle tree from account info.
    /// Should be used in solana programs.
    /// Checks that:
    /// 1. the program owner is the light account compression program,
    /// 2. discriminator,
    /// 3. tree type is batched address tree type.
    pub fn address_from_account_info(
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_account_info::<BATCHED_ADDRESS_TREE_TYPE>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    fn from_account_info<const TREE_TYPE: u64>(
        program_id: &solana_program::pubkey::Pubkey,
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        check_account_info::<Self>(program_id, account_info)?;
        let mut data = account_info.try_borrow_mut_data()?;

        // Necessary to convince the borrow checker.
        let data_slice: &'a mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
        Self::from_bytes::<TREE_TYPE>(data_slice)
    }

    /// Deserialize a state BatchedMerkleTreeAccount from bytes.
    /// Should only be used in client.
    /// Checks the discriminator and tree type.
    #[cfg(not(target_os = "solana"))]
    pub fn address_from_bytes(
        account_data: &'a mut [u8],
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_bytes::<BATCHED_ADDRESS_TREE_TYPE>(account_data)
    }

    fn from_bytes<const TREE_TYPE: u64>(
        account_data: &'a mut [u8],
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        let account_data_len = account_data.len();
        // Discriminator is already checked in check_account_info.
        let (_discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        let (metadata, account_data) =
            Ref::<&'a mut [u8], BatchedMerkleTreeMetadata>::from_prefix(account_data)
                .map_err(|e| BatchedMerkleTreeError::ZeroCopyCastError(e.to_string()))?;
        if metadata.tree_type != TREE_TYPE {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        if account_data_len != metadata.get_account_size()? {
            return Err(ZeroCopyError::InvalidAccountSize.into());
        }

        let (root_history, account_data) = ZeroCopyCyclicVecU64::from_bytes_at(account_data)?;
        let (batches, value_vecs, bloom_filter_stores, hashchain_store) = input_queue_from_bytes(
            &metadata.queue_metadata,
            account_data,
            QueueType::BatchedInput as u64,
        )?;

        Ok(BatchedMerkleTreeAccount {
            metadata,
            root_history,
            batches,
            value_vecs,
            bloom_filter_stores,
            hashchain_store,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(
        account_data: &'a mut [u8],
        metadata: MerkleTreeMetadata,
        root_history_capacity: u32,
        num_batches_input_queue: u64,
        input_queue_batch_size: u64,
        input_queue_zkp_batch_size: u64,
        height: u32,
        num_iters: u64,
        bloom_filter_capacity: u64,
        tree_type: TreeType,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        let account_data_len = account_data.len();
        let (discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        set_discriminator::<Self>(discriminator)?;

        let (mut account_metadata, account_data) =
            Ref::<&'a mut [u8], BatchedMerkleTreeMetadata>::from_prefix(account_data)
                .map_err(|e| BatchedMerkleTreeError::ZeroCopyCastError(e.to_string()))?;
        account_metadata.metadata = metadata;
        account_metadata.root_history_capacity = root_history_capacity;
        account_metadata.height = height;
        account_metadata.tree_type = tree_type as u64;
        account_metadata.capacity = 2u64.pow(height);
        account_metadata.queue_metadata.init(
            num_batches_input_queue,
            input_queue_batch_size,
            input_queue_zkp_batch_size,
        )?;
        account_metadata.queue_metadata.bloom_filter_capacity = bloom_filter_capacity;
        if account_data_len != account_metadata.get_account_size()? {
            msg!("merkle_tree_metadata: {:?}", account_metadata);
            msg!("account_data.len(): {}", account_data.len());
            msg!(
                "account.get_account_size(): {}",
                account_metadata.get_account_size()?
            );
            return Err(ZeroCopyError::InvalidAccountSize.into());
        }

        let (mut root_history, account_data) = ZeroCopyCyclicVecU64::new_at(
            account_metadata.root_history_capacity as u64,
            account_data,
        )?;

        // Initialize root history with zero bytes to enable
        // unified logic to zero out roots.
        // An unitialized root history vector
        // would be an edge case.
        for _ in 0..root_history.capacity() {
            root_history.push([0u8; 32]);
        }
        if tree_type == TreeType::BatchedState {
            root_history.push(light_hasher::Poseidon::zero_bytes()[height as usize]);
        } else if tree_type == TreeType::BatchedAddress {
            // Initialized indexed Merkle tree root.
            root_history.push(ADDRESS_TREE_INIT_ROOT_40);
            // The initialized indexed Merkle tree contains two elements.
            account_metadata.next_index = 2;
        }

        let (batches, value_vecs, bloom_filter_stores, hashchain_store) = init_queue(
            &account_metadata.queue_metadata,
            QueueType::BatchedInput as u64,
            account_data,
            num_iters,
            bloom_filter_capacity,
            account_metadata.next_index,
        )?;
        Ok(BatchedMerkleTreeAccount {
            metadata: account_metadata,
            root_history,
            batches,
            value_vecs,
            bloom_filter_stores,
            hashchain_store,
        })
    }

    /// Update the tree from the output queue account.
    /// 1. Checks that the tree and queue are associated.
    /// 2. Updates the tree with the output queue account.
    /// 3. Returns the batch append event.
    pub fn update_tree_from_output_queue_account_info(
        &mut self,
        queue_account_info: &AccountInfo<'_>,
        instruction_data: InstructionDataBatchAppendInputs,
        id: [u8; 32],
    ) -> Result<BatchAppendEvent, BatchedMerkleTreeError> {
        if self.tree_type != TreeType::BatchedState as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        if self.metadata.metadata.associated_queue != (*queue_account_info.key).into() {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into());
        }
        let queue_account = &mut BatchedQueueAccount::output_from_account_info(queue_account_info)?;
        self.update_tree_from_output_queue_account(queue_account, instruction_data, id)
    }

    /// Update the tree from the output queue account.
    /// 1. Create public inputs hash.
    /// 2. Verify update proof and update tree account.
    ///     2.1. Verify proof.
    ///     2.2. Increment sequence number.
    ///     2.3. Increment next index.
    ///     2.4. Append new root to root history.
    /// 3. Mark zkp batch as inserted in the merkle tree.
    ///     3.1. Checks that the batch is ready.
    ///     3.2. Increment the number of inserted zkps.
    ///     3.3. If all zkps are inserted, set batch state to inserted.
    /// 4. Increment next full batch index if inserted.
    /// 5. Return the batch append event.
    ///     
    /// Note: when proving inclusion by index in
    ///     value array we need to insert the value into a bloom_filter once it is
    ///     inserted into the tree. Check this with get_num_inserted_zkps
    pub fn update_tree_from_output_queue_account(
        &mut self,
        queue_account: &mut BatchedQueueAccount,
        instruction_data: InstructionDataBatchAppendInputs,
        id: [u8; 32],
    ) -> Result<BatchAppendEvent, BatchedMerkleTreeError> {
        let full_batch_index = queue_account.batch_metadata.next_full_batch_index as usize;
        let new_root = instruction_data.new_root;
        let circuit_batch_size = queue_account.batch_metadata.zkp_batch_size;
        let start_index = self.next_index;
        let full_batch = &mut queue_account.batches[full_batch_index];
        let num_zkps = full_batch.get_first_ready_zkp_batch()?;

        // 1. Create public inputs hash.
        let public_input_hash = {
            let leaves_hashchain =
                queue_account.hashchain_store[full_batch_index][num_zkps as usize];
            let old_root = self
                .root_history
                .last()
                .ok_or(BatchedMerkleTreeError::InvalidIndex)?;
            let mut start_index_bytes = [0u8; 32];
            start_index_bytes[24..].copy_from_slice(&start_index.to_be_bytes());
            create_hash_chain_from_array([
                *old_root,
                new_root,
                leaves_hashchain,
                start_index_bytes,
            ])?
        };

        // 2. Verify update proof and update tree account.
        self.verify_update::<5>(
            circuit_batch_size,
            instruction_data.compressed_proof,
            public_input_hash,
            new_root,
        )?;

        let root_index = self.root_history.last_index() as u32;

        // Update metadata and batch.
        {
            // 3. Mark zkp batch as inserted in the merkle tree.
            let full_batch_state = full_batch.mark_as_inserted_in_merkle_tree(
                self.metadata.sequence_number,
                root_index,
                self.root_history_capacity,
            )?;
            // 4. Increment next full batch index if inserted.
            queue_account
                .batch_metadata
                .increment_next_full_batch_index_if_inserted(full_batch_state);
        }
        // 5. Return the batch append event.
        Ok(BatchAppendEvent {
            id,
            batch_index: full_batch_index as u64,
            zkp_batch_index: num_zkps,
            old_next_index: start_index,
            new_next_index: start_index + circuit_batch_size,
            batch_size: circuit_batch_size,
            new_root,
            root_index,
            sequence_number: self.sequence_number,
        })
    }

    /// Update the tree from the input queue account.
    pub fn update_tree_from_input_queue(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent, BatchedMerkleTreeError> {
        if self.tree_type != TreeType::BatchedState as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        self.update_input_queue::<3>(instruction_data, id)
    }

    /// Update the tree from the address queue account.
    pub fn update_tree_from_address_queue(
        &mut self,
        instruction_data: InstructionDataAddressAppendInputs,
        id: [u8; 32],
    ) -> Result<BatchAddressAppendEvent, BatchedMerkleTreeError> {
        if self.tree_type != TreeType::BatchedAddress as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        self.update_input_queue::<4>(instruction_data, id)
    }

    /// Update the tree from the input/address queue account.
    /// 1. Create public inputs hash.
    /// 2. Verify update proof and update tree account.
    ///     2.1. Verify proof.
    ///     2.2. Increment sequence number.
    ///     2.3. If address tree increment next index.
    ///     2.4. Append new root to root history.
    /// 3. Mark batch as inserted in the merkle tree.
    ///     3.1. Checks that the batch is ready.
    ///     3.2. Increment the number of inserted zkps.
    ///     3.3. If all zkps are inserted, set the state to inserted.
    /// 4. Zero out previous batch bloom filter if current batch is 50% inserted.
    /// 5. Increment next full batch index if inserted.
    /// 6. Return the batch nullify event.
    fn update_input_queue<const QUEUE_TYPE: u64>(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent, BatchedMerkleTreeError> {
        let full_batch_index = self.queue_metadata.next_full_batch_index as usize;
        let num_zkps = self.batches[full_batch_index].get_first_ready_zkp_batch()?;
        let new_root = instruction_data.new_root;
        let circuit_batch_size = self.queue_metadata.zkp_batch_size;

        // 1. Create public inputs hash.
        let public_input_hash = {
            let leaves_hashchain = self.hashchain_store[full_batch_index][num_zkps as usize];
            let old_root = self
                .root_history
                .last()
                .ok_or(BatchedMerkleTreeError::InvalidIndex)?;

            if QUEUE_TYPE == QueueType::BatchedInput as u64 {
                create_hash_chain_from_array([*old_root, new_root, leaves_hashchain])?
            } else if QUEUE_TYPE == QueueType::BatchedAddress as u64 {
                let mut next_index_bytes = [0u8; 32];
                next_index_bytes[24..].copy_from_slice(self.next_index.to_be_bytes().as_slice());
                create_hash_chain_from_array([
                    *old_root,
                    new_root,
                    leaves_hashchain,
                    next_index_bytes,
                ])?
            } else {
                return Err(MerkleTreeMetadataError::InvalidQueueType.into());
            }
        };

        // 2. Verify update proof and update tree account.
        self.verify_update::<QUEUE_TYPE>(
            circuit_batch_size,
            instruction_data.compressed_proof,
            public_input_hash,
            new_root,
        )?;

        let root_index = self.root_history.last_index() as u32;

        // Update queue metadata.
        {
            let root_history_capacity = self.root_history_capacity;
            let sequence_number = self.sequence_number;
            // 3. Mark batch as inserted in the merkle tree.
            let full_batch_state = self.batches[full_batch_index].mark_as_inserted_in_merkle_tree(
                sequence_number,
                root_index,
                root_history_capacity,
            )?;

            // 4. Zero out previous batch bloom filter
            //     if current batch is 50% inserted.
            // Needs to be executed prior to
            // incrementing next full batch index,
            // but post mark_as_inserted_in_merkle_tree.
            self.zero_out_previous_batch_bloom_filter()?;

            // 5. Increment next full batch index if inserted.
            self.metadata
                .queue_metadata
                .increment_next_full_batch_index_if_inserted(full_batch_state);
        }

        // 6. Return the batch nullify/address append event.
        Ok(BatchNullifyEvent {
            id,
            batch_index: full_batch_index as u64,
            batch_size: circuit_batch_size,
            zkp_batch_index: num_zkps,
            new_root,
            root_index,
            sequence_number: self.sequence_number,
        })
    }

    /// Verify update proof and update the tree.
    /// 1. Verify update proof.
    /// 2. Increment next index (unless queue type is BatchedInput).
    /// 3. Increment sequence number.
    /// 4. Append new root to root history.
    fn verify_update<const QUEUE_TYPE: u64>(
        &mut self,
        batch_size: u64,
        proof: CompressedProof,
        public_input_hash: [u8; 32],
        new_root: [u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        // 1. Verify update proof.
        if QUEUE_TYPE == QueueType::BatchedOutput as u64 {
            verify_batch_append_with_proofs(batch_size, public_input_hash, &proof)?;
            // 2. Increment next index.
            self.metadata.next_index += batch_size;
        } else if QUEUE_TYPE == QueueType::BatchedInput as u64 {
            verify_batch_update(batch_size, public_input_hash, &proof)?;
            // 2. skip incrementing next index.
            // The input queue update does not append new values
            // hence no need to increment next_index.
        } else if QUEUE_TYPE == QueueType::BatchedAddress as u64 {
            verify_batch_address_update(batch_size, public_input_hash, &proof)?;
            // 2. Increment next index.
            self.metadata.next_index += batch_size;
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType.into());
        }
        // 3. Increment sequence number.
        self.metadata.sequence_number += 1;
        // 4. Append new root to root history.
        // root_history is a cyclic vec
        // it will overwrite the oldest root
        // once it is full.
        self.root_history.push(new_root);
        Ok(())
    }

    /// Insert nullifier into current batch.
    /// 1. Check that the tree is a state tree.
    /// 2. Create nullifier Hash(value,leaf_index, tx_hash).
    /// 3. Insert nullifier into current batch.
    ///     3.1. Insert compressed_account_hash into bloom filter.
    ///         (bloom filter enables non-inclusion proofs in later txs)
    ///     3.2. Add nullifier to leaves hash chain.
    ///         (Nullification means, the compressed_account_hash in the tree,
    ///         is overwritten with a nullifier hash)
    ///     3.3. Check that compressed_account_hash
    ///         does not exist in any other bloom filter.
    pub fn insert_nullifier_into_current_batch(
        &mut self,
        compressed_account_hash: &[u8; 32],
        leaf_index: u64,
        tx_hash: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        // Note, no need to check whether the tree is full
        // since nullifier insertions update existing values
        // in the tree and do not append new values.

        // 1. Check that the tree is a state tree.
        if self.tree_type != TreeType::BatchedState as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }

        // 2. Create nullifier Hash(value,leaf_index, tx_hash).
        let nullifier = {
            let leaf_index_bytes = leaf_index.to_be_bytes();
            // Inclusion of the tx_hash enables zk proofs of how a value was spent.
            Poseidon::hashv(&[compressed_account_hash, &leaf_index_bytes, tx_hash])?
        };
        // 3. Insert nullifier into current batch.
        self.insert_into_current_batch(compressed_account_hash, &nullifier)
    }

    pub fn insert_address_into_current_batch(
        &mut self,
        address: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        if self.tree_type != TreeType::BatchedAddress as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        // Check that the tree is not full.
        self.check_tree_is_full()?;

        self.insert_into_current_batch(address, address)
    }

    /// Insert value into the current batch.
    /// 1. Insert value
    /// 2. Zero out roots if bloom filter
    ///     was zeroed out in (insert_into_current_batch).
    fn insert_into_current_batch(
        &mut self,
        bloom_filter_value: &[u8; 32],
        leaves_hash_value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let (root_index, sequence_number) = insert_into_current_batch(
            QueueType::BatchedInput as u64,
            &mut self.metadata.queue_metadata,
            &mut self.batches,
            &mut self.value_vecs,
            &mut self.bloom_filter_stores,
            &mut self.hashchain_store,
            leaves_hash_value,
            Some(bloom_filter_value),
            None,
        )?;

        /*
         * Note on security for root buffer:
         * Account {
         *   bloom_filter: [B0, B1],
         *     roots: [R0, R1, R2, R3, R4, R5, R6, R7, R8, R9],
         * }
         *
         * Timeslot 0:
         * - insert into B0 until full
         *
         * Timeslot 1:
         * - insert into B1 until full
         * - update tree with B0 in 4 partial updates, don't clear B0 yet
         * -> R0 -> B0.1
         * -> R1 -> B0.2
         * -> R2 -> B0.3
         * -> R3 -> B0.4 - final B0 root
         * B0.sequence_number = 13 (3 + account.root.length)
         * B0.root_index = 3
         * - execute some B1 root updates
         * -> R4 -> B1.1
         * -> R5 -> B1.2
         * -> R6 -> B1.3
         * -> R7 -> B1.4 - final B1 (update batch 0) root
         * B0.sequence_number = 17 (7 + account.root.length)
         * B0.root_index = 7
         * current_sequence_number = 8
         * Timeslot 2:
         * - clear B0
         *   - current_sequence_number < 14 -> zero out all roots until root index is 3
         * - R8 -> 0
         * - R9 -> 0
         * - R0 -> 0
         * - R1 -> 0
         * - R2 -> 0
         * - now all roots containing values nullified in the final B0 root update are zeroed
         * .-> B0 is safe to clear
         */
        if let Some(sequence_number) = sequence_number {
            // If the sequence number is greater than current sequence number
            // there is still at least one root which can be used to prove
            // inclusion of a value which was in the batch that was just zeroed out.
            self.zero_out_roots(
                sequence_number,
                root_index.ok_or(BatchedMerkleTreeError::InvalidIndex)?,
            );
        }

        Ok(())
    }

    /// Zero out roots corresponding to batch.sequence numbers > tree.sequence_number.
    /// batch.sequence_number marks the sequence number all roots are overwritten
    /// which can prove inclusion of a value inserted in the queue.
    /// 1. Check whether overlapping roots exist.
    /// 2. If yes:
    ///     2.1 Get, first safe root index.
    ///     2.2 Zero out roots from the oldest root to first safe root.
    fn zero_out_roots(&mut self, sequence_number: u64, first_safe_root_index: u32) {
        // 1. Check whether overlapping roots exist.
        let overlapping_roots_exits = sequence_number > self.sequence_number;
        if overlapping_roots_exits {
            let mut oldest_root_index = self.root_history.first_index();
            // 2.1. Get, num of remaining roots.
            //    Remaining roots have not been updated since
            //    the update of the previous batch hence enable to prove
            //    inclusion of values nullified in the previous batch.
            let num_remaining_roots = sequence_number - self.sequence_number;
            println!("sequence_number: {}", sequence_number);
            println!("self.sequence_number: {}", self.sequence_number);
            println!("oldest_root_index: {}", oldest_root_index);
            println!("first_safe_root_index: {}", first_safe_root_index);
            println!("num_remaining_roots: {}", num_remaining_roots);
            println!(
                "self.root_history.len() as u64: {}",
                self.root_history.len() as u64
            );
            // 2.2. Zero out roots oldest to first safe root index.
            //      Skip one iteration we don't need to zero out
            //      the first safe root.
            for _ in 1..num_remaining_roots {
                println!("zeroing out root index: {}", oldest_root_index);
                self.root_history[oldest_root_index] = [0u8; 32];
                oldest_root_index += 1;
                oldest_root_index %= self.root_history.len();
            }
            assert_eq!(
                oldest_root_index as u32, first_safe_root_index,
                "Zeroing out roots failed."
            );
        }
    }

    /// Zero out bloom filter of previous batch if 50% of the
    /// current batch has been processed.
    ///
    /// Idea:
    /// 1. Zeroing out the bloom filter of the previous batch is expensive
    ///     -> the forester should do it.
    /// 2. We don't want to zero out the bloom filter when inserting
    ///     the last zkp of a batch for this might result in failing user tx.
    /// 3. Wait until next batch is 50% full as grace period for clients
    ///     to switch from proof by index to proof by zkp
    ///     for values inserted in the previous batch.
    ///
    /// Steps:
    /// 1. Previous batch must be inserted and bloom filter must not be zeroed out.
    /// 2. Current batch must be 50% full
    /// 3. if yes
    ///    3.1 zero out bloom filter
    ///    3.2 mark bloom filter as zeroed
    ///    3.3 zero out roots if needed
    fn zero_out_previous_batch_bloom_filter(&mut self) -> Result<(), BatchedMerkleTreeError> {
        let current_batch = self.queue_metadata.next_full_batch_index as usize;
        let batch_size = self.queue_metadata.batch_size;
        let previous_full_batch_index = current_batch.saturating_sub(1);
        let previous_full_batch_index = if previous_full_batch_index == current_batch {
            self.queue_metadata.num_batches as usize - 1
        } else {
            previous_full_batch_index
        };

        let current_batch_is_half_full = {
            let num_inserted_elements = self
                .batches
                .get(current_batch)
                .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?
                .get_num_inserted_elements();
            // Keep for finegrained unit test
            println!("current_batch: {}", current_batch);
            println!("previous_full_batch_index: {}", previous_full_batch_index);
            println!("num_inserted_elements: {}", num_inserted_elements);
            println!("batch_size: {}", batch_size);
            println!("batch_size / 2: {}", batch_size / 2);
            println!(
                "num_inserted_elements >= batch_size / 2: {}",
                num_inserted_elements >= batch_size / 2
            );
            num_inserted_elements >= batch_size / 2
        };

        let previous_full_batch = self
            .batches
            .get_mut(previous_full_batch_index)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;

        let batch_is_inserted = previous_full_batch.get_state() == BatchState::Inserted;
        let previous_batch_is_ready =
            batch_is_inserted && !previous_full_batch.bloom_filter_is_zeroed();

        if previous_batch_is_ready && current_batch_is_half_full {
            // Keep for finegrained unit test
            println!("Wiping bloom filter of previous batch");
            println!("current_batch: {}", current_batch);
            println!("previous_full_batch_index: {}", previous_full_batch_index);
            // 3.1 Zero out bloom filter.
            {
                let bloom_filter = self
                    .bloom_filter_stores
                    .get_mut(previous_full_batch_index)
                    .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;
                bloom_filter.as_mut_slice().iter_mut().for_each(|x| *x = 0);
            }
            // 3.2 Mark bloom filter zeroed.
            previous_full_batch.set_bloom_filter_to_zeroed();
            // 3.3 Zero out roots if a root exists in root history
            // which allows to prove inclusion of a value
            // that was inserted into the bloom filter just zeroed out.
            {
                let seq = previous_full_batch.sequence_number;
                let root_index = previous_full_batch.root_index;
                self.zero_out_roots(seq, root_index);
            }
        }

        Ok(())
    }

    pub fn get_root_index(&self) -> u32 {
        self.root_history.last_index() as u32
    }

    pub fn get_root(&self) -> Option<[u8; 32]> {
        self.root_history.last().copied()
    }

    pub fn get_root_by_index(&self, index: usize) -> Option<&[u8; 32]> {
        self.root_history.get(index)
    }

    pub fn get_metadata(&self) -> &BatchedMerkleTreeMetadata {
        &self.metadata
    }

    pub fn get_metadata_mut(&mut self) -> &mut BatchedMerkleTreeMetadata {
        &mut self.metadata
    }

    // TODO: add unit test
    /// Checks non-inclusion in all bloom filters
    /// which are not zeroed.
    pub fn check_input_queue_non_inclusion(
        &mut self,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let num_bloom_filters = self.bloom_filter_stores.len();
        for i in 0..num_bloom_filters {
            let bloom_filter_store = self.bloom_filter_stores[i].as_mut_slice();
            let batch = &self.batches[i];
            if !batch.bloom_filter_is_zeroed() {
                batch.check_non_inclusion(value, bloom_filter_store)?;
            }
        }
        Ok(())
    }

    pub fn tree_is_full(&self) -> bool {
        self.next_index == self.capacity
    }

    pub fn check_tree_is_full(&self) -> Result<(), BatchedMerkleTreeError> {
        if self.tree_is_full() {
            return Err(BatchedMerkleTreeError::TreeIsFull);
        }
        Ok(())
    }
}

pub fn get_merkle_tree_account_size_default() -> usize {
    let mt_account = BatchedMerkleTreeMetadata::default();
    mt_account.get_account_size().unwrap()
}

impl Deref for BatchedMerkleTreeAccount<'_> {
    type Target = BatchedMerkleTreeMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl DerefMut for BatchedMerkleTreeAccount<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

pub fn get_merkle_tree_account_size(
    batch_size: u64,
    bloom_filter_capacity: u64,
    zkp_batch_size: u64,
    root_history_capacity: u32,
    height: u32,
    num_batches: u64,
) -> usize {
    let mt_account = BatchedMerkleTreeMetadata {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: TreeType::BatchedState as u64,
        height,
        root_history_capacity,
        queue_metadata: BatchMetadata {
            num_batches,
            batch_size,
            bloom_filter_capacity,
            zkp_batch_size,
            ..Default::default()
        },
        capacity: 2u64.pow(height),
    };
    mt_account.get_account_size().unwrap()
}

pub fn assert_nullify_event(
    event: BatchNullifyEvent,
    new_root: [u8; 32],
    old_account: &BatchedMerkleTreeAccount,
    mt_pubkey: Pubkey,
) {
    let batch_index = old_account.queue_metadata.next_full_batch_index;
    let batch = old_account.batches.get(batch_index as usize).unwrap();
    let ref_event = BatchNullifyEvent {
        id: mt_pubkey.to_bytes(),
        batch_index,
        zkp_batch_index: batch.get_num_inserted_zkps(),
        new_root,
        root_index: (old_account.get_root_index() + 1) % old_account.root_history_capacity,
        sequence_number: old_account.sequence_number + 1,
        batch_size: old_account.queue_metadata.zkp_batch_size,
    };
    assert_eq!(event, ref_event);
}

pub fn assert_batch_append_event_event(
    event: BatchAppendEvent,
    new_root: [u8; 32],
    old_output_queue_account: &BatchedQueueAccount,
    old_account: &BatchedMerkleTreeAccount,
    mt_pubkey: Pubkey,
) {
    let batch_index = old_output_queue_account
        .batch_metadata
        .next_full_batch_index;
    let batch = old_output_queue_account
        .batches
        .get(batch_index as usize)
        .unwrap();
    let ref_event = BatchAppendEvent {
        id: mt_pubkey.to_bytes(),
        batch_index,
        zkp_batch_index: batch.get_num_inserted_zkps(),
        new_root,
        root_index: (old_account.get_root_index() + 1) % old_account.root_history_capacity,
        sequence_number: old_account.sequence_number + 1,
        batch_size: old_account.queue_metadata.zkp_batch_size,
        old_next_index: old_account.next_index,
        new_next_index: old_account.next_index
            + old_output_queue_account.batch_metadata.zkp_batch_size,
    };
    assert_eq!(event, ref_event);
}
