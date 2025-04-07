use zerocopy::Ref;

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    zero_copy::{
        ZNewAddressParamsPacked, ZPackedMerkleContext, ZPackedReadOnlyAddress,
        ZPackedReadOnlyCompressedAccount,
    },
};
use crate::{compressed_account::CompressedAccountData, pubkey::Pubkey, CompressedAccountError};

pub trait InstructionDataTrait<'a> {
    fn owner(&self) -> Pubkey;
    fn new_addresses(&self) -> &[ZNewAddressParamsPacked];
    fn input_accounts(&self) -> &[impl InputAccountTrait<'a>];
    fn output_accounts(&self) -> &[impl OutputAccountTrait<'a>];
    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]>;
    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]>;
    fn is_compress(&self) -> bool;
    fn compress_or_decompress_lamports(&self) -> Option<u64>;
    fn proof(&self) -> Option<Ref<&'a [u8], CompressedProof>>;
    fn cpi_context(&self) -> Option<CompressedCpiContext>;
}

pub trait InputAccountTrait<'a> {
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

pub trait OutputAccountTrait<'a> {
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
