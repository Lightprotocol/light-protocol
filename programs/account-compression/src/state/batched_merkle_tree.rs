use crate::{
    batch::BatchState,
    batched_queue::ZeroCopyBatchedQueueAccount,
    bytes_to_struct_checked,
    errors::AccountCompressionErrorCode,
    utils::{
        check_signer_is_registered_or_authority::GroupAccess,
        constants::{ADDRESS_TREE_INIT_ROOT_26, TEST_DEFAULT_BATCH_SIZE},
    },
    InitAddressTreeAccountsInstructionData, InitStateTreeAccountsInstructionData,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_bounded_vec::{BoundedVec, CyclicBoundedVec, CyclicBoundedVecMetadata};
use light_hasher::{Hasher, Poseidon};
use light_utils::fee::compute_rollover_fee;
use light_verifier::{
    verify_batch_address_update, verify_batch_append_with_proofs, verify_batch_update,
    CompressedProof,
};
use std::mem::{size_of, ManuallyDrop};

use super::{
    batch::Batch,
    batched_queue::{
        init_queue, input_queue_bytes, insert_into_current_batch, queue_account_size, BatchedQueue,
    },
    AccessMetadata, MerkleTreeMetadata, QueueType, RolloverMetadata,
};

#[derive(Debug, PartialEq, Default)]
#[aligned_sized(anchor)]
#[account(zero_copy)]
pub struct BatchedMerkleTreeMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,
    // Queue associated with this Merkle tree.
    pub associated_output_queue: Pubkey,
    // Next Merkle tree to be used after rollover.
    pub next_merkle_tree: Pubkey,
    pub tree_type: u64,
}

impl GroupAccess for ZeroCopyBatchedMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.get_account().metadata.access_metadata.owner
    }

    fn get_program_owner(&self) -> &Pubkey {
        &self.get_account().metadata.access_metadata.program_owner
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchAppendEvent {
    pub id: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    pub batch_size: u64,
    pub old_next_index: u64,
    pub new_next_index: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
}
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchNullifyEvent {
    pub id: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
    pub batch_size: u64,
}

#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TreeType {
    State = 1,
    Address = 2,
    BatchedState = 3,
    BatchedAddress = 4,
}

#[derive(Debug, PartialEq, Default)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct BatchedMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
    pub sequence_number: u64,
    pub tree_type: u64,
    pub next_index: u64,
    pub height: u32,
    pub root_history_capacity: u32,
    pub queue: BatchedQueue,
}

impl BatchedMerkleTreeAccount {
    pub fn size(&self) -> Result<usize> {
        let account_size = Self::LEN;
        let root_history_size = size_of::<CyclicBoundedVecMetadata>()
            + (size_of::<[u8; 32]>() * self.root_history_capacity as usize);
        let size = account_size
            + root_history_size
            + queue_account_size(&self.queue, QueueType::Input as u64)?;
        Ok(size)
    }

    pub fn get_state_tree_default(
        owner: Pubkey,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        rollover_threshold: Option<u64>,
        index: u64,
        network_fee: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        bloom_filter_capacity: u64,
        root_history_capacity: u32,
        associated_queue: Pubkey,
        height: u32,
        num_batches: u64,
    ) -> Self {
        Self::get_tree_default(
            TreeType::BatchedState,
            owner,
            program_owner,
            forester,
            rollover_threshold,
            index,
            network_fee,
            batch_size,
            zkp_batch_size,
            bloom_filter_capacity,
            root_history_capacity,
            associated_queue,
            height,
            num_batches,
            0,
        )
    }
    pub fn get_address_tree_default(
        owner: Pubkey,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        rollover_threshold: Option<u64>,
        index: u64,
        network_fee: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        bloom_filter_capacity: u64,
        root_history_capacity: u32,
        height: u32,
        num_batches: u64,
        rent: u64,
    ) -> Self {
        let rollover_fee = match rollover_threshold {
            Some(rollover_threshold) => {
                compute_rollover_fee(rollover_threshold, height, rent).unwrap()
            }
            None => 0,
        };
        let mut tree = Self::get_tree_default(
            TreeType::BatchedAddress,
            owner,
            program_owner,
            forester,
            rollover_threshold,
            index,
            network_fee,
            batch_size,
            zkp_batch_size,
            bloom_filter_capacity,
            root_history_capacity,
            Pubkey::default(),
            height,
            num_batches,
            rollover_fee,
        );
        // inited address tree contains two elements.
        tree.next_index = 2;
        tree
    }
    pub fn get_tree_default(
        tree_type: TreeType,
        owner: Pubkey,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        rollover_threshold: Option<u64>,
        index: u64,
        network_fee: u64,
        batch_size: u64,
        zkp_batch_size: u64,
        bloom_filter_capacity: u64,
        root_history_capacity: u32,
        associated_queue: Pubkey,
        height: u32,
        num_batches: u64,
        rollover_fee: u64,
    ) -> Self {
        Self {
            metadata: MerkleTreeMetadata {
                next_merkle_tree: Pubkey::default(),
                access_metadata: AccessMetadata::new(owner, program_owner, forester),
                rollover_metadata: RolloverMetadata::new(
                    index,
                    rollover_fee,
                    rollover_threshold,
                    network_fee,
                    None,
                    None,
                ),
                associated_queue,
            },
            sequence_number: 0,
            tree_type: tree_type as u64,
            next_index: 0,
            height,
            root_history_capacity,
            queue: BatchedQueue::get_input_queue_default(
                batch_size,
                bloom_filter_capacity,
                zkp_batch_size,
                num_batches,
            ),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ZeroCopyBatchedMerkleTreeAccount {
    account: *mut BatchedMerkleTreeAccount,
    pub root_history: ManuallyDrop<CyclicBoundedVec<[u8; 32]>>,
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloom_filter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
    pub hashchain_store: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
}

/// Get batch from account.
/// Hash all public inputs into one poseidon hash.
/// Public inputs:
/// 1. old root (get from account by index)
/// 2. new root (send to chain and )
/// 3. start index (get from batch)
/// 4. end index (get from batch start index plus batch size)
#[derive(Debug, PartialEq, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub struct InstructionDataBatchNullifyInputs {
    pub public_inputs: BatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

#[derive(Debug, PartialEq, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub struct BatchProofInputsIx {
    pub new_root: [u8; 32],
    pub old_root_index: u16,
}

#[derive(Debug, PartialEq, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub struct InstructionDataBatchAppendInputs {
    pub public_inputs: AppendBatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct AppendBatchProofInputsIx {
    pub new_root: [u8; 32],
}

impl ZeroCopyBatchedMerkleTreeAccount {
    pub fn get_account(&self) -> &BatchedMerkleTreeAccount {
        unsafe { self.account.as_ref() }.unwrap()
    }
    pub fn get_account_mut(&mut self) -> &mut BatchedMerkleTreeAccount {
        unsafe { self.account.as_mut() }.unwrap()
    }

    pub fn state_tree_from_account_info_mut(
        account_info: &AccountInfo<'_>,
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        if *account_info.owner != crate::ID {
            return err!(ErrorCode::AccountOwnedByWrongProgram);
        }
        if !account_info.is_writable {
            return err!(ErrorCode::AccountNotMutable);
        }
        let account_data = &mut account_info.try_borrow_mut_data()?;
        let merkle_tree =
            ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(account_data)?;
        Ok(merkle_tree)
    }

    // TODO: add failing test
    pub fn state_tree_from_bytes_mut(
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        let merkle_tree = ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account_data)?;
        if merkle_tree.get_account().tree_type != TreeType::BatchedState as u64 {
            return err!(AccountCompressionErrorCode::InvalidTreeType);
        }
        Ok(merkle_tree)
    }

    pub fn address_tree_from_account_info_mut(
        account_info: &AccountInfo<'_>,
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        if *account_info.owner != crate::ID {
            return err!(ErrorCode::AccountOwnedByWrongProgram);
        }
        if !account_info.is_writable {
            return err!(ErrorCode::AccountNotMutable);
        }
        let account_data = &mut account_info.try_borrow_mut_data()?;

        let merkle_tree =
            ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(account_data)?;
        Ok(merkle_tree)
    }

    // TODO: add failing test
    pub fn address_tree_from_bytes_mut(
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        let merkle_tree = ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account_data)?;
        if merkle_tree.get_account().tree_type != TreeType::BatchedAddress as u64 {
            return err!(AccountCompressionErrorCode::InvalidTreeType);
        }
        Ok(merkle_tree)
    }

    fn from_bytes_mut(account_data: &mut [u8]) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        unsafe {
            let account = bytes_to_struct_checked::<BatchedMerkleTreeAccount, false>(account_data)?;
            if account_data.len() != (*account).size()? {
                return err!(AccountCompressionErrorCode::SizeMismatch);
            }
            let mut start_offset = BatchedMerkleTreeAccount::LEN;
            let root_history = CyclicBoundedVec::deserialize(account_data, &mut start_offset)
                .map_err(ProgramError::from)?;
            let (batches, value_vecs, bloom_filter_stores, hashchain_store) = input_queue_bytes(
                &(*account).queue,
                account_data,
                QueueType::Input as u64,
                &mut start_offset,
            )?;

            Ok(ZeroCopyBatchedMerkleTreeAccount {
                account,
                root_history,
                batches,
                value_vecs,
                bloom_filter_stores,
                hashchain_store,
            })
        }
    }

    pub fn init(
        metadata: MerkleTreeMetadata,
        root_history_capacity: u32,
        num_batches_input_queue: u64,
        input_queue_batch_size: u64,
        input_queue_zkp_batch_size: u64,
        height: u32,
        account_data: &mut [u8],
        num_iters: u64,
        bloom_filter_capacity: u64,
        tree_type: TreeType,
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        unsafe {
            let account = bytes_to_struct_checked::<BatchedMerkleTreeAccount, true>(account_data)?;
            (*account).metadata = metadata;
            (*account).root_history_capacity = root_history_capacity;
            (*account).height = height;
            (*account).tree_type = tree_type as u64;
            (*account).queue.init(
                num_batches_input_queue,
                input_queue_batch_size,
                input_queue_zkp_batch_size,
            )?;
            (*account).queue.bloom_filter_capacity = bloom_filter_capacity;
            if account_data.len() != (*account).size()? {
                msg!("merkle_tree_account: {:?}", (*account));
                msg!("account_data.len(): {}", account_data.len());
                msg!("account.size(): {}", (*account).size()?);
                return err!(AccountCompressionErrorCode::SizeMismatch);
            }
            let mut start_offset = BatchedMerkleTreeAccount::LEN;

            let mut root_history = CyclicBoundedVec::init(
                (*account).root_history_capacity as usize,
                account_data,
                &mut start_offset,
                false,
            )
            .map_err(ProgramError::from)?;
            if tree_type == TreeType::BatchedState {
                root_history.push(light_hasher::Poseidon::zero_bytes()[height as usize]);
            } else if tree_type == TreeType::BatchedAddress {
                // Initialized indexed Merkle tree root
                root_history.push(ADDRESS_TREE_INIT_ROOT_26);
                (*account).next_index = 2;
            }
            let (batches, value_vecs, bloom_filter_stores, hashchain_store) = init_queue(
                &(*account).queue,
                QueueType::Input as u64,
                account_data,
                num_iters,
                bloom_filter_capacity,
                &mut start_offset,
                (*account).next_index,
            )?;
            Ok(ZeroCopyBatchedMerkleTreeAccount {
                account,
                root_history,
                batches,
                value_vecs,
                bloom_filter_stores,
                hashchain_store,
            })
        }
    }

    // Note: when proving inclusion by index in
    // value array we need to insert the value into a bloom_filter once it is
    // inserted into the tree. Check this with get_num_inserted_zkps
    pub fn update_output_queue(
        &mut self,
        queue_account_data: &mut [u8],
        instruction_data: InstructionDataBatchAppendInputs,
        id: [u8; 32],
    ) -> Result<BatchAppendEvent> {
        let mut queue_account =
            ZeroCopyBatchedQueueAccount::from_bytes_mut(queue_account_data).unwrap();

        let batch_index = queue_account.get_account().queue.next_full_batch_index;
        let circuit_batch_size = queue_account.get_account().queue.zkp_batch_size;
        let batches = &mut queue_account.batches;
        let full_batch = batches.get_mut(batch_index as usize).unwrap();

        let new_root = instruction_data.public_inputs.new_root;
        let num_zkps = full_batch.get_first_ready_zkp_batch()?;

        let leaves_hashchain = queue_account
            .hashchain_store
            .get(batch_index as usize)
            .unwrap()
            .get(num_zkps as usize)
            .unwrap();
        let old_root = self.root_history.last().unwrap();
        let start_index = self.get_account().next_index;
        let mut start_index_bytes = [0u8; 32];
        start_index_bytes[24..].copy_from_slice(&start_index.to_be_bytes());
        let public_input_hash =
            create_hash_chain([*old_root, new_root, *leaves_hashchain, start_index_bytes])?;

        self.update::<5>(
            circuit_batch_size as usize,
            instruction_data.compressed_proof,
            public_input_hash,
        )?;
        let account = self.get_account_mut();
        account.next_index += circuit_batch_size;
        let root_history_capacity = account.root_history_capacity;
        let sequence_number = account.sequence_number;
        self.root_history.push(new_root);
        let root_index = self.root_history.last_index() as u32;
        full_batch.mark_as_inserted_in_merkle_tree(
            sequence_number,
            root_index,
            root_history_capacity,
        )?;
        if full_batch.get_state() == BatchState::Inserted {
            queue_account.get_account_mut().queue.next_full_batch_index += 1;
            queue_account.get_account_mut().queue.next_full_batch_index %=
                queue_account.get_account_mut().queue.num_batches;
        }
        Ok(BatchAppendEvent {
            id,
            batch_index,
            batch_size: circuit_batch_size,
            zkp_batch_index: num_zkps,
            old_next_index: start_index,
            new_next_index: start_index + circuit_batch_size,
            new_root,
            root_index,
            sequence_number: self.get_account().sequence_number,
        })
    }

    pub fn update_input_queue(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent> {
        self._update_input_queue::<3>(instruction_data, id)
    }

    pub fn update_address_queue(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent> {
        self._update_input_queue::<4>(instruction_data, id)
    }

    fn _update_input_queue<const QUEUE_TYPE: u64>(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent> {
        let batch_index = self.get_account().queue.next_full_batch_index;

        let full_batch = self.batches.get(batch_index as usize).unwrap();

        let num_zkps = full_batch.get_first_ready_zkp_batch()?;

        let leaves_hashchain = self
            .hashchain_store
            .get(batch_index as usize)
            .unwrap()
            .get(num_zkps as usize)
            .unwrap();
        let old_root = self
            .root_history
            .get(instruction_data.public_inputs.old_root_index as usize)
            .unwrap();
        let new_root = instruction_data.public_inputs.new_root;

        let public_input_hash = if QUEUE_TYPE == QueueType::Input as u64 {
            create_hash_chain([*old_root, new_root, *leaves_hashchain])?
        } else if QUEUE_TYPE == QueueType::Address as u64 {
            let mut next_index_bytes = [0u8; 32];
            next_index_bytes[24..]
                .copy_from_slice(self.get_account().next_index.to_be_bytes().as_slice());
            create_hash_chain([*old_root, new_root, *leaves_hashchain, next_index_bytes])?
        } else {
            return err!(AccountCompressionErrorCode::InvalidQueueType);
        };
        let circuit_batch_size = self.get_account().queue.zkp_batch_size;
        self.update::<QUEUE_TYPE>(
            circuit_batch_size as usize,
            instruction_data.compressed_proof,
            public_input_hash,
        )?;
        self.root_history.push(new_root);

        let root_history_capacity = self.get_account().root_history_capacity;
        let sequence_number = self.get_account().sequence_number;
        let full_batch = self.batches.get_mut(batch_index as usize).unwrap();
        full_batch.mark_as_inserted_in_merkle_tree(
            sequence_number,
            self.root_history.last_index() as u32,
            root_history_capacity,
        )?;
        if full_batch.get_state() == BatchState::Inserted {
            let account = self.get_account_mut();
            account.queue.next_full_batch_index += 1;
            account.queue.next_full_batch_index %= account.queue.num_batches;
        }
        if QUEUE_TYPE == QueueType::Address as u64 {
            self.get_account_mut().next_index += circuit_batch_size;
        }

        self.wipe_previous_batch_bloom_filter()?;

        Ok(BatchNullifyEvent {
            id,
            batch_index,
            batch_size: circuit_batch_size,
            zkp_batch_index: num_zkps,
            new_root,
            root_index: self.root_history.last_index() as u32,
            sequence_number: self.get_account().sequence_number,
        })
    }

    fn update<const QUEUE_TYPE: u64>(
        &mut self,
        batch_size: usize,
        proof: CompressedProof,
        public_input_hash: [u8; 32],
    ) -> Result<()> {
        if QUEUE_TYPE == QueueType::Output as u64 {
            verify_batch_append_with_proofs(batch_size, public_input_hash, &proof)
                .map_err(ProgramError::from)?;
        } else if QUEUE_TYPE == QueueType::Input as u64 {
            verify_batch_update(batch_size, public_input_hash, &proof)
                .map_err(ProgramError::from)?;
        } else if QUEUE_TYPE == QueueType::Address as u64 {
            verify_batch_address_update(batch_size, public_input_hash, &proof)
                .map_err(ProgramError::from)?;
        } else {
            return err!(AccountCompressionErrorCode::InvalidQueueType);
        }
        self.get_account_mut().sequence_number += 1;
        Ok(())
    }

    /// State nullification:
    /// - value is committed to bloom_filter for non-inclusion proof
    /// - nullifier is Hash(value, tx_hash), committed to leaves hashchain
    /// - tx_hash is hash of all inputs and outputs
    /// -> we can access the history of how commitments are spent in zkps for example fraud proofs
    pub fn insert_nullifier_into_current_batch(
        &mut self,
        compressed_account_hash: &[u8; 32],
        leaf_index: u64,
        tx_hash: &[u8; 32],
    ) -> Result<()> {
        if self.get_account().tree_type != TreeType::BatchedState as u64 {
            return err!(AccountCompressionErrorCode::InvalidTreeType);
        }
        let leaf_index_bytes = leaf_index.to_be_bytes();
        let nullifier = Poseidon::hashv(&[compressed_account_hash, &leaf_index_bytes, tx_hash])
            .map_err(ProgramError::from)?;
        self.insert_into_current_batch(compressed_account_hash, &nullifier)
    }

    pub fn insert_address_into_current_batch(&mut self, address: &[u8; 32]) -> Result<()> {
        if self.get_account().tree_type != TreeType::BatchedAddress as u64 {
            return err!(AccountCompressionErrorCode::InvalidTreeType);
        }
        self.insert_into_current_batch(address, address)
    }

    fn insert_into_current_batch(
        &mut self,
        bloom_filter_value: &[u8; 32],
        leaves_hash_value: &[u8; 32],
    ) -> Result<()> {
        unsafe {
            let (root_index, sequence_number) = insert_into_current_batch(
                QueueType::Input as u64,
                &mut (*self.account).queue,
                &mut self.batches,
                &mut self.value_vecs,
                &mut self.bloom_filter_stores,
                &mut self.hashchain_store,
                bloom_filter_value,
                Some(leaves_hash_value),
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
                // inclusion of a value which was in the batch that was just wiped.
                self.zero_out_roots(sequence_number, root_index);
            }
        }
        Ok(())
    }

    fn zero_out_roots(&mut self, sequence_number: u64, root_index: Option<u32>) {
        if sequence_number > self.get_account().sequence_number {
            println!("zeroing out roots");
            // advance root history array current index from latest root
            // to root_index and overwrite all roots with zeros
            if let Some(root_index) = root_index {
                let root_index = root_index as usize;
                let start = self.root_history.last_index();
                let end = self.root_history.len() + root_index;
                for index in start + 1..end {
                    let index = index % self.root_history.len();
                    if index == root_index {
                        break;
                    }
                    let root = self.root_history.get_mut(index).unwrap();
                    *root = [0u8; 32];
                }
            }
        }
    }

    /// Wipe bloom filter after a batch has been inserted and 50% of the
    /// subsequent batch been processed.
    /// 1. Previous batch must be inserted and bloom filter must not be wiped.
    /// 2. Current batch must be 50% full
    /// 3. if yes
    /// 3.1 zero out bloom filter
    /// 3.2 mark bloom filter as wiped
    /// 3.3 zero out roots if needed
    pub fn wipe_previous_batch_bloom_filter(&mut self) -> Result<()> {
        let current_batch = self.get_account().queue.currently_processing_batch_index;
        let batch_size = self.get_account().queue.batch_size;
        let previous_full_batch_index = self
            .get_account()
            .queue
            .next_full_batch_index
            .saturating_sub(1) as usize;
        let num_inserted_elements = self
            .batches
            .get(current_batch as usize)
            .unwrap()
            .get_num_inserted_elements();
        let previous_full_batch = self.batches.get_mut(previous_full_batch_index).unwrap();
        println!(
            "wipe_previous_batch_bloom_filter: current_batch: {}, previous_full_batch_index: {}, num_inserted_elements: {}",
            current_batch, previous_full_batch_index, num_inserted_elements
        );
        if previous_full_batch.get_state() == BatchState::Inserted
            && batch_size / 2 > num_inserted_elements
            && !previous_full_batch.bloom_filter_is_wiped
        {
            println!("wiping bloom filter index {}", previous_full_batch_index);
            let bloom_filter = self
                .bloom_filter_stores
                .get_mut(previous_full_batch_index)
                .unwrap();
            bloom_filter.as_mut_slice().iter_mut().for_each(|x| *x = 0);
            previous_full_batch.bloom_filter_is_wiped = true;
            let seq = previous_full_batch.sequence_number;
            let root_index = previous_full_batch.root_index;
            self.zero_out_roots(seq, Some(root_index));
        } else {
            println!("not wiping bloom filter");
        }

        Ok(())
    }

    pub fn get_root_index(&self) -> u32 {
        self.root_history.last_index() as u32
    }
    pub fn get_root(&self) -> Option<[u8; 32]> {
        self.root_history.last().copied()
    }
}

pub fn create_hash_chain<const T: usize>(inputs: [[u8; 32]; T]) -> Result<[u8; 32]> {
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input]).map_err(ProgramError::from)?;
    }
    Ok(hash_chain)
}

pub fn create_hash_chain_from_vec(inputs: Vec<[u8; 32]>) -> Result<[u8; 32]> {
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input]).map_err(ProgramError::from)?;
    }
    Ok(hash_chain)
}

pub fn create_hash_chain_from_slice(inputs: &[[u8; 32]]) -> Result<[u8; 32]> {
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input]).map_err(ProgramError::from)?;
    }
    Ok(hash_chain)
}

pub fn get_merkle_tree_account_size_default() -> usize {
    let mt_account = BatchedMerkleTreeAccount {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: TreeType::BatchedState as u64,
        height: 26,
        root_history_capacity: 20,
        queue: BatchedQueue {
            currently_processing_batch_index: 0,
            num_batches: 2,
            batch_size: TEST_DEFAULT_BATCH_SIZE,
            bloom_filter_capacity: 20_000 * 8,
            // next_index: 0,
            zkp_batch_size: 10,
            ..Default::default()
        },
    };
    mt_account.size().unwrap()
}

pub fn get_state_merkle_tree_account_size_from_params(
    params: InitStateTreeAccountsInstructionData,
) -> usize {
    get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    )
}
pub fn get_address_merkle_tree_account_size_from_params(
    params: InitAddressTreeAccountsInstructionData,
) -> usize {
    get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    )
}

pub fn get_merkle_tree_account_size(
    batch_size: u64,
    bloom_filter_capacity: u64,
    zkp_batch_size: u64,
    root_history_capacity: u32,
    height: u32,
    num_batches: u64,
) -> usize {
    let mt_account = BatchedMerkleTreeAccount {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: TreeType::BatchedState as u64,
        height,
        root_history_capacity,
        queue: BatchedQueue {
            num_batches,
            batch_size,
            bloom_filter_capacity,
            zkp_batch_size,
            ..Default::default()
        },
    };
    mt_account.size().unwrap()
}
pub fn assert_nullify_event(
    event: BatchNullifyEvent,
    new_root: [u8; 32],
    old_zero_copy_account: &ZeroCopyBatchedMerkleTreeAccount,
    mt_pubkey: Pubkey,
) {
    let batch_index = old_zero_copy_account
        .get_account()
        .queue
        .next_full_batch_index;
    let batch = old_zero_copy_account
        .batches
        .get(batch_index as usize)
        .unwrap();
    let ref_event = BatchNullifyEvent {
        id: mt_pubkey.to_bytes(),
        batch_index,
        zkp_batch_index: batch.get_num_inserted_zkps(),
        new_root,
        root_index: (old_zero_copy_account.get_root_index() + 1)
            % old_zero_copy_account.get_account().root_history_capacity,
        sequence_number: old_zero_copy_account.get_account().sequence_number + 1,
        batch_size: old_zero_copy_account.get_account().queue.zkp_batch_size,
    };
    assert_eq!(event, ref_event);
}

pub fn assert_batch_append_event_event(
    event: BatchAppendEvent,
    new_root: [u8; 32],
    old_output_queue_account: &ZeroCopyBatchedQueueAccount,
    old_zero_copy_account: &ZeroCopyBatchedMerkleTreeAccount,
    mt_pubkey: Pubkey,
) {
    let batch_index = old_output_queue_account
        .get_account()
        .queue
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
        root_index: (old_zero_copy_account.get_root_index() + 1)
            % old_zero_copy_account.get_account().root_history_capacity,
        sequence_number: old_zero_copy_account.get_account().sequence_number + 1,
        batch_size: old_zero_copy_account.get_account().queue.zkp_batch_size,
        old_next_index: old_zero_copy_account.get_account().next_index,
        new_next_index: old_zero_copy_account.get_account().next_index
            + old_output_queue_account.get_account().queue.zkp_batch_size,
    };
    assert_eq!(event, ref_event);
}
#[cfg(test)]
mod tests {
    #![allow(warnings)]

    use light_bloom_filter::{BloomFilter, BloomFilterError};
    use light_concurrent_merkle_tree::event::NullifierEvent;
    use light_merkle_tree_reference::MerkleTree;
    use light_prover_client::{
        gnark::helpers::{spawn_prover, ProofType, ProverConfig},
        helpers::bigint_to_u8_32,
        mock_batched_forester::{
            self, MockBatchedAddressForester, MockBatchedForester, MockTxEvent,
        },
    };
    use num_bigint::BigInt;
    use num_traits::zero;
    use serial_test::serial;
    use std::{cmp::min, ops::Deref};

    use rand::{rngs::StdRng, Rng};

    use crate::{
        batch::BatchState,
        batched_queue::{
            get_output_queue_account_size_default, get_output_queue_account_size_from_params,
            BatchedQueueAccount,
        },
        init_batched_address_merkle_tree_account, init_batched_state_merkle_tree_accounts,
    };

    use super::*;

    pub fn assert_nullifier_queue_insert(
        mut pre_account: BatchedMerkleTreeAccount,
        mut pre_batches: ManuallyDrop<BoundedVec<Batch>>,
        pre_value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        pre_roots: Vec<[u8; 32]>,
        mut pre_hashchains: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        mut merkle_tree_zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
        bloom_filter_insert_values: Vec<[u8; 32]>,
        leaf_indices: Vec<u64>,
        tx_hash: [u8; 32],
        input_is_in_tree: Vec<bool>,
        array_indices: Vec<usize>,
    ) -> Result<()> {
        let mut leaf_hashchain_insert_values = vec![];
        for (insert_value, leaf_index) in bloom_filter_insert_values.iter().zip(leaf_indices.iter())
        {
            let nullifier =
                Poseidon::hashv(&[insert_value.as_slice(), &leaf_index.to_be_bytes(), &tx_hash])
                    .unwrap();
            leaf_hashchain_insert_values.push(nullifier);
        }
        assert_input_queue_insert(
            pre_account,
            pre_batches,
            pre_value_vecs,
            pre_roots,
            pre_hashchains,
            merkle_tree_zero_copy_account,
            bloom_filter_insert_values,
            leaf_hashchain_insert_values,
            input_is_in_tree,
            array_indices,
        )
    }
    /// Insert into input queue:
    /// 1. New value exists in the current batch bloom_filter
    /// 2. New value does not exist in the other batch bloom_filters
    /// 3.
    pub fn assert_input_queue_insert(
        mut pre_account: BatchedMerkleTreeAccount,
        mut pre_batches: ManuallyDrop<BoundedVec<Batch>>,
        pre_value_vecs: &mut Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        pre_roots: Vec<[u8; 32]>,
        mut pre_hashchains: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        mut merkle_tree_zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
        bloom_filter_insert_values: Vec<[u8; 32]>,
        leaf_hashchain_insert_values: Vec<[u8; 32]>,
        input_is_in_tree: Vec<bool>,
        array_indices: Vec<usize>,
    ) -> Result<()> {
        let mut should_be_wiped = false;
        for (i, insert_value) in bloom_filter_insert_values.iter().enumerate() {
            if !input_is_in_tree[i] {
                let value_vec_index = array_indices[i];
                assert!(
                    pre_value_vecs.iter_mut().any(|value_vec| {
                        if value_vec.len() > value_vec_index {
                            ({
                                if value_vec[value_vec_index] == *insert_value {
                                    value_vec[value_vec_index] = [0u8; 32];
                                    true
                                } else {
                                    false
                                }
                            })
                        } else {
                            false
                        }
                    }),
                    "Value not in value vec."
                );
            }

            let post_roots: Vec<[u8; 32]> = merkle_tree_zero_copy_account
                .root_history
                .iter()
                .cloned()
                .collect();
            // if root buffer changed it must be only overwritten by [0u8;32]
            if post_roots != pre_roots {
                let only_zero_overwrites = post_roots
                    .iter()
                    .zip(pre_roots.iter())
                    .all(|(post, pre)| *post == *pre || *post == [0u8; 32]);
                if !only_zero_overwrites {
                    panic!("Root buffer changed.")
                }
            }

            let current_batch_index = merkle_tree_zero_copy_account
                .get_account()
                .queue
                .currently_processing_batch_index as usize;
            let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;
            let expected_batch = pre_batches.get_mut(inserted_batch_index).unwrap();
            println!(
                "assert input queue batch update: expected_batch: {:?}",
                expected_batch
            );
            println!(
                "assert input queue batch update: expected_batch.get_num_inserted_elements(): {}",
                expected_batch.get_num_inserted_elements()
            );
            println!(
                "assert input queue batch update: expected_batch.batch_size / 2: {}",
                expected_batch.batch_size / 2
            );

            if !should_be_wiped && expected_batch.get_state() == BatchState::Inserted {
                should_be_wiped =
                    expected_batch.get_num_inserted_elements() == expected_batch.batch_size / 2;
            }
            println!(
                "assert input queue batch update: should_be_wiped: {}",
                should_be_wiped
            );
            if expected_batch.get_state() == BatchState::Inserted {
                println!("assert input queue batch update: clearing batch");
                pre_hashchains[inserted_batch_index].clear();
                expected_batch.sequence_number = 0;
                expected_batch.advance_state_to_can_be_filled().unwrap();
                expected_batch.bloom_filter_is_wiped = false;
            }
            println!(
                "assert input queue batch update: inserted_batch_index: {}",
                inserted_batch_index
            );
            // New value exists in the current batch bloom filter
            let mut bloom_filter = light_bloom_filter::BloomFilter::new(
                merkle_tree_zero_copy_account.batches[inserted_batch_index].num_iters as usize,
                merkle_tree_zero_copy_account.batches[inserted_batch_index].bloom_filter_capacity,
                merkle_tree_zero_copy_account.bloom_filter_stores[inserted_batch_index]
                    .as_mut_slice(),
            )
            .unwrap();
            println!(
                "assert input queue batch update: insert_value: {:?}",
                insert_value
            );
            assert!(bloom_filter.contains(&insert_value));
            let mut pre_hashchain = pre_hashchains.get_mut(inserted_batch_index).unwrap();

            expected_batch
                .add_to_hash_chain(&leaf_hashchain_insert_values[i], &mut pre_hashchain)?;

            // New value does not exist in the other batch bloom_filters
            for (i, batch) in merkle_tree_zero_copy_account.batches.iter_mut().enumerate() {
                // Skip current batch it is already checked above
                if i != inserted_batch_index {
                    let mut bloom_filter = light_bloom_filter::BloomFilter::new(
                        batch.num_iters as usize,
                        batch.bloom_filter_capacity,
                        merkle_tree_zero_copy_account.bloom_filter_stores[i].as_mut_slice(),
                    )
                    .unwrap();
                    assert!(!bloom_filter.contains(&insert_value));
                }
            }
            // if the currently processing batch changed it should
            // increment by one and the old batch should be ready to
            // update
            if expected_batch.get_current_zkp_batch_index() == expected_batch.get_num_zkp_batches()
            {
                assert_eq!(
                    merkle_tree_zero_copy_account.batches
                        [pre_account.queue.currently_processing_batch_index as usize]
                        .get_state(),
                    BatchState::Full
                );
                pre_account.queue.currently_processing_batch_index += 1;
                pre_account.queue.currently_processing_batch_index %= pre_account.queue.num_batches;
                assert_eq!(
                    merkle_tree_zero_copy_account.batches[inserted_batch_index],
                    *expected_batch
                );
                assert_eq!(
                    merkle_tree_zero_copy_account.hashchain_store[inserted_batch_index]
                        .last()
                        .unwrap(),
                    pre_hashchain.last().unwrap(),
                    "Hashchain store inconsistent."
                );
            }
        }

        assert_eq!(
            *merkle_tree_zero_copy_account.get_account(),
            pre_account,
            "BatchedMerkleTreeAccount changed."
        );
        let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;
        let mut expected_batch = pre_batches[inserted_batch_index].clone();
        if should_be_wiped {
            expected_batch.bloom_filter_is_wiped = true;
        }
        assert_eq!(
            merkle_tree_zero_copy_account.batches[inserted_batch_index],
            expected_batch
        );
        let other_batch = if inserted_batch_index == 0 { 1 } else { 0 };
        assert_eq!(
            merkle_tree_zero_copy_account.batches[other_batch],
            pre_batches[other_batch]
        );
        assert_eq!(
            merkle_tree_zero_copy_account.hashchain_store, *pre_hashchains,
            "Hashchain store inconsistent."
        );
        Ok(())
    }

    /// Expected behavior for insert into output queue:
    /// - add value to value array
    /// - batch.num_inserted += 1
    /// - if batch is full after insertion advance state to ReadyToUpdateTree
    pub fn assert_output_queue_insert(
        mut pre_account: BatchedQueueAccount,
        mut pre_batches: ManuallyDrop<BoundedVec<Batch>>,
        mut pre_value_store: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        mut pre_hashchains: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        mut output_zero_copy_account: ZeroCopyBatchedQueueAccount,
        insert_values: Vec<[u8; 32]>,
    ) -> Result<()> {
        for batch in output_zero_copy_account.batches.iter_mut() {
            println!("output_zero_copy_account.batch: {:?}", batch);
        }
        for batch in pre_batches.iter() {
            println!("pre_batch: {:?}", batch);
        }
        for insert_value in insert_values.iter() {
            // There are no bloom_filters
            for store in output_zero_copy_account.bloom_filter_stores.iter() {
                assert_eq!(store.capacity(), 0);
            }
            // if the currently processing batch changed it should
            // increment by one and the old batch should be ready to
            // update

            let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;
            let mut expected_batch = &mut pre_batches[inserted_batch_index];
            let pre_value_store = pre_value_store.get_mut(inserted_batch_index).unwrap();
            let pre_hashchain = pre_hashchains.get_mut(inserted_batch_index).unwrap();
            if expected_batch.get_state() == BatchState::Inserted {
                expected_batch.advance_state_to_can_be_filled().unwrap();
                pre_value_store.clear();
                pre_hashchain.clear();
                expected_batch.start_index = pre_account.next_index;
            }
            pre_account.next_index += 1;
            expected_batch.store_and_hash_value(&insert_value, pre_value_store, pre_hashchain)?;

            let other_batch = if inserted_batch_index == 0 { 1 } else { 0 };
            assert!(output_zero_copy_account.value_vecs[inserted_batch_index]
                .as_mut_slice()
                .to_vec()
                .contains(&insert_value));
            assert!(!output_zero_copy_account.value_vecs[other_batch]
                .as_mut_slice()
                .to_vec()
                .contains(&insert_value));
            if expected_batch.get_num_zkp_batches() == expected_batch.get_current_zkp_batch_index()
            {
                assert!(
                    output_zero_copy_account.batches
                        [pre_account.queue.currently_processing_batch_index as usize]
                        .get_state()
                        == BatchState::Full
                );
                pre_account.queue.currently_processing_batch_index += 1;
                pre_account.queue.currently_processing_batch_index %= pre_account.queue.num_batches;
                assert_eq!(
                    output_zero_copy_account.batches[inserted_batch_index],
                    *expected_batch
                );
            }
        }
        let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;
        let expected_batch = &pre_batches[inserted_batch_index];
        assert_eq!(
            output_zero_copy_account.batches[inserted_batch_index],
            *expected_batch
        );
        assert_eq!(
            *output_zero_copy_account.get_account(),
            pre_account,
            "ZeroCopyBatchedQueueAccount changed."
        );
        assert_eq!(pre_hashchains, output_zero_copy_account.hashchain_store);
        assert_eq!(pre_value_store, output_zero_copy_account.value_vecs);
        assert_eq!(pre_batches, output_zero_copy_account.batches);
        Ok(())
    }

    #[derive(Debug, PartialEq, Clone)]
    pub struct MockTransactionInputs {
        inputs: Vec<[u8; 32]>,
        outputs: Vec<[u8; 32]>,
    }

    pub fn simulate_transaction(
        instruction_data: MockTransactionInputs,
        merkle_tree_account_data: &mut [u8],
        output_queue_account_data: &mut [u8],
        reference_merkle_tree: &MerkleTree<Poseidon>,
    ) -> Result<MockTxEvent> {
        let mut output_zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_bytes_mut(output_queue_account_data).unwrap();
        let mut merkle_tree_zero_copy_account =
            ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(merkle_tree_account_data)
                .unwrap();
        let flattened_inputs = instruction_data
            .inputs
            .iter()
            .cloned()
            .chain(instruction_data.outputs.iter().cloned())
            .collect::<Vec<[u8; 32]>>();
        let tx_hash = create_hash_chain_from_vec(flattened_inputs)?;

        for input in instruction_data.inputs.iter() {
            // zkp inclusion in Merkle tree
            let inclusion = reference_merkle_tree.get_leaf_index(input);
            let leaf_index = if inclusion.is_none() {
                println!("simulate_transaction: inclusion is none");
                let mut included = false;
                let mut leaf_index = 0;
                let next_index = merkle_tree_zero_copy_account.get_account().next_index;
                let batch_size = output_zero_copy_account.get_account().queue.batch_size;

                for (batch_index, value_vec) in
                    output_zero_copy_account.value_vecs.iter_mut().enumerate()
                {
                    for (value_index, value) in value_vec.iter_mut().enumerate() {
                        if *value == *input {
                            let batch_start_index = output_zero_copy_account
                                .batches
                                .get(batch_index)
                                .unwrap()
                                .start_index;
                            included = true;
                            *value = [0u8; 32];
                            leaf_index = value_index as u64 + batch_start_index;
                        }
                    }
                }
                if !included {
                    panic!("Value not included in any output queue or trees.");
                }
                leaf_index
            } else {
                inclusion.unwrap() as u64
            };

            println!(
                "sim tx input: \n {:?} \nleaf index : {:?}, \ntx hash {:?}",
                input, leaf_index, tx_hash,
            );
            merkle_tree_zero_copy_account
                .insert_nullifier_into_current_batch(input, leaf_index, &tx_hash)?;
        }

        for output in instruction_data.outputs.iter() {
            let leaf_index = output_zero_copy_account.get_account().next_index;
            println!(
                "sim tx output: \n  {:?} \nleaf index : {:?}",
                output, leaf_index
            );
            output_zero_copy_account.insert_into_current_batch(output)?;
        }
        Ok(MockTxEvent {
            inputs: instruction_data.inputs.clone(),
            outputs: instruction_data.outputs.clone(),
            tx_hash,
        })
    }

    #[serial]
    #[tokio::test]
    async fn test_simulate_transactions() {
        spawn_prover(
            true,
            ProverConfig {
                run_mode: None,
                circuits: vec![
                    ProofType::BatchAppendWithProofsTest,
                    ProofType::BatchUpdateTest,
                ],
            },
        )
        .await;
        let mut mock_indexer = mock_batched_forester::MockBatchedForester::<26>::default();

        let num_tx = 2200;
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = crate::ID;

        let params = crate::InitStateTreeAccountsInstructionData::test_default();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;

        init_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);
        let mut in_ready_for_update = false;
        let mut out_ready_for_update = false;
        let mut num_output_updates = 0;
        let mut num_input_updates = 0;
        let mut num_input_values = 0;
        let mut num_output_values = 0;

        for tx in 0..num_tx {
            println!("tx: {}", tx);
            println!("num_input_updates: {}", num_input_updates);
            println!("num_output_updates: {}", num_output_updates);
            {
                println!("Simulate tx {} -----------------------------", tx);
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let number_of_outputs = rng.gen_range(0..7);
                let mut outputs = vec![];
                for _ in 0..number_of_outputs {
                    outputs.push(get_rnd_bytes(&mut rng));
                }
                let number_of_inputs = if rng.gen_bool(0.5) {
                    let number_of_inputs = if !mock_indexer.active_leaves.is_empty() {
                        let x = min(mock_indexer.active_leaves.len(), 5);
                        rng.gen_range(0..x)
                    } else {
                        0
                    };
                    number_of_inputs
                } else {
                    0
                };

                let mut inputs = vec![];
                let mut input_is_in_tree = vec![];
                let mut leaf_indices = vec![];
                let mut array_indices = vec![];
                let mut retries = min(10, mock_indexer.active_leaves.len());
                while inputs.len() < number_of_inputs && retries > 0 {
                    let (leaf_array_index, leaf) =
                        get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);
                    let inserted = mock_indexer.merkle_tree.get_leaf_index(&leaf);
                    if let Some(leaf_index) = inserted {
                        inputs.push(leaf);
                        leaf_indices.push(leaf_index as u64);
                        input_is_in_tree.push(true);
                        array_indices.push(0);
                    } else if rng.gen_bool(0.1) {
                        inputs.push(leaf);
                        let output_queue = ZeroCopyBatchedQueueAccount::from_bytes_mut(
                            &mut output_queue_account_data,
                        )
                        .unwrap();
                        let mut leaf_array_index = 0;
                        let mut batch_index = 0;
                        for (i, vec) in output_queue.value_vecs.iter().enumerate() {
                            let pos = vec.iter().position(|value| *value == leaf);
                            if let Some(pos) = pos {
                                leaf_array_index = pos;
                                batch_index = i;
                                break;
                            }
                            if i == output_queue.value_vecs.len() - 1 {
                                panic!("Leaf not found in output queue.");
                            }
                        }
                        let batch = output_queue.batches.get(batch_index).unwrap();
                        array_indices.push(leaf_array_index);
                        let leaf_index: u64 = batch.start_index + leaf_array_index as u64;
                        leaf_indices.push(leaf_index);
                        input_is_in_tree.push(false);
                    }
                    retries -= 1;
                }
                let number_of_inputs = inputs.len();
                println!("number_of_inputs: {}", number_of_inputs);

                let instruction_data = MockTransactionInputs {
                    inputs: inputs.clone(),
                    outputs: outputs.clone(),
                };

                let merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                println!(
                    "input queue: {:?}",
                    merkle_tree_zero_copy_account.batches[0].get_num_inserted()
                );
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();
                let mut pre_mt_data = mt_account_data.clone();
                let pre_output_account = output_zero_copy_account.get_account().clone();
                let pre_output_batches = output_zero_copy_account.batches.clone();
                let mut pre_output_value_stores = output_zero_copy_account.value_vecs.clone();
                let pre_hashchains = output_zero_copy_account.hashchain_store.clone();

                let pre_mt_account = merkle_tree_zero_copy_account.get_account().clone();
                let pre_batches = merkle_tree_zero_copy_account.batches.clone();
                let pre_roots = merkle_tree_zero_copy_account
                    .root_history
                    .iter()
                    .cloned()
                    .collect();
                let pre_mt_hashchains = merkle_tree_zero_copy_account.hashchain_store.clone();

                if !outputs.is_empty() || !inputs.is_empty() {
                    println!("Simulating tx with inputs: {:?}", instruction_data);
                    let event = simulate_transaction(
                        instruction_data,
                        &mut pre_mt_data,
                        &mut output_queue_account_data,
                        &mock_indexer.merkle_tree,
                    )
                    .unwrap();
                    mock_indexer.tx_events.push(event.clone());

                    if !inputs.is_empty() {
                        let merkle_tree_zero_copy_account =
                            ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                                &mut pre_mt_data,
                            )
                            .unwrap();
                        println!("inputs: {:?}", inputs);
                        assert_nullifier_queue_insert(
                            pre_mt_account,
                            pre_batches,
                            &mut pre_output_value_stores,
                            pre_roots,
                            pre_mt_hashchains,
                            merkle_tree_zero_copy_account,
                            inputs.clone(),
                            leaf_indices.clone(),
                            event.tx_hash,
                            input_is_in_tree,
                            array_indices,
                        )
                        .unwrap();
                    }

                    if !outputs.is_empty() {
                        assert_output_queue_insert(
                            pre_output_account,
                            pre_output_batches,
                            pre_output_value_stores,
                            pre_hashchains,
                            output_zero_copy_account.clone(),
                            outputs.clone(),
                        )
                        .unwrap();
                    }

                    for i in 0..number_of_inputs {
                        mock_indexer
                            .input_queue_leaves
                            .push((inputs[i], leaf_indices[i] as usize));
                    }
                    for i in 0..number_of_outputs {
                        mock_indexer.active_leaves.push(outputs[i]);
                        mock_indexer.output_queue_leaves.push(outputs[i]);
                    }

                    num_output_values += number_of_outputs;
                    num_input_values += number_of_inputs;
                    let merkle_tree_zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                            &mut pre_mt_data,
                        )
                        .unwrap();
                    in_ready_for_update = merkle_tree_zero_copy_account
                        .batches
                        .iter()
                        .any(|batch| batch.get_first_ready_zkp_batch().is_ok());
                    out_ready_for_update = output_zero_copy_account
                        .batches
                        .iter()
                        .any(|batch| batch.get_first_ready_zkp_batch().is_ok());

                    mt_account_data = pre_mt_data.clone();
                } else {
                    println!("Skipping simulate tx for no inputs or outputs");
                }
            }

            if in_ready_for_update && rng.gen_bool(1.0) {
                println!("Input update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let mut pre_mt_account_data = mt_account_data.clone();
                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                let (input_res, new_root) = {
                    let mut zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                            &mut pre_mt_account_data,
                        )
                        .unwrap();
                    println!("batches {:?}", zero_copy_account.batches);

                    let old_root_index = zero_copy_account.root_history.last_index();
                    let next_full_batch =
                        zero_copy_account.get_account().queue.next_full_batch_index;
                    let batch = zero_copy_account
                        .batches
                        .get(next_full_batch as usize)
                        .unwrap();
                    println!(
                        "zero_copy_account
                        .hashchain_store {:?}",
                        zero_copy_account.hashchain_store
                    );
                    println!(
                        "hashchain store len {:?}",
                        zero_copy_account.hashchain_store.len()
                    );
                    println!(
                        "batch.get_num_inserted_zkps() as usize {:?}",
                        batch.get_num_inserted_zkps() as usize
                    );
                    let leaves_hashchain = zero_copy_account
                        .hashchain_store
                        .get(next_full_batch as usize)
                        .unwrap()
                        .get(batch.get_num_inserted_zkps() as usize)
                        .unwrap();

                    let (proof, new_root) = mock_indexer
                        .get_batched_update_proof(
                            zero_copy_account.get_account().queue.zkp_batch_size as u32,
                            *leaves_hashchain,
                        )
                        .await
                        .unwrap();
                    let instruction_data = InstructionDataBatchNullifyInputs {
                        public_inputs: BatchProofInputsIx {
                            new_root,
                            old_root_index: old_root_index as u16,
                        },
                        compressed_proof: CompressedProof {
                            a: proof.a,
                            b: proof.b,
                            c: proof.c,
                        },
                    };

                    (
                        zero_copy_account
                            .update_input_queue(instruction_data, mt_pubkey.to_bytes()),
                        new_root,
                    )
                };
                println!("Input update -----------------------------");
                println!("res {:?}", input_res);
                assert!(input_res.is_ok());
                let nullify_event = input_res.unwrap();
                in_ready_for_update = false;
                // assert Merkle tree
                // sequence number increased X
                // next index increased X
                // current root index increased X
                // One root changed one didn't

                let zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut pre_mt_account_data,
                    )
                    .unwrap();
                assert_nullify_event(nullify_event, new_root, &old_zero_copy_account, mt_pubkey);
                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    None,
                    None,
                    new_root,
                );
                mt_account_data = pre_mt_account_data.clone();

                num_input_updates += 1;
            }

            if out_ready_for_update && rng.gen_bool(1.0) {
                println!("Output update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);

                let mut pre_mt_account_data = mt_account_data.clone();
                let mut zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut pre_mt_account_data,
                    )
                    .unwrap();
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();

                let next_index = zero_copy_account.get_account().next_index;
                let next_full_batch = output_zero_copy_account
                    .get_account()
                    .queue
                    .next_full_batch_index;
                let batch = output_zero_copy_account
                    .batches
                    .get(next_full_batch as usize)
                    .unwrap();
                let leaves_hashchain = output_zero_copy_account
                    .hashchain_store
                    .get(next_full_batch as usize)
                    .unwrap()
                    .get(batch.get_num_inserted_zkps() as usize)
                    .unwrap();
                let (proof, new_root) = mock_indexer
                    .get_batched_append_proof(
                        next_index as usize,
                        batch.get_num_inserted_zkps() as u32,
                        batch.zkp_batch_size as u32,
                        *leaves_hashchain,
                        batch.get_num_zkp_batches() as u32,
                    )
                    .await
                    .unwrap();

                let instruction_data = InstructionDataBatchAppendInputs {
                    public_inputs: AppendBatchProofInputsIx { new_root },
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };

                let mut pre_output_queue_state = output_queue_account_data.clone();
                println!("Output update -----------------------------");

                let output_res = zero_copy_account.update_output_queue(
                    &mut pre_output_queue_state,
                    instruction_data,
                    mt_pubkey.to_bytes(),
                );
                assert!(output_res.is_ok());
                let batch_append_event = output_res.unwrap();

                assert_eq!(
                    *zero_copy_account.root_history.last().unwrap(),
                    mock_indexer.merkle_tree.root()
                );
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut pre_output_queue_state)
                        .unwrap();
                let old_output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();

                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();

                println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
                assert_batch_append_event_event(
                    batch_append_event,
                    new_root,
                    &old_output_zero_copy_account,
                    &old_zero_copy_account,
                    mt_pubkey,
                );
                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    Some(old_output_zero_copy_account),
                    Some(output_zero_copy_account),
                    new_root,
                );

                output_queue_account_data = pre_output_queue_state;
                mt_account_data = pre_mt_account_data;
                out_ready_for_update = false;
                num_output_updates += 1;
            }
        }
        let output_zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data).unwrap();
        println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
        println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
        println!("num_output_updates: {}", num_output_updates);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_values: {}", num_output_values);
        println!("num_input_values: {}", num_input_values);
    }

    // Get random leaf that is not in the input queue.
    pub fn get_random_leaf(
        rng: &mut StdRng,
        active_leaves: &mut Vec<[u8; 32]>,
    ) -> (usize, [u8; 32]) {
        if active_leaves.len() == 0 {
            return (0, [0u8; 32]);
        }
        let index = rng.gen_range(0..active_leaves.len());
        // get random leaf from vector and remove it
        (index, active_leaves.remove(index))
    }

    /// queues with a counter which keeps things below X tps and an if that
    /// executes tree updates when possible.
    #[serial]
    #[tokio::test]
    async fn test_e2e() {
        spawn_prover(
            true,
            ProverConfig {
                run_mode: None,
                circuits: vec![
                    ProofType::BatchAppendWithProofsTest,
                    ProofType::BatchUpdateTest,
                ],
            },
        )
        .await;
        let mut mock_indexer = mock_batched_forester::MockBatchedForester::<26>::default();

        let num_tx = 2200;
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let params = crate::InitStateTreeAccountsInstructionData::test_default();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;

        init_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);
        let mut in_ready_for_update = false;
        let mut out_ready_for_update = false;
        let mut num_output_updates = 0;
        let mut num_input_updates = 0;
        let mut num_input_values = 0;
        let mut num_output_values = 0;

        for tx in 0..num_tx {
            println!("tx: {}", tx);
            println!("num_input_updates: {}", num_input_updates);
            println!("num_output_updates: {}", num_output_updates);
            // Output queue
            {
                let mut output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();
                if rng.gen_bool(0.5) {
                    println!("Output insert -----------------------------");
                    println!("num_output_values: {}", num_output_values);
                    let mut rnd_bytes = get_rnd_bytes(&mut rng);

                    let pre_account = output_zero_copy_account.get_account().clone();
                    let pre_batches = output_zero_copy_account.batches.clone();
                    let pre_value_store = output_zero_copy_account.value_vecs.clone();
                    let pre_hashchains = output_zero_copy_account.hashchain_store.clone();

                    output_zero_copy_account
                        .insert_into_current_batch(&rnd_bytes)
                        .unwrap();
                    assert_output_queue_insert(
                        pre_account,
                        pre_batches,
                        pre_value_store,
                        pre_hashchains,
                        output_zero_copy_account.clone(),
                        vec![rnd_bytes],
                    )
                    .unwrap();
                    num_output_values += 1;
                    mock_indexer.output_queue_leaves.push(rnd_bytes);
                }
                out_ready_for_update = output_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.get_state() == BatchState::Full);
            }

            // Input queue
            {
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();

                if rng.gen_bool(0.5) && !mock_indexer.active_leaves.is_empty() {
                    println!("Input insert -----------------------------");
                    let (_, leaf) = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);

                    let pre_batches: ManuallyDrop<BoundedVec<Batch>> =
                        merkle_tree_zero_copy_account.batches.clone();
                    let pre_account = merkle_tree_zero_copy_account.get_account().clone();
                    let pre_roots = merkle_tree_zero_copy_account
                        .root_history
                        .iter()
                        .cloned()
                        .collect();
                    let pre_hashchains = merkle_tree_zero_copy_account.hashchain_store.clone();
                    let tx_hash = create_hash_chain_from_vec(vec![leaf].to_vec()).unwrap();
                    let leaf_index = mock_indexer.merkle_tree.get_leaf_index(&leaf).unwrap();
                    mock_indexer.input_queue_leaves.push((leaf, leaf_index));
                    mock_indexer.tx_events.push(MockTxEvent {
                        inputs: vec![leaf],
                        outputs: vec![],
                        tx_hash,
                    });

                    merkle_tree_zero_copy_account
                        .insert_nullifier_into_current_batch(
                            &leaf.to_vec().try_into().unwrap(),
                            leaf_index as u64,
                            &tx_hash,
                        )
                        .unwrap();

                    {
                        let merkle_tree_zero_copy_account =
                            ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                                &mut mt_account_data,
                            )
                            .unwrap();
                        assert_nullifier_queue_insert(
                            pre_account,
                            pre_batches,
                            &mut vec![],
                            pre_roots,
                            pre_hashchains,
                            merkle_tree_zero_copy_account,
                            vec![leaf],
                            vec![leaf_index as u64],
                            tx_hash,
                            vec![true],
                            vec![],
                        )
                        .unwrap();
                    }
                    num_input_values += 1;
                }

                in_ready_for_update = merkle_tree_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.get_state() == BatchState::Full);
            }

            if in_ready_for_update {
                println!("Input update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let mut pre_mt_account_data = mt_account_data.clone();
                in_ready_for_update = false;
                perform_input_update(&mut pre_mt_account_data, &mut mock_indexer, true, mt_pubkey)
                    .await;
                mt_account_data = pre_mt_account_data.clone();

                num_input_updates += 1;
            }

            if out_ready_for_update {
                println!("Output update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let mut pre_mt_account_data = mt_account_data.clone();
                let mut zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut pre_mt_account_data,
                    )
                    .unwrap();
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();

                let next_index = zero_copy_account.get_account().next_index;
                let next_full_batch = output_zero_copy_account
                    .get_account()
                    .queue
                    .next_full_batch_index;
                let batch = output_zero_copy_account
                    .batches
                    .get(next_full_batch as usize)
                    .unwrap();
                let leaves = output_zero_copy_account
                    .value_vecs
                    .get(next_full_batch as usize)
                    .unwrap()
                    .deref()
                    .clone()
                    .to_vec();
                println!("leaves {:?}", leaves.len());
                let leaves_hashchain = output_zero_copy_account
                    .hashchain_store
                    .get(next_full_batch as usize)
                    .unwrap()
                    .get(batch.get_num_inserted_zkps() as usize)
                    .unwrap();
                let (proof, new_root) = mock_indexer
                    .get_batched_append_proof(
                        next_index as usize,
                        batch.get_num_inserted_zkps() as u32,
                        batch.zkp_batch_size as u32,
                        *leaves_hashchain,
                        batch.get_num_zkp_batches() as u32,
                    )
                    .await
                    .unwrap();
                let start = batch.get_num_inserted_zkps() as usize * batch.zkp_batch_size as usize;
                let end = start + batch.zkp_batch_size as usize;
                for i in start..end {
                    // Storing the leaf in the output queue indexer so that it
                    // can be inserted into the input queue later.
                    mock_indexer.active_leaves.push(leaves[i]);
                }

                let instruction_data = InstructionDataBatchAppendInputs {
                    public_inputs: AppendBatchProofInputsIx { new_root },
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };

                let mut pre_output_queue_state = output_queue_account_data.clone();
                println!("Output update -----------------------------");

                let output_res = zero_copy_account.update_output_queue(
                    &mut pre_output_queue_state,
                    instruction_data,
                    mt_pubkey.to_bytes(),
                );

                assert_eq!(
                    *zero_copy_account.root_history.last().unwrap(),
                    mock_indexer.merkle_tree.root()
                );
                println!(
                    "post update: sequence number: {}",
                    zero_copy_account.get_account().sequence_number
                );
                println!("output_res {:?}", output_res);
                assert!(output_res.is_ok());

                println!("output update success {}", num_output_updates);
                println!("num_output_values: {}", num_output_values);
                println!("num_input_values: {}", num_input_values);
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut pre_output_queue_state)
                        .unwrap();
                let old_output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();

                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();

                println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
                let nullify_event = output_res.unwrap();
                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    Some(old_output_zero_copy_account),
                    Some(output_zero_copy_account),
                    new_root,
                );

                output_queue_account_data = pre_output_queue_state;
                mt_account_data = pre_mt_account_data;
                out_ready_for_update = false;
                num_output_updates += 1;
            }
        }
        let output_zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data).unwrap();
        println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
        println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
        println!("num_output_updates: {}", num_output_updates);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_values: {}", num_output_values);
        println!("num_input_values: {}", num_input_values);
    }
    pub async fn perform_input_update(
        mt_account_data: &mut [u8],
        mock_indexer: &mut MockBatchedForester<26>,
        enable_assert: bool,
        mt_pubkey: Pubkey,
    ) {
        let mut cloned_mt_account_data = (*mt_account_data).to_vec();
        let old_zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
            cloned_mt_account_data.as_mut_slice(),
        )
        .unwrap();
        let (input_res, root) = {
            let mut zero_copy_account =
                ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(mt_account_data)
                    .unwrap();

            let old_root_index = zero_copy_account.root_history.last_index();
            let next_full_batch = zero_copy_account.get_account().queue.next_full_batch_index;
            let batch = zero_copy_account
                .batches
                .get(next_full_batch as usize)
                .unwrap();
            let leaves_hashchain = zero_copy_account
                .hashchain_store
                .get(next_full_batch as usize)
                .unwrap()
                .get(batch.get_num_inserted_zkps() as usize)
                .unwrap();
            let (proof, new_root) = mock_indexer
                .get_batched_update_proof(
                    zero_copy_account.get_account().queue.zkp_batch_size as u32,
                    *leaves_hashchain,
                )
                .await
                .unwrap();
            let instruction_data = InstructionDataBatchNullifyInputs {
                public_inputs: BatchProofInputsIx {
                    new_root,
                    old_root_index: old_root_index as u16,
                },
                compressed_proof: CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
            };

            (
                zero_copy_account.update_input_queue(instruction_data, mt_pubkey.to_bytes()),
                new_root,
            )
        };
        println!("Input update -----------------------------");
        println!("res {:?}", input_res);
        assert!(input_res.is_ok());
        let event = input_res.unwrap();

        // assert Merkle tree
        // sequence number increased X
        // next index increased X
        // current root index increased X
        // One root changed one didn't

        let zero_copy_account =
            ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(mt_account_data).unwrap();
        if enable_assert {
            assert_merkle_tree_update(old_zero_copy_account, zero_copy_account, None, None, root);
        }
    }

    pub async fn perform_address_update(
        mt_account_data: &mut [u8],
        mock_indexer: &mut MockBatchedAddressForester<26>,
        enable_assert: bool,
        mt_pubkey: Pubkey,
    ) {
        println!("pre address update -----------------------------");
        let mut cloned_mt_account_data = (*mt_account_data).to_vec();
        let old_zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
            cloned_mt_account_data.as_mut_slice(),
        )
        .unwrap();
        let (input_res, root, pre_next_full_batch) = {
            let mut zero_copy_account =
                ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(mt_account_data)
                    .unwrap();

            let old_root_index = zero_copy_account.root_history.last_index();
            let next_full_batch = zero_copy_account.get_account().queue.next_full_batch_index;
            let next_index = zero_copy_account.get_account().next_index;
            println!("next index {:?}", next_index);
            let batch = zero_copy_account
                .batches
                .get(next_full_batch as usize)
                .unwrap();
            let batch_start_index = batch.start_index;
            let leaves_hashchain = zero_copy_account
                .hashchain_store
                .get(next_full_batch as usize)
                .unwrap()
                .get(batch.get_num_inserted_zkps() as usize)
                .unwrap();
            let current_root = zero_copy_account.root_history.last().unwrap();
            let (proof, new_root) = mock_indexer
                .get_batched_address_proof(
                    zero_copy_account.get_account().queue.batch_size as u32,
                    zero_copy_account.get_account().queue.zkp_batch_size as u32,
                    *leaves_hashchain,
                    next_index as usize,
                    batch_start_index as usize,
                    *current_root,
                )
                .await
                .unwrap();
            let instruction_data = InstructionDataBatchNullifyInputs {
                public_inputs: BatchProofInputsIx {
                    new_root,
                    old_root_index: old_root_index as u16,
                },
                compressed_proof: CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
            };

            (
                zero_copy_account.update_address_queue(instruction_data, mt_pubkey.to_bytes()),
                new_root,
                next_full_batch,
            )
        };
        println!("post address update -----------------------------");
        println!("res {:?}", input_res);
        assert!(input_res.is_ok());
        let event = input_res.unwrap();

        // assert Merkle tree
        // sequence number increased X
        // next index increased X
        // current root index increased X
        // One root changed one didn't

        let zero_copy_account =
            ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(mt_account_data).unwrap();

        {
            let next_full_batch = zero_copy_account.get_account().queue.next_full_batch_index;
            let batch = zero_copy_account
                .batches
                .get(next_full_batch as usize)
                .unwrap();
            // println!("batch {:?}", batch);
            // println!("account state {:?}", batch.get_state());
            if pre_next_full_batch != next_full_batch {
                mock_indexer.finalize_batch_address_update(batch.batch_size as usize);
            }
        }
        if enable_assert {
            assert_merkle_tree_update(old_zero_copy_account, zero_copy_account, None, None, root);
        }
    }

    fn assert_merkle_tree_update(
        old_zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
        zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
        old_queue_account: Option<ZeroCopyBatchedQueueAccount>,
        queue_account: Option<ZeroCopyBatchedQueueAccount>,
        root: [u8; 32],
    ) {
        let mut expected_account = old_zero_copy_account.get_account().clone();
        expected_account.sequence_number += 1;
        let actual_account = zero_copy_account.get_account().clone();

        let (
            batches,
            previous_batchs,
            previous_processing,
            expected_queue_account,
            mut next_full_batch_index,
        ) = if let Some(queue_account) = queue_account.as_ref() {
            let expected_queue_account = old_queue_account.as_ref().unwrap().get_account().clone();

            let previous_processing = if queue_account
                .get_account()
                .queue
                .currently_processing_batch_index
                == 0
            {
                queue_account.get_account().queue.num_batches - 1
            } else {
                queue_account
                    .get_account()
                    .queue
                    .currently_processing_batch_index
                    - 1
            };
            expected_account.next_index += queue_account.batches.get(0).unwrap().zkp_batch_size;
            let next_full_batch_index = expected_queue_account.queue.next_full_batch_index;
            (
                queue_account.batches.clone(),
                old_queue_account.as_ref().unwrap().batches.clone(),
                previous_processing,
                Some(expected_queue_account),
                next_full_batch_index,
            )
        } else {
            // We only have two batches.
            let previous_processing =
                if expected_account.queue.currently_processing_batch_index == 0 {
                    1
                } else {
                    0
                };
            (
                zero_copy_account.batches.clone(),
                old_zero_copy_account.batches.clone(),
                previous_processing,
                None,
                0,
            )
        };

        let mut checked_one = false;
        for (i, batch) in batches.iter().enumerate() {
            let previous_batch = previous_batchs.get(i).unwrap();

            let expected_sequence_number = zero_copy_account.root_history.capacity() as u64
                + zero_copy_account.get_account().sequence_number;
            let batch_fully_inserted = batch.sequence_number == expected_sequence_number
                && batch.get_state() == BatchState::Inserted;

            let updated_batch = previous_batch.get_first_ready_zkp_batch().is_ok() && !checked_one;
            // Assert fully inserted batch
            if batch_fully_inserted {
                if queue_account.is_some() {
                    next_full_batch_index += 1;
                    next_full_batch_index %= expected_queue_account.unwrap().queue.num_batches;
                } else {
                    expected_account.queue.next_full_batch_index += 1;
                    expected_account.queue.next_full_batch_index %=
                        expected_account.queue.num_batches;
                }
                assert_eq!(
                    batch.root_index as usize,
                    zero_copy_account.root_history.last_index()
                );
                assert_eq!(batch.get_num_inserted_zkps(), 0);
                assert_eq!(batch.get_num_inserted(), previous_batch.get_num_inserted());
                assert_eq!(batch.get_num_inserted(), 0);
                assert_ne!(batch.sequence_number, previous_batch.sequence_number);
                assert_eq!(batch.get_current_zkp_batch_index(), 0);
                assert_ne!(batch.get_state(), previous_batch.get_state());
            }
            // assert updated batch
            else if updated_batch {
                checked_one = true;
                assert_eq!(
                    batch.get_num_inserted_zkps(),
                    previous_batch.get_num_inserted_zkps() + 1
                );
                assert_eq!(batch.get_num_inserted(), previous_batch.get_num_inserted());

                assert_eq!(batch.sequence_number, previous_batch.sequence_number);
                assert_eq!(batch.root_index, previous_batch.root_index);
                assert_eq!(
                    batch.get_current_zkp_batch_index(),
                    previous_batch.get_current_zkp_batch_index()
                );
                assert_eq!(batch.get_state(), previous_batch.get_state());
                assert_eq!(batch.get_num_inserted(), previous_batch.get_num_inserted());
            } else {
                assert_eq!(*batch, *previous_batch);
            }
        }
        if let Some(queue_account) = queue_account.as_ref() {
            let mut expected_queue_account = expected_queue_account.unwrap();
            expected_queue_account.queue.next_full_batch_index = next_full_batch_index;
            assert_eq!(*queue_account.get_account(), expected_queue_account);
        }

        assert_eq!(actual_account, expected_account);
        for (i, root) in zero_copy_account.root_history.iter().enumerate() {
            println!("current: i {:?}", i);
            println!("current: root {:?}", root);
        }
        for (i, root) in old_zero_copy_account.root_history.iter().enumerate() {
            println!("old_zero_copy_account: i {:?}", i);
            println!("old_zero_copy_account: root {:?}", root);
        }
        assert_eq!(*zero_copy_account.root_history.last().unwrap(), root);
    }

    pub fn get_rnd_bytes(rng: &mut StdRng) -> [u8; 32] {
        let mut rnd_bytes = rng.gen::<[u8; 32]>();
        rnd_bytes[0] = 0;
        rnd_bytes
    }

    #[serial]
    #[tokio::test]
    async fn test_fill_queues_completely() {
        spawn_prover(
            true,
            ProverConfig {
                run_mode: None,
                circuits: vec![
                    ProofType::BatchAppendWithProofsTest,
                    ProofType::BatchUpdateTest,
                ],
            },
        )
        .await;
        let roothistory_capacity = vec![17, 80]; //
        for root_history_capacity in roothistory_capacity {
            let mut mock_indexer = mock_batched_forester::MockBatchedForester::<26>::default();

            let mut params = crate::InitStateTreeAccountsInstructionData::test_default();
            params.output_queue_batch_size = params.input_queue_batch_size * 10;
            // Root history capacity which is greater than the input updates
            params.root_history_capacity = root_history_capacity;

            let owner = Pubkey::new_unique();

            let queue_account_size = get_output_queue_account_size_from_params(params);

            let mut output_queue_account_data = vec![0; queue_account_size];
            let output_queue_pubkey = Pubkey::new_unique();

            let mt_account_size = get_state_merkle_tree_account_size_from_params(params);
            let mut mt_account_data = vec![0; mt_account_size];
            let mt_pubkey = Pubkey::new_unique();

            let merkle_tree_rent = 1_000_000_000;
            let queue_rent = 1_000_000_000;
            let additional_bytes_rent = 1000;

            init_batched_state_merkle_tree_accounts(
                owner,
                params,
                &mut output_queue_account_data,
                output_queue_pubkey,
                queue_rent,
                &mut mt_account_data,
                mt_pubkey,
                merkle_tree_rent,
                additional_bytes_rent,
            )
            .unwrap();
            use rand::SeedableRng;
            let mut rng = StdRng::seed_from_u64(0);
            let mut in_ready_for_update = false;
            let mut out_ready_for_update = false;
            let mut num_output_updates = 0;
            let mut num_input_updates = 0;
            let mut num_input_values = 0;
            let mut num_output_values = 0;
            let mut output_zero_copy_account =
                ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                    .unwrap();
            let num_tx = params.output_queue_num_batches * params.output_queue_batch_size;

            for tx in 0..num_tx {
                // Output queue
                let mut output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();

                let mut rnd_bytes = get_rnd_bytes(&mut rng);

                let pre_account = output_zero_copy_account.get_account().clone();
                let pre_batches = output_zero_copy_account.batches.clone();
                let pre_value_store = output_zero_copy_account.value_vecs.clone();
                let pre_hashchains = output_zero_copy_account.hashchain_store.clone();

                output_zero_copy_account
                    .insert_into_current_batch(&rnd_bytes)
                    .unwrap();
                assert_output_queue_insert(
                    pre_account,
                    pre_batches,
                    pre_value_store,
                    pre_hashchains,
                    output_zero_copy_account.clone(),
                    vec![rnd_bytes],
                )
                .unwrap();
                mock_indexer.output_queue_leaves.push(rnd_bytes);
                num_output_values += 1;
            }
            let rnd_bytes = get_rnd_bytes(&mut rng);
            let result = output_zero_copy_account.insert_into_current_batch(&rnd_bytes);
            assert_eq!(
                result.unwrap_err(),
                AccountCompressionErrorCode::BatchNotReady.into()
            );

            output_zero_copy_account
                .batches
                .iter()
                .for_each(|b| assert_eq!(b.get_state(), BatchState::Full));

            for i in 0..output_zero_copy_account
                .get_account()
                .queue
                .get_num_zkp_batches()
            {
                println!("Output update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let mut pre_mt_account_data = mt_account_data.clone();
                let mut zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut pre_mt_account_data,
                    )
                    .unwrap();
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();
                let mut pre_output_queue_state = output_queue_account_data.clone();
                let next_index = zero_copy_account.get_account().next_index;
                let next_full_batch = output_zero_copy_account
                    .get_account()
                    .queue
                    .next_full_batch_index;
                let batch = output_zero_copy_account
                    .batches
                    .get(next_full_batch as usize)
                    .unwrap();
                let leaves = mock_indexer.output_queue_leaves.clone();
                let leaves_hashchain = output_zero_copy_account
                    .hashchain_store
                    .get(next_full_batch as usize)
                    .unwrap()
                    .get(batch.get_num_inserted_zkps() as usize)
                    .unwrap();
                let (proof, new_root) = mock_indexer
                    .get_batched_append_proof(
                        next_index as usize,
                        batch.get_num_inserted_zkps() as u32,
                        batch.zkp_batch_size as u32,
                        *leaves_hashchain,
                        batch.get_num_zkp_batches() as u32,
                    )
                    .await
                    .unwrap();
                let start = batch.get_num_inserted_zkps() as usize * batch.zkp_batch_size as usize;
                let end = start + batch.zkp_batch_size as usize;
                for i in start..end {
                    // Storing the leaf in the output queue indexer so that it
                    // can be inserted into the input queue later.
                    mock_indexer.active_leaves.push(leaves[i]);
                }

                let instruction_data = InstructionDataBatchAppendInputs {
                    public_inputs: AppendBatchProofInputsIx { new_root },
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };

                println!("Output update -----------------------------");

                let output_res = zero_copy_account.update_output_queue(
                    &mut pre_output_queue_state,
                    instruction_data,
                    mt_pubkey.to_bytes(),
                );
                assert!(output_res.is_ok());

                assert_eq!(
                    *zero_copy_account.root_history.last().unwrap(),
                    mock_indexer.merkle_tree.root()
                );

                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut pre_output_queue_state)
                        .unwrap();
                let old_output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                        .unwrap();

                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();

                output_queue_account_data = pre_output_queue_state;
                mt_account_data = pre_mt_account_data;
                out_ready_for_update = false;
                num_output_updates += 1;
            }

            let num_tx = params.input_queue_num_batches * params.input_queue_batch_size;
            let mut first_value = [0u8; 32];
            for tx in 0..num_tx {
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();

                println!("Input insert -----------------------------");
                let (_, leaf) = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);
                let leaf_index = mock_indexer.merkle_tree.get_leaf_index(&leaf).unwrap();

                let pre_batches: ManuallyDrop<BoundedVec<Batch>> =
                    merkle_tree_zero_copy_account.batches.clone();
                let pre_account = merkle_tree_zero_copy_account.get_account().clone();
                let pre_roots = merkle_tree_zero_copy_account
                    .root_history
                    .iter()
                    .cloned()
                    .collect();
                let pre_hashchains = merkle_tree_zero_copy_account.hashchain_store.clone();
                let tx_hash = create_hash_chain_from_vec(vec![leaf].to_vec()).unwrap();
                // Index input queue insert event
                mock_indexer.input_queue_leaves.push((leaf, leaf_index));
                mock_indexer.tx_events.push(MockTxEvent {
                    inputs: vec![leaf],
                    outputs: vec![],
                    tx_hash,
                });
                println!("leaf {:?}", leaf);
                println!("leaf_index {:?}", leaf_index);
                merkle_tree_zero_copy_account
                    .insert_nullifier_into_current_batch(
                        &leaf.to_vec().try_into().unwrap(),
                        leaf_index as u64,
                        &tx_hash,
                    )
                    .unwrap();
                assert_nullifier_queue_insert(
                    pre_account,
                    pre_batches,
                    &mut vec![],
                    pre_roots,
                    pre_hashchains,
                    merkle_tree_zero_copy_account,
                    vec![leaf],
                    vec![leaf_index as u64],
                    tx_hash,
                    vec![true],
                    vec![],
                )
                .unwrap();

                // Insert the same value twice
                {
                    // copy data so that failing test doesn't affect the state of
                    // subsequent tests
                    let mut mt_account_data = mt_account_data.clone();
                    let mut merkle_tree_zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                            &mut mt_account_data,
                        )
                        .unwrap();
                    let result = merkle_tree_zero_copy_account.insert_nullifier_into_current_batch(
                        &leaf.to_vec().try_into().unwrap(),
                        leaf_index as u64,
                        &tx_hash,
                    );
                    result.unwrap_err();
                    // assert_eq!(
                    //     result.unwrap_err(),
                    //     AccountCompressionErrorCode::BatchInsertFailed.into()
                    // );
                }
                // Try to insert first value into any batch
                if tx == 0 {
                    first_value = leaf;
                } else {
                    let mut mt_account_data = mt_account_data.clone();
                    let mut merkle_tree_zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                            &mut mt_account_data,
                        )
                        .unwrap();
                    let result = merkle_tree_zero_copy_account.insert_nullifier_into_current_batch(
                        &first_value.to_vec().try_into().unwrap(),
                        leaf_index as u64,
                        &tx_hash,
                    );
                    // assert_eq!(
                    //     result.unwrap_err(),
                    //     AccountCompressionErrorCode::BatchInsertFailed.into()
                    // );
                    result.unwrap_err();
                    // assert_eq!(result.unwrap_err(), BloomFilterError::Full.into());
                }
            }
            // Assert input queue is full and doesn't accept more inserts
            {
                let merkle_tree_zero_copy_account =
                    &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                let rnd_bytes = get_rnd_bytes(&mut rng);
                let tx_hash = get_rnd_bytes(&mut rng);
                let result = merkle_tree_zero_copy_account
                    .insert_nullifier_into_current_batch(&rnd_bytes, 0, &tx_hash);
                assert_eq!(
                    result.unwrap_err(),
                    AccountCompressionErrorCode::BatchNotReady.into()
                );
            }
            // Root of the final batch of first input queue batch
            let mut first_input_batch_update_root_value = [0u8; 32];
            let num_updates = params.input_queue_batch_size / params.input_queue_zkp_batch_size
                * params.input_queue_num_batches;
            for i in 0..num_updates {
                println!("input update ----------------------------- {}", i);
                perform_input_update(&mut mt_account_data, &mut mock_indexer, false, mt_pubkey)
                    .await;
                if i == 5 {
                    let merkle_tree_zero_copy_account =
                        &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                            &mut mt_account_data,
                        )
                        .unwrap();
                    let batch = merkle_tree_zero_copy_account.batches.get(0).unwrap();
                    assert!(batch.bloom_filter_is_wiped);
                }
                println!(
                    "performed input queue batched update {} created root {:?}",
                    i,
                    mock_indexer.merkle_tree.root()
                );
                if i == 4 {
                    first_input_batch_update_root_value = mock_indexer.merkle_tree.root();
                }
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                println!(
                    "root {:?}",
                    merkle_tree_zero_copy_account.root_history.last().unwrap()
                );
                println!(
                    "root last index {:?}",
                    merkle_tree_zero_copy_account.root_history.last_index()
                );
            }
            // assert all bloom_filters are inserted
            {
                let merkle_tree_zero_copy_account =
                    &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                for (i, batch) in merkle_tree_zero_copy_account.batches.iter().enumerate() {
                    println!("batch {:?}", batch);
                    assert_eq!(batch.get_state(), BatchState::Inserted);
                    if i == 0 {
                        assert!(batch.bloom_filter_is_wiped);
                    } else {
                        assert!(!batch.bloom_filter_is_wiped);
                    }
                }
            }
            // do one insert and expect that roots until  merkle_tree_zero_copy_account.batches[0].root_index are zero
            {
                let merkle_tree_zero_copy_account =
                    &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                let pre_batch_zero = merkle_tree_zero_copy_account
                    .batches
                    .get(0)
                    .unwrap()
                    .clone();

                let value = &get_rnd_bytes(&mut rng);
                let tx_hash = &get_rnd_bytes(&mut rng);
                merkle_tree_zero_copy_account
                    .insert_nullifier_into_current_batch(value, 0, tx_hash);
                {
                    let post_batch = merkle_tree_zero_copy_account
                        .batches
                        .get(0)
                        .unwrap()
                        .clone();
                    assert_eq!(post_batch.get_state(), BatchState::CanBeFilled);
                    assert_eq!(post_batch.get_num_inserted(), 1);
                    let mut bloom_filter_store = merkle_tree_zero_copy_account
                        .bloom_filter_stores
                        .get_mut(0)
                        .unwrap();
                    let mut bloom_filter = BloomFilter::new(
                        params.bloom_filter_num_iters as usize,
                        params.bloom_filter_capacity,
                        bloom_filter_store.as_mut_slice(),
                    )
                    .unwrap();
                    assert!(bloom_filter.contains(value));
                }

                let root_history_len = merkle_tree_zero_copy_account
                    .get_account()
                    .root_history_capacity;
                for root in merkle_tree_zero_copy_account.root_history.iter() {
                    println!("root {:?}", root);
                }
                println!(
                    "root in root index {:?}",
                    merkle_tree_zero_copy_account.root_history[pre_batch_zero.root_index as usize]
                );
                // check that all roots have been overwritten except the root index
                // of the update
                let root_history_len: u32 = merkle_tree_zero_copy_account.root_history.len() as u32;
                let start = merkle_tree_zero_copy_account.root_history.last_index() as u32;
                println!("start {:?}", start);
                for root in start + 1..pre_batch_zero.root_index + root_history_len {
                    println!("actual index {:?}", root);
                    let index = root % root_history_len;

                    if index == pre_batch_zero.root_index {
                        let root_index = pre_batch_zero.root_index as usize;

                        assert_eq!(
                            merkle_tree_zero_copy_account.root_history[root_index],
                            first_input_batch_update_root_value
                        );
                        assert_eq!(
                            merkle_tree_zero_copy_account.root_history[root_index - 1],
                            [0u8; 32]
                        );
                        break;
                    }
                    println!("index {:?}", index);
                    assert_eq!(
                        merkle_tree_zero_copy_account.root_history[index as usize],
                        [0u8; 32]
                    );
                }
            }
        }
    }
    // TODO: add test that we cannot insert a batch that is not ready

    #[serial]
    #[tokio::test]
    async fn test_fill_address_tree_completely() {
        spawn_prover(
            true,
            ProverConfig {
                run_mode: None,
                circuits: vec![ProofType::BatchAddressAppendTest],
            },
        )
        .await;
        let roothistory_capacity = vec![17, 80]; //
        for root_history_capacity in roothistory_capacity {
            let mut mock_indexer =
                mock_batched_forester::MockBatchedAddressForester::<26>::default();

            let mut params = crate::InitAddressTreeAccountsInstructionData::test_default();
            // Root history capacity which is greater than the input updates
            params.root_history_capacity = root_history_capacity;

            let owner = Pubkey::new_unique();

            let mt_account_size = get_address_merkle_tree_account_size_from_params(params);
            let mut mt_account_data = vec![0; mt_account_size];
            let mt_pubkey = Pubkey::new_unique();

            let merkle_tree_rent = 1_000_000_000;

            init_batched_address_merkle_tree_account(
                owner,
                params,
                &mut mt_account_data,
                merkle_tree_rent,
            )
            .unwrap();
            use rand::SeedableRng;
            let mut rng = StdRng::seed_from_u64(0);
            let mut in_ready_for_update = false;
            let mut out_ready_for_update = false;
            let mut num_output_updates = 0;
            let mut num_input_updates = 0;
            let mut num_input_values = 0;
            let mut num_output_values = 0;

            let num_tx = params.input_queue_num_batches * params.input_queue_batch_size;
            let mut first_value = [0u8; 32];
            for tx in 0..num_tx {
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();

                println!("Input insert -----------------------------");
                let mut rnd_address = get_rnd_bytes(&mut rng);
                rnd_address[0] = 0;

                let pre_batches: ManuallyDrop<BoundedVec<Batch>> =
                    merkle_tree_zero_copy_account.batches.clone();
                let pre_account = merkle_tree_zero_copy_account.get_account().clone();
                let pre_roots = merkle_tree_zero_copy_account
                    .root_history
                    .iter()
                    .cloned()
                    .collect();
                let pre_hashchains = merkle_tree_zero_copy_account.hashchain_store.clone();

                merkle_tree_zero_copy_account
                    .insert_address_into_current_batch(&rnd_address)
                    .unwrap();
                assert_input_queue_insert(
                    pre_account,
                    pre_batches,
                    &mut vec![],
                    pre_roots,
                    pre_hashchains,
                    merkle_tree_zero_copy_account,
                    vec![rnd_address],
                    vec![rnd_address],
                    vec![true],
                    vec![],
                )
                .unwrap();
                mock_indexer.queue_leaves.push(rnd_address);

                // Insert the same value twice
                {
                    // copy data so that failing test doesn't affect the state of
                    // subsequent tests
                    let mut mt_account_data = mt_account_data.clone();
                    let mut merkle_tree_zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                            &mut mt_account_data,
                        )
                        .unwrap();
                    let result = merkle_tree_zero_copy_account
                        .insert_address_into_current_batch(&rnd_address);
                    result.unwrap_err();
                    // assert_eq!(
                    //     result.unwrap_err(),
                    //     AccountCompressionErrorCode::BatchInsertFailed.into()
                    // );
                }
                // Try to insert first value into any batch
                if tx == 0 {
                    first_value = rnd_address;
                } else {
                    let mut mt_account_data = mt_account_data.clone();
                    let mut merkle_tree_zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                            &mut mt_account_data,
                        )
                        .unwrap();

                    let result = merkle_tree_zero_copy_account.insert_address_into_current_batch(
                        &first_value.to_vec().try_into().unwrap(),
                    );
                    // assert_eq!(
                    //     result.unwrap_err(),
                    //     AccountCompressionErrorCode::BatchInsertFailed.into()
                    // );
                    result.unwrap_err();
                    // assert_eq!(result.unwrap_err(), BloomFilterError::Full.into());
                }
            }
            // Assert input queue is full and doesn't accept more inserts
            {
                let merkle_tree_zero_copy_account =
                    &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                let rnd_bytes = get_rnd_bytes(&mut rng);
                let result =
                    merkle_tree_zero_copy_account.insert_address_into_current_batch(&rnd_bytes);
                assert_eq!(
                    result.unwrap_err(),
                    AccountCompressionErrorCode::BatchNotReady.into()
                );
            }
            // Root of the final batch of first input queue batch
            let mut first_input_batch_update_root_value = [0u8; 32];
            let num_updates = params.input_queue_batch_size / params.input_queue_zkp_batch_size
                * params.input_queue_num_batches;
            for i in 0..num_updates {
                println!("address update ----------------------------- {}", i);
                perform_address_update(&mut mt_account_data, &mut mock_indexer, false, mt_pubkey)
                    .await;
                if i == 4 {
                    first_input_batch_update_root_value = mock_indexer.merkle_tree.root();
                }
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                let batch = merkle_tree_zero_copy_account.batches.get(0).unwrap();
                let batch_one = merkle_tree_zero_copy_account.batches.get(1).unwrap();
                assert!(!batch_one.bloom_filter_is_wiped);

                if i >= 4 {
                    assert!(batch.bloom_filter_is_wiped);
                } else {
                    assert!(!batch.bloom_filter_is_wiped);
                }
            }
            // assert all bloom_filters are inserted
            {
                let merkle_tree_zero_copy_account =
                    &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                for (i, batch) in merkle_tree_zero_copy_account.batches.iter().enumerate() {
                    assert_eq!(batch.get_state(), BatchState::Inserted);
                    if i == 0 {
                        assert!(batch.bloom_filter_is_wiped);
                    } else {
                        assert!(!batch.bloom_filter_is_wiped);
                    }
                }
            }
            // do one insert and expect that roots until  merkle_tree_zero_copy_account.batches[0].root_index are zero
            {
                let merkle_tree_zero_copy_account =
                    &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap();
                println!(
                    "root history {:?}",
                    merkle_tree_zero_copy_account.root_history
                );
                let pre_batch_zero = merkle_tree_zero_copy_account
                    .batches
                    .get(0)
                    .unwrap()
                    .clone();

                // let mut address = get_rnd_bytes(&mut rng);
                // address[0] = 0;
                // merkle_tree_zero_copy_account.insert_address_into_current_batch(&address);
                // {
                //     let post_batch = merkle_tree_zero_copy_account
                //         .batches
                //         .get(0)
                //         .unwrap()
                //         .clone();
                //     assert_eq!(post_batch.get_state(), BatchState::CanBeFilled);
                //     assert_eq!(post_batch.get_num_inserted(), 1);
                //     let mut bloom_filter_store = merkle_tree_zero_copy_account
                //         .bloom_filter_stores
                //         .get_mut(0)
                //         .unwrap();
                //     let mut bloom_filter = BloomFilter::new(
                //         params.bloom_filter_num_iters as usize,
                //         params.bloom_filter_capacity,
                //         bloom_filter_store.as_mut_slice(),
                //     )
                //     .unwrap();
                //     assert!(bloom_filter.contains(&address));
                // }

                let root_history_len = merkle_tree_zero_copy_account
                    .get_account()
                    .root_history_capacity;
                for root in merkle_tree_zero_copy_account.root_history.iter() {
                    println!("root {:?}", root);
                }
                println!(
                    "root in root index {:?}",
                    merkle_tree_zero_copy_account.root_history[pre_batch_zero.root_index as usize]
                );
                // check that all roots have been overwritten except the root index
                // of the update
                let root_history_len: u32 = merkle_tree_zero_copy_account.root_history.len() as u32;
                let start = merkle_tree_zero_copy_account.root_history.last_index() as u32;
                println!("start {:?}", start);
                for root in start + 1..pre_batch_zero.root_index + root_history_len {
                    println!("actual index {:?}", root);
                    let index = root % root_history_len;

                    if index == pre_batch_zero.root_index {
                        let root_index = pre_batch_zero.root_index as usize;

                        assert_eq!(
                            merkle_tree_zero_copy_account.root_history[root_index],
                            first_input_batch_update_root_value
                        );
                        assert_eq!(
                            merkle_tree_zero_copy_account.root_history[root_index - 1],
                            [0u8; 32]
                        );
                        break;
                    }
                    println!("index {:?}", index);
                    assert_eq!(
                        merkle_tree_zero_copy_account.root_history[index as usize],
                        [0u8; 32]
                    );
                }
            }
        }
    }
    // TODO: add test that we cannot insert a batch that is not ready
}
