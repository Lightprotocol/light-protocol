use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, merkle_tree_metadata::BatchedMerkleTreeMetadata,
    queue::BatchedQueueAccount,
};
use light_compressed_account::{pubkey::Pubkey, QueueType, TreeType};
use light_merkle_tree_metadata::{merkle_tree::MerkleTreeMetadata, queue::QueueMetadata};

/// Builder for creating valid and invalid BatchedMerkleTreeAccount test data.
pub struct MerkleTreeAccountBuilder {
    tree_type: TreeType,
    batch_size: u64,
    zkp_batch_size: u64,
    root_history_capacity: u32,
    height: u32,
    num_iters: u64,
    bloom_filter_capacity: u64,
}

impl MerkleTreeAccountBuilder {
    /// Create a state tree builder with default test parameters.
    pub fn state_tree() -> Self {
        Self {
            tree_type: TreeType::StateV2,
            batch_size: 5,
            zkp_batch_size: 1,
            root_history_capacity: 10,
            height: 40,
            num_iters: 1,
            bloom_filter_capacity: 8000,
        }
    }

    pub fn with_tree_type(mut self, tree_type: TreeType) -> Self {
        self.tree_type = tree_type;
        self
    }

    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_zkp_batch_size(mut self, zkp_batch_size: u64) -> Self {
        self.zkp_batch_size = zkp_batch_size;
        self
    }

    pub fn with_root_history_capacity(mut self, capacity: u32) -> Self {
        self.root_history_capacity = capacity;
        self
    }

    pub fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    pub fn with_bloom_filter_capacity(mut self, capacity: u64) -> Self {
        self.bloom_filter_capacity = capacity;
        self
    }

    pub fn with_num_iters(mut self, num_iters: u64) -> Self {
        self.num_iters = num_iters;
        self
    }

    /// Pre-calculate the exact account size needed for these parameters.
    fn calculate_size(&self) -> usize {
        let mut temp_metadata = BatchedMerkleTreeMetadata::default();
        temp_metadata.root_history_capacity = self.root_history_capacity;
        temp_metadata.height = self.height;
        temp_metadata.tree_type = self.tree_type as u64;
        temp_metadata.capacity = 2u64.pow(self.height);
        temp_metadata
            .queue_batches
            .init(self.batch_size, self.zkp_batch_size)
            .unwrap();
        temp_metadata.queue_batches.bloom_filter_capacity = self.bloom_filter_capacity;
        temp_metadata.get_account_size().unwrap()
    }

    /// Build a valid account with correctly initialized data.
    pub fn build(self) -> (Vec<u8>, Pubkey) {
        let pubkey = Pubkey::new_unique();
        let size = self.calculate_size();
        let mut data = vec![0u8; size];
        BatchedMerkleTreeAccount::init(
            &mut data,
            &pubkey,
            MerkleTreeMetadata::default(),
            self.root_history_capacity,
            self.batch_size,
            self.zkp_batch_size,
            self.height,
            self.num_iters,
            self.bloom_filter_capacity,
            self.tree_type,
        )
        .unwrap();
        (data, pubkey)
    }

    /// Build account with corrupted discriminator.
    pub fn build_with_bad_discriminator(self) -> (Vec<u8>, Pubkey) {
        let (mut data, pubkey) = self.build();
        data[0..8].copy_from_slice(b"BadDiscr");
        (data, pubkey)
    }

    /// Build account with wrong tree type field (but correct discriminator).
    pub fn build_with_wrong_tree_type(self, wrong_type: u64) -> (Vec<u8>, Pubkey) {
        let (mut data, pubkey) = self.build();
        // tree_type is the first field of BatchedMerkleTreeMetadata, right after discriminator
        let tree_type_offset = 8; // 8 bytes discriminator
        data[tree_type_offset..tree_type_offset + 8].copy_from_slice(&wrong_type.to_le_bytes());
        (data, pubkey)
    }
}

/// Builder for creating valid and invalid BatchedQueueAccount test data.
pub struct QueueAccountBuilder {
    associated_merkle_tree: Pubkey,
    batch_size: u64,
    zkp_batch_size: u64,
    tree_capacity: u64,
}

impl QueueAccountBuilder {
    /// Create an output queue builder with default test parameters.
    pub fn output_queue() -> Self {
        Self {
            associated_merkle_tree: Pubkey::new_unique(),
            batch_size: 4,
            zkp_batch_size: 2,
            tree_capacity: 16,
        }
    }

    pub fn with_associated_tree(mut self, tree_pubkey: Pubkey) -> Self {
        self.associated_merkle_tree = tree_pubkey;
        self
    }

    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_zkp_batch_size(mut self, zkp_batch_size: u64) -> Self {
        self.zkp_batch_size = zkp_batch_size;
        self
    }

    pub fn with_tree_capacity(mut self, tree_capacity: u64) -> Self {
        self.tree_capacity = tree_capacity;
        self
    }

    /// Pre-calculate exact account size using a temporary metadata struct.
    fn calculate_size(&self) -> usize {
        use light_batched_merkle_tree::queue_batch_metadata::QueueBatches;
        let mut temp_batches = QueueBatches::default();
        temp_batches
            .init(self.batch_size, self.zkp_batch_size)
            .unwrap();
        // queue_account_size already includes BatchedQueueMetadata::LEN
        // which contains discriminator via aligned_sized(anchor)
        temp_batches
            .queue_account_size(QueueType::OutputStateV2 as u64)
            .unwrap()
    }

    /// Build a valid queue account with correctly initialized data.
    pub fn build(self) -> (Vec<u8>, Pubkey) {
        let pubkey = Pubkey::new_unique();
        let queue_metadata = QueueMetadata {
            associated_merkle_tree: self.associated_merkle_tree,
            queue_type: QueueType::OutputStateV2 as u64,
            ..Default::default()
        };

        let size = self.calculate_size();
        let mut data = vec![0u8; size];
        BatchedQueueAccount::init(
            &mut data,
            queue_metadata,
            self.batch_size,
            self.zkp_batch_size,
            0, // num_iters (output queues don't use bloom filters)
            0, // bloom_filter_capacity
            pubkey,
            self.tree_capacity,
        )
        .unwrap();
        (data, pubkey)
    }

    /// Build account with corrupted discriminator.
    pub fn build_with_bad_discriminator(self) -> (Vec<u8>, Pubkey) {
        let (mut data, pubkey) = self.build();
        data[0..8].copy_from_slice(b"BadQueue");
        (data, pubkey)
    }

    /// Build account with wrong queue type field (but correct discriminator).
    pub fn build_with_wrong_queue_type(self, wrong_type: u64) -> (Vec<u8>, Pubkey) {
        let (mut data, pubkey) = self.build();
        // In BatchedQueueMetadata, metadata is QueueMetadata which has:
        // AccessMetadata (3 pubkeys = 96 bytes) + RolloverMetadata (7*u64 = 56 bytes) +
        // associated_merkle_tree (32 bytes) + next_queue (32 bytes) + queue_type (8 bytes)
        // Total offset from start of metadata = 96 + 56 + 32 + 32 = 216
        // Plus 8 for discriminator = 224
        let queue_type_offset = 8 + 96 + 56 + 32 + 32;
        data[queue_type_offset..queue_type_offset + 8].copy_from_slice(&wrong_type.to_le_bytes());
        (data, pubkey)
    }

    /// Build account with insufficient size (truncated).
    pub fn build_too_small(self) -> (Vec<u8>, Pubkey) {
        let (data, pubkey) = self.build();
        (data[..data.len() / 2].to_vec(), pubkey)
    }
}
