use crate::{
    batched_queue::{deserialize_cyclic_bounded_vec, ZeroCopyBatchedAddressQueueAccount},
    errors::AccountCompressionErrorCode,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{BoundedVecMetadata, CyclicBoundedVec};
use light_hasher::{Hasher, Poseidon};
use light_verifier::CompressedProof;
use std::mem::ManuallyDrop;

use super::{
    batched_queue::{init_bounded_cyclic_vec, BatchedAddressQueueAccount},
    AccessMetadata, RolloverMetadata,
};

#[derive(Debug, PartialEq)]
#[account(zero_copy)]
pub struct BatchedMerkleTreeMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,
    // Queue associated with this Merkle tree.
    pub associated_input_queue: Pubkey,
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

#[derive(Debug, PartialEq)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct BatchedMerkleTreeAccount {
    pub metadata: BatchedMerkleTreeMetadata,
    pub sequence_number: u64,
    pub tree_type: u64,
    pub next_index: u64,
    pub height: u64,
    pub root_history_capacity: u64,
    pub current_root_index: u64,
}

impl BatchedMerkleTreeAccount {
    pub fn size(&self) -> Result<usize> {
        let account_size = std::mem::size_of::<Self>();
        let root_history_size = (std::mem::size_of::<BoundedVecMetadata>()
            + std::mem::size_of::<[u8; 32]>())
            * self.root_history_capacity as usize;
        let size = account_size + root_history_size;
        Ok(size)
    }
}

// Instead of a rollover we just push roots into root history. TODO: figure out
// how we nullify in old trees. (it's probably easier to just stick to the
// current model.)
pub struct ZeroCopyBatchedMerkleTreeAccount<'a> {
    pub account: &'a mut BatchedMerkleTreeAccount,
    pub root_buffer: ManuallyDrop<CyclicBoundedVec<[u8; 32]>>,
    // pub root_history: ManuallyDrop<CyclicBoundedVec<[u8; 32]>>,
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
    // pub fn size(root_history_capacity: usize) -> usize {
    //     std::mem::size_of::<BatchedMerkleTreeAccount>()
    //         + std::mem::size_of::<CyclicBoundedVecMetadata>()
    //         + 32 * root_history_capacity
    // }

    // TODO: add from_account_info,  and from_account_loader
    pub fn from_account(
        account: &'a mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount<'a>> {
        if account_data.len() != account.size()? {
            return err!(AccountCompressionErrorCode::SizeMismatch);
        }
        let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>();
        let root_buffer = deserialize_cyclic_bounded_vec(account_data, &mut start_offset);
        Ok(ZeroCopyBatchedMerkleTreeAccount {
            account,
            root_buffer,
        })
    }

    pub fn init_from_account(
        account: &'a mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount<'a>> {
        if account_data.len() != account.size()? {
            return err!(AccountCompressionErrorCode::SizeMismatch);
        }
        let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>();
        let root_buffer: ManuallyDrop<CyclicBoundedVec<[u8; 32]>> = init_bounded_cyclic_vec(
            account.root_history_capacity as usize,
            account_data,
            &mut start_offset,
            false,
        );
        Ok(ZeroCopyBatchedMerkleTreeAccount {
            account,
            root_buffer,
        })
    }

    pub fn get_public_inputs_from_queue_account(
        &mut self,
        queue_account: &mut ZeroCopyBatchedAddressQueueAccount,
        instruction_data: &InstructionDataBatchUpdateProofInputs,
    ) -> Result<(BatchProofInputs, u8)> {
        let batch_capacity = queue_account.account.batch_size as usize;
        // for _ in 0..instruction_data.public_inputs.num_batches_in_proof {
        let batch = queue_account.get_next_full_batch()?;
        if !batch.is_ready_to_update_tree() {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        let public_inputs = BatchProofInputs {
            old_root: *self.root_buffer.first().unwrap_or(&[0u8; 32]),
            new_root: instruction_data.public_inputs.new_root,
            start_index: self.account.next_index,
            end_index: self.account.next_index + batch_capacity as u64,
            // * instruction_data.public_inputs.map_circuit_id() as u64,
            user_hash_chain: batch.user_hash_chain,
            input_hash_chain: batch.prover_hash_chain,
            output_hash_chain: batch.prover_hash_chain,
        };
        Ok((public_inputs, batch.id))
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

    // TODO: add flexibility for using multiple proofs to consume one queue batch.
    pub fn update(
        &mut self,
        queue_account: &mut BatchedAddressQueueAccount,
        queue_account_data: &mut [u8],
        instruction_data: InstructionDataBatchUpdateProofInputs,
    ) -> Result<()> {
        let mut queue_account =
            ZeroCopyBatchedAddressQueueAccount::from_account(queue_account, queue_account_data)
                .unwrap();

        let (public_inputs, batch_index) =
            self.get_public_inputs_from_queue_account(&mut queue_account, &instruction_data)?;
        if queue_account.batches[batch_index as usize].is_inserted {
            return err!(AccountCompressionErrorCode::BatchAlreadyInserted);
        } else if !queue_account.batches[batch_index as usize].is_ready_to_update_tree() {
            return err!(AccountCompressionErrorCode::BatchNotReady);
        }

        println!("batch_index: {}", batch_index);
        println!("batch : {:?}", queue_account.batches[batch_index as usize]);
        let public_input_hash = self.compress_public_inputs(&public_inputs)?;
        // TODO: replace with actual verification in light-verifier
        verify_mock_circuit(&public_input_hash, &instruction_data.compressed_proof)?;
        self.root_buffer.push(public_inputs.new_root);
        self.account.next_index = public_inputs.end_index;

        queue_account.batches[batch_index as usize].is_inserted = true;
        queue_account.batches[batch_index as usize].user_hash_chain = [0u8; 32];

        self.account.sequence_number += 1;
        Ok(())
    }
}

pub fn verify_mock_circuit(_public_input_hash: &[u8; 32], _proof: &CompressedProof) -> Result<()> {
    // 1. Recreate public input hash
    // end index == start index + batch_size * num_batches
    // execute nullify, indexed insert or append ciruit logic
    Ok(())
}

#[cfg(test)]
mod tests {

    use rand::{rngs::StdRng, Rng};

    use crate::{
        batched_queue::tests::get_test_account_and_account_data, AccessMetadata, QueueType,
        RolloverMetadata,
    };

    use super::*;

    pub fn get_test_mt_account_and_account_data(
        tree_type: TreeType,
        height: u64,
        root_history_capacity: u64,
    ) -> (BatchedMerkleTreeAccount, Vec<u8>) {
        let metadata = BatchedMerkleTreeMetadata {
            next_merkle_tree: Pubkey::new_unique(),
            access_metadata: AccessMetadata::default(),
            rollover_metadata: RolloverMetadata::default(),
            tree_type: tree_type as u64,
            associated_input_queue: Pubkey::new_unique(),
            associated_output_queue: Pubkey::new_unique(),
        };

        let account = BatchedMerkleTreeAccount {
            metadata: metadata.clone(),
            next_index: 0,
            sequence_number: 0,
            tree_type: tree_type as u64,
            height,
            root_history_capacity,
            current_root_index: 0,
        };
        let account_data: Vec<u8> = vec![0; account.size().unwrap()];
        (account, account_data)
    }

    fn assert_mt_zero_copy_inited(
        tree_type: TreeType,
        height: u64,
        root_history_capacity: u64,
        zero_copy_account: &ZeroCopyBatchedMerkleTreeAccount,
        account: &BatchedMerkleTreeAccount,
    ) {
        assert_eq!(*zero_copy_account.account, *account, "metadata mismatch");
        assert_eq!(
            zero_copy_account.root_buffer.capacity(),
            root_history_capacity as usize,
            "root_history_capacity mismatch"
        );
        assert_eq!(zero_copy_account.account.height, height, "height mismatch");
        assert_eq!(
            zero_copy_account.account.tree_type, tree_type as u64,
            "tree_type mismatch"
        );
        assert_eq!(zero_copy_account.account.next_index, 0, "next_index != 0");
        assert_eq!(
            zero_copy_account.account.sequence_number, 0,
            "sequence_number != 0"
        );
        assert_eq!(
            zero_copy_account.account.current_root_index, 0,
            "current_root_index != 0"
        );

        // if account.metadata.tree_type == TreeType::State as u64 {
        //     assert_eq!(zero_copy_account.value_vecs.len(), 0, "value_vecs mismatch");
        //     assert_eq!(
        //         zero_copy_account.value_vecs.capacity(),
        //         0,
        //         "value_vecs mismatch"
        //     );
        // }
        // else {
        //     assert_eq!(
        //         zero_copy_account.value_vecs.capacity(),
        //         num_batches,
        //         "value_vecs mismatch"
        //     );
        //     assert_eq!(
        //         zero_copy_account.value_vecs.len(),
        //         num_batches,
        //         "value_vecs mismatch"
        //     );
        // }
        // if account.metadata.queue_type == QueueType::Output as u64 {
        //     assert_eq!(
        //         zero_copy_account.bloomfilter_stores.capacity(),
        //         0,
        //         "bloomfilter_stores mismatch"
        //     );
        // } else {
        //     assert_eq!(
        //         zero_copy_account.bloomfilter_stores.capacity(),
        //         num_batches,
        //         "bloomfilter_stores mismatch"
        //     );
        //     assert_eq!(
        //         zero_copy_account.bloomfilter_stores.len(),
        //         num_batches,o
        //         "bloomfilter_stores mismatch"
        //     );
        // }

        // for vec in zero_copy_account.bloomfilter_stores.iter() {
        //     assert_eq!(
        //         vec.capacity(),
        //         account.bloom_filter_capacity as usize,
        //         "bloom_filter_capacity mismatch"
        //     );
        //     assert_eq!(
        //         vec.len(),
        //         account.bloom_filter_capacity as usize,
        //         "bloom_filter_capacity mismatch"
        //     );
        // }

        // for vec in zero_copy_account.value_vecs.iter() {
        //     assert_eq!(vec.capacity(), batch_size, "batch_size mismatch");
        //     assert_eq!(vec.len(), 0, "batch_size mismatch");
        // }
    }

    /// TODO: add infinite test with random values filling both input and output
    /// queues with a counter which keeps things below X tps and an if that
    /// executes tree updates when possible.
    #[test]
    fn test_mt_account_functional() {
        let queue_types = vec![QueueType::Input, QueueType::Output];
        for queue_type in queue_types {
            let batch_size = 100;
            let num_batches = 2;

            let height = 26;
            // Should be twice to num batches since num batches is the number input and output batches -> 2x
            let root_history_capacity = num_batches;
            let bloomfilter_capacity = 20_000 * 8;
            let bloomfilter_num_iters = 3;
            let (mut queue_account, mut queue_account_data) = get_test_account_and_account_data(
                batch_size,
                num_batches,
                queue_type,
                bloomfilter_capacity,
            );
            let pre_init_account_data = queue_account_data.clone();
            // Init
            {
                ZeroCopyBatchedAddressQueueAccount::init_from_account(
                    &mut queue_account,
                    &mut queue_account_data,
                    bloomfilter_num_iters,
                )
                .unwrap();
            }
            assert_ne!(queue_account_data, pre_init_account_data);

            // Fill queue with values
            for i in 0..batch_size {
                let mut zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
                    &mut queue_account,
                    &mut queue_account_data,
                )
                .unwrap();
                let mut value = [0u8; 32];
                value[24..].copy_from_slice(&i.to_le_bytes());
                zero_copy_account.insert_into_current_batch(&value).unwrap();
            }

            // assert values are in bloom filter and value vec
            {
                let mut zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
                    &mut queue_account,
                    &mut queue_account_data,
                )
                .unwrap();

                // value exists in bloom filter
                // value exists in value array
                for i in 0..batch_size {
                    let mut value = [0u8; 32];
                    value[24..].copy_from_slice(&i.to_le_bytes());
                    println!("value: {:?}", value);
                    if queue_type == QueueType::Output {
                        assert!(zero_copy_account.value_vecs[0]
                            .as_mut_slice()
                            .to_vec()
                            .contains(&value));
                    } else {
                        let mut bloomfilter = light_bloom_filter::BloomFilter::new(
                            zero_copy_account.batches[0].num_iters as usize,
                            zero_copy_account.batches[0].bloomfilter_capacity,
                            zero_copy_account.bloomfilter_stores[0].as_mut_slice(),
                        )
                        .unwrap();
                        assert!(bloomfilter.contains(&value));
                    } // TODO: assert for output queue
                      // assert!(zero_copy_account.value_vecs[0]
                      //     .as_mut_slice()
                      //     .to_vec()
                      //     .contains(&value));
                }
                println!(
                    "zero_copy_account.batches[0]: {:?}",
                    zero_copy_account.batches.get(0).unwrap()
                );
                // batch ready
                assert!(zero_copy_account.batches[0].is_ready_to_update_tree());
            }

            for tree_type in vec![TreeType::State] {
                println!("root_history_capacity: {}", root_history_capacity);
                let (mut account, mut account_data) =
                    get_test_mt_account_and_account_data(tree_type, height, root_history_capacity);
                let ref_account = account.clone();
                let mut zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::init_from_account(
                    &mut account,
                    &mut account_data,
                )
                .unwrap();

                assert_mt_zero_copy_inited(
                    tree_type,
                    height,
                    root_history_capacity,
                    &zero_copy_account,
                    &ref_account,
                );
                let instruction_data = InstructionDataBatchUpdateProofInputs {
                    public_inputs: BatchProofInputsIx {
                        circuit_id: 1,
                        new_root: [1u8; 32],
                        output_hash_chain: [1u8; 32],
                        root_index: 0,
                    },
                    compressed_proof: CompressedProof::default(),
                };
                zero_copy_account
                    .update(
                        &mut queue_account,
                        &mut queue_account_data,
                        instruction_data,
                    )
                    .unwrap();
                // assert merkle tree
                {
                    println!("self.root_buffer: {:?}", zero_copy_account.root_buffer);
                    // There should be a root now
                    assert_eq!(
                        *zero_copy_account.root_buffer.get(0).unwrap(),
                        instruction_data.public_inputs.new_root
                    );
                    // sequence number + 1
                    assert_eq!(zero_copy_account.account.sequence_number, 1);
                }
                // assert queue account
                {
                    // Second batch proof should fail
                    assert!(zero_copy_account
                        .update(
                            &mut queue_account,
                            &mut queue_account_data,
                            instruction_data,
                        )
                        .is_err(),);
                    let mut zero_copy_queue_account =
                        ZeroCopyBatchedAddressQueueAccount::from_account(
                            &mut queue_account,
                            &mut queue_account_data,
                        )
                        .unwrap();
                    assert!(!zero_copy_queue_account.batches[0].is_ready_to_update_tree());
                    // New inserts should go to Batch 1
                    {
                        assert_eq!(zero_copy_queue_account.batches[1].num_inserted, 0);
                        println!(
                            "zero_copy_queue_account.batches[0]: {:?}",
                            zero_copy_queue_account.batches[0]
                        );
                        println!(
                            "zero_copy_queue_account.batches[1]: {:?}",
                            zero_copy_queue_account.batches[1]
                        );

                        let mut value = [0u8; 32];
                        value[24..].copy_from_slice(&3232u64.to_le_bytes());
                        zero_copy_queue_account
                            .insert_into_current_batch(&value)
                            .unwrap();
                        println!(
                            "zero_copy_queue_account.batches[0]: {:?}",
                            zero_copy_queue_account.batches[0]
                        );
                        println!(
                            "zero_copy_queue_account.batches[1]: {:?}",
                            zero_copy_queue_account.batches[1]
                        );
                        assert_eq!(zero_copy_queue_account.batches[1].num_inserted, 1);
                    }
                    // As soon as the current input batch switches the sequence
                    // number needs to catch up to the sequence number of the next
                    // input batch so that it is unlocked.
                    // For output batches it doesn't matter.
                    // TODO: unify queue and tree accounts there is no benefit in keeping these separate.
                    /**
                     * 1. Insert values into output queue until it's full
                     * 2. Update Merkle tree with output queue
                     * - it was always possible to spend values from the output queue, output queue must be updated before input queue
                     * -> output queues can always be updated
                     * -> input queues can only be updated if the input is already in the tree
                     * -> this mess can be avoided by zeroing the value in the output queue and not inserting
                     * it into the input queue if the output batch has not been inserted into the tree yet
                     * 3.
                     */
                    struct Dummy;
                }
            }
        }
    }

    /// TODO: add input queue
    /// TODO: add complete asserts
    /// TODO: do full sweep and write specs
    /// TODO: add circuits for update
    /// queues with a counter which keeps things below X tps and an if that
    /// executes tree updates when possible.
    #[test]
    fn test_e2e() {
        let num_tx = 220000;
        let batch_size = 6000;
        let num_batches = 2;

        let height = 26;
        // Should be twice to num batches since num batches is the number input and output batches -> 2x
        let root_history_capacity = num_batches as u64;
        let bloomfilter_capacity = 200_000 * 8;
        let bloomfilter_num_iters = 3;
        let (mut input_queue_account, mut input_queue_account_data) =
            get_test_account_and_account_data(
                batch_size,
                num_batches,
                QueueType::Input,
                bloomfilter_capacity,
            );
        ZeroCopyBatchedAddressQueueAccount::init_from_account(
            &mut input_queue_account,
            &mut input_queue_account_data,
            bloomfilter_num_iters,
        )
        .unwrap();

        let (mut output_queue_account, mut output_queue_account_data) =
            get_test_account_and_account_data(
                batch_size,
                num_batches,
                QueueType::Output,
                bloomfilter_capacity,
            );
        ZeroCopyBatchedAddressQueueAccount::init_from_account(
            &mut output_queue_account,
            &mut output_queue_account_data,
            bloomfilter_num_iters,
        )
        .unwrap();

        let (mut mt_account, mut mt_account_data) =
            get_test_mt_account_and_account_data(TreeType::State, height, root_history_capacity);
        ZeroCopyBatchedMerkleTreeAccount::init_from_account(&mut mt_account, &mut mt_account_data)
            .unwrap();
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);
        let mut in_ready_for_update = false;
        let mut out_ready_for_update = false;
        let mut num_output_updates = 0;
        let mut num_input_updates = 0;
        let mut num_input_values = 0;
        let mut num_output_values = 0;
        let mut current_batch = 0;

        for tx in 0..num_tx {
            println!("tx: {}", tx);
            println!("num_input_updates: {}", num_input_updates);
            println!("num_output_updates: {}", num_output_updates);
            let mut input_zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
                &mut input_queue_account,
                &mut input_queue_account_data,
            )
            .unwrap();

            if rng.gen_bool(0.5) && current_batch == 0 {
                println!("Input insert -----------------------------");

                let rnd_bytes = get_rnd_bytes(&mut rng);
                println!("rnd_bytes: {:?}", rnd_bytes);
                // println!(
                //     "is full: {:?}",
                //     input_zero_copy_account.batches[0].is_ready_to_update_tree()
                // );
                // println!(
                //     "is full: {:?}",
                //     input_zero_copy_account.batches[1].is_ready_to_update_tree()
                // );

                input_zero_copy_account
                    .insert_into_current_batch(&rnd_bytes.to_vec().try_into().unwrap())
                    .unwrap();
                let current_batch_index = input_zero_copy_account
                    .account
                    .currently_processing_batch_index
                    as usize;
                let other_batch = if current_batch_index == 0 { 1 } else { 0 };

                // New value does not exist in the other batch bloom filter
                let mut bloomfilter = light_bloom_filter::BloomFilter::new(
                    input_zero_copy_account.batches[other_batch].num_iters as usize,
                    input_zero_copy_account.batches[other_batch].bloomfilter_capacity,
                    input_zero_copy_account.bloomfilter_stores[other_batch].as_mut_slice(),
                )
                .unwrap();
                assert!(!bloomfilter.contains(&rnd_bytes));
                // New value exists in the current batch bloom filter
                let mut bloomfilter = light_bloom_filter::BloomFilter::new(
                    input_zero_copy_account.batches[current_batch_index].num_iters as usize,
                    input_zero_copy_account.batches[current_batch_index].bloomfilter_capacity,
                    input_zero_copy_account.bloomfilter_stores[current_batch_index].as_mut_slice(),
                )
                .unwrap();
                assert!(bloomfilter.contains(&rnd_bytes));
                num_input_values += 1;
            }
            in_ready_for_update = input_zero_copy_account.batches[0].is_ready_to_update_tree()
                || input_zero_copy_account.batches[1].is_ready_to_update_tree();
            let mut output_zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
                &mut output_queue_account,
                &mut output_queue_account_data,
            )
            .unwrap();
            if output_zero_copy_account.batches[0].is_ready_to_update_tree() == true
                && true == output_zero_copy_account.batches[1].is_ready_to_update_tree()
            {
                println!("batch 0 ready: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1 ready: {:?}", output_zero_copy_account.batches[1]);
                panic!("output batches are both ready to update");
            }
            if output_zero_copy_account.batches[0].is_ready_to_update_tree() {
                current_batch = 0;
            } else if output_zero_copy_account.batches[1].is_ready_to_update_tree() {
                current_batch = 1;
            }

            if rng.gen_bool(0.5) && current_batch == 1 {
                println!("Output insert -----------------------------");
                let rnd_bytes = get_rnd_bytes(&mut rng);
                output_zero_copy_account
                    .insert_into_current_batch(&rnd_bytes)
                    .unwrap();
                let current_batch_index = output_zero_copy_account
                    .account
                    .currently_processing_batch_index
                    as usize;
                let other_batch = if current_batch_index == 0 { 1 } else { 0 };
                assert!(output_zero_copy_account.value_vecs[current_batch_index]
                    .as_mut_slice()
                    .to_vec()
                    .contains(&rnd_bytes));
                assert!(!output_zero_copy_account.value_vecs[other_batch]
                    .as_mut_slice()
                    .to_vec()
                    .contains(&rnd_bytes));
                num_output_values += 1;
            }
            if output_zero_copy_account.batches[0].is_ready_to_update_tree() {
                current_batch = 0;
            } else if output_zero_copy_account.batches[1].is_ready_to_update_tree() {
                current_batch = 1;
            }
            out_ready_for_update = output_zero_copy_account.batches[0].is_ready_to_update_tree()
                || output_zero_copy_account.batches[1].is_ready_to_update_tree();
            // if rng.gen_bool(0.5) | in_ready_for_update | out_ready_for_update {
            let mut zero_copy_account = ZeroCopyBatchedMerkleTreeAccount::from_account(
                &mut mt_account,
                &mut mt_account_data,
            )
            .unwrap();

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
            let input_res = zero_copy_account.update(
                &mut input_queue_account,
                &mut input_queue_account_data,
                instruction_data,
            );
            if !in_ready_for_update {
                assert!(input_res.is_err());
            } else {
                assert!(input_res.is_ok());
                in_ready_for_update = false;
                num_input_updates += 1;
                println!("input update success {}", num_input_updates);
                println!("num_output_values: {}", num_output_values);
                println!("num_input_values: {}", num_input_values);
                let input_zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
                    &mut input_queue_account,
                    &mut input_queue_account_data,
                )
                .unwrap();
                println!("batch 0: {:?}", input_zero_copy_account.batches[0].clone());
                println!("batch 1: {:?}", input_zero_copy_account.batches[1].clone());
            }
            let mut pre_account_state = output_queue_account.clone();
            let mut pre_output_queue_state = output_queue_account_data.clone();
            let output_res = zero_copy_account.update(
                &mut pre_account_state,
                &mut pre_output_queue_state,
                instruction_data,
            );
            println!(
                "post update: sequence number: {}",
                zero_copy_account.account.sequence_number
            );
            if !out_ready_for_update {
                assert!(output_res.is_err());
                // assert_ne!(output_queue_account_data, pre_output_queue_state);
                // assert_ne!(output_queue_account, pre_account_state);
            } else {
                assert!(output_res.is_ok());
                output_queue_account = pre_account_state;
                output_queue_account_data = pre_output_queue_state;
                out_ready_for_update = false;
                num_output_updates += 1;
                println!("output update success {}", num_output_updates);
                println!("num_output_values: {}", num_output_values);
                println!("num_input_values: {}", num_input_values);
                let output_zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
                    &mut output_queue_account,
                    &mut output_queue_account_data,
                )
                .unwrap();
                println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
                println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
            }
            // }
            // }
        }
        let output_zero_copy_account = ZeroCopyBatchedAddressQueueAccount::from_account(
            &mut output_queue_account,
            &mut output_queue_account_data,
        )
        .unwrap();
        println!("batch 0: {:?}", output_zero_copy_account.batches[0]);
        println!("batch 1: {:?}", output_zero_copy_account.batches[1]);
    }
    pub fn get_rnd_bytes(rng: &mut StdRng) -> [u8; 32] {
        let mut rnd_bytes = rng.gen::<[u8; 32]>();
        rnd_bytes[0] = 0;
        rnd_bytes
    }
}
