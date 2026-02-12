use std::ops::Deref;

use light_account_checks::{
    checks::check_account_info,
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
    AccountInfoTrait,
};
use light_compressed_account::{
    pubkey::Pubkey, ADDRESS_MERKLE_TREE_TYPE_V2, STATE_MERKLE_TREE_TYPE_V2,
};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_zero_copy::{cyclic_vec::ZeroCopyCyclicVecU64, errors::ZeroCopyError};
use zerocopy::Ref;

use crate::{
    batch::Batch, constants::ACCOUNT_COMPRESSION_PROGRAM_ID, errors::BatchedMerkleTreeError,
    merkle_tree::BatchedMerkleTreeAccount, merkle_tree_metadata::BatchedMerkleTreeMetadata,
};

/// Immutable batched Merkle tree reference.
///
/// Uses `try_borrow_data()` + `&'a [u8]` instead of
/// `try_borrow_mut_data()` + `&'a mut [u8]`, avoiding UB from
/// dropping a `RefMut` guard while a raw-pointer-based mutable
/// reference continues to live.
///
/// Only contains the fields that external consumers actually read:
/// metadata, root history, and bloom filter stores.
/// Hash chain stores are not parsed (only needed inside account-compression).
#[derive(Debug)]
pub struct BatchedMerkleTreeRef<'a> {
    pubkey: Pubkey,
    metadata: Ref<&'a [u8], BatchedMerkleTreeMetadata>,
    /// Cyclic vec metadata: [current_index, length, capacity]
    _root_history_meta: Ref<&'a [u8], [u64; 3]>,
    root_history_data: Ref<&'a [u8], [[u8; 32]]>,
    pub bloom_filter_stores: [&'a [u8]; 2],
}

impl Discriminator for BatchedMerkleTreeRef<'_> {
    const LIGHT_DISCRIMINATOR: [u8; 8] = *b"BatchMta";
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = b"BatchMta";
}

impl<'a> BatchedMerkleTreeRef<'a> {
    /// Deserialize a batched state Merkle tree (immutable) from account info.
    pub fn state_from_account_info<A: AccountInfoTrait>(
        account_info: &A,
    ) -> Result<BatchedMerkleTreeRef<'a>, BatchedMerkleTreeError> {
        Self::from_account_info::<STATE_MERKLE_TREE_TYPE_V2, A>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    /// Deserialize an address tree (immutable) from account info.
    pub fn address_from_account_info<A: AccountInfoTrait>(
        account_info: &A,
    ) -> Result<BatchedMerkleTreeRef<'a>, BatchedMerkleTreeError> {
        Self::from_account_info::<ADDRESS_MERKLE_TREE_TYPE_V2, A>(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            account_info,
        )
    }

    pub(crate) fn from_account_info<const TREE_TYPE: u64, A: AccountInfoTrait>(
        program_id: &[u8; 32],
        account_info: &A,
    ) -> Result<BatchedMerkleTreeRef<'a>, BatchedMerkleTreeError> {
        check_account_info::<BatchedMerkleTreeAccount, A>(program_id, account_info)?;
        let data = account_info.try_borrow_data()?;
        // SAFETY: We extend the lifetime of the borrowed data to 'a.
        // The borrow is shared (immutable), so dropping the Ref guard
        // restores pinocchio's borrow state correctly for shared borrows.
        let data_slice: &'a [u8] = unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };
        Self::from_bytes::<TREE_TYPE>(data_slice, &account_info.key().into())
    }

    /// Deserialize a state tree (immutable) from bytes.
    #[cfg(not(target_os = "solana"))]
    pub fn state_from_bytes(
        account_data: &'a [u8],
        pubkey: &Pubkey,
    ) -> Result<BatchedMerkleTreeRef<'a>, BatchedMerkleTreeError> {
        light_account_checks::checks::check_discriminator::<BatchedMerkleTreeAccount>(
            account_data,
        )?;
        Self::from_bytes::<STATE_MERKLE_TREE_TYPE_V2>(account_data, pubkey)
    }

    /// Deserialize an address tree (immutable) from bytes.
    #[cfg(not(target_os = "solana"))]
    pub fn address_from_bytes(
        account_data: &'a [u8],
        pubkey: &Pubkey,
    ) -> Result<BatchedMerkleTreeRef<'a>, BatchedMerkleTreeError> {
        light_account_checks::checks::check_discriminator::<BatchedMerkleTreeAccount>(
            account_data,
        )?;
        Self::from_bytes::<ADDRESS_MERKLE_TREE_TYPE_V2>(account_data, pubkey)
    }

    pub(crate) fn from_bytes<const TREE_TYPE: u64>(
        account_data: &'a [u8],
        pubkey: &Pubkey,
    ) -> Result<BatchedMerkleTreeRef<'a>, BatchedMerkleTreeError> {
        // 1. Skip discriminator.
        let (_discriminator, account_data) = account_data.split_at(DISCRIMINATOR_LEN);

        // 2. Parse metadata.
        let (metadata, account_data) =
            Ref::<&'a [u8], BatchedMerkleTreeMetadata>::from_prefix(account_data)
                .map_err(ZeroCopyError::from)?;
        if metadata.tree_type != TREE_TYPE {
            return Err(MerkleTreeMetadataError::InvalidTreeType.into());
        }

        // 3. Parse cyclic vec (root history) inline.
        //    Layout: [u64; 3] metadata (current_index, length, capacity), then [u8; 32] * capacity.
        let metadata_size = ZeroCopyCyclicVecU64::<[u8; 32]>::metadata_size();
        let (meta_bytes, account_data) = account_data.split_at(metadata_size);
        let (root_history_meta, _padding) =
            Ref::<&'a [u8], [u64; 3]>::from_prefix(meta_bytes).map_err(ZeroCopyError::from)?;
        let capacity = root_history_meta[2] as usize; // CAPACITY_INDEX = 2
        let (root_history_data, account_data) =
            Ref::<&'a [u8], [[u8; 32]]>::from_prefix_with_elems(account_data, capacity)
                .map_err(ZeroCopyError::from)?;

        // 4. Parse bloom filter stores (immutable).
        let bloom_filter_size = metadata.queue_batches.get_bloomfilter_size_bytes();
        let (bf_store_0, account_data) = account_data.split_at(bloom_filter_size);
        let (bf_store_1, _account_data) = account_data.split_at(bloom_filter_size);

        // 5. Stop here -- hash_chain_stores are not needed for read-only access.

        Ok(BatchedMerkleTreeRef {
            pubkey: *pubkey,
            metadata,
            _root_history_meta: root_history_meta,
            root_history_data,
            bloom_filter_stores: [bf_store_0, bf_store_1],
        })
    }

    /// Check non-inclusion in all bloom filters which are not zeroed.
    pub fn check_input_queue_non_inclusion(
        &self,
        value: &[u8; 32],
    ) -> Result<(), BatchedMerkleTreeError> {
        for i in 0..self.queue_batches.num_batches as usize {
            Batch::check_non_inclusion_ref(
                self.queue_batches.batches[i].num_iters as usize,
                self.queue_batches.batches[i].bloom_filter_capacity,
                value,
                self.bloom_filter_stores[i],
            )?;
        }
        Ok(())
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }
}

impl Deref for BatchedMerkleTreeRef<'_> {
    type Target = BatchedMerkleTreeMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

/// Provide index access to root_history for compatibility with
/// existing code that does `merkle_tree.root_history[index]`.
pub struct RootHistoryAccess<'a> {
    data: &'a Ref<&'a [u8], [[u8; 32]]>,
}

impl<'a> std::ops::Index<usize> for RootHistoryAccess<'a> {
    type Output = [u8; 32];

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<'a> BatchedMerkleTreeRef<'a> {
    /// Access root_history with index syntax for compatibility with
    /// code like `merkle_tree.root_history[root_index as usize]`.
    pub fn root_history(&self) -> RootHistoryAccess<'_> {
        RootHistoryAccess {
            data: &self.root_history_data,
        }
    }
}
