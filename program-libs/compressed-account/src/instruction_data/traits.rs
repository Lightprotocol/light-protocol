use std::fmt::Debug;

use zerocopy::Ref;

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    zero_copy::{ZPackedMerkleContext, ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
};
use crate::{compressed_account::CompressedAccountData, pubkey::Pubkey, CompressedAccountError};

pub trait InstructionData<'a> {
    fn owner(&self) -> Pubkey;
    fn new_addresses(&self) -> &[impl NewAddress<'a>];
    fn input_accounts(&self) -> &[impl InputAccount<'a>];
    fn output_accounts(&self) -> &[impl OutputAccount<'a>];
    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]>;
    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]>;
    fn is_compress(&self) -> bool;
    fn compress_or_decompress_lamports(&self) -> Option<u64>;
    fn proof(&self) -> Option<Ref<&'a [u8], CompressedProof>>;
    fn cpi_context(&self) -> Option<CompressedCpiContext>;
    fn bump(&self) -> Option<u8>;
    fn account_option_config(&self) -> AccountOptions;
    fn with_transaction_hash(&self) -> bool;
}

pub trait NewAddress<'a>
where
    Self: Debug,
{
    fn seed(&self) -> [u8; 32];
    fn address_queue_index(&self) -> u8;
    fn address_merkle_tree_account_index(&self) -> u8;
    fn address_merkle_tree_root_index(&self) -> u16;
    fn assigned_compressed_account_index(&self) -> Option<usize>;
}

pub trait InputAccount<'a>
where
    Self: Debug,
{
    fn owner(&self) -> &crate::pubkey::Pubkey;
    fn lamports(&self) -> u64;
    fn address(&self) -> Option<[u8; 32]>;
    fn merkle_context(&self) -> ZPackedMerkleContext;
    fn has_data(&self) -> bool;
    fn data(&self) -> Option<CompressedAccountData>;
    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError>;

    fn root_index(&self) -> u16;
}

pub trait OutputAccount<'a>
where
    Self: Debug,
{
    fn lamports(&self) -> u64;
    fn address(&self) -> Option<[u8; 32]>;
    fn has_data(&self) -> bool;
    fn data(&self) -> Option<CompressedAccountData>;
    fn owner(&self) -> Pubkey;
    fn merkle_tree_index(&self) -> u8;
    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError>;
}

pub struct AccountOptions {
    pub sol_pool_pda: bool,
    pub decompression_recipient: bool,
    pub cpi_context_account: bool,
}
