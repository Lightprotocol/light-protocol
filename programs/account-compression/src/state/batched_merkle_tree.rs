use crate::{
    batch::BatchState,
    batched_queue::ZeroCopyBatchedQueueAccount,
    bytes_to_struct,
    errors::AccountCompressionErrorCode,
    utils::constants::{DEFAULT_BATCH_SIZE, HEIGHT_26_SUBTREE_ZERO_HASH},
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{BoundedVec, CyclicBoundedVec, CyclicBoundedVecMetadata};
use light_hasher::{Hasher, Poseidon};
use light_verifier::{verify_batch_append, verify_batch_update, CompressedProof};
use std::mem::ManuallyDrop;

use super::{
    batch::Batch,
    batched_queue::{
        batched_queue_from_account, init_queue_from_account, insert_into_current_batch,
        queue_account_size, BatchedQueue,
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
pub enum BatchedTreeType {
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
    pub subtree_hash: [u8; 32],
    pub queue: BatchedQueue,
}

impl BatchedMerkleTreeAccount {
    pub fn size(&self) -> Result<usize> {
        let account_size = std::mem::size_of::<Self>();
        let root_history_size = std::mem::size_of::<CyclicBoundedVecMetadata>()
            + std::mem::size_of::<[u8; 32]>() * self.root_history_capacity as usize;
        #[cfg(target_os = "solana")]
        {
            msg!("size of root history: {}", root_history_size);
            msg!("account size: {}", account_size);
            msg!(
                "queue size: {}",
                queue_account_size(&self.queue, QueueType::Input as u64)?
            );
        }
        let size = account_size
            + root_history_size
            + queue_account_size(&self.queue, QueueType::Input as u64)?
            + 8; // discriminator;
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
            subtree_hash: HEIGHT_26_SUBTREE_ZERO_HASH,
            sequence_number: 0,
            tree_type: BatchedTreeType::State as u64,
            next_index: 0,
            height: 26,
            root_history_capacity,
            queue: BatchedQueue::get_input_queue_default(
                batch_size,
                bloom_filter_capacity,
                zkp_batch_size,
            ),
        }
    }
}

pub struct ZeroCopyBatchedMerkleTreeAccount {
    account: *mut BatchedMerkleTreeAccount,
    pub root_history: ManuallyDrop<CyclicBoundedVec<[u8; 32]>>,
    pub batches: ManuallyDrop<BoundedVec<Batch>>,
    pub value_vecs: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
    pub bloomfilter_stores: Vec<ManuallyDrop<BoundedVec<u8>>>,
    pub hashchain_store: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BatchProofInputsIx {
    pub new_root: [u8; 32],
    pub output_hash_chain: [u8; 32],
    pub old_root_index: u16,
    pub new_subtrees_hash: [u8; 32],
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct InstructionDataBatchAppendProofInputs {
    pub public_inputs: AppendBatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct AppendBatchProofInputsIx {
    pub new_root: [u8; 32],
    pub new_subtrees_hash: [u8; 32],
}

impl ZeroCopyBatchedMerkleTreeAccount {
    pub fn get_account(&self) -> &BatchedMerkleTreeAccount {
        unsafe { self.account.as_ref() }.unwrap()
    }
    pub fn get_account_mut(&mut self) -> &mut BatchedMerkleTreeAccount {
        unsafe { self.account.as_mut() }.unwrap()
    }

    // TODO: add from_account_info,  and from_account_loader
    pub fn from_account(account_data: &mut [u8]) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        unsafe {
            let account = bytes_to_struct::<BatchedMerkleTreeAccount, false>(account_data);
            println!("account {:?}", account);
            if account_data.len() != (*account).size()? {
                return err!(AccountCompressionErrorCode::SizeMismatch);
            }
            let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>() + 8;
            let root_history = CyclicBoundedVec::deserialize(account_data, &mut start_offset);
            let (batches, value_vecs, bloomfilter_stores, hashchain_store) =
                batched_queue_from_account(
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
                bloomfilter_stores,
                hashchain_store,
            })
        }
    }

    pub fn init_from_account(
        metadata: MerkleTreeMetadata,
        root_history_capacity: u32,
        num_batches_input_queue: u64,
        input_queue_batch_size: u64,
        input_queue_zkp_batch_size: u64,
        height: u32,
        account_data: &mut [u8],
        num_iters: u64,
        bloomfilter_capacity: u64,
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount> {
        unsafe {
            let account = bytes_to_struct::<BatchedMerkleTreeAccount, true>(account_data);
            (*account).metadata = metadata;
            (*account).root_history_capacity = root_history_capacity;
            (*account).height = height;
            (*account).tree_type = BatchedTreeType::State as u64;
            (*account).subtree_hash = HEIGHT_26_SUBTREE_ZERO_HASH;
            (*account).queue.init(
                num_batches_input_queue,
                input_queue_batch_size,
                input_queue_zkp_batch_size,
            );
            (*account).queue.bloom_filter_capacity = bloomfilter_capacity;
            msg!("mt account : {:?}", (*account));
            if account_data.len() != (*account).size()? {
                msg!("merkle_tree_account: {:?}", (*account));
                msg!("account_data.len(): {}", account_data.len());
                msg!("account.size(): {}", (*account).size()?);
                return err!(AccountCompressionErrorCode::SizeMismatch);
            }
            let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>() + 8;
            println!("start_offset: {}", start_offset);
            println!(
                "cycle bounded vec size: {}",
                std::mem::size_of::<CyclicBoundedVecMetadata>()
            );
            let root_history = CyclicBoundedVec::init(
                (*account).root_history_capacity as usize,
                account_data,
                &mut start_offset,
                false,
            )
            .map_err(ProgramError::from)?;
            msg!("pre init queue from account mt account:");
            msg!("input bloomfilter capacity : {:?}", bloomfilter_capacity);
            msg!(
                "queue bloomfilter capacity : {:?}",
                (*account).queue.bloom_filter_capacity
            );
            let (batches, value_vecs, bloomfilter_stores, hashchain_store) =
                init_queue_from_account(
                    &(*account).queue,
                    QueueType::Input as u64,
                    account_data,
                    num_iters,
                    bloomfilter_capacity,
                    &mut start_offset,
                )?;
            Ok(ZeroCopyBatchedMerkleTreeAccount {
                account,
                root_history,
                batches,
                value_vecs,
                bloomfilter_stores,
                hashchain_store,
            })
        }
    }

    // TODO: consider storing subtrees in mt account so that the append proofs
    // are independed from the indexer
    // TODO: when proving inclusion by index in
    // value array we need to insert the value into a bloomfilter once it is
    // inserted into the tree. Check this with get_num_inserted_zkps
    pub fn update_output_queue(
        &mut self,
        queue_account_data: &mut [u8],
        instruction_data: InstructionDataBatchAppendProofInputs,
    ) -> Result<()> {
        let mut queue_account =
            ZeroCopyBatchedQueueAccount::from_account(queue_account_data).unwrap();

        let batch_index = queue_account
            .get_account()
            .queue
            .next_full_batch_index
            .clone();
        let batch_size = queue_account.get_account().queue.zkp_batch_size.clone();
        let circuit_batch_size = queue_account.get_account().queue.zkp_batch_size.clone();
        let batches = &mut queue_account.batches;
        let full_batch = batches.get_mut(batch_index as usize).unwrap();

        if full_batch.get_state() == BatchState::Inserted {
            return err!(AccountCompressionErrorCode::BatchAlreadyInserted);
        } else if full_batch.get_state() == BatchState::CanBeFilled {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        let new_root = instruction_data.public_inputs.new_root;
        let old_subtree_hash = self.get_account().subtree_hash;
        let num_zkps = full_batch.get_num_inserted_zkps();

        let start_index = num_zkps * batch_size;
        let end_index = start_index + batch_size as u64;

        let leaves_hashchain = {
            let values = queue_account.value_vecs.get(batch_index as usize).unwrap();
            let mut leaves_hashchain = values[start_index as usize];
            for i in start_index as usize + 1..end_index as usize {
                leaves_hashchain = Poseidon::hashv(&[&leaves_hashchain, &values[i]]).unwrap();
            }
            leaves_hashchain
        };

        let start_index = self.get_account().next_index;

        /*
         * old_subtree_hashchain,
        new_subtree_hashchain,
        merkle_tree.root(),
        leaves_hashchain,
        start_index,
         */
        println!("old_subtree_hash: {:?}", old_subtree_hash);
        println!(
            "new_subtrees_hash: {:?}",
            instruction_data.public_inputs.new_subtrees_hash
        );
        println!("new_root: {:?}", new_root);
        println!("leaves_hashchain: {:?}", leaves_hashchain);
        println!("start_index: {:?}", start_index);
        let mut start_index_bytes = [0u8; 32];
        start_index_bytes[24..].copy_from_slice(&start_index.to_be_bytes());
        let public_input_hash = create_hash_chain([
            old_subtree_hash,
            instruction_data.public_inputs.new_subtrees_hash,
            new_root,
            leaves_hashchain,
            start_index_bytes,
        ])?;

        println!("public input hash {:?}", public_input_hash);

        self.update::<5>(
            circuit_batch_size as usize,
            instruction_data.compressed_proof,
            public_input_hash,
        )?;
        let account = self.get_account_mut();
        println!("previous batched index: {}", account.next_index);
        println!("circut_batch_size: {}", circuit_batch_size);
        account.next_index += circuit_batch_size;
        account.subtree_hash = instruction_data.public_inputs.new_subtrees_hash;
        let root_history_capacity = account.root_history_capacity;
        let sequence_number = account.sequence_number;
        self.root_history.push(new_root);
        full_batch.mark_as_inserted(
            sequence_number,
            self.root_history.last_index() as u32,
            root_history_capacity,
        )?;
        if full_batch.get_state() == BatchState::Inserted {
            queue_account.get_account_mut().queue.next_full_batch_index += 1;
            queue_account.get_account_mut().queue.next_full_batch_index %=
                queue_account.get_account_mut().queue.num_batches;
        }
        Ok(())
    }

    pub fn update_input_queue(
        &mut self,
        instruction_data: InstructionDataBatchUpdateProofInputs,
    ) -> Result<()> {
        let batch_index = self.get_account().queue.next_full_batch_index.clone();

        let full_batch = self.batches.get(batch_index as usize).unwrap();

        if full_batch.get_state() == BatchState::Inserted {
            return err!(AccountCompressionErrorCode::BatchAlreadyInserted);
        } else if full_batch.get_state() == BatchState::CanBeFilled {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        let num_zkps = full_batch.get_num_inserted_zkps();

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
        println!("old_root: {:?}", old_root);
        println!("new_root: {:?}", new_root);
        println!("leaves_hashchain: {:?}", leaves_hashchain);
        let public_input_hash = create_hash_chain([*old_root, new_root, *leaves_hashchain])?;
        println!("public_input_hash: {:?}", public_input_hash);
        let circuit_batch_size = self.get_account().queue.zkp_batch_size;
        self.update::<3>(
            circuit_batch_size as usize,
            instruction_data.compressed_proof,
            public_input_hash,
        )?;
        // TODO: add new subtrees hash to public input hash once circuit is updated
        self.root_history.push(new_root);
        let sequence_number = self.get_account().sequence_number;
        let root_history_capacity = self.get_account().root_history_capacity;
        let full_batch = self.batches.get_mut(batch_index as usize).unwrap();

        full_batch.mark_as_inserted(
            sequence_number,
            self.root_history.last_index() as u32,
            root_history_capacity,
        )?;

        if full_batch.get_state() == BatchState::Inserted {
            let account = self.get_account_mut();
            account.queue.next_full_batch_index += 1;
            account.queue.next_full_batch_index %= account.queue.num_batches;
        }
        let account = self.get_account_mut();
        account.subtree_hash = instruction_data.public_inputs.new_subtrees_hash;

        Ok(())
    }

    fn update<const QUEUE_TYPE: u64>(
        &mut self,
        batch_size: usize,
        proof: CompressedProof,
        public_input_hash: [u8; 32],
    ) -> Result<()> {
        if QUEUE_TYPE == QueueType::Output as u64 {
            verify_batch_append(batch_size, public_input_hash, &proof)
                .map_err(ProgramError::from)?;
        } else if QUEUE_TYPE == QueueType::Input as u64 {
            verify_batch_update(batch_size, public_input_hash, &proof)
                .map_err(ProgramError::from)?;
        } else {
            return err!(AccountCompressionErrorCode::InvalidQueueType);
        }
        self.get_account_mut().sequence_number += 1;
        Ok(())
    }

    /*
     * Indexer:
     *
     * Input state:
     * - input queue elements are taken from the PublicTransactionEventV2
     * Output state:
     * - can be read either from the output queue or from PublicTransactionEvent
     *
     * New addresses:
     * - new addresses are taken from the PublicTransactionEventV2
     *
     * Indexer forester:
     * - forester indexer doesn't care about the output state itself just the hashes
     * -
     * - input queue elements are taken from the PublicTransactionEventV2
     * -
     *
     *  Event:
     *  - create a transaction event V2, which adds Destination mt pubkey for inputs
     *  -
     */

    // TODO: add security test
    pub fn insert_into_current_batch(&mut self, value: &[u8; 32]) -> Result<()> {
        unsafe {
            let (root_index, sequence_number) = insert_into_current_batch(
                QueueType::Input as u64,
                &mut (*self.account).queue,
                &mut self.batches,
                &mut self.value_vecs,
                &mut self.bloomfilter_stores,
                &mut self.hashchain_store,
                value,
            )?;

            if let Some(sequence_number) = sequence_number {
                // TODO: move queue insert before proof verification
                // TODO: double check security of this
                // If the sequence number is greater than current sequence number
                // there is still at least one root which can be used to prove
                // inclusion of a value which was in the batch that was just wiped.
                if sequence_number > self.get_account().sequence_number {
                    // advance root history array current index to save index
                    if let Some(root_index) = root_index {
                        while self.root_history.last_index() != root_index as usize {
                            self.root_history.push([0u8; 32]);
                        }
                    }
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
                 *   bloomfilter: [B0, B1, B2],
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
                 * B0.sequence_number = 14
                 * B0.root_index = 4
                 * - execute some append root updates
                 * -> R4 -> A0.1
                 * -> R5 -> A0.2
                 * -> R6 -> A0.3
                 * -> R7 -> A0.4 - final A0 (append batch 0) root
                 *
                 * TODO: benchmark how much zeroing out costs
                 * Timeslot 2:
                 * - advance root to index 4 and zero out all other roots
                 * - insert into B0 until full
                 * - update tree with B1, don't clear B1 yet
                 * -> R0 -> B0
                 * -> R1 -> B1
                 * B1.sequence_number = 11
                 * B1.root_index = 1
                 *
                 * Timeslot 3:
                 * -> R0 -> B0
                 * -> R1 -> B1
                 * - clear B0
                 *
                 * Timeslot 3:
                 * -> R0 -> B0
                 * -> R1 -> B1
                 * - clear B0
                 * - advance root to index 0
                 * - insert into B0 until full
                 * - update tree with B2, don't clear B2 yet
                 * -> R0 -> B2
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
            }
        }

        Ok(())
    }
}

pub fn create_hash_chain<const T: usize>(inputs: [[u8; 32]; T]) -> Result<[u8; 32]> {
    let mut hash_chain = inputs[0];
    for i in 1..T {
        hash_chain = Poseidon::hashv(&[&hash_chain, &inputs[i]]).map_err(ProgramError::from)?;
    }
    Ok(hash_chain)
}

pub fn get_merkle_tree_account_size_default() -> usize {
    // TODO: implement a default config for BatchedMerkleTreeAccount using a
    // default for BatchedInputQueue
    let mt_account = BatchedMerkleTreeAccount {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: BatchedTreeType::State as u64,
        height: 26,
        root_history_capacity: 20,
        subtree_hash: HEIGHT_26_SUBTREE_ZERO_HASH,
        queue: BatchedQueue {
            currently_processing_batch_index: 0,
            num_batches: 4,
            batch_size: DEFAULT_BATCH_SIZE,
            bloom_filter_capacity: 200_000 * 8,
            next_index: 0,
            zkp_batch_size: 10,
            ..Default::default()
        },
    };
    mt_account.size().unwrap()
}

pub fn get_merkle_tree_account_size(
    batch_size: u64,
    bloom_filter_capacity: u64,
    zkp_batch_size: u64,
    root_history_capacity: u32,
) -> usize {
    // TODO: implement a default config for BatchedMerkleTreeAccount using a
    // default for BatchedInputQueue
    let mt_account = BatchedMerkleTreeAccount {
        metadata: MerkleTreeMetadata::default(),
        next_index: 0,
        sequence_number: 0,
        tree_type: BatchedTreeType::State as u64,
        height: 26,
        root_history_capacity,
        subtree_hash: HEIGHT_26_SUBTREE_ZERO_HASH,
        queue: BatchedQueue {
            num_batches: 4,
            batch_size,
            bloom_filter_capacity,
            zkp_batch_size,
            ..Default::default()
        },
    };
    mt_account.size().unwrap()
}

#[cfg(test)]
mod tests {

    use std::{cmp::min, ops::Deref};

    use light_merkle_tree_reference::MerkleTree;
    use light_prover_client::{
        gnark::helpers::{spawn_prover, ProofType},
        mock_indexer::{self},
    };

    use rand::{rngs::StdRng, Rng};

    use crate::{
        batch::BatchState,
        batched_queue::{
            assert_queue_inited, get_output_queue_account_size_default, BatchedQueueAccount,
        },
        init_batched_state_merkle_tree_accounts, QueueType,
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

        let mut zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::from_account(account_data)
            .expect("from_account failed");
        assert_eq!(
            *zero_copy_account.get_account(),
            ref_account,
            "metadata mismatch"
        );

        assert_eq!(
            zero_copy_account.root_history.capacity(),
            ref_account.root_history_capacity as usize,
            "root_history_capacity mismatch"
        );

        assert!(
            zero_copy_account.root_history.is_empty(),
            "root_history not empty"
        );
        assert_eq!(
            zero_copy_account.get_account().subtree_hash,
            ref_account.subtree_hash
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

    /// Insert into input queue:
    /// 1. New value exists in the current batch bloomfilter
    /// 2. New value does not exist in the other batch bloomfilters
    /// 3.
    pub fn assert_input_queue_insert(
        mut pre_account: BatchedMerkleTreeAccount,
        pre_batches: ManuallyDrop<BoundedVec<Batch>>,
        pre_roots: Vec<[u8; 32]>,
        mut pre_hashchains: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        merkle_tree_zero_copy_account: &mut ZeroCopyBatchedMerkleTreeAccount,
        insert_value: [u8; 32],
    ) -> Result<()> {
        let current_batch_index = merkle_tree_zero_copy_account
            .get_account()
            .queue
            .currently_processing_batch_index as usize;
        let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;

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
                    .get_state(),
                BatchState::ReadyToUpdateTree
            );
            pre_account.queue.currently_processing_batch_index += 1;
            pre_account.queue.currently_processing_batch_index %= pre_account.queue.num_batches;
        }
        let mut expected_batch = pre_batches[inserted_batch_index].clone();

        if expected_batch.get_state() == BatchState::Inserted {
            pre_hashchains[inserted_batch_index].clear();
            expected_batch.sequence_number = 0;
            // TODO:_ add a function get_root_index();
            // let root_index = merkle_tree_zero_copy_account.root_history.last_index() as u32;
            // let root_history_length = merkle_tree_zero_copy_account.root_history.len() as u32;
            // let sequence_number = pre_account.sequence_number;
            // expected_batch
            //     .mark_as_inserted(sequence_number, root_index, root_history_length)
            //     .unwrap();

            expected_batch.advance_state_to_can_be_filled().unwrap();
        }
        assert_eq!(
            *merkle_tree_zero_copy_account.get_account(),
            pre_account,
            "BatchedMerkleTreeAccount changed."
        );
        let post_roots: Vec<[u8; 32]> = merkle_tree_zero_copy_account
            .root_history
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
        let mut pre_hashchain = pre_hashchains.get_mut(inserted_batch_index).unwrap();
        // let previous_hashchain = expected_batch.user_hash_chain;
        expected_batch.add_to_hash_chain(&insert_value, &mut pre_hashchain)?;
        // assert_ne!(expected_batch.user_hash_chain, previous_hashchain);
        // assert_eq!(
        //     merkle_tree_zero_copy_account.hashchain_store[inserted_batch_index]
        //         .last()
        //         .unwrap(),
        //     pre_hashchain.last().unwrap(),
        //     "Hashchain store inconsistent."
        // );
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
        mut pre_hashchains: Vec<ManuallyDrop<BoundedVec<[u8; 32]>>>,
        output_zero_copy_account: &mut ZeroCopyBatchedQueueAccount,
        insert_value: [u8; 32],
    ) -> Result<()> {
        let inserted_batch_index = pre_account.queue.currently_processing_batch_index as usize;
        let current_batch_index = output_zero_copy_account
            .get_account()
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
                    .get_state()
                    == BatchState::ReadyToUpdateTree
            );
            pre_account.queue.currently_processing_batch_index += 1;
            pre_account.queue.currently_processing_batch_index %= pre_account.queue.num_batches;
        }
        let mut expected_batch = pre_batches[inserted_batch_index].clone();
        let mut pre_hashchain = pre_hashchains.get_mut(inserted_batch_index).unwrap();

        if expected_batch.get_state() == BatchState::Inserted {
            expected_batch.advance_state_to_can_be_filled().unwrap();
            pre_hashchain.clear();
        }
        // TODO: make only is_inserted true if it was recently inserted, replace with state enum
        pre_account.next_index += 1;
        assert_eq!(
            *output_zero_copy_account.get_account(),
            pre_account,
            "ZeroCopyBatchedQueueAccount changed."
        );

        let previous_hashchain = if let Some(hashchain) = pre_hashchain.last() {
            *hashchain
        } else {
            [0u8; 32]
        };
        println!("expected_batch: {:?}", expected_batch);
        expected_batch.add_to_hash_chain(&insert_value, &mut pre_hashchain)?;

        assert_ne!(
            *output_zero_copy_account.hashchain_store[inserted_batch_index]
                .last()
                .unwrap(),
            previous_hashchain
        );
        assert_eq!(
            output_zero_copy_account.hashchain_store[inserted_batch_index]
                .last()
                .unwrap(),
            pre_hashchain.last().unwrap(),
            "Hashchain store inconsistent."
        );
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

    #[derive(Debug, PartialEq, Clone)]
    pub struct MockTransactionInputs {
        inputs: Vec<[u8; 32]>,
        outputs: Vec<[u8; 32]>,
    }

    pub fn simulate_transaction(
        instruction_data: MockTransactionInputs,
        merkle_tree_account_data: &mut [u8],
        output_queue_account_data: &mut [u8],
        reference_merkle_tree: &mut MerkleTree<Poseidon>,
    ) -> Result<()> {
        let mut output_zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_account(output_queue_account_data).unwrap();
        let mut merkle_tree_zero_copy_account =
            ZeroCopyBatchedMerkleTreeAccount::from_account(merkle_tree_account_data).unwrap();

        for input in instruction_data.inputs.iter() {
            let inclusion = reference_merkle_tree.get_leaf_index(input);
            if inclusion.is_none() {
                let mut included = false;

                // TODO: insert into batch regardless if the batch is already
                // part of the Merkle tree (don't skip the check though)
                for value_vec in output_zero_copy_account.value_vecs.iter_mut() {
                    for value in value_vec.iter_mut() {
                        // TODO: test double spending
                        if *value == *input {
                            included = true;
                            *value = [0u8; 32];
                        }
                    }
                }
                if !included {
                    panic!("Value not included in any output queue or trees.");
                }
                continue;
            } else {
                println!("1insert input into batch");
                merkle_tree_zero_copy_account.insert_into_current_batch(input)?;
            }
        }
        for output in instruction_data.outputs.iter() {
            output_zero_copy_account.insert_into_current_batch(output)?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_simulate_transactions() {
        spawn_prover(false, &[ProofType::BatchAppend, ProofType::BatchUpdate]).await;
        let mut mock_indexer = mock_indexer::MockIndexer::<26>::new();

        let num_tx = 220000;
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let params = crate::InitStateTreeAccountsInstructionData::default();

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
            /*
            Issue:
            - values committed to the output queue shall be spendable immediately
            - we usually insert values that we spend into the nullifier queue
            - if we insert values from the output queue into the input queue
                and insert the input queue batch before the output queue
                we might try to nullify leaves which don't exist yet.

            Solution:
            - second leaves hashchain for leaves which are zeroed out already
            - we hash the index of the leaf in the value array
            - don't insert the value into the nullifier queue
                -> all values in the nullifier queue are in part of the tree

            // TODO:
            - modify the batch append circuit to allow to skip leaves in the leaveshashchain
            - for every leaf select

            circuit implementation:
            - problem naive solutions result in squared complexity

            Other solution:
            - generate leaves hashchain when updating the tree (downside this limits the batch size to ~1500 (800 CU per poseidon hash))
             */
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
                // TODO: add full test for inputs
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
                let mut retries = min(10, mock_indexer.active_leaves.len());
                while inputs.len() < number_of_inputs && retries > 0 {
                    let leaf = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);
                    let inserted = mock_indexer.merkle_tree.get_leaf_index(&leaf);
                    if inserted.is_some() {
                        inputs.push(leaf);
                        mock_indexer.input_queue_leaves.push(leaf);
                    } else if rng.gen_bool(0.1) {
                        inputs.push(leaf);
                        println!("input not inserted into tree");
                    }

                    retries -= 1;
                }
                // if inputs.is_empty() && !mock_indexer.active_leaves.is_empty() {
                //     inputs.push(mock_indexer.active_leaves.remove(0));
                //     println!("adding leaf to input queue");
                // }
                let number_of_inputs = inputs.len();
                println!("number_of_inputs: {}", number_of_inputs);

                let instruction_data = MockTransactionInputs {
                    inputs: inputs.clone(),
                    outputs: outputs.clone(),
                };

                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut mt_account_data).unwrap();
                println!(
                    "input queue: {:?}",
                    merkle_tree_zero_copy_account.batches[0].get_num_inserted()
                );
                let mut output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data)
                        .unwrap();
                let mut pre_output_account = output_zero_copy_account.get_account().clone();

                let pre_output_batches = output_zero_copy_account.batches.clone();
                let pre_output_hashchains = output_zero_copy_account.hashchain_store.clone();

                let mut pre_mt_account = merkle_tree_zero_copy_account.account.clone();
                let mut pre_mt_data = mt_account_data.clone();
                let pre_mt_batches = merkle_tree_zero_copy_account.batches.clone();
                // let pre_mt_roots = merkle_tree_zero_copy_account
                //     .root_history
                //     .iter()
                //     .cloned()
                //     .collect();
                let pre_mt_hashchains = merkle_tree_zero_copy_account.hashchain_store.clone();

                println!("Simulating tx with inputs: {:?}", instruction_data);

                simulate_transaction(
                    instruction_data,
                    &mut pre_mt_data,
                    &mut output_queue_account_data,
                    &mut mock_indexer.merkle_tree,
                )
                .unwrap();
                // if !outputs.is_empty() {
                //     assert_output_queue_insert(
                //         pre_output_account,
                //         pre_output_batches,
                //         pre_output_hashchains,
                //         &mut output_zero_copy_account,
                //         outputs[0],
                //     )
                //     .unwrap();
                // }
                // if !inputs.is_empty() {
                //     assert_input_queue_insert(
                //         pre_mt_account,
                //         pre_mt_batches,
                //         pre_mt_roots,
                //         pre_mt_hashchains,
                //         &mut merkle_tree_zero_copy_account,
                //         inputs[0],
                //     )
                //     .unwrap();
                // }
                for i in 0..number_of_outputs {
                    mock_indexer.active_leaves.push(outputs[i]);
                }

                num_output_values += number_of_outputs;
                num_input_values += number_of_inputs;
                let merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_data).unwrap();
                in_ready_for_update = merkle_tree_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.get_state() == BatchState::ReadyToUpdateTree);
                out_ready_for_update = output_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.get_state() == BatchState::ReadyToUpdateTree);

                mt_account_data = pre_mt_data.clone();
            }

            // Get random leaf that is not in the input queue.
            pub fn get_random_leaf(
                rng: &mut StdRng,
                active_leaves: &mut Vec<[u8; 32]>,
            ) -> [u8; 32] {
                if active_leaves.len() == 0 {
                    return [0u8; 32];
                }
                // get random leaf from vector and remove it
                active_leaves.remove(rng.gen_range(0..active_leaves.len()))
            }

            if in_ready_for_update && rng.gen_bool(0.7) {
                println!("Input update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let mut pre_mt_account_data = mt_account_data.clone();
                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut mt_account_data).unwrap();
                let (input_res, root, new_subtrees_hash) = {
                    let mut zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_account_data)
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
                    let new_subtrees = mock_indexer.merkle_tree.get_subtrees();
                    let new_subtrees_hash =
                        create_hash_chain::<26>(new_subtrees.try_into().unwrap()).unwrap();
                    let instruction_data = InstructionDataBatchUpdateProofInputs {
                        public_inputs: BatchProofInputsIx {
                            new_root,
                            output_hash_chain: *leaves_hashchain,
                            old_root_index: old_root_index as u16,
                            new_subtrees_hash,
                        },
                        compressed_proof: CompressedProof {
                            a: proof.a,
                            b: proof.b,
                            c: proof.c,
                        },
                    };

                    (
                        zero_copy_account.update_input_queue(instruction_data),
                        new_root,
                        new_subtrees_hash,
                    )
                };
                println!("Input update -----------------------------");
                println!("res {:?}", input_res);
                assert!(input_res.is_ok());
                in_ready_for_update = false;
                // assert Merkle tree
                // sequence number increased X
                // next index increased X
                // current root index increased X
                // One root changed one didn't

                let zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_account_data)
                        .unwrap();

                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    None,
                    None,
                    root,
                    new_subtrees_hash,
                );
                mt_account_data = pre_mt_account_data.clone();

                num_input_updates += 1;
            }

            if out_ready_for_update && rng.gen_bool(0.7) {
                println!("Output update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);

                let mut pre_mt_account_data = mt_account_data.clone();
                let mut zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_account_data)
                        .unwrap();
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data)
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
                let leaves = output_zero_copy_account
                    .value_vecs
                    .get(next_full_batch as usize)
                    .unwrap()
                    .deref()
                    .clone()
                    .to_vec();
                println!("leaves {:?}", leaves.len());
                let (proof, new_root, new_subtree_hash) = mock_indexer
                    .get_batched_append_proof(
                        next_index as usize,
                        // *leaves_hashchain,
                        leaves.clone(),
                        batch.get_num_inserted_zkps() as u32,
                        batch.zkp_batch_size as u32,
                    )
                    .await
                    .unwrap();
                // let start = batch.get_num_inserted_zkps() as usize * batch.zkp_batch_size as usize;
                // let end = start + batch.zkp_batch_size as usize;
                // for i in start..end {
                //     // Storing the leaf in the output queue indexer so that it
                //     // can be inserted into the input queue later.
                //     mock_indexer.active_leaves.push(leaves[i]);
                // }

                let instruction_data = InstructionDataBatchAppendProofInputs {
                    public_inputs: AppendBatchProofInputsIx {
                        new_root,
                        new_subtrees_hash: new_subtree_hash,
                    },
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };

                let mut pre_output_queue_state = output_queue_account_data.clone();
                println!("Output update -----------------------------");

                let output_res = zero_copy_account
                    .update_output_queue(&mut pre_output_queue_state, instruction_data);

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
                    ZeroCopyBatchedQueueAccount::from_account(&mut pre_output_queue_state).unwrap();
                let old_output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data)
                        .unwrap();

                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut mt_account_data).unwrap();

                println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    Some(old_output_zero_copy_account),
                    Some(output_zero_copy_account),
                    new_root,
                    new_subtree_hash,
                );

                output_queue_account_data = pre_output_queue_state;
                mt_account_data = pre_mt_account_data;
                out_ready_for_update = false;
                num_output_updates += 1;
            }
        }
        let output_zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data).unwrap();
        println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
        println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
        println!("num_output_updates: {}", num_output_updates);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_values: {}", num_output_values);
        println!("num_input_values: {}", num_input_values);
    }

    /// queues with a counter which keeps things below X tps and an if that
    /// executes tree updates when possible.
    #[tokio::test]
    async fn test_e2e() {
        spawn_prover(false, &[ProofType::BatchAppend, ProofType::BatchUpdate]).await;
        let mut mock_indexer = mock_indexer::MockIndexer::<26>::new();

        let num_tx = 220000;
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let params = crate::InitStateTreeAccountsInstructionData::default();

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
            /*
            Issue:
            - values committed to the output queue shall be spendable immediately
            - we usually insert values that we spend into the nullifier queue
            - if we insert values from the output queue into the input queue
                and insert the input queue batch before the output queue
                we might try to nullify leaves which don't exist yet.

            Solution:
            - second leaves hashchain for leaves which are zeroed out already
            - we hash the index of the leaf in the value array
            - don't insert the value into the nullifier queue
                -> all values in the nullifier queue are in part of the tree

            // TODO:
            - modify the batch append circuit to allow to skip leaves in the leaveshashchain
            - for every leaf select

            circuit implementation:
            - problem naive solutions result in squared complexity

            Other solution:
            - generate leaves hashchain when updating the tree (downside this limits the batch size to ~1500 (800 CU per poseidon hash))
             */

            // Output queue
            {
                let mut output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data)
                        .unwrap();
                if rng.gen_bool(0.5) {
                    println!("Output insert -----------------------------");
                    println!("num_output_values: {}", num_output_values);
                    let rnd_bytes = get_rnd_bytes(&mut rng);

                    let pre_account = output_zero_copy_account.get_account().clone();
                    let pre_batches = output_zero_copy_account.batches.clone();
                    let pre_hashchains = output_zero_copy_account.hashchain_store.clone();
                    output_zero_copy_account
                        .insert_into_current_batch(&rnd_bytes)
                        .unwrap();
                    assert_output_queue_insert(
                        pre_account,
                        pre_batches,
                        pre_hashchains,
                        &mut output_zero_copy_account,
                        rnd_bytes,
                    )
                    .unwrap();
                    num_output_values += 1;
                }
                out_ready_for_update = output_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.get_state() == BatchState::ReadyToUpdateTree);
            }

            // Get random leaf that is not in the input queue.
            pub fn get_random_leaf(
                rng: &mut StdRng,
                active_leaves: &mut Vec<[u8; 32]>,
            ) -> [u8; 32] {
                // get random leaf from vector and remove it
                active_leaves.remove(rng.gen_range(0..active_leaves.len()))
            }

            // Input queue
            {
                let mut merkle_tree_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut mt_account_data).unwrap();

                if rng.gen_bool(0.5) && !mock_indexer.active_leaves.is_empty() {
                    println!("Input insert -----------------------------");
                    let leaf = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);

                    let pre_batches: ManuallyDrop<BoundedVec<Batch>> =
                        merkle_tree_zero_copy_account.batches.clone();
                    let pre_account = merkle_tree_zero_copy_account.get_account().clone();
                    let pre_roots = merkle_tree_zero_copy_account
                        .root_history
                        .iter()
                        .cloned()
                        .collect();
                    let pre_hashchains = merkle_tree_zero_copy_account.hashchain_store.clone();

                    // Index input queue insert event
                    mock_indexer.input_queue_leaves.push(leaf);
                    merkle_tree_zero_copy_account
                        .insert_into_current_batch(&leaf.to_vec().try_into().unwrap())
                        .unwrap();

                    assert_input_queue_insert(
                        pre_account,
                        pre_batches,
                        pre_roots,
                        pre_hashchains,
                        &mut merkle_tree_zero_copy_account,
                        leaf,
                    )
                    .unwrap();
                    num_input_values += 1;
                }

                in_ready_for_update = merkle_tree_zero_copy_account
                    .batches
                    .iter()
                    .any(|batch| batch.get_state() == BatchState::ReadyToUpdateTree);
            }

            if in_ready_for_update {
                println!("Input update -----------------------------");
                println!("Num inserted values: {}", num_input_values);
                println!("Num input updates: {}", num_input_updates);
                println!("Num output updates: {}", num_output_updates);
                println!("Num output values: {}", num_output_values);
                let mut pre_mt_account_data = mt_account_data.clone();
                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut mt_account_data).unwrap();
                let (input_res, root, new_subtrees_hash) = {
                    let mut zero_copy_account =
                        ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_account_data)
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
                    let new_subtrees = mock_indexer.merkle_tree.get_subtrees();
                    let new_subtrees_hash =
                        create_hash_chain::<26>(new_subtrees.try_into().unwrap()).unwrap();
                    let instruction_data = InstructionDataBatchUpdateProofInputs {
                        public_inputs: BatchProofInputsIx {
                            new_root,
                            // TODO: remove
                            output_hash_chain: [0u8; 32],
                            old_root_index: old_root_index as u16,
                            new_subtrees_hash,
                        },
                        compressed_proof: CompressedProof {
                            a: proof.a,
                            b: proof.b,
                            c: proof.c,
                        },
                    };

                    (
                        zero_copy_account.update_input_queue(instruction_data),
                        new_root,
                        new_subtrees_hash,
                    )
                };
                println!("Input update -----------------------------");
                println!("res {:?}", input_res);
                assert!(input_res.is_ok());
                in_ready_for_update = false;
                // assert Merkle tree
                // sequence number increased X
                // next index increased X
                // current root index increased X
                // One root changed one didn't

                let zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_account_data)
                        .unwrap();

                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    None,
                    None,
                    root,
                    new_subtrees_hash,
                );

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
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut pre_mt_account_data)
                        .unwrap();
                let output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data)
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
                // let leaves_hashchain = output_zero_copy_account
                //     .hashchain_store
                //     .get(next_full_batch as usize)
                //     .unwrap()
                //     .get(batch.get_num_inserted_zkps() as usize)
                //     .unwrap();
                let leaves = output_zero_copy_account
                    .value_vecs
                    .get(next_full_batch as usize)
                    .unwrap()
                    .deref()
                    .clone()
                    .to_vec();
                println!("leaves {:?}", leaves.len());
                let (proof, new_root, new_subtree_hash) = mock_indexer
                    .get_batched_append_proof(
                        next_index as usize,
                        // *leaves_hashchain,
                        leaves.clone(),
                        batch.get_num_inserted_zkps() as u32,
                        batch.zkp_batch_size as u32,
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

                let instruction_data = InstructionDataBatchAppendProofInputs {
                    public_inputs: AppendBatchProofInputsIx {
                        new_root,
                        new_subtrees_hash: new_subtree_hash,
                    },
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };

                let mut pre_output_queue_state = output_queue_account_data.clone();
                println!("Output update -----------------------------");

                let output_res = zero_copy_account
                    .update_output_queue(&mut pre_output_queue_state, instruction_data);

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
                    ZeroCopyBatchedQueueAccount::from_account(&mut pre_output_queue_state).unwrap();
                let old_output_zero_copy_account =
                    ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data)
                        .unwrap();

                let old_zero_copy_account =
                    ZeroCopyBatchedMerkleTreeAccount::from_account(&mut mt_account_data).unwrap();

                println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
                assert_merkle_tree_update(
                    old_zero_copy_account,
                    zero_copy_account,
                    Some(old_output_zero_copy_account),
                    Some(output_zero_copy_account),
                    new_root,
                    new_subtree_hash,
                );

                output_queue_account_data = pre_output_queue_state;
                mt_account_data = pre_mt_account_data;
                out_ready_for_update = false;
                num_output_updates += 1;
            }
        }
        let output_zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_account(&mut output_queue_account_data).unwrap();
        println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
        println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
        println!("num_output_updates: {}", num_output_updates);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_values: {}", num_output_values);
        println!("num_input_values: {}", num_input_values);
    }

    fn assert_merkle_tree_update(
        old_zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
        zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
        old_queue_account: Option<ZeroCopyBatchedQueueAccount>,
        queue_account: Option<ZeroCopyBatchedQueueAccount>,
        root: [u8; 32],
        new_subtree_hash: [u8; 32],
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
            let previous_processing =
                if expected_account.queue.currently_processing_batch_index == 0 {
                    expected_account.queue.num_batches - 1
                } else {
                    expected_account.queue.currently_processing_batch_index - 1
                };
            (
                zero_copy_account.batches.clone(),
                old_zero_copy_account.batches.clone(),
                previous_processing,
                None,
                0,
            )
        };

        for (i, batch) in batches.iter().enumerate() {
            let previous_batch = previous_batchs.get(i).unwrap();
            if batch.sequence_number != 0
                && batch.get_state() == BatchState::Inserted
                && previous_processing == i as u64
            {
                println!("batch {:?}", batch);

                if queue_account.is_some() {
                    next_full_batch_index += 1;
                    println!(
                        "expected_queue_account {:?}",
                        expected_queue_account.unwrap().queue
                    );
                    next_full_batch_index %= expected_queue_account.unwrap().queue.num_batches;
                } else {
                    expected_account.queue.next_full_batch_index += 1;
                    expected_account.queue.next_full_batch_index %=
                        expected_account.queue.num_batches;
                }

                println!("batch {:?}", batch);
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
            } else if batch.get_state() == BatchState::ReadyToUpdateTree {
                assert_eq!(
                    batch.get_num_inserted_zkps(),
                    previous_batch.get_num_inserted_zkps() + 1
                );
                assert_eq!(batch.get_num_inserted(), previous_batch.get_num_inserted());

                assert_eq!(batch.sequence_number, previous_batch.sequence_number);
                assert_eq!(batch.root_index, previous_batch.root_index);
                assert_eq!(
                    batch.get_current_zkp_batch_index(),
                    batch.get_num_zkp_batches()
                );
                assert_eq!(batch.get_state(), previous_batch.get_state());
                assert_eq!(batch.get_num_inserted(), 0);
            } else {
                assert_eq!(*batch, *previous_batch);
            }
        }
        if let Some(queue_account) = queue_account.as_ref() {
            let mut expected_queue_account = expected_queue_account.unwrap();
            expected_queue_account.queue.next_full_batch_index = next_full_batch_index;
            println!(
                "old queue account {:?}",
                old_queue_account.as_ref().unwrap().get_account()
            );
            assert_eq!(*queue_account.get_account(), expected_queue_account);
        }
        // subtrees are only updated with append proofs

        expected_account.subtree_hash = new_subtree_hash;

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

        // if old_zero_copy_account.root_history.len() > 1 {
        //     let last_index = zero_copy_account.root_history.last_index();
        //     println!("last index {:?}", last_index);
        //     for (i, _) in old_zero_copy_account.root_history.iter().enumerate() {
        //         println!("i {:?}", i);
        //         let zero_bytes_root: [u8; 32] = [
        //             18, 12, 88, 241, 67, 212, 145, 233, 89, 2, 247, 245, 39, 119, 120, 162, 224,
        //             173, 81, 104, 246, 173, 215, 86, 105, 147, 38, 48, 206, 97, 21, 24,
        //         ];
        //         // check that root is not zero bytes root if so don't assert
        //         if zero_bytes_root != *zero_copy_account.root_history.get(i).unwrap() {
        //             if last_index == i {
        //                 assert_ne!(
        //                     zero_copy_account.root_history.get(i).unwrap(),
        //                     old_zero_copy_account.root_history.get(i).unwrap()
        //                 );
        //             } else {
        //                 assert_eq!(
        //                     zero_copy_account.root_history.get(i).unwrap(),
        //                     old_zero_copy_account.root_history.get(i).unwrap()
        //                 );
        //             }
        //         }
        //     }
        //     assert_ne!(*old_zero_copy_account.root_history.last().unwrap(), root);
        // }
    }

    pub fn get_rnd_bytes(rng: &mut StdRng) -> [u8; 32] {
        let mut rnd_bytes = rng.gen::<[u8; 32]>();
        rnd_bytes[0] = 0;
        rnd_bytes
    }
}
