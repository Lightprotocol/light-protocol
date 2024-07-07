use crate::{
    batched_queue::{deserialize_cyclic_bounded_vec, ZeroCopyBatchedAddressQueueAccount},
    errors::AccountCompressionErrorCode,
    MerkleTreeMetadata,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::{BoundedVecMetadata, CyclicBoundedVec, CyclicBoundedVecMetadata};
use light_hasher::{Hasher, Poseidon};
use light_verifier::CompressedProof;
use std::mem::ManuallyDrop;

pub enum TreeType {
    State = 1,
    Address = 2,
}

#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct BatchedMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
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

pub struct ZeroCopyBatchedMerkleTreeAccount<'a> {
    pub account: &'a mut BatchedMerkleTreeAccount,
    pub root_history: ManuallyDrop<CyclicBoundedVec<[u8; 32]>>,
}

/// Get batch from account.
/// Hash all public inputs into one poseidon hash.
/// Public inputs:
/// 1. old root (get from account by index)
/// 2. new root (send to chain and )
/// 3. start index (get from batch)
/// 4. end index (get from batch start index plus batch size)
pub struct InstructionDataBatchUpdateProofInputs {
    pub public_inputs: BatchProofInputsIx,
    pub compressed_proof: CompressedProof,
}

pub struct BatchProofInputsIx {
    pub num_batches_in_proof: u16,
    pub new_root: [u8; 32],
}

pub struct BatchProofInputs {
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
    pub start_index: u64,
    pub end_index: u64,
    pub hash_chains: Vec<[u8; 32]>,
}

impl<'a> ZeroCopyBatchedMerkleTreeAccount<'a> {
    pub fn size(root_history_capacity: usize) -> usize {
        std::mem::size_of::<BatchedMerkleTreeAccount>()
            + std::mem::size_of::<CyclicBoundedVecMetadata>()
            + 32 * root_history_capacity
    }

    // TODO: add from_account_info,  and from_account_loader
    pub fn from_account(
        account: &'a mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount<'a>> {
        if account_data.len() != Self::size(account.root_history_capacity as usize) {
            return err!(AccountCompressionErrorCode::SizeMismatch);
        }
        let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>();
        let root_history = deserialize_cyclic_bounded_vec(account_data, &mut start_offset);
        Ok(ZeroCopyBatchedMerkleTreeAccount {
            account,
            root_history,
        })
    }

    pub fn init_from_account(
        account: &'a mut BatchedMerkleTreeAccount,
        account_data: &mut [u8],
        root_history_capacity: usize,
    ) -> Result<ZeroCopyBatchedMerkleTreeAccount<'a>> {
        if account_data.len() != Self::size(root_history_capacity) {
            return err!(AccountCompressionErrorCode::SizeMismatch);
        }
        let mut start_offset = std::mem::size_of::<BatchedMerkleTreeAccount>();
        let root_history = deserialize_cyclic_bounded_vec(account_data, &mut start_offset);
        Ok(ZeroCopyBatchedMerkleTreeAccount {
            account,
            root_history,
        })
    }

    pub fn get_public_inputs_from_queue_account(
        &mut self,
        queue_account: &mut ZeroCopyBatchedAddressQueueAccount,
        instruction_data: &InstructionDataBatchUpdateProofInputs,
    ) -> Result<BatchProofInputs> {
        let mut hash_chains = Vec::new();
        let batch_capacity = queue_account.account.batch_size as usize;
        for _ in 0..instruction_data.public_inputs.num_batches_in_proof {
            let batch = queue_account.get_next_full_batch()?;
            if !batch.is_ready_to_update_tree() {
                return err!(AccountCompressionErrorCode::BatchNotReady);
            }
            let sequence_threshold = self.account.root_history_capacity;
            batch.mark_with_sequence_number(self.account.sequence_number, sequence_threshold);
            hash_chains.push(batch.hash_chain)
        }
        let public_inputs = BatchProofInputs {
            old_root: *self.root_history.first().unwrap(),
            new_root: instruction_data.public_inputs.new_root,
            start_index: self.account.next_index,
            end_index: self.account.next_index
                + batch_capacity as u64
                    * instruction_data.public_inputs.num_batches_in_proof as u64,
            hash_chains,
        };
        Ok(public_inputs)
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

        let mut current_hash_chain_hash = public_inputs.hash_chains[0];
        // If only one batch no need to hash
        for hash_chain in public_inputs.hash_chains.iter().skip(1) {
            current_hash_chain_hash = Poseidon::hashv(&[&current_hash_chain_hash, hash_chain])
                .map_err(ProgramError::from)?;
        }
        // Poseidon hashes are cheaper than public inputs.
        let public_input_hash = Poseidon::hashv(&[&meta_poseidon_hash, &current_hash_chain_hash])
            .map_err(ProgramError::from)?;

        Ok(public_input_hash)
    }

    pub fn update(
        &mut self,
        queue_account: &mut ZeroCopyBatchedAddressQueueAccount,
        instruction_data: InstructionDataBatchUpdateProofInputs,
    ) -> Result<()> {
        // Increment sequence number here because we mark the batch with
        // sequence number already in get_public_inputs_from_queue_account.
        self.account.sequence_number += 1;

        let public_inputs =
            self.get_public_inputs_from_queue_account(queue_account, &instruction_data)?;

        let public_input_hash = self.compress_public_inputs(&public_inputs)?;
        // TODO: replace with actual verification in light-verifier
        verify_mock_circuit(&public_input_hash, &instruction_data.compressed_proof)?;
        self.root_history.push(public_inputs.new_root);
        self.account.next_index = public_inputs.end_index;

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

    use crate::{
        batched_queue::{BatchedAddressQueueAccount, ZeroCopyBatchedAddressQueueAccount},
        AccessMetadata, QueueMetadata, QueueType, RolloverMetadata,
    };

    use super::*;

    pub fn get_test_account_and_account_data(
        batch_size: u64,
        num_batches: u64,
        queue_type: QueueType,
    ) -> (BatchedAddressQueueAccount, Vec<u8>) {
        let metadata = QueueMetadata {
            next_queue: Pubkey::new_unique(),
            access_metadata: AccessMetadata::default(),
            rollover_metadata: RolloverMetadata::default(),
            queue_type: queue_type as u64,
            associated_merkle_tree: Pubkey::new_unique(),
        };

        let account = BatchedAddressQueueAccount {
            metadata: metadata.clone(),
            batch_size: batch_size as u64,
            num_batches: num_batches as u64,
            currently_processing_batch_index: 0,
            next_index: 0,
            sequence_number: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity: 20_000,
        };
        let account_data: Vec<u8> = vec![0; account.size().unwrap()];
        (account, account_data)
    }

    fn assert_queue_zero_copy_inited(
        batch_size: usize,
        num_batches: usize,
        zero_copy_account: &ZeroCopyBatchedAddressQueueAccount,
        account: &BatchedAddressQueueAccount,
    ) {
        assert_eq!(zero_copy_account.account, account, "metadata mismatch");
        assert_eq!(
            zero_copy_account.batches.len(),
            num_batches,
            "batches mismatch"
        );
        if account.metadata.queue_type == QueueType::Input as u64 {
            assert_eq!(zero_copy_account.value_vecs.len(), 0, "value_vecs mismatch");
            assert_eq!(
                zero_copy_account.value_vecs.capacity(),
                0,
                "value_vecs mismatch"
            );
        } else {
            assert_eq!(
                zero_copy_account.value_vecs.capacity(),
                num_batches,
                "value_vecs mismatch"
            );
            assert_eq!(
                zero_copy_account.value_vecs.len(),
                num_batches,
                "value_vecs mismatch"
            );
        }
        if account.metadata.queue_type == QueueType::Output as u64 {
            assert_eq!(
                zero_copy_account.bloomfilter_stores.capacity(),
                0,
                "bloomfilter_stores mismatch"
            );
        } else {
            assert_eq!(
                zero_copy_account.bloomfilter_stores.capacity(),
                num_batches,
                "bloomfilter_stores mismatch"
            );
            assert_eq!(
                zero_copy_account.bloomfilter_stores.len(),
                num_batches,
                "bloomfilter_stores mismatch"
            );
        }

        for vec in zero_copy_account.bloomfilter_stores.iter() {
            assert_eq!(
                vec.capacity(),
                account.bloom_filter_capacity as usize,
                "bloom_filter_capacity mismatch"
            );
            assert_eq!(
                vec.len(),
                account.bloom_filter_capacity as usize,
                "bloom_filter_capacity mismatch"
            );
        }

        for vec in zero_copy_account.value_vecs.iter() {
            assert_eq!(vec.capacity(), batch_size, "batch_size mismatch");
            assert_eq!(vec.len(), 0, "batch_size mismatch");
        }
    }

    #[test]
    fn test_unpack_output_queue_account() {
        let batch_size = 2;
        let num_batches = 2;
        let bloomfilter_capacity = 20_000 * 8;
        let bloomfilter_num_iters = 3;
        for queue_type in vec![QueueType::Input, QueueType::Output, QueueType::Address] {
            let (mut account, mut account_data) =
                get_test_account_and_account_data(batch_size, num_batches, queue_type);
            let ref_account = account.clone();
            let mut zero_copy_account = ZeroCopyBatchedAddressQueueAccount::init_from_account(
                &mut account,
                &mut account_data,
                bloomfilter_num_iters,
                bloomfilter_capacity,
            )
            .unwrap();

            assert_queue_zero_copy_inited(
                batch_size as usize,
                num_batches as usize,
                &zero_copy_account,
                &ref_account,
            );
            let value = [1u8; 32];
            println!("queue_type: {:?}", queue_type);
            assert!(zero_copy_account.insert_into_current_batch(&value).is_ok());
            if queue_type != QueueType::Output {
                assert!(zero_copy_account.insert_into_current_batch(&value).is_err());
            }
            // TODO: add full assert
        }
    }
}
