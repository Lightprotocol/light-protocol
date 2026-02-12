use std::ops::Deref;

use light_account_checks::{
    checks::check_account_info,
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
    AccountInfoTrait,
};
use light_compressed_account::{pubkey::Pubkey, OUTPUT_STATE_QUEUE_TYPE_V2};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_zero_copy::{errors::ZeroCopyError, vec::ZeroCopyVecU64};
use zerocopy::Ref;

use crate::{
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID,
    errors::BatchedMerkleTreeError,
    queue::{BatchedQueueAccount, BatchedQueueMetadata},
};

/// Immutable batched queue reference.
///
/// Uses `try_borrow_data()` + `&'a [u8]` instead of
/// `try_borrow_mut_data()` + `&'a mut [u8]`.
///
/// Only contains the fields that external consumers actually read:
/// metadata and value vecs. Hash chain stores are not parsed.
#[derive(Debug)]
pub struct BatchedQueueRef<'a> {
    pubkey: Pubkey,
    metadata: Ref<&'a [u8], BatchedQueueMetadata>,
    /// Value vec metadata: [length, capacity] per batch, parsed inline.
    _value_vec_metas: [Ref<&'a [u8], [u64; 2]>; 2],
    value_vec_data: [Ref<&'a [u8], [[u8; 32]]>; 2],
}

impl Discriminator for BatchedQueueRef<'_> {
    const LIGHT_DISCRIMINATOR: [u8; 8] = *b"queueacc";
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = b"queueacc";
}

impl<'a> BatchedQueueRef<'a> {
    /// Deserialize an output queue (immutable) from account info.
    pub fn output_from_account_info<A: AccountInfoTrait>(
        account_info: &A,
    ) -> Result<BatchedQueueRef<'a>, BatchedMerkleTreeError> {
        Self::from_account_info::<OUTPUT_STATE_QUEUE_TYPE_V2, A>(
            &Pubkey::new_from_array(ACCOUNT_COMPRESSION_PROGRAM_ID),
            account_info,
        )
    }

    pub(crate) fn from_account_info<const QUEUE_TYPE: u64, A: AccountInfoTrait>(
        program_id: &Pubkey,
        account_info: &A,
    ) -> Result<BatchedQueueRef<'a>, BatchedMerkleTreeError> {
        check_account_info::<BatchedQueueAccount, A>(&program_id.to_bytes(), account_info)?;
        let data = account_info.try_borrow_data()?;
        // SAFETY: We extend the lifetime of the borrowed data to 'a.
        // The borrow is shared (immutable), so dropping the Ref guard
        // restores pinocchio's borrow state correctly for shared borrows.
        let data_slice: &'a [u8] = unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };
        Self::from_bytes::<QUEUE_TYPE>(data_slice, account_info.key().into())
    }

    /// Deserialize an output queue (immutable) from bytes.
    #[cfg(not(target_os = "solana"))]
    pub fn output_from_bytes(
        account_data: &'a [u8],
    ) -> Result<BatchedQueueRef<'a>, BatchedMerkleTreeError> {
        light_account_checks::checks::check_discriminator::<BatchedQueueAccount>(account_data)?;
        Self::from_bytes::<OUTPUT_STATE_QUEUE_TYPE_V2>(account_data, Pubkey::default())
    }

    pub(crate) fn from_bytes<const QUEUE_TYPE: u64>(
        account_data: &'a [u8],
        pubkey: Pubkey,
    ) -> Result<BatchedQueueRef<'a>, BatchedMerkleTreeError> {
        // 1. Skip discriminator.
        let (_discriminator, account_data) = account_data.split_at(DISCRIMINATOR_LEN);

        // 2. Parse metadata.
        let (metadata, account_data) =
            Ref::<&'a [u8], BatchedQueueMetadata>::from_prefix(account_data)
                .map_err(ZeroCopyError::from)?;

        if metadata.metadata.queue_type != QUEUE_TYPE {
            return Err(MerkleTreeMetadataError::InvalidQueueType.into());
        }

        // 3. Parse two value vecs inline.
        //    ZeroCopyVecU64 layout: [u64; 2] metadata (length, capacity), then [u8; 32] * capacity.
        let metadata_size = ZeroCopyVecU64::<[u8; 32]>::metadata_size();

        let (meta0_bytes, account_data) = account_data.split_at(metadata_size);
        let (value_vec_meta0, _padding) =
            Ref::<&'a [u8], [u64; 2]>::from_prefix(meta0_bytes).map_err(ZeroCopyError::from)?;
        let capacity0 = value_vec_meta0[1] as usize; // CAPACITY_INDEX = 1
        let (value_vec_data0, account_data) =
            Ref::<&'a [u8], [[u8; 32]]>::from_prefix_with_elems(account_data, capacity0)
                .map_err(ZeroCopyError::from)?;

        let (meta1_bytes, account_data) = account_data.split_at(metadata_size);
        let (value_vec_meta1, _padding) =
            Ref::<&'a [u8], [u64; 2]>::from_prefix(meta1_bytes).map_err(ZeroCopyError::from)?;
        let capacity1 = value_vec_meta1[1] as usize;
        let (value_vec_data1, _account_data) =
            Ref::<&'a [u8], [[u8; 32]]>::from_prefix_with_elems(account_data, capacity1)
                .map_err(ZeroCopyError::from)?;

        // 4. Stop here -- hash_chain_stores are not needed for read-only access.

        Ok(BatchedQueueRef {
            pubkey,
            metadata,
            _value_vec_metas: [value_vec_meta0, value_vec_meta1],
            value_vec_data: [value_vec_data0, value_vec_data1],
        })
    }

    /// Proves inclusion of leaf index if it exists in one of the batches.
    /// Returns true if leaf index exists in one of the batches.
    pub fn prove_inclusion_by_index(
        &self,
        leaf_index: u64,
        hash_chain_value: &[u8; 32],
    ) -> Result<bool, BatchedMerkleTreeError> {
        if leaf_index >= self.batch_metadata.next_index {
            return Err(BatchedMerkleTreeError::InvalidIndex);
        }
        for (batch_index, batch) in self.batch_metadata.batches.iter().enumerate() {
            if batch.leaf_index_exists(leaf_index) {
                let index = batch.get_value_index_in_batch(leaf_index)?;
                let element = self.value_vec_data[batch_index]
                    .get(index as usize)
                    .ok_or(BatchedMerkleTreeError::InclusionProofByIndexFailed)?;

                if *element == *hash_chain_value {
                    return Ok(true);
                } else {
                    #[cfg(target_os = "solana")]
                    {
                        solana_msg::msg!(
                            "Index found but value doesn't match leaf_index {} compressed account hash: {:?} expected compressed account hash {:?}. (If the expected element is [0u8;32] it was already spent. Other possibly causes, data hash, discriminator, leaf index, or Merkle tree mismatch.)",
                            leaf_index,
                            hash_chain_value, *element
                        );
                    }
                    return Err(BatchedMerkleTreeError::InclusionProofByIndexFailed);
                }
            }
        }
        Ok(false)
    }

    /// Check if the pubkey is the associated Merkle tree of the queue.
    pub fn check_is_associated(&self, pubkey: &Pubkey) -> Result<(), BatchedMerkleTreeError> {
        if self.metadata.metadata.associated_merkle_tree != *pubkey {
            return Err(MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into());
        }
        Ok(())
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }
}

impl Deref for BatchedQueueRef<'_> {
    type Target = BatchedQueueMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}
