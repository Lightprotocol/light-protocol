use aligned_sized::aligned_sized;
use light_compressed_account::{hash_to_bn254_field_size_be, pubkey::Pubkey, QueueType, TreeType};
use light_merkle_tree_metadata::{
    access::AccessMetadata, fee::compute_rollover_fee, merkle_tree::MerkleTreeMetadata,
    rollover::RolloverMetadata,
};
use light_zero_copy::cyclic_vec::ZeroCopyCyclicVecU64;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{
    constants::{
        DEFAULT_BATCH_ROOT_HISTORY_LEN, DEFAULT_BATCH_STATE_TREE_HEIGHT, NUM_BATCHES,
        TEST_DEFAULT_BATCH_SIZE, TEST_DEFAULT_ZKP_BATCH_SIZE,
    },
    errors::BatchedMerkleTreeError,
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    queue_batch_metadata::QueueBatches,
    BorshDeserialize, BorshSerialize,
};

#[repr(C)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    PartialEq,
    Clone,
    Copy,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
)]
#[aligned_sized(anchor)]
pub struct BatchedMerkleTreeMetadata {
    pub tree_type: u64,
    pub metadata: MerkleTreeMetadata,
    pub sequence_number: u64,
    pub next_index: u64,
    pub height: u32,
    pub root_history_capacity: u32,
    pub capacity: u64,
    pub queue_batches: QueueBatches,
    /// Hashed and truncated (big endian, 31 bytes
    /// + 1 byte padding) Merkle tree pubkey.
    pub hashed_pubkey: [u8; 32],
    pub nullifier_next_index: u64,
    pub placeholder_bytes: [u8; 128],
}

impl Default for BatchedMerkleTreeMetadata {
    fn default() -> Self {
        BatchedMerkleTreeMetadata {
            metadata: MerkleTreeMetadata::default(),
            next_index: 0,
            sequence_number: 0,
            tree_type: TreeType::StateV2 as u64,
            height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            root_history_capacity: DEFAULT_BATCH_ROOT_HISTORY_LEN,
            capacity: 2u64.pow(DEFAULT_BATCH_STATE_TREE_HEIGHT),
            queue_batches: QueueBatches {
                currently_processing_batch_index: 0,
                num_batches: NUM_BATCHES as u64,
                batch_size: TEST_DEFAULT_BATCH_SIZE,
                bloom_filter_capacity: 20_000 * 8,
                zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
                ..Default::default()
            },
            hashed_pubkey: [0u8; 32],
            nullifier_next_index: 0,
            placeholder_bytes: [0u8; 128],
        }
    }
}

impl BatchedMerkleTreeMetadata {
    pub fn get_account_size(&self) -> Result<usize, BatchedMerkleTreeError> {
        let metadata_size = Self::LEN;
        let root_history_size = ZeroCopyCyclicVecU64::<[u8; 32]>::required_size_for_capacity(
            self.root_history_capacity as u64,
        );
        let size = metadata_size
            + root_history_size
            + self
                .queue_batches
                .queue_account_size(QueueType::InputStateV2 as u64)?;
        Ok(size)
    }

    pub fn new_state_tree(params: CreateTreeParams, associated_queue: Pubkey) -> Self {
        Self::new_tree(TreeType::StateV2, params, associated_queue, 0)
    }

    pub fn new_address_tree(params: CreateTreeParams, rent: u64) -> Self {
        let rollover_fee = match params.rollover_threshold {
            Some(rollover_threshold) => {
                compute_rollover_fee(rollover_threshold, params.height, rent).unwrap()
            }
            None => 0,
        };
        let mut tree = Self::new_tree(TreeType::AddressV2, params, Pubkey::default(), rollover_fee);
        // inited address tree contains two elements.
        tree.next_index = 1;
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
            num_iters,
            tree_pubkey,
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
            queue_batches: QueueBatches::new_input_queue(
                batch_size,
                bloom_filter_capacity,
                zkp_batch_size,
                num_iters,
                if tree_type == TreeType::AddressV2 {
                    1
                } else {
                    0
                },
            )
            .unwrap(),
            capacity: 2u64.pow(height),
            hashed_pubkey: hash_to_bn254_field_size_be(&tree_pubkey.to_bytes()),
            nullifier_next_index: 0,
            placeholder_bytes: [0u8; 128],
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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
    pub num_iters: u64,
    pub tree_pubkey: Pubkey,
}
impl CreateTreeParams {
    pub fn from_state_ix_params(
        data: InitStateTreeAccountsInstructionData,
        owner: Pubkey,
        tree_pubkey: Pubkey,
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
            num_iters: data.bloom_filter_num_iters,
            tree_pubkey,
        }
    }

    pub fn from_address_ix_params(
        data: InitAddressTreeAccountsInstructionData,
        owner: Pubkey,
        tree_pubkey: Pubkey,
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
            num_iters: data.bloom_filter_num_iters,
            tree_pubkey,
        }
    }
}
