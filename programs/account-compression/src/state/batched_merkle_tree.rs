use crate::{
    batch::BatchState, batched_queue::ZeroCopyBatchedQueueAccount,
    errors::AccountCompressionErrorCode,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{BoundedVec, BoundedVecMetadata, CyclicBoundedVec};
use light_hasher::{Hasher, Poseidon};
use light_verifier::CompressedProof;
use std::mem::ManuallyDrop;

use super::{
    batch::Batch,
    batched_queue::{
        batched_queue_from_account, init_queue_from_account, insert_into_current_batch,
        queue_account_size, queue_get_next_full_batch, BatchedQueue, BatchedQueueAccount,
    },
    AccessMetadata, MerkleTreeMetadata, QueueType, RolloverMetadata,
};

#[derive(Debug, PartialEq, Default)]
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

#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TreeType {
    State = 1,
    Address = 2,
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
        let account_size = std::mem::size_of::<Self>();
        let root_history_size = (std::mem::size_of::<BoundedVecMetadata>()
            + std::mem::size_of::<[u8; 32]>())
            * self.root_history_capacity as usize;
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
        bloom_filter_capacity: u64,
        associated_queue: Pubkey,
    ) -> Self {
        Self {
            metadata: MerkleTreeMetadata {
                next_merkle_tree: Pubkey::default(),
                access_metadata: AccessMetadata::new(owner, program_owner, forester),
                rollover_metadata: RolloverMetadata::new(
                    index,
                    0,
                    rollover_threshold,
                    network_fee,
                    None,
                    None,
                ),
                associated_queue,
            },
            sequence_number: 0,
            tree_type: TreeType::State as u64,
            next_index: 0,
            height: 26,
            root_history_capacity: 2,
            queue: BatchedQueue::get_input_queue_default(batch_size, bloom_filter_capacity),
        }
    }
}

// Instead of a rollover we just push roots into root history. TODO: figure out
// how we nullify in old trees. (it's probably easier to just stick to the
// current model.)
pub struct ZeroCopyBatchedMerkleTreeAccount<'a> {
    pub account: &'a mut BatchedMerkleTreeAccount,
    pub root_buffer: ManuallyDrop<CyclicBoundedVec<[u8; 32]>>,
    // TODO: add root history which is just a bounded vec of roots, with
    // intermediate updates.
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloomfilter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
}

/// Get batch from account.
/// Hash all public inputs into one poseidon hash.
/// Public inputs:
/// 1. old root (get from account by index)
/// 2. new root (send to chain and )
/// 3. start index (get from batch)
/// 4. end index (get from batch start index plus batch size)
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct InstructionDataBatchUpdateProofInputs {
    pub public_inputs: BatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

pub enum Circuit {
    Batch100,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BatchProofInputsIx {
    pub circuit_id: u16,
    pub new_root: [u8; 32],
    pub output_hash_chain: [u8; 32],
    pub root_index: u16,
}

impl BatchProofInputsIx {
    pub fn map_circuit_id(&self) -> u16 {
        match self.circuit_id {
            1 => 10,
            _ => panic!("Invalid circuit id"),
        }
    }
}

pub struct BatchProofInputs {
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
    pub start_index: u64,
    pub end_index: u64,
    pub user_hash_chain: [u8; 32],
    pub input_hash_chain: [u8; 32],
    pub output_hash_chain: [u8; 32],
}

impl<'a> ZeroCopyBatchedMerkleTreeAccount<'a> {
    // TODO: add from_account_info,  and from_account_loader
    pub fn from_account(
        account: &'a mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount<'a>> {
        if account_data.len() != account.size()? {
            return err!(AccountCompressionErrorCode::SizeMismatch);
        }
        let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>();
        let root_buffer = CyclicBoundedVec::deserialize(account_data, &mut start_offset);
        let (batches, value_vecs, bloomfilter_stores) = batched_queue_from_account(
            &mut account.queue,
            account_data,
            QueueType::Input as u64,
            &mut start_offset,
        )?;
        Ok(ZeroCopyBatchedMerkleTreeAccount {
            account,
            root_buffer,
            batches,
            value_vecs,
            bloomfilter_stores,
        })
    }

    pub fn init_from_account(
        account: &'a mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
        num_iters: u64,
        bloomfilter_capacity: u64,
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount<'a>> {
        if account_data.len() != account.size()? {
            msg!("merkle_tree_account: {:?}", account);
            msg!("account_data.len(): {}", account_data.len());
            msg!("account.size(): {}", account.size()?);
            return err!(AccountCompressionErrorCode::SizeMismatch);
        }
        let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>();

        let root_buffer = CyclicBoundedVec::init(
            account.root_history_capacity as usize,
            account_data,
            &mut start_offset,
            false,
        )
        .map_err(ProgramError::from)?;

        let (batches, value_vecs, bloomfilter_stores) = init_queue_from_account(
            &mut account.queue,
            QueueType::Input as u64,
            account_data,
            num_iters,
            bloomfilter_capacity,
            &mut start_offset,
        )?;
        Ok(ZeroCopyBatchedMerkleTreeAccount {
            account,
            root_buffer,
            batches,
            value_vecs,
            bloomfilter_stores,
        })
    }

    pub fn get_public_inputs_from_queue_account(
        next_index: u64,
        old_root: [u8; 32],
        account: &mut BatchedQueue,
        batches: &mut ManuallyDrop<BoundedVec<Batch>>,
        instruction_data: &InstructionDataBatchUpdateProofInputs,
    ) -> Result<(BatchProofInputs, u8)> {
        let batch_capacity = account.batch_size as usize;
        let (batch, batch_index) = queue_get_next_full_batch(account, batches)?;

        let public_inputs = BatchProofInputs {
            old_root,
            new_root: instruction_data.public_inputs.new_root,
            start_index: next_index,
            // TODO: relax to enable partial updates
            end_index: next_index + batch_capacity as u64,
            // * instruction_data.public_inputs.map_circuit_id() as u64,
            user_hash_chain: batch.user_hash_chain,
            input_hash_chain: batch.prover_hash_chain,
            output_hash_chain: batch.prover_hash_chain,
        };
        Ok((public_inputs, batch_index))
    }

    pub fn compress_public_inputs(&self, public_inputs: &BatchProofInputs) -> Result<[u8; 32]> {
        // compress all public inputs into one hash

        // Hash(meta_poseidon_hash, compressed_hash_chains)
        let meta_poseidon_hash = Poseidon::hashv(&[
            &public_inputs.old_root,
            &public_inputs.new_root,
            &public_inputs.start_index.to_le_bytes(),
            &public_inputs.end_index.to_le_bytes(),
        ])
        .map_err(ProgramError::from)?;

        let current_hash_chain_hash = Poseidon::hashv(&[
            &public_inputs.user_hash_chain,
            &public_inputs.input_hash_chain,
            &public_inputs.output_hash_chain,
        ])
        .map_err(ProgramError::from)?;
        // Poseidon hashes are cheaper than public inputs.
        let public_input_hash = Poseidon::hashv(&[&meta_poseidon_hash, &current_hash_chain_hash])
            .map_err(ProgramError::from)?;

        Ok(public_input_hash)
    }

    pub fn update_output_queue(
        &mut self,
        queue_account: &mut BatchedQueueAccount,
        queue_account_data: &mut [u8],
        instruction_data: InstructionDataBatchUpdateProofInputs,
    ) -> Result<()> {
        let mut queue_account =
            ZeroCopyBatchedQueueAccount::from_account(queue_account, queue_account_data).unwrap();
        let account = queue_account.account;
        let batches = &mut queue_account.batches;
        let (public_inputs, batch_index) =
            ZeroCopyBatchedMerkleTreeAccount::get_public_inputs_from_queue_account(
                self.account.next_index,
                *self.root_buffer.first().unwrap_or(&[0u8; 32]),
                &mut account.queue,
                batches,
                &instruction_data,
            )?;

        self.update(instruction_data, batch_index, Some(batches), public_inputs)?;
        Ok(())
    }

    pub fn update_input_queue(
        &mut self,
        instruction_data: InstructionDataBatchUpdateProofInputs,
    ) -> Result<()> {
        let (public_inputs, batch_index) =
            ZeroCopyBatchedMerkleTreeAccount::get_public_inputs_from_queue_account(
                self.account.next_index,
                *self.root_buffer.first().unwrap_or(&[0u8; 32]),
                &mut self.account.queue,
                &mut self.batches,
                &instruction_data,
            )?;

        self.update(instruction_data, batch_index, None, public_inputs)?;
        Ok(())
    }

    fn update(
        &mut self,
        instruction_data: InstructionDataBatchUpdateProofInputs,
        batch_index: u8,
        batches: Option<&mut ManuallyDrop<BoundedVec<Batch>>>,
        public_inputs: BatchProofInputs,
    ) -> Result<()> {
        let batches = if let Some(batches) = batches {
            batches
        } else {
            &mut self.batches
        };
        if batches[batch_index as usize].state == BatchState::Inserted {
            return err!(AccountCompressionErrorCode::BatchAlreadyInserted);
        } else if batches[batch_index as usize].state == BatchState::CanBeFilled {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        println!("batch_index: {}", batch_index);
        println!("batch : {:?}", batches[batch_index as usize]);
        batches[batch_index as usize].state = BatchState::Inserted;
        batches[batch_index as usize].user_hash_chain = [0u8; 32];
        batches[batch_index as usize].num_inserted = 0;

        let public_input_hash = self.compress_public_inputs(&public_inputs)?;
        // TODO: replace with actual verification in light-verifier
        verify_mock_circuit(&public_input_hash, &instruction_data.compressed_proof)?;
        self.root_buffer.push(public_inputs.new_root);
        self.account.next_index = public_inputs.end_index;
        self.account.sequence_number += 1;
        Ok(())
    }

    pub fn insert_into_current_batch(&mut self, value: &[u8; 32]) -> Result<()> {
        insert_into_current_batch(
            QueueType::Input as u64,
            &mut self.account.queue,
            &mut self.batches,
            &mut self.value_vecs,
            &mut self.bloomfilter_stores,
            value,
        )
    }
}

pub fn verify_mock_circuit(_public_input_hash: &[u8; 32], _proof: &CompressedProof) -> Result<()> {
    // 1. Recreate public input hash
    // end index == start index + batch_size * num_batches
    // execute nullify, indexed insert or append ciruit logic
    Ok(())
}

pub fn get_merkle_tree_account_size_default() -> usize {
    // TODO: implement a default config for BatchedMerkleTreeAccount using a
    // default for BatchedInputQueue
    let mt_account = BatchedMerkleTreeAccount {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: TreeType::State as u64,
        height: 26,
        root_history_capacity: 2,
        queue: BatchedQueue {
            currently_processing_batch_index: 0,
            num_batches: 4,
            batch_size: 5000,
            bloom_filter_capacity: 200_000 * 8,
            next_index: 0,
            ..Default::default()
        },
    };
    mt_account.size().unwrap()
}
pub fn get_merkle_tree_account_size(batch_size: u64, bloom_filter_capacity: u64) -> usize {
    // TODO: implement a default config for BatchedMerkleTreeAccount using a
    // default for BatchedInputQueue
    let mt_account = BatchedMerkleTreeAccount {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: TreeType::State as u64,
        height: 26,
        root_history_capacity: 2,
        queue: BatchedQueue {
            num_batches: 4,
            batch_size,
            bloom_filter_capacity,
            ..Default::default()
        },
    };
    mt_account.size().unwrap()
}

#[cfg(test)]
mod tests {

    use std::default;

    use light_utils::fee::compute_rollover_fee;
    use rand::{rngs::StdRng, Rng};

    use crate::{
        batch::BatchState,
        batched_queue::{
            get_output_queue_account_size, get_output_queue_account_size_default,
            tests::{assert_queue_inited, assert_queue_zero_copy_inited},
        },
        initialize_address_queue::check_rollover_fee_sufficient,
        AccessMetadata, MerkleTreeMetadata, QueueMetadata, QueueType, RolloverMetadata,
    };

    use super::*;

    fn assert_mt_zero_copy_inited(
        account: &mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
        ref_account: BatchedMerkleTreeAccount,
        num_iters: u64,
    ) {
        let queue = account.queue.clone();
        let ref_queue = ref_account.queue.clone();
        let queue_type = QueueType::Input as u64;
        let num_batches = ref_queue.num_batches as usize;

        let mut zero_copy_account =
            ZeroCopyBatchedMerkleTreeAccount::from_account(account, account_data)
                .expect("from_account failed");
        assert_eq!(*zero_copy_account.account, ref_account, "metadata mismatch");

        assert_eq!(
            zero_copy_account.root_buffer.capacity(),
            ref_account.root_history_capacity as usize,
            "root_history_capacity mismatch"
        );

        assert!(
            zero_copy_account.root_buffer.is_empty(),
            "root_buffer not empty"
        );

        assert_queue_inited(
            queue,
            ref_queue,
            queue_type,
            &mut zero_copy_account.value_vecs,
            &mut zero_copy_account.bloomfilter_stores,
            &mut zero_copy_account.batches,
            num_batches,
            num_iters,
        );
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct CreateStateTreeAccountsInstructionData {
        pub index: u64,
        pub program_owner: Option<Pubkey>,
        pub forester: Option<Pubkey>,
        pub additional_bytes: u64,
        pub bloomfilter_num_iters: u64,
        pub input_queue_batch_size: u64,
        pub output_queue_batch_size: u64,
        pub bloom_filter_capacity: u64,
        pub network_fee: Option<u64>,
        pub rollover_threshold: Option<u64>,
        pub close_threshold: Option<u64>,
    }
    impl default::Default for CreateStateTreeAccountsInstructionData {
        fn default() -> Self {
            Self {
                index: 0,
                program_owner: None,
                forester: None,
                additional_bytes: 1,
                bloomfilter_num_iters: 3,
                input_queue_batch_size: 5000,
                output_queue_batch_size: 5000,
                bloom_filter_capacity: 200_000 * 8,
                network_fee: Some(5000),
                rollover_threshold: Some(95),
                close_threshold: None,
            }
        }
    }

    pub fn create_batched_state_merkle_tree_accounts<'a>(
        owner: Pubkey,
        params: CreateStateTreeAccountsInstructionData,
        output_queue_account: &'a mut BatchedQueueAccount,
        output_queue_account_data: &mut [u8],
        output_queue_pubkey: Pubkey,
        queue_rent: u64,
        mt_account: &'a mut BatchedMerkleTreeAccount,
        mt_account_data: &mut [u8],
        mt_pubkey: Pubkey,
        merkle_tree_rent: u64,
        additional_bytes_rent: u64,
    ) -> Result<()> {
        if params.bloom_filter_capacity % 8 != 0 {
            println!(
                "params.bloom_filter_capacity: {}",
                params.bloom_filter_capacity
            );
            println!("Blooms must be divisible by 8 or it will create unaligned memory.");
            return err!(AccountCompressionErrorCode::InvalidBloomFilterCapacity);
        }
        /*
         * Note on security for length of
         * root buffer, bloomfilter and output batch arrays:
         * Example on input and output number of batches and root buffer length:
         * - 2 root buffer length
         * - 4 bloom filters in the input (nullifier) queue
         * - 2 large output batches so that these don’t switch roots too often
         *
         * Account {
         *   bloomfilter: [B0, B1, B2, B3],
         *     roots: [R0, R1],
         * }
         *
         * Timeslot 0:
         * - insert into B0 until full
         *
         * Timeslot 1:
         * - insert into B1 until full
         * - update tree with B0, don't clear B0 yet
         * -> R0 -> B0
         *
         * Timeslot 2:
         * - insert into B2 until full
         * - update tree with B1, don't clear B1 yet
         * -> R0 -> B0
         * -> R1 -> B1
         *
         * Timeslot 3:
         * -> R0 -> B0
         * -> R1 -> B1
         * - clear B3
         * - insert into B3 until full
         * - update tree with B2, don't clear B2 yet
         * -> R0 -> B2 (B0 is save to clear now)
         *
         * Timeslot 4:
         * -> R0 -> B2
         * -> R1 -> B1
         * - clear B0
         * - insert into B0 until full
         * - update tree with B3, don't clear B3 yet
         * -> R1 -> B3 (B1 is save to clear now)
         *
         * Timeslot 5:
         * -> R0 -> B2
         * -> R1 -> B3
         * - clear B1
         * - insert into B1 until full
         * - update tree with B0, don't clear B0 yet
         * -> R0 -> B0 (B2 is save to clear now)
         */
        let num_batches_input_queue = 4;
        let num_batches_output_queue = 2;
        let height = 26;
        let root_history_capacity = 2;

        // Output queue
        {
            let rollover_fee = match params.rollover_threshold {
                Some(rollover_threshold) => {
                    let rent = merkle_tree_rent + additional_bytes_rent + queue_rent;
                    let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)
                        .map_err(ProgramError::from)?;
                    check_rollover_fee_sufficient(
                        rollover_fee,
                        0,
                        rent,
                        rollover_threshold,
                        height,
                    )?;
                    rollover_fee
                }
                None => 0,
            };
            let metadata = QueueMetadata {
                next_queue: Pubkey::default(),
                access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
                rollover_metadata: RolloverMetadata::new(
                    params.index,
                    rollover_fee,
                    params.rollover_threshold,
                    params.network_fee.unwrap_or_default(),
                    params.close_threshold,
                    Some(params.additional_bytes),
                ),
                queue_type: QueueType::Output as u64,
                associated_merkle_tree: mt_pubkey,
            };
            output_queue_account.init(
                metadata,
                num_batches_output_queue,
                params.output_queue_batch_size,
            );
            ZeroCopyBatchedQueueAccount::init_from_account(
                output_queue_account,
                output_queue_account_data,
                0,
                0,
            )?;
        }
        let metadata = MerkleTreeMetadata {
            next_merkle_tree: Pubkey::default(),
            access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
            rollover_metadata: RolloverMetadata::new(
                params.index,
                // Complete rollover fee is charged when creating an output
                // compressed account by inserting it into the output queue.
                0,
                params.rollover_threshold,
                params.network_fee.unwrap_or_default(),
                params.close_threshold,
                None,
            ),
            associated_queue: output_queue_pubkey,
        };
        mt_account.metadata = metadata;
        mt_account.root_history_capacity = root_history_capacity;
        mt_account.height = height;
        mt_account.tree_type = TreeType::State as u64;
        mt_account
            .queue
            .init(num_batches_input_queue, params.input_queue_batch_size);
        mt_account.queue.bloom_filter_capacity = params.bloom_filter_capacity;

        ZeroCopyBatchedMerkleTreeAccount::init_from_account(
            mt_account,
            mt_account_data,
            params.bloomfilter_num_iters,
            params.bloom_filter_capacity,
        )?;
        Ok(())
    }

    #[test]
    fn test_account_init() {
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account = BatchedQueueAccount::default();
        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account = BatchedMerkleTreeAccount::default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let params = CreateStateTreeAccountsInstructionData::default();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;
        create_batched_state_merkle_tree_accounts(
            owner,
            params.clone(),
            &mut output_queue_account,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        let ref_output_queue_account = BatchedQueueAccount::get_output_queue_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.output_queue_batch_size,
            params.additional_bytes,
            merkle_tree_rent + additional_bytes_rent + queue_rent,
            mt_pubkey,
        );
        assert_queue_zero_copy_inited(
            &mut output_queue_account,
            output_queue_account_data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
        let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.network_fee.unwrap_or_default(),
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            output_queue_pubkey,
        );
        assert_mt_zero_copy_inited(
            &mut mt_account,
            &mut mt_account_data,
            ref_mt_account,
            params.bloomfilter_num_iters,
        );
    }

    #[test]
    fn test_rnd_account_init() {
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..10000 {
            let owner = Pubkey::new_unique();

            let program_owner = if rng.gen_bool(0.5) {
                Some(Pubkey::new_unique())
            } else {
                None
            };
            let forester = if rng.gen_bool(0.5) {
                Some(Pubkey::new_unique())
            } else {
                None
            };

            let params = CreateStateTreeAccountsInstructionData {
                index: rng.gen_range(0..1000),
                program_owner,
                forester,
                additional_bytes: rng.gen_range(0..1000),
                bloomfilter_num_iters: rng.gen_range(0..1000),
                input_queue_batch_size: rng.gen_range(0..1000),
                output_queue_batch_size: rng.gen_range(0..1000),
                // 8 bits per byte, divisible by 8 for aligned memory
                bloom_filter_capacity: rng.gen_range(0..1000) * 8 * 8,
                network_fee: Some(rng.gen_range(0..1000)),
                rollover_threshold: Some(rng.gen_range(0..100)),
                close_threshold: None,
            };
            let queue_account_size = get_output_queue_account_size(params.output_queue_batch_size);

            let mut output_queue_account = BatchedQueueAccount::default();
            let mut output_queue_account_data = vec![0; queue_account_size];
            let output_queue_pubkey = Pubkey::new_unique();

            let mt_account_size = get_merkle_tree_account_size(
                params.input_queue_batch_size,
                params.bloom_filter_capacity,
            );
            let mut mt_account = BatchedMerkleTreeAccount::default();
            let mut mt_account_data = vec![0; mt_account_size];
            let mt_pubkey = Pubkey::new_unique();

            let merkle_tree_rent = rng.gen_range(0..10000000);
            let queue_rent = rng.gen_range(0..10000000);
            let additional_bytes_rent = rng.gen_range(0..10000000);
            create_batched_state_merkle_tree_accounts(
                owner,
                params.clone(),
                &mut output_queue_account,
                &mut output_queue_account_data,
                output_queue_pubkey,
                queue_rent,
                &mut mt_account,
                &mut mt_account_data,
                mt_pubkey,
                merkle_tree_rent,
                additional_bytes_rent,
            )
            .unwrap();
            let ref_output_queue_account = BatchedQueueAccount::get_output_queue(
                owner,
                program_owner,
                forester,
                params.rollover_threshold,
                params.index,
                params.output_queue_batch_size,
                params.additional_bytes,
                merkle_tree_rent + additional_bytes_rent + queue_rent,
                mt_pubkey,
                params.network_fee.unwrap_or_default(),
            );
            assert_queue_zero_copy_inited(
                &mut output_queue_account,
                output_queue_account_data.as_mut_slice(),
                ref_output_queue_account,
                0,
            );
            let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
                owner,
                program_owner,
                forester,
                params.rollover_threshold,
                params.index,
                params.network_fee.unwrap_or_default(),
                params.input_queue_batch_size,
                params.bloom_filter_capacity,
                output_queue_pubkey,
            );
            assert_mt_zero_copy_inited(
                &mut mt_account,
                &mut mt_account_data,
                ref_mt_account,
                params.bloomfilter_num_iters,
            );
        }
    }
    /// Insert into input queue:
    /// 1. New value exists in the current batch bloomfilter
    /// 2. New value does not exist in the other batch bloomfilters
    /// 3.
    pub fn assert_input_queue_insert(
        mut pre_account: BatchedMerkleTreeAccount,
        pre_batches: ManuallyDrop<BoundedVec<Batch>>,
        pre_roots: Vec<[u8; 32]>,
        merkle_tree_zero_copy_account: &mut ZeroCopyBatchedMerkleTreeAccount,
        insert_value: [u8; 32],
    ) -> Result<()> {
        let current_batch_index = merkle_tree_zero_copy_account
            .account
            .queue
            .currently_processing_batch_index as usize;
        let mut inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;

        // if the currently processing batch changed it should
        // increment by one and the old batch should be ready to
        // update
        if current_batch_index != pre_account.queue.currently_processing_batch_index as usize {
            println!("merkle_tree_zero_copy_account.batches
                                [pre_account.queue.currently_processing_batch_index as usize] : {:?}",merkle_tree_zero_copy_account.batches
                                [pre_account.queue.currently_processing_batch_index as usize]);
            assert_eq!(
                merkle_tree_zero_copy_account.batches
                    [pre_account.queue.currently_processing_batch_index as usize]
                    .state,
                BatchState::ReadyToUpdateTree
            );
            pre_account.queue.currently_processing_batch_index += 1;
            pre_account.queue.currently_processing_batch_index %= pre_account.queue.num_batches;
        }
        let mut expected_batch = pre_batches[inserted_batch_index].clone();

        if expected_batch.state == BatchState::Inserted {
            expected_batch.state = BatchState::CanBeFilled;
        }
        assert_eq!(
            *merkle_tree_zero_copy_account.account, pre_account,
            "BatchedMerkleTreeAccount changed."
        );
        let post_roots: Vec<[u8; 32]> = merkle_tree_zero_copy_account
            .root_buffer
            .iter()
            .cloned()
            .collect();
        assert_eq!(post_roots, pre_roots, "Root buffer changed.");

        // New value exists in the current batch bloom filter
        let mut bloomfilter = light_bloom_filter::BloomFilter::new(
            merkle_tree_zero_copy_account.batches[inserted_batch_index].num_iters as usize,
            merkle_tree_zero_copy_account.batches[inserted_batch_index].bloomfilter_capacity,
            merkle_tree_zero_copy_account.bloomfilter_stores[inserted_batch_index].as_mut_slice(),
        )
        .unwrap();
        assert!(bloomfilter.contains(&insert_value));
        let previous_hashchain = expected_batch.user_hash_chain;
        expected_batch.add_to_hash_chain(&insert_value)?;
        assert_ne!(expected_batch.user_hash_chain, previous_hashchain);

        assert_eq!(
            merkle_tree_zero_copy_account.batches[inserted_batch_index],
            expected_batch
        );

        // New value does not exist in the other batch bloomfilters
        for (i, batch) in merkle_tree_zero_copy_account.batches.iter_mut().enumerate() {
            // Skip current batch it is already checked above
            if i == inserted_batch_index {
                continue;
            }
            let mut bloomfilter = light_bloom_filter::BloomFilter::new(
                batch.num_iters as usize,
                batch.bloomfilter_capacity,
                merkle_tree_zero_copy_account.bloomfilter_stores[i].as_mut_slice(),
            )
            .unwrap();
            assert!(!bloomfilter.contains(&insert_value));
        }
        Ok(())
    }

    pub fn assert_output_queue_insert(
        mut pre_account: BatchedQueueAccount,
        pre_batches: ManuallyDrop<BoundedVec<Batch>>,
        output_zero_copy_account: &mut ZeroCopyBatchedQueueAccount,
        insert_value: [u8; 32],
    ) -> Result<()> {
        let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;
        let current_batch_index = output_zero_copy_account
            .account
            .queue
            .currently_processing_batch_index as usize;
        // if the currently processing batch changed it should
        // increment by one and the old batch should be ready to
        // update
        if current_batch_index != pre_account.queue.currently_processing_batch_index as usize {
            println!("merkle_tree_zero_copy_account.batches
                                [pre_account.queue.currently_processing_batch_index as usize] : {:?}",output_zero_copy_account.batches
                                [pre_account.queue.currently_processing_batch_index as usize]);
            assert!(
                output_zero_copy_account.batches
                    [pre_account.queue.currently_processing_batch_index as usize]
                    .state
                    == BatchState::ReadyToUpdateTree
            );
            pre_account.queue.currently_processing_batch_index += 1;
            pre_account.queue.currently_processing_batch_index %= pre_account.queue.num_batches;
        }
        let mut expected_batch = pre_batches[inserted_batch_index].clone();

        if expected_batch.state == BatchState::Inserted {
            expected_batch.state = BatchState::CanBeFilled;
        }
        // TODO: make only is_inserted true if it was recently inserted, replace with state enum
        assert_eq!(
            *output_zero_copy_account.account, pre_account,
            "ZeroCopyBatchedQueueAccount changed."
        );

        let previous_hashchain = expected_batch.user_hash_chain.clone();
        println!("expected_batch: {:?}", expected_batch);
        expected_batch.add_to_hash_chain(&insert_value)?;
        // if expected_batch.num_inserted > expected_batch.batch_size {
        //     expected_batch.num_inserted -= expected_batch.batch_size;
        // }
        assert_ne!(expected_batch.user_hash_chain, previous_hashchain);
        assert_eq!(
            output_zero_copy_account.batches[inserted_batch_index],
            expected_batch
        );

        let other_batch = if inserted_batch_index == 0 { 1 } else { 0 };
        assert_eq!(
            output_zero_copy_account.batches[other_batch],
            pre_batches[other_batch]
        );
        assert!(output_zero_copy_account.value_vecs[inserted_batch_index]
            .as_mut_slice()
            .to_vec()
            .contains(&insert_value));
        assert!(!output_zero_copy_account.value_vecs[other_batch]
            .as_mut_slice()
            .to_vec()
            .contains(&insert_value));
        Ok(())
    }

    /// TODO: add circuits for update
    /// queues with a counter which keeps things below X tps and an if that
    /// executes tree updates when possible.
    #[test]
    fn test_e2e() {
        let num_tx = 220000;
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account = BatchedQueueAccount::default();
        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account = BatchedMerkleTreeAccount::default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let params = CreateStateTreeAccountsInstructionData::default();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;

        create_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut output_queue_account,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account,
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
            // Input queue
            {
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(
                        &mut mt_account,
                        &mut mt_account_data,
                    )
                    .unwrap();

                if rng.gen_bool(0.8) {
                    println!("Input insert -----------------------------");

                    let pre_batches: ManuallyDrop<BoundedVec<Batch>> =
                        merkle_tree_zero_copy_account.batches.clone();
                    let pre_account = merkle_tree_zero_copy_account.account.clone();
                    let pre_roots = merkle_tree_zero_copy_account
                        .root_buffer
                        .iter()
                        .cloned()
                        .collect();
                    let rnd_bytes = get_rnd_bytes(&mut rng);

                    merkle_tree_zero_copy_account
                        .insert_into_current_batch(&rnd_bytes.to_vec().try_into().unwrap())
                        .unwrap();

                    assert_input_queue_insert(
                        pre_account,
                        pre_batches,
                        pre_roots,
                        &mut merkle_tree_zero_copy_account,
                        rnd_bytes,
                    )
                    .unwrap();
                    num_input_values += 1;
                }

                in_ready_for_update = merkle_tree_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.state == BatchState::ReadyToUpdateTree);
            }
            // Output queue
            {
                let mut output_zero_copy_account = ZeroCopyBatchedQueueAccount::from_account(
                    &mut output_queue_account,
                    &mut output_queue_account_data,
                )
                .unwrap();
                if rng.gen_bool(0.5) {
                    println!("Output insert -----------------------------");
                    println!("num_output_values: {}", num_output_values);
                    let rnd_bytes = get_rnd_bytes(&mut rng);
                    let pre_account = output_zero_copy_account.account.clone();
                    let pre_batches = output_zero_copy_account.batches.clone();
                    output_zero_copy_account
                        .insert_into_current_batch(&rnd_bytes)
                        .unwrap();
                    assert_output_queue_insert(
                        pre_account,
                        pre_batches,
                        &mut output_zero_copy_account,
                        rnd_bytes,
                    )
                    .unwrap();
                    num_output_values += 1;
                }
                out_ready_for_update = output_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.state == BatchState::ReadyToUpdateTree);
            }
            let root = get_rnd_bytes(&mut rng);

            let new_output_hash_chain = get_rnd_bytes(&mut rng);
            let instruction_data = InstructionDataBatchUpdateProofInputs {
                public_inputs: BatchProofInputsIx {
                    circuit_id: 1,
                    new_root: root,
                    output_hash_chain: new_output_hash_chain,
                    root_index: 0, // TODO: test with rootindex and simulate security this way
                },
                compressed_proof: CompressedProof::default(),
            };
            let mut pre_mt_account = mt_account.clone();
            let mut pre_mt_account_data = mt_account_data.clone();
            let input_res = {
                let mut zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::from_account(
                    &mut pre_mt_account,
                    &mut pre_mt_account_data,
                )
                .unwrap();
                zero_copy_account.update_input_queue(instruction_data)
            };
            if !in_ready_for_update {
                assert!(input_res.is_err());
            } else {
                println!("res {:?}", input_res);
                assert!(input_res.is_ok());
                in_ready_for_update = false;
                // assert Merkle tree
                // sequence number increased X
                // next index increased X
                // current root index increased X
                // One root changed one didn't
                let expected_next_index = mt_account.next_index + mt_account.queue.batch_size;
                assert_eq!(pre_mt_account.next_index, expected_next_index);
                let expected_sequence_number = mt_account.sequence_number + 1;
                assert_eq!(pre_mt_account.sequence_number, expected_sequence_number);
                // let expected_current_root_index =
                //     mt_account.current_root_index + 1 % mt_account.root_history_capacity;
                // assert_eq!(
                //     pre_mt_account.current_root_index,
                //     expected_current_root_index
                // );
                let zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::from_account(
                    &mut pre_mt_account,
                    &mut pre_mt_account_data,
                )
                .unwrap();
                let old_zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::from_account(
                    &mut mt_account,
                    &mut mt_account_data,
                )
                .unwrap();
                for i in 0..zero_copy_account.root_buffer.len() {
                    println!("i: {}", i);
                    println!(
                        "new root buffer: {:?}",
                        zero_copy_account.root_buffer.get(i as usize)
                    );
                    println!(
                        "old root buffer: {:?}",
                        old_zero_copy_account.root_buffer.get(i as usize)
                    );
                }

                if old_zero_copy_account.root_buffer.len() > 1 {
                    // TODO: investigate
                    // This seems weird to me
                    assert_ne!(
                        zero_copy_account.root_buffer.last(),
                        old_zero_copy_account.root_buffer.first()
                    );
                    assert_eq!(
                        zero_copy_account.root_buffer.first(),
                        old_zero_copy_account.root_buffer.last(),
                    );
                } else {
                    // assert_eq!(
                    //     zero_copy_account.root_buffer.last(),
                    //     old_zero_copy_account.root_buffer.first(),
                    // );
                    // assert_ne!(
                    //     zero_copy_account.root_buffer.first(),
                    //     old_zero_copy_account.root_buffer.last(),
                    // );
                }
                // // let expected_old_root_index = mt_account.first;
                // assert_eq!(
                //     zero_copy_account.root_buffer.last(),
                //     old_zero_copy_account.root_buffer.last(),
                // );
                mt_account = pre_mt_account.clone();
                mt_account_data = pre_mt_account_data.clone();

                num_input_updates += 1;
            }
            let root = get_rnd_bytes(&mut rng);

            let new_output_hash_chain = get_rnd_bytes(&mut rng);
            let instruction_data = InstructionDataBatchUpdateProofInputs {
                public_inputs: BatchProofInputsIx {
                    circuit_id: 1,
                    new_root: root,
                    output_hash_chain: new_output_hash_chain,
                    root_index: 0, // TODO: test with rootindex and simulate security this way
                },
                compressed_proof: CompressedProof::default(),
            };
            let mut zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::from_account(
                &mut pre_mt_account,
                &mut pre_mt_account_data,
            )
            .unwrap();
            let mut pre_output_account_state = output_queue_account.clone();
            let mut pre_output_queue_state = output_queue_account_data.clone();
            let output_res = zero_copy_account.update_output_queue(
                &mut pre_output_account_state,
                &mut pre_output_queue_state,
                instruction_data,
            );
            println!(
                "post update: sequence number: {}",
                zero_copy_account.account.sequence_number
            );
            if !out_ready_for_update {
                assert!(output_res.is_err());
            } else {
                assert!(output_res.is_ok());
                output_queue_account = pre_output_account_state;
                output_queue_account_data = pre_output_queue_state;
                out_ready_for_update = false;
                num_output_updates += 1;
                println!("output update success {}", num_output_updates);
                println!("num_output_values: {}", num_output_values);
                println!("num_input_values: {}", num_input_values);
                let output_zero_copy_account = ZeroCopyBatchedQueueAccount::from_account(
                    &mut output_queue_account,
                    &mut output_queue_account_data,
                )
                .unwrap();
                println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
            }
        }
        let output_zero_copy_account = ZeroCopyBatchedQueueAccount::from_account(
            &mut output_queue_account,
            &mut output_queue_account_data,
        )
        .unwrap();
        println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
        println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
        println!("num_output_updates: {}", num_output_updates);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_values: {}", num_output_values);
        println!("num_input_values: {}", num_input_values);
    }

    pub fn get_rnd_bytes(rng: &mut StdRng) -> [u8; 32] {
        let mut rnd_bytes = rng.gen::<[u8; 32]>();
        rnd_bytes[0] = 0;
        rnd_bytes
    }
}
