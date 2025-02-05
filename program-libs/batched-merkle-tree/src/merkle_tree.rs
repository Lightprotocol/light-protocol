use std::ops::{Deref, DerefMut};

use light_hasher::{Discriminator, Hasher, Poseidon};
use light_merkle_tree_metadata::{
    errors::MerkleTreeMetadataError,
    merkle_tree::{MerkleTreeMetadata, TreeType},
    queue::QueueType,
};
use light_utils::{
    account::{check_account_info, set_discriminator, DISCRIMINATOR_LEN},
    hash_to_bn254_field_size_be,
    hashchain::create_hash_chain_from_array,
    instruction::compressed_proof::CompressedProof,
    pubkey::Pubkey,
};
use light_verifier::{
    verify_batch_address_update, verify_batch_append_with_proofs, verify_batch_update,
};
use light_zero_copy::{
    cyclic_vec::ZeroCopyCyclicVecU64, errors::ZeroCopyError, vec::ZeroCopyVecU64,
};
use solana_program::{account_info::AccountInfo, msg};
use zerocopy::Ref;

use super::{batch::Batch, queue::insert_into_current_batch};
use crate::{
    batch::BatchState,
    batch_metadata::BatchMetadata,
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, ADDRESS_TREE_INIT_ROOT_40, BATCHED_ADDRESS_TREE_TYPE,
        BATCHED_STATE_TREE_TYPE, NUM_BATCHES,
    },
    errors::BatchedMerkleTreeError,
    event::{BatchAddressAppendEvent, BatchAppendEvent, BatchNullifyEvent},
    merkle_tree_metadata::BatchedMerkleTreeMetadata,
    queue::{deserialize_bloom_filter_stores, BatchedQueueAccount},
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
    pubkey: Pubkey,
    metadata: Ref<&'a mut [u8], BatchedMerkleTreeMetadata>,
    pub root_history: ZeroCopyCyclicVecU64<'a, [u8; 32]>,
    pub bloom_filter_stores: [&'a mut [u8]; 2],
    pub hash_chain_stores: [ZeroCopyVecU64<'a, [u8; 32]>; 2],
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
        let mut tree = Self::from_bytes::<TREE_TYPE>(data_slice)?;
        tree.pubkey = (*account_info.key).into();
        Ok(tree)
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
        // Discriminator is already checked in check_account_info.
        let (_discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        let (metadata, account_data) =
            Ref::<&'a mut [u8], BatchedMerkleTreeMetadata>::from_prefix(account_data)
                .map_err(ZeroCopyError::from)?;
        if metadata.tree_type != TREE_TYPE {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }

        let (root_history, account_data) = ZeroCopyCyclicVecU64::from_bytes_at(account_data)?;

        let (bloom_filter_stores, account_data) = deserialize_bloom_filter_stores(
            metadata.queue_metadata.get_bloomfilter_size_bytes(),
            account_data,
        );

        let (vec_1, account_data) = ZeroCopyVecU64::from_bytes_at(account_data)?;
        let (vec_2, _) = ZeroCopyVecU64::from_bytes_at(account_data)?;
        Ok(BatchedMerkleTreeAccount {
            pubkey: Pubkey::default(),
            metadata,
            root_history,
            bloom_filter_stores,
            hash_chain_stores: [vec_1, vec_2],
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(
        account_data: &'a mut [u8],
        pubkey: &Pubkey,
        metadata: MerkleTreeMetadata,
        root_history_capacity: u32,
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
                .map_err(ZeroCopyError::from)?;
        account_metadata.hashed_pubkey = hash_to_bn254_field_size_be(&pubkey.to_bytes()).unwrap().0;
        account_metadata.metadata = metadata;
        account_metadata.root_history_capacity = root_history_capacity;
        account_metadata.height = height;
        account_metadata.tree_type = tree_type as u64;
        account_metadata.capacity = 2u64.pow(height);
        account_metadata
            .queue_metadata
            .init(input_queue_batch_size, input_queue_zkp_batch_size)?;

        account_metadata.queue_metadata.bloom_filter_capacity = bloom_filter_capacity;
        if account_data_len != account_metadata.get_account_size()? {
            msg!("merkle_tree_metadata: {:?}", account_metadata);
            msg!("account_data.len(): {}", account_data.len());
            msg!(
                "account.get_account_size(): {}",
                account_metadata.get_account_size()?
            );
            return Err(ZeroCopyError::Size.into());
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
        let next_index = account_metadata.next_index;

        for (i, batches) in account_metadata
            .queue_metadata
            .batches
            .iter_mut()
            .enumerate()
        {
            *batches = Batch::new(
                num_iters,
                bloom_filter_capacity,
                input_queue_batch_size,
                input_queue_zkp_batch_size,
                input_queue_batch_size * (i as u64) + next_index,
            );
        }

        let (bloom_filter_stores, account_data) = crate::queue::deserialize_bloom_filter_stores(
            account_metadata.queue_metadata.get_bloomfilter_size_bytes(),
            account_data,
        );
        let (vec_1, account_data) = ZeroCopyVecU64::new_at(
            account_metadata.queue_metadata.get_num_zkp_batches(),
            account_data,
        )?;
        let (vec_2, _account_data) = ZeroCopyVecU64::new_at(
            account_metadata.queue_metadata.get_num_zkp_batches(),
            account_data,
        )?;
        Ok(BatchedMerkleTreeAccount {
            pubkey: Pubkey::default(),
            metadata: account_metadata,
            root_history,
            bloom_filter_stores,
            hash_chain_stores: [vec_1, vec_2],
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
        let num_zkps =
            queue_account.batch_metadata.batches[full_batch_index].get_first_ready_zkp_batch()?;

        // 1. Create public inputs hash.
        let public_input_hash = {
            let leaves_hashchain =
                queue_account.hash_chain_stores[full_batch_index][num_zkps as usize];
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
            let full_batch_state = queue_account.batch_metadata.batches[full_batch_index]
                .mark_as_inserted_in_merkle_tree(
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
        let num_zkps = self.queue_metadata.batches[full_batch_index].get_first_ready_zkp_batch()?;
        let new_root = instruction_data.new_root;
        let circuit_batch_size = self.queue_metadata.zkp_batch_size;

        // 1. Create public inputs hash.
        let public_input_hash = {
            let leaves_hashchain = self.hash_chain_stores[full_batch_index][num_zkps as usize];
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
            let full_batch_state = self.queue_metadata.batches[full_batch_index]
                .mark_as_inserted_in_merkle_tree(
                    sequence_number,
                    root_index,
                    root_history_capacity,
                )?;
            // 4. Increment next full batch index if inserted.
            self.metadata
                .queue_metadata
                .increment_next_full_batch_index_if_inserted(full_batch_state);
            // 5. Zero out previous batch bloom filter
            //     if current batch is 50% inserted.
            // Needs to be executed post mark_as_inserted_in_merkle_tree.
            self.zero_out_previous_batch_bloom_filter()?;
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
        //      3.1. nullifier is inserted into the hash chain
        //          so that it can be inserted into the tree.
        //          It is ok that the nullifier is tx specific
        //          by depending on the tx_hash since we replace
        //          the compresed account hash with the nullifier
        //          (any value other than the hash itself would nullify it).
        //      3.2. Insert compressed_account_hash into bloom filter
        //          to prevent spending the same value twice.
        //          We cannot insert the nullifier into the bloom filter
        //          since it depends on the tx hash which changes.
        insert_into_current_batch(
            QueueType::BatchedInput as u64,
            &mut self.metadata.queue_metadata,
            &mut [],
            &mut self.bloom_filter_stores,
            &mut self.hash_chain_stores,
            &nullifier,
            Some(compressed_account_hash),
            None,
        )
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

        insert_into_current_batch(
            QueueType::BatchedInput as u64,
            &mut self.metadata.queue_metadata,
            &mut [],
            &mut self.bloom_filter_stores,
            &mut self.hash_chain_stores,
            address,
            Some(address),
            None,
        )?;
        self.queue_metadata.next_index += 1;
        Ok(())
    }

    /// Zero out roots corresponding to batch.sequence numbers > tree.sequence_number.
    /// batch.sequence_number marks the sequence number all roots are overwritten
    /// which can prove inclusion of a value inserted in the queue.
    /// 1. Check whether overlapping roots exist.
    /// 2. If yes:
    ///     2.1. Get, first safe root index.
    ///     2.2. Zero out roots from the oldest root to first safe root.
    ///
    /// Note on security for root buffer:
    /// Account {
    ///   bloom_filter: [B0, B1],
    ///     roots: [R0, R1, R2, R3, R4, R5, R6, R7, R8, R9],
    /// }
    ///
    /// Timeslot 0:
    /// - insert into B0 until full
    ///
    /// Timeslot 1:
    /// - insert into B1 until full
    /// - update tree with B0 in 4 partial updates, don't clear B0 yet
    ///     -> R0 -> B0.1
    ///     -> R1 -> B0.2
    ///     -> R2 -> B0.3
    ///     -> R3 -> B0.4 - final B0 root
    ///     B0.sequence_number = 13 (3 + account.root.length)
    ///     B0.root_index = 3
    /// - execute some B1 root updates
    ///     -> R4 -> B1.1
    ///     -> R5 -> B1.2
    ///     -> R6 -> B1.3
    ///     -> R7 -> B1.4 - final B1 (update batch 0) root
    ///     B0.sequence_number = 17 (7 + account.root.length)
    ///     B0.root_index = 7
    ///     current_sequence_number = 8
    ///
    /// Timeslot 2:
    ///     - clear B0
    ///     - current_sequence_number < 14 -> zero out all roots until root index is 3
    ///     - R8 -> 0
    ///     - R9 -> 0
    ///     - R0 -> 0
    ///     - R1 -> 0
    ///     - R2 -> 0
    ///     - now all roots containing values nullified in the final B0 root update are zeroed
    ///     - B0 is safe to clear
    ///
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
            // 2.2. Zero out roots oldest to first safe root index.
            //      Skip one iteration we don't need to zero out
            //      the first safe root.
            for _ in 1..num_remaining_roots {
                self.root_history[oldest_root_index] = [0u8; 32];
                oldest_root_index += 1;
                oldest_root_index %= self.root_history.len();
            }
            // Defensive assert, it should never be triggered.
            assert_eq!(
                oldest_root_index, first_safe_root_index as usize,
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
    ///    3.1. mark bloom filter as zeroed
    ///    3.2. zero out bloom filter
    ///    3.3. zero out roots if needed
    fn zero_out_previous_batch_bloom_filter(&mut self) -> Result<(), BatchedMerkleTreeError> {
        let current_batch = self.queue_metadata.next_full_batch_index as usize;
        let batch_size = self.queue_metadata.batch_size;
        let previous_full_batch_index = {
            let previous_full_batch_index = current_batch.saturating_sub(1);
            if previous_full_batch_index == current_batch {
                self.queue_metadata.num_batches as usize - 1
            } else {
                previous_full_batch_index
            }
        };
        let current_batch_is_half_full = {
            let num_inserted_elements =
                self.queue_metadata.batches[current_batch].get_num_inserted_elements();
            num_inserted_elements >= batch_size / 2
        };

        let previous_full_batch = self
            .queue_metadata
            .batches
            .get_mut(previous_full_batch_index)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;

        let batch_is_inserted = previous_full_batch.get_state() == BatchState::Inserted;
        let previous_batch_is_ready =
            batch_is_inserted && !previous_full_batch.bloom_filter_is_zeroed();

        if previous_batch_is_ready && current_batch_is_half_full {
            // 3.1. Mark bloom filter zeroed.
            previous_full_batch.set_bloom_filter_to_zeroed();
            let seq = previous_full_batch.sequence_number;
            let root_index = previous_full_batch.root_index;
            // 3.2. Zero out bloom filter.
            {
                let bloom_filter = self
                    .bloom_filter_stores
                    .get_mut(previous_full_batch_index)
                    .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;
                bloom_filter.iter_mut().for_each(|x| *x = 0);
            }
            // 3.3. Zero out roots if a root exists in root history
            // which allows to prove inclusion of a value
            // that was inserted into the bloom filter just zeroed out.
            {
                self.zero_out_roots(seq, root_index);
            }
        }

        Ok(())
    }

    /// Return the latest root index.
    pub fn get_root_index(&self) -> u32 {
        self.root_history.last_index() as u32
    }

    /// Return the latest root of the tree.
    pub fn get_root(&self) -> Option<[u8; 32]> {
        self.root_history.last().copied()
    }

    /// Return root from the root history by index.
    pub fn get_root_by_index(&self, index: usize) -> Option<&[u8; 32]> {
        self.root_history.get(index)
    }

    /// Return a reference to the metadata of the tree.
    pub fn get_metadata(&self) -> &BatchedMerkleTreeMetadata {
        &self.metadata
    }

    /// Return a mutable reference to the metadata of the tree.
    pub fn get_metadata_mut(&mut self) -> &mut BatchedMerkleTreeMetadata {
        &mut self.metadata
    }

    /// Check non-inclusion in all bloom filters
    /// which are not zeroed.
    pub fn check_input_queue_non_inclusion(
        &mut self,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        for i in 0..(self.queue_metadata.num_batches as usize) {
            Batch::check_non_inclusion(
                self.queue_metadata.batches[i].num_iters as usize,
                self.queue_metadata.batches[i].bloom_filter_capacity,
                value,
                self.bloom_filter_stores[i],
            )?;
        }
        Ok(())
    }

    /// Return whether the tree is full.
    /// For address trees it is based on the queue next index.
    /// For state trees it is based on the tree next index.
    pub fn tree_is_full(&self) -> bool {
        if self.tree_type == TreeType::BatchedState as u64 {
            self.next_index == self.capacity
        } else if self.tree_type == TreeType::BatchedAddress as u64 {
            self.queue_metadata.next_index == self.capacity
        } else {
            unreachable!("Invalid tree type.")
        }
    }

    pub fn check_tree_is_full(&self) -> Result<(), BatchedMerkleTreeError> {
        if self.tree_is_full() {
            return Err(BatchedMerkleTreeError::TreeIsFull);
        }
        Ok(())
    }

    pub fn get_associated_queue(&self) -> &Pubkey {
        &self.metadata.metadata.associated_queue
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
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
) -> usize {
    let mt_account = BatchedMerkleTreeMetadata {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: TreeType::BatchedState as u64,
        height,
        root_history_capacity,
        queue_metadata: BatchMetadata {
            num_batches: NUM_BATCHES as u64,
            batch_size,
            bloom_filter_capacity,
            zkp_batch_size,
            ..Default::default()
        },
        capacity: 2u64.pow(height),
        ..Default::default()
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
    let batch = old_account
        .queue_metadata
        .batches
        .get(batch_index as usize)
        .unwrap();
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
        .batch_metadata
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

#[cfg(test)]
mod test {
    use rand::{Rng, SeedableRng};

    use super::*;

    #[test]
    fn test_from_bytes_invalid_tree_type() {
        let mut account_data = vec![0u8; get_merkle_tree_account_size_default()];
        let account = BatchedMerkleTreeAccount::from_bytes::<6>(&mut account_data);
        assert_eq!(
            account.unwrap_err(),
            MerkleTreeMetadataError::InvalidTreeType.into()
        );
    }

    #[test]
    fn test_from_bytes_invalid_account_size() {
        let mut account_data = vec![0u8; 200];
        let account =
            BatchedMerkleTreeAccount::from_bytes::<BATCHED_STATE_TREE_TYPE>(&mut account_data);
        assert_eq!(account.unwrap_err(), ZeroCopyError::Size.into());
    }

    #[test]
    fn test_init_invalid_account_size() {
        let mut account_data = vec![0u8; 200];
        let account =
            BatchedMerkleTreeAccount::from_bytes::<BATCHED_STATE_TREE_TYPE>(&mut account_data);
        assert_eq!(account.unwrap_err(), ZeroCopyError::Size.into());
    }

    /// 1. No batch is ready -> nothing should happen.
    /// 2. Batch 0 is inserted but Batch 1 is empty -> nothing should happen.
    /// 3. Batch 0 is inserted but Batch 1 is 25% full (not the required half)
    ///     -> nothing should happen.
    /// 4. Batch 0 is inserted and Batch 1 is full
    ///     -> should zero out all existing roots except the last one.
    /// 5. Batch 1 is inserted and Batch 0 is empty
    ///     -> nothing should happen.
    /// 6. Batch 1 is inserted and Batch 0 is 25% full (not the required half)
    ///     -> nothing should happen.
    /// 7. Batch 1 is inserted and Batch 0 is half full and no overlapping roots exist
    ///     -> bloom filter is zeroed, roots are untouched.
    /// 8. Batch 1 is already zeroed -> nothing should happen
    /// 9. Batch 1 is inserted and Batch 0 is full and overlapping roots exist
    #[test]
    fn test_zero_out() {
        let mut account_data = vec![0u8; 3160];
        let batch_size = 4;
        let zkp_batch_size = 1;
        let num_zkp_updates = batch_size / zkp_batch_size;
        let root_history_len = 10;
        let num_iter = 1;
        let bloom_filter_capacity = 8000;
        let pubkey = Pubkey::new_unique();
        BatchedMerkleTreeAccount::init(
            &mut account_data,
            &pubkey,
            MerkleTreeMetadata::default(),
            root_history_len,
            batch_size,
            zkp_batch_size,
            10,
            num_iter,
            bloom_filter_capacity,
            TreeType::BatchedAddress,
        )
        .unwrap();
        let rng = &mut rand::rngs::StdRng::from_seed([0u8; 32]);
        let mut latest_root_0 = [0u8; 32];
        let mut latest_root_1 = [0u8; 32];

        // 1. No batch is ready
        //   -> nothing should happen.
        {
            let mut account = insert_rnd_addresses(&mut account_data, batch_size, rng).unwrap();

            assert_eq!(
                account.queue_metadata.batches[0].get_state(),
                BatchState::Full
            );
            // simulate batch insertion
            for _ in 0..num_zkp_updates {
                let rnd_root = rng.gen();
                account.root_history.push(rnd_root);
                latest_root_0 = rnd_root;
                account.metadata.sequence_number += 1;
                let root_index = account.get_root_index();
                println!("root_index: {}", root_index);
                let sequence_number = account.sequence_number;

                let state = account.queue_metadata.batches[0]
                    .mark_as_inserted_in_merkle_tree(sequence_number, root_index, root_history_len)
                    .unwrap();
                account
                    .queue_metadata
                    .increment_next_full_batch_index_if_inserted(state);
            }
            assert_eq!(
                account.queue_metadata.batches[0].get_state(),
                BatchState::Inserted
            );
            assert_eq!(account.queue_metadata.next_full_batch_index, 1);
            let index = account.queue_metadata.batches[0].root_index;
            assert_eq!(account.root_history[index as usize], latest_root_0);
        }
        // 2. Batch 0 is inserted but Batch 1 is not half full
        //    -> nothing should happen.
        {
            let mut account_data = account_data.clone();
            let account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            account.zero_out_previous_batch_bloom_filter().unwrap();
            assert_eq!(account_data, account_data_ref);
        }

        // 3. Batch 0 is inserted but Batch 1 is not half full
        //    -> nothing should happen.
        {
            // Make Batch 1 almost half full
            {
                insert_rnd_addresses(&mut account_data, 1, rng).unwrap();
            }
            let mut account_data = account_data.clone();
            let account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            account.zero_out_previous_batch_bloom_filter().unwrap();
            assert_eq!(account_data, account_data_ref);
        }
        // 4. Batch 0 is inserted and Batch 1 is half full
        //    -> should zero out all existing roots except the last one.
        {
            // Make Batch 1 half full
            {
                insert_rnd_addresses(&mut account_data, 1, rng).unwrap();
            }
            let mut account_data = account_data.clone();
            // let account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            println!(
                "currently inserted elements: {:?}",
                account.queue_metadata.batches[1].get_num_inserted_elements()
            );
            let previous_roots = account.root_history.to_vec();
            account.zero_out_previous_batch_bloom_filter().unwrap();
            let current_roots = account.root_history.to_vec();
            println!("previous_roots: {:?}", previous_roots);
            assert_ne!(previous_roots, current_roots);
            let root_index = account.queue_metadata.batches[0].root_index;
            assert_eq!(
                account.root_history[root_index as usize],
                previous_roots[root_index as usize]
            );
            assert_eq!(
                account.queue_metadata.batches[0].get_state(),
                BatchState::Inserted
            );
            assert_eq!(account.queue_metadata.batches[0].sequence_number, 14);
            assert_eq!(account.queue_metadata.batches[0].root_index, 4);
            assert!(account.queue_metadata.batches[0].bloom_filter_is_zeroed());
            assert_eq!(
                account.queue_metadata.batches[0].get_num_inserted_zkps(),
                num_zkp_updates
            );

            for i in 0..root_history_len as usize {
                if i == root_index as usize {
                    assert_eq!(account.root_history[i], latest_root_0);
                } else {
                    assert_eq!(account.root_history[i], [0u8; 32]);
                }
            }
        }
        // Make Batch 1 full and insert
        {
            let mut account = insert_rnd_addresses(&mut account_data, 2, rng).unwrap();

            assert_eq!(
                account.queue_metadata.batches[1].get_state(),
                BatchState::Full
            );
            // simulate batch insertion
            for _ in 0..num_zkp_updates {
                let rnd_root = rng.gen();
                account.root_history.push(rnd_root);
                latest_root_1 = rnd_root;
                account.metadata.sequence_number += 1;
                let root_index = account.get_root_index();
                let sequence_number = account.sequence_number;

                let state = account.queue_metadata.batches[1]
                    .mark_as_inserted_in_merkle_tree(sequence_number, root_index, root_history_len)
                    .unwrap();
                account
                    .queue_metadata
                    .increment_next_full_batch_index_if_inserted(state);
                account.zero_out_previous_batch_bloom_filter().unwrap();
            }
            assert_eq!(
                account.queue_metadata.batches[1].get_state(),
                BatchState::Inserted
            );
            assert_eq!(account.queue_metadata.next_full_batch_index, 0);
            let index = account.queue_metadata.batches[1].root_index;
            assert_eq!(account.root_history[index as usize], latest_root_1);
        }
        println!("pre 4");
        // 5. Batch 1 is inserted and Batch 0 is empty
        // -> nothing should happen
        {
            let mut account_data = account_data.clone();
            let account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            for batch in account.queue_metadata.batches.iter_mut() {
                println!("batch state: {:?}", batch);
            }
            assert_eq!(
                account.queue_metadata.batches[0].get_num_inserted_elements(),
                0
            );
            account.zero_out_previous_batch_bloom_filter().unwrap();
            assert_eq!(account_data, account_data_ref);
        }
        println!("pre 5");
        let mut account = BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
        for batch in account.queue_metadata.batches.iter_mut() {
            println!("batch state: {:?}", batch);
        }
        // 6. Batch 1 is inserted and Batch 0 is almost half full
        // -> nothing should happen
        {
            // Make Batch 0 quater full
            {
                insert_rnd_addresses(&mut account_data, 1, rng).unwrap();
            }
            let mut account_data = account_data.clone();
            let account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            account.zero_out_previous_batch_bloom_filter().unwrap();
            assert_eq!(account_data, account_data_ref);
        }
        println!("pre 6");
        // 7. Batch 1 is inserted and Batch 0 is half full but no overlapping roots exist
        // -> bloom filter zeroed, roots untouched
        {
            // Make Batch 0 half full
            {
                insert_rnd_addresses(&mut account_data, 1, rng).unwrap();
            }
            // simulate 10 other batch insertions from an output queue
            {
                let mut account =
                    BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
                for _ in 0..10 {
                    let rnd_root = rng.gen();
                    account.root_history.push(rnd_root);
                    account.metadata.sequence_number += 1;
                }
            }
            let mut account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            account.zero_out_previous_batch_bloom_filter().unwrap();
            let mut account_ref =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data_ref).unwrap();
            account_ref.bloom_filter_stores[1]
                .iter_mut()
                .for_each(|x| *x = 0);
            account_ref.queue_metadata.batches[1].set_bloom_filter_to_zeroed();
            assert_eq!(account.get_metadata(), account_ref.get_metadata());
            assert_eq!(account, account_ref);
        }
        // 8. Batch 1 is already zeroed -> nothing should happen
        {
            let mut account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            account.zero_out_previous_batch_bloom_filter().unwrap();
            let account_ref =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data_ref).unwrap();
            assert_eq!(account, account_ref);
        }
        // 9. Batch 0 is inserted and Batch 1 is full
        //    -> should zero out Batch 0s bloom filter and overlapping roots
        {
            // Make Batch 0 and 1 full
            {
                insert_rnd_addresses(&mut account_data, batch_size + 2, rng).unwrap();
            }
            // simulate batch 0 insertion
            {
                let mut account =
                    BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
                for _ in 0..num_zkp_updates {
                    let rnd_root = rng.gen();
                    account.root_history.push(rnd_root);
                    account.metadata.sequence_number += 1;
                    let root_index = account.get_root_index();
                    let sequence_number = account.sequence_number;

                    let state = account.queue_metadata.batches[0]
                        .mark_as_inserted_in_merkle_tree(
                            sequence_number,
                            root_index,
                            root_history_len,
                        )
                        .unwrap();
                    account
                        .queue_metadata
                        .increment_next_full_batch_index_if_inserted(state);
                }
            }
            println!("pre 9");
            let mut account_data_ref = account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            assert_eq!(
                account.queue_metadata.batches[0].get_state(),
                BatchState::Inserted
            );
            assert_eq!(
                account.queue_metadata.batches[1].get_state(),
                BatchState::Full
            );
            account.zero_out_previous_batch_bloom_filter().unwrap();
            let mut account_ref =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data_ref).unwrap();
            let root_index = account.queue_metadata.batches[0].root_index;
            account_ref.bloom_filter_stores[0]
                .iter_mut()
                .for_each(|x| *x = 0);
            account_ref.queue_metadata.batches[0].set_bloom_filter_to_zeroed();
            assert_eq!(account.get_metadata(), account_ref.get_metadata());
            for i in 0..root_history_len as usize {
                if i == root_index as usize {
                    continue;
                } else {
                    account_ref.root_history[i] = [0u8; 32];
                }
            }
            assert_eq!(account, account_ref);
        }

        // simulate batch 1 insertion
        {
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            for _ in 0..num_zkp_updates {
                let rnd_root = rng.gen();
                account.root_history.push(rnd_root);
                account.metadata.sequence_number += 1;
                let root_index = account.get_root_index();
                let sequence_number = account.sequence_number;

                let state = account.queue_metadata.batches[1]
                    .mark_as_inserted_in_merkle_tree(sequence_number, root_index, root_history_len)
                    .unwrap();
                account
                    .queue_metadata
                    .increment_next_full_batch_index_if_inserted(state);
            }
            assert_eq!(
                account.queue_metadata.batches[0].get_state(),
                BatchState::Inserted
            );
            assert_eq!(
                account.queue_metadata.batches[1].get_state(),
                BatchState::Inserted
            );
            assert_eq!(
                account.bloom_filter_stores[0].to_vec(),
                vec![0u8; account.bloom_filter_stores[0].len()]
            );
            assert_ne!(
                account.bloom_filter_stores[1].to_vec(),
                vec![0u8; account.bloom_filter_stores[0].len()]
            );
        }
        println!("pre 9.1");

        // Zero out batch 1 with user tx
        {
            // fill batch 0
            {
                insert_rnd_addresses(&mut account_data, batch_size, rng).unwrap();
            }
            println!("pre 9.2");
            // the insertion into batch 1 fails since the bloom filter of batch 0 is not zeroed out.
            let mut account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut account_data).unwrap();
            let address = rng.gen();
            let result = account.insert_address_into_current_batch(&address);
            assert_eq!(
                result.unwrap_err(),
                BatchedMerkleTreeError::BloomFilterNotZeroed
            );
        }
    }

    fn insert_rnd_addresses<'a>(
        account_data: &'a mut [u8],
        batch_size: u64,
        rng: &mut rand::prelude::StdRng,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        let mut account = BatchedMerkleTreeAccount::address_from_bytes(account_data).unwrap();
        for i in 0..batch_size {
            println!("inserting address: {}", i);
            let address = rng.gen();
            account.insert_address_into_current_batch(&address)?;
        }
        Ok(account)
    }

    #[test]
    fn test_tree_is_full() {
        let mut account_data = vec![0u8; 15704];
        let batch_size = 200;
        let zkp_batch_size = 1;
        let root_history_len = 10;
        let num_iter = 1;
        let bloom_filter_capacity = 8000;
        let height = 4;
        let tree_capacity = 2u64.pow(height);
        let account = BatchedMerkleTreeAccount::init(
            &mut account_data,
            &Pubkey::new_unique(),
            MerkleTreeMetadata::default(),
            root_history_len,
            batch_size,
            zkp_batch_size,
            height,
            num_iter,
            bloom_filter_capacity,
            TreeType::BatchedAddress,
        )
        .unwrap();
        // 1. empty tree is not full
        assert!(!account.tree_is_full());

        let rng = &mut rand::rngs::StdRng::from_seed([0u8; 32]);
        let account = insert_rnd_addresses(&mut account_data, tree_capacity - 1, rng).unwrap();
        // 2. tree at capacity - 1 is not full
        assert!(!account.tree_is_full());
        // 3. tree at capacity is full
        let account = insert_rnd_addresses(&mut account_data, 1, rng).unwrap();
        assert!(account.tree_is_full());
        assert_eq!(
            account.check_tree_is_full().unwrap_err(),
            BatchedMerkleTreeError::TreeIsFull
        );
    }

    #[test]
    fn test_check_non_inclusion() {
        let mut account_data = vec![0u8; 3224];
        let batch_size = 5;
        let zkp_batch_size = 1;
        let root_history_len = 10;
        let num_iter = 1;
        let bloom_filter_capacity = 8000;
        let height = 4;
        let mut account = BatchedMerkleTreeAccount::init(
            &mut account_data,
            &Pubkey::new_unique(),
            MerkleTreeMetadata::default(),
            root_history_len,
            batch_size,
            zkp_batch_size,
            height,
            num_iter,
            bloom_filter_capacity,
            TreeType::BatchedAddress,
        )
        .unwrap();
        // 1. empty tree is not full
        assert!(!account.tree_is_full());

        let mut inserted_elements = vec![];
        let rng = &mut rand::rngs::StdRng::from_seed([0u8; 32]);
        // fill batch 0
        for _ in 0..batch_size {
            let address = rng.gen();
            inserted_elements.push(address);
            account.insert_address_into_current_batch(&address).unwrap();
        }
        // 1. Non inclusion of inserted elements should fail
        for address in inserted_elements.iter() {
            assert_eq!(
                account
                    .check_input_queue_non_inclusion(address)
                    .unwrap_err(),
                BatchedMerkleTreeError::NonInclusionCheckFailed
            );
        }
        // 2. Non inclusion of random address should pass
        for _ in 0..100 {
            let address = rng.gen();
            account.check_input_queue_non_inclusion(&address).unwrap();
        }
        // fill batch 1
        for _ in 0..batch_size {
            let address = rng.gen();
            inserted_elements.push(address);
            account.insert_address_into_current_batch(&address).unwrap();
        }
        // 3. Non inclusion of inserted elements should fail
        for address in inserted_elements.iter() {
            assert_eq!(
                account
                    .check_input_queue_non_inclusion(address)
                    .unwrap_err(),
                BatchedMerkleTreeError::NonInclusionCheckFailed
            );
        }
        // clear bloom filter 0
        account.bloom_filter_stores[0]
            .iter_mut()
            .for_each(|x| *x = 0);
        // 4. Non inclusion of batch 0 inserted elements should pass
        for address in inserted_elements.iter().take(batch_size as usize) {
            account.check_input_queue_non_inclusion(address).unwrap();
        }
        // 5. Non inclusion of batch 1 inserted elements should fail
        for address in inserted_elements[batch_size as usize..].iter() {
            assert_eq!(
                account
                    .check_input_queue_non_inclusion(address)
                    .unwrap_err(),
                BatchedMerkleTreeError::NonInclusionCheckFailed
            );
        }
        // clear bloom filter 1
        account.bloom_filter_stores[1]
            .iter_mut()
            .for_each(|x| *x = 0);
        // 6. Non inclusion of batch 0 inserted elements should pass
        for address in inserted_elements.iter() {
            account.check_input_queue_non_inclusion(address).unwrap();
        }
    }
}
