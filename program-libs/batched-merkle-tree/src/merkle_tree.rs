use std::ops::{Deref, DerefMut};

use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_hasher::{Discriminator, Hasher, Poseidon};
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    errors::MerkleTreeMetadataError,
    merkle_tree::{MerkleTreeMetadata, TreeType},
    queue::QueueType,
    rollover::RolloverMetadata,
};
use light_utils::{
    account::{check_account_info_mut, check_discriminator, set_discriminator, DISCRIMINATOR_LEN},
    fee::compute_rollover_fee,
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
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref};

use super::{
    batch::Batch,
    queue::{init_queue, input_queue_bytes, insert_into_current_batch, queue_account_size},
};
use crate::{
    batch::BatchState,
    batch_metadata::BatchMetadata,
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, ADDRESS_TREE_INIT_ROOT_40, BATCHED_ADDRESS_TREE_TYPE,
        BATCHED_STATE_TREE_TYPE, DEFAULT_BATCH_STATE_TREE_HEIGHT, TEST_DEFAULT_BATCH_SIZE,
    },
    errors::BatchedMerkleTreeError,
    event::{BatchAppendEvent, BatchNullifyEvent},
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    queue::BatchedQueueAccount,
    BorshDeserialize, BorshSerialize,
};

#[repr(C)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    PartialEq,
    Pod,
    Zeroable,
    Clone,
    Copy,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
)]
#[aligned_sized(anchor)]
pub struct BatchedMerkleTreeMetadata {
    pub metadata: MerkleTreeMetadata,
    pub sequence_number: u64,
    pub tree_type: u64,
    pub next_index: u64,
    pub height: u32,
    pub root_history_capacity: u32,
    pub queue_metadata: BatchMetadata,
}

// TODO: make anchor consistent
impl Discriminator for BatchedMerkleTreeAccount<'_> {
    const DISCRIMINATOR: [u8; 8] = *b"BatchMka";
}

impl Default for BatchedMerkleTreeMetadata {
    fn default() -> Self {
        BatchedMerkleTreeMetadata {
            metadata: MerkleTreeMetadata::default(),
            next_index: 0,
            sequence_number: 0,
            tree_type: TreeType::BatchedState as u64,
            height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            root_history_capacity: 20,
            queue_metadata: BatchMetadata {
                currently_processing_batch_index: 0,
                num_batches: 2,
                batch_size: TEST_DEFAULT_BATCH_SIZE,
                bloom_filter_capacity: 20_000 * 8,
                zkp_batch_size: 10,
                ..Default::default()
            },
        }
    }
}

#[repr(C)]
pub struct CreateTreeParams {
    pub owner: Pubkey,
    pub program_owner: Option<Pubkey>,
    pub forester: Option<Pubkey>,
    pub rollover_threshold: Option<u64>,
    pub index: u64,
    pub network_fee: u64,
    pub batch_size: u64,
    pub zkp_batch_size: u64,
    pub bloom_filter_capacity: u64,
    pub root_history_capacity: u32,
    pub height: u32,
    pub num_batches: u64,
}
impl CreateTreeParams {
    pub fn from_state_ix_params(data: InitStateTreeAccountsInstructionData, owner: Pubkey) -> Self {
        CreateTreeParams {
            owner,
            program_owner: data.program_owner,
            forester: data.forester,
            rollover_threshold: data.rollover_threshold,
            index: data.index,
            network_fee: data.network_fee.unwrap_or(0),
            batch_size: data.input_queue_batch_size,
            zkp_batch_size: data.input_queue_zkp_batch_size,
            bloom_filter_capacity: data.bloom_filter_capacity,
            root_history_capacity: data.root_history_capacity,
            height: data.height,
            num_batches: data.input_queue_num_batches,
        }
    }

    pub fn from_address_ix_params(
        data: InitAddressTreeAccountsInstructionData,
        owner: Pubkey,
    ) -> Self {
        CreateTreeParams {
            owner,
            program_owner: data.program_owner,
            forester: data.forester,
            rollover_threshold: data.rollover_threshold,
            index: data.index,
            network_fee: data.network_fee.unwrap_or(0),
            batch_size: data.input_queue_batch_size,
            zkp_batch_size: data.input_queue_zkp_batch_size,
            bloom_filter_capacity: data.bloom_filter_capacity,
            root_history_capacity: data.root_history_capacity,
            height: data.height,
            num_batches: data.input_queue_num_batches,
        }
    }
}

impl BatchedMerkleTreeMetadata {
    pub fn get_account_size(&self) -> Result<usize, BatchedMerkleTreeError> {
        let account_size = Self::LEN;
        let root_history_size = ZeroCopyCyclicVecU64::<[u8; 32]>::required_size_for_capacity(
            self.root_history_capacity as u64,
        );
        let size = account_size
            + root_history_size
            + queue_account_size(&self.queue_metadata, QueueType::Input as u64)?;
        Ok(size)
    }

    pub fn new_state_tree(params: CreateTreeParams, associated_queue: Pubkey) -> Self {
        Self::new_tree(TreeType::BatchedState, params, associated_queue, 0)
    }

    pub fn new_address_tree(params: CreateTreeParams, rent: u64) -> Self {
        let rollover_fee = match params.rollover_threshold {
            Some(rollover_threshold) => {
                compute_rollover_fee(rollover_threshold, params.height, rent).unwrap()
            }
            None => 0,
        };
        let mut tree = Self::new_tree(
            TreeType::BatchedAddress,
            params,
            Pubkey::default(),
            rollover_fee,
        );
        // inited address tree contains two elements.
        tree.next_index = 2;
        tree
    }

    fn new_tree(
        tree_type: TreeType,
        params: CreateTreeParams,
        associated_queue: Pubkey,
        rollover_fee: u64,
    ) -> Self {
        let CreateTreeParams {
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
            height,
            num_batches,
        } = params;
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
            queue_metadata: BatchMetadata::new_input_queue(
                batch_size,
                bloom_filter_capacity,
                zkp_batch_size,
                num_batches,
            ),
        }
    }
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct BatchedMerkleTreeAccount<'a> {
    metadata: Ref<&'a mut [u8], BatchedMerkleTreeMetadata>,
    pub root_history: ZeroCopyCyclicVecU64<'a, [u8; 32]>,
    pub batches: ZeroCopySliceMutU64<'a, Batch>,
    pub value_vecs: Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
    pub bloom_filter_stores: Vec<ZeroCopySliceMutU64<'a, u8>>,
    pub hashchain_store: Vec<ZeroCopyVecU64<'a, [u8; 32]>>,
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

/// Get batch from account.
/// Hash all public inputs into one poseidon hash.
/// Public inputs:
/// 1. old root (get from account by index)
/// 2. new root (send to chain and )
/// 3. start index (get from batch)
/// 4. end index (get from batch start index plus batch size)
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataBatchNullifyInputs {
    pub public_inputs: BatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct BatchProofInputsIx {
    pub new_root: [u8; 32],
    pub old_root_index: u16,
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataBatchAppendInputs {
    pub public_inputs: AppendBatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct AppendBatchProofInputsIx {
    pub new_root: [u8; 32],
}

impl<'a> BatchedMerkleTreeAccount<'a> {
    // TODO: remove
    pub fn get_metadata(&self) -> &BatchedMerkleTreeMetadata {
        &self.metadata
    }
    // TODO: remove
    pub fn get_metadata_mut(&mut self) -> &mut BatchedMerkleTreeMetadata {
        &mut self.metadata
    }

    // // TODO: add unit test
    pub fn state_tree_from_account_info_mut(
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_account_info_mut::<BATCHED_STATE_TREE_TYPE>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    // TODO: add failing test
    #[cfg(not(target_os = "solana"))]
    pub fn state_tree_from_bytes_mut(
        account_data: &'a mut [u8],
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_bytes_mut::<BATCHED_STATE_TREE_TYPE>(account_data)
    }

    pub fn address_tree_from_account_info_mut(
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_account_info_mut::<BATCHED_ADDRESS_TREE_TYPE>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    pub fn from_account_info_mut<const TREE_TYPE: u64>(
        program_id: &solana_program::pubkey::Pubkey,
        account_info: &AccountInfo<'a>,
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        check_account_info_mut::<Self>(program_id, account_info)?;
        let mut data = account_info.try_borrow_mut_data()?;

        // Necessary to convince the borrow checker.
        let data_slice: &'a mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
        Self::from_bytes_mut::<TREE_TYPE>(data_slice)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn address_tree_from_bytes_mut(
        account_data: &'a mut [u8],
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        Self::from_bytes_mut::<BATCHED_ADDRESS_TREE_TYPE>(account_data)
    }

    fn from_bytes_mut<const TREE_TYPE: u64>(
        account_data: &'a mut [u8],
    ) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
        let account_data_len = account_data.len();
        let (discriminator, account_data) = account_data.split_at_mut(DISCRIMINATOR_LEN);
        check_discriminator::<Self>(discriminator)?;
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
        let (batches, value_vecs, bloom_filter_stores, hashchain_store) = input_queue_bytes(
            &metadata.queue_metadata,
            account_data,
            QueueType::Input as u64,
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
        if tree_type == TreeType::BatchedState {
            root_history.push(light_hasher::Poseidon::zero_bytes()[height as usize]);
        } else if tree_type == TreeType::BatchedAddress {
            // Initialized indexed Merkle tree root
            root_history.push(ADDRESS_TREE_INIT_ROOT_40);
            account_metadata.next_index = 2;
        }
        let (batches, value_vecs, bloom_filter_stores, hashchain_store) = init_queue(
            &account_metadata.queue_metadata,
            QueueType::Input as u64,
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

    pub fn update_output_queue_account_info(
        &mut self,
        queue_account_info: &AccountInfo<'_>,
        instruction_data: InstructionDataBatchAppendInputs,
        id: [u8; 32],
    ) -> Result<BatchAppendEvent, BatchedMerkleTreeError> {
        if self.get_metadata().metadata.associated_queue != (*queue_account_info.key).into() {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into());
        }
        let queue_account =
            &mut BatchedQueueAccount::output_queue_from_account_info_mut(queue_account_info)?;
        self.update_output_queue_account(queue_account, instruction_data, id)
    }

    // Note: when proving inclusion by index in
    // value array we need to insert the value into a bloom_filter once it is
    // inserted into the tree. Check this with get_num_inserted_zkps
    pub fn update_output_queue_account(
        &mut self,
        queue_account: &mut BatchedQueueAccount,
        instruction_data: InstructionDataBatchAppendInputs,
        id: [u8; 32],
    ) -> Result<BatchAppendEvent, BatchedMerkleTreeError> {
        let batch_index = queue_account.batch_metadata.next_full_batch_index;
        let circuit_batch_size = queue_account.get_metadata().batch_metadata.zkp_batch_size;
        let batches = &mut queue_account.batches;
        let full_batch = batches
            .get_mut(batch_index as usize)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;

        let new_root = instruction_data.public_inputs.new_root;
        let num_zkps = full_batch.get_first_ready_zkp_batch()?;

        let leaves_hashchain = queue_account
            .hashchain_store
            .get(batch_index as usize)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?
            .get(num_zkps as usize)
            .ok_or(BatchedMerkleTreeError::InvalidIndex)?;
        let old_root = self
            .root_history
            .last()
            .ok_or(BatchedMerkleTreeError::InvalidIndex)?;
        let start_index = self.get_metadata().next_index;
        let mut start_index_bytes = [0u8; 32];
        start_index_bytes[24..].copy_from_slice(&start_index.to_be_bytes());
        let public_input_hash = create_hash_chain_from_array([
            *old_root,
            new_root,
            *leaves_hashchain,
            start_index_bytes,
        ])?;

        self.update::<5>(
            circuit_batch_size as usize,
            instruction_data.compressed_proof,
            public_input_hash,
        )?;
        let account = self.get_metadata_mut();
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
            queue_account
                .get_metadata_mut()
                .batch_metadata
                .next_full_batch_index += 1;
            queue_account
                .get_metadata_mut()
                .batch_metadata
                .next_full_batch_index %=
                queue_account.get_metadata_mut().batch_metadata.num_batches;
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
            sequence_number: self.get_metadata().sequence_number,
        })
    }

    pub fn update_input_queue(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent, BatchedMerkleTreeError> {
        self.private_update_input_queue::<3>(instruction_data, id)
    }

    pub fn update_address_queue(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent, BatchedMerkleTreeError> {
        self.private_update_input_queue::<4>(instruction_data, id)
    }

    fn private_update_input_queue<const QUEUE_TYPE: u64>(
        &mut self,
        instruction_data: InstructionDataBatchNullifyInputs,
        id: [u8; 32],
    ) -> Result<BatchNullifyEvent, BatchedMerkleTreeError> {
        let batch_index = self.get_metadata().queue_metadata.next_full_batch_index;

        let full_batch = self
            .batches
            .get(batch_index as usize)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;

        let num_zkps = full_batch.get_first_ready_zkp_batch()?;

        let leaves_hashchain = self
            .hashchain_store
            .get(batch_index as usize)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?
            .get(num_zkps as usize)
            .ok_or(BatchedMerkleTreeError::InvalidIndex)?;
        let old_root = self
            .root_history
            .get(instruction_data.public_inputs.old_root_index as usize)
            .ok_or(BatchedMerkleTreeError::InvalidIndex)?;
        let new_root = instruction_data.public_inputs.new_root;

        let public_input_hash = if QUEUE_TYPE == QueueType::Input as u64 {
            create_hash_chain_from_array([*old_root, new_root, *leaves_hashchain])?
        } else if QUEUE_TYPE == QueueType::Address as u64 {
            let mut next_index_bytes = [0u8; 32];
            next_index_bytes[24..]
                .copy_from_slice(self.get_metadata().next_index.to_be_bytes().as_slice());
            create_hash_chain_from_array([
                *old_root,
                new_root,
                *leaves_hashchain,
                next_index_bytes,
            ])?
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType.into());
        };
        let circuit_batch_size = self.get_metadata().queue_metadata.zkp_batch_size;
        self.update::<QUEUE_TYPE>(
            circuit_batch_size as usize,
            instruction_data.compressed_proof,
            public_input_hash,
        )?;
        self.root_history.push(new_root);

        let root_history_capacity = self.get_metadata().root_history_capacity;
        let sequence_number = self.get_metadata().sequence_number;
        let full_batch = self
            .batches
            .get_mut(batch_index as usize)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;
        full_batch.mark_as_inserted_in_merkle_tree(
            sequence_number,
            self.root_history.last_index() as u32,
            root_history_capacity,
        )?;
        if full_batch.get_state() == BatchState::Inserted {
            let account = self.get_metadata_mut();
            account.queue_metadata.next_full_batch_index += 1;
            account.queue_metadata.next_full_batch_index %= account.queue_metadata.num_batches;
        }
        if QUEUE_TYPE == QueueType::Address as u64 {
            self.get_metadata_mut().next_index += circuit_batch_size;
        }

        self.wipe_previous_batch_bloom_filter()?;

        Ok(BatchNullifyEvent {
            id,
            batch_index,
            batch_size: circuit_batch_size,
            zkp_batch_index: num_zkps,
            new_root,
            root_index: self.root_history.last_index() as u32,
            sequence_number: self.get_metadata().sequence_number,
        })
    }

    fn update<const QUEUE_TYPE: u64>(
        &mut self,
        batch_size: usize,
        proof: CompressedProof,
        public_input_hash: [u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        if QUEUE_TYPE == QueueType::Output as u64 {
            verify_batch_append_with_proofs(batch_size, public_input_hash, &proof)?;
        } else if QUEUE_TYPE == QueueType::Input as u64 {
            verify_batch_update(batch_size, public_input_hash, &proof)?;
        } else if QUEUE_TYPE == QueueType::Address as u64 {
            verify_batch_address_update(batch_size, public_input_hash, &proof)?;
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType.into());
        }
        self.metadata.sequence_number += 1;
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
    ) -> Result<(), BatchedMerkleTreeError> {
        if self.get_metadata().tree_type != TreeType::BatchedState as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        let leaf_index_bytes = leaf_index.to_be_bytes();
        let nullifier = Poseidon::hashv(&[compressed_account_hash, &leaf_index_bytes, tx_hash])?;
        self.insert_into_current_batch(compressed_account_hash, &nullifier)
    }

    pub fn insert_address_into_current_batch(
        &mut self,
        address: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        if self.get_metadata().tree_type != TreeType::BatchedAddress as u64 {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }
        self.insert_into_current_batch(address, address)
    }

    fn insert_into_current_batch(
        &mut self,
        bloom_filter_value: &[u8; 32],
        leaves_hash_value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let (root_index, sequence_number) = insert_into_current_batch(
            QueueType::Input as u64,
            &mut self.metadata.queue_metadata,
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

        Ok(())
    }

    fn zero_out_roots(&mut self, sequence_number: u64, root_index: Option<u32>) {
        if sequence_number > self.get_metadata().sequence_number {
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
    pub fn wipe_previous_batch_bloom_filter(&mut self) -> Result<(), BatchedMerkleTreeError> {
        let current_batch = self
            .get_metadata()
            .queue_metadata
            .currently_processing_batch_index;
        let batch_size = self.get_metadata().queue_metadata.batch_size;
        let previous_full_batch_index = self
            .get_metadata()
            .queue_metadata
            .next_full_batch_index
            .saturating_sub(1) as usize;
        let num_inserted_elements = self
            .batches
            .get(current_batch as usize)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?
            .get_num_inserted_elements();
        let previous_full_batch = self
            .batches
            .get_mut(previous_full_batch_index)
            .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;
        if previous_full_batch.get_state() == BatchState::Inserted
            && batch_size / 2 > num_inserted_elements
            && !previous_full_batch.bloom_filter_is_wiped()
        {
            let bloom_filter = self
                .bloom_filter_stores
                .get_mut(previous_full_batch_index)
                .ok_or(BatchedMerkleTreeError::InvalidBatchIndex)?;
            bloom_filter.as_mut_slice().iter_mut().for_each(|x| *x = 0);
            previous_full_batch.set_bloom_filter_is_wiped();
            let seq = previous_full_batch.sequence_number;
            let root_index = previous_full_batch.root_index;
            self.zero_out_roots(seq, Some(root_index));
        }

        Ok(())
    }

    pub fn get_root_index(&self) -> u32 {
        self.root_history.last_index() as u32
    }
    pub fn get_root(&self) -> Option<[u8; 32]> {
        self.root_history.last().copied()
    }

    // TODO: add unit test
    /// Checks non-inclusion in all bloom filters
    /// which are not wiped.
    pub fn check_input_queue_non_inclusion(
        &mut self,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        let num_bloom_filters = self.bloom_filter_stores.len();
        for i in 0..num_bloom_filters {
            let bloom_filter_store = self.bloom_filter_stores[i].as_mut_slice();
            let batch = &self.batches[i];
            if !batch.bloom_filter_is_wiped() {
                batch.check_non_inclusion(value, bloom_filter_store)?;
            }
        }
        Ok(())
    }
}

pub fn get_merkle_tree_account_size_default() -> usize {
    let mt_account = BatchedMerkleTreeMetadata::default();
    mt_account.get_account_size().unwrap()
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
    };
    mt_account.get_account_size().unwrap()
}

pub fn assert_nullify_event(
    event: BatchNullifyEvent,
    new_root: [u8; 32],
    old_account: &BatchedMerkleTreeAccount,
    mt_pubkey: Pubkey,
) {
    let batch_index = old_account
        .get_metadata()
        .queue_metadata
        .next_full_batch_index;
    let batch = old_account.batches.get(batch_index as usize).unwrap();
    let ref_event = BatchNullifyEvent {
        id: mt_pubkey.to_bytes(),
        batch_index,
        zkp_batch_index: batch.get_num_inserted_zkps(),
        new_root,
        root_index: (old_account.get_root_index() + 1)
            % old_account.get_metadata().root_history_capacity,
        sequence_number: old_account.get_metadata().sequence_number + 1,
        batch_size: old_account.get_metadata().queue_metadata.zkp_batch_size,
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
        .get_metadata()
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
        root_index: (old_account.get_root_index() + 1)
            % old_account.get_metadata().root_history_capacity,
        sequence_number: old_account.get_metadata().sequence_number + 1,
        batch_size: old_account.get_metadata().queue_metadata.zkp_batch_size,
        old_next_index: old_account.get_metadata().next_index,
        new_next_index: old_account.get_metadata().next_index
            + old_output_queue_account
                .get_metadata()
                .batch_metadata
                .zkp_batch_size,
    };
    assert_eq!(event, ref_event);
}
