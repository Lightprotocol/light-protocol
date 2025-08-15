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
    fn new_address_owner(&self) -> Vec<Option<Pubkey>>;
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

pub fn new_addresses_eq<'a>(left: &[impl NewAddress<'a>], right: &[impl NewAddress<'a>]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right.iter()).all(|(l, r)| {
        l.seed() == r.seed()
            && l.address_queue_index() == r.address_queue_index()
            && l.address_merkle_tree_account_index() == r.address_merkle_tree_account_index()
            && l.address_merkle_tree_root_index() == r.address_merkle_tree_root_index()
            && l.assigned_compressed_account_index() == r.assigned_compressed_account_index()
    })
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
    fn skip(&self) -> bool;
    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError>;

    fn root_index(&self) -> u16;
}

pub fn input_accounts_eq<'a>(
    left: &[impl InputAccount<'a>],
    right: &[impl InputAccount<'a>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right.iter()).all(|(l, r)| {
        l.owner() == r.owner()
            && l.lamports() == r.lamports()
            && l.address() == r.address()
            && l.merkle_context() == r.merkle_context()
            && l.skip() == r.skip()
            && l.has_data() == r.has_data()
            && l.data() == r.data()
            && l.root_index() == r.root_index()
    })
}

pub trait OutputAccount<'a>
where
    Self: Debug,
{
    fn lamports(&self) -> u64;
    fn address(&self) -> Option<[u8; 32]>;
    fn has_data(&self) -> bool;
    fn skip(&self) -> bool;
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

pub fn output_accounts_eq<'a>(
    left: &[impl OutputAccount<'a>],
    right: &[impl OutputAccount<'a>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right.iter()).all(|(l, r)| {
        l.owner() == r.owner()
            && l.lamports() == r.lamports()
            && l.address() == r.address()
            && l.merkle_tree_index() == r.merkle_tree_index()
            && l.skip() == r.skip()
            && l.has_data() == r.has_data()
            && l.data() == r.data()
    })
}

/// Compares:
/// 1. new address
/// 2. input account
/// 3. output account
/// 4. read-only address
/// 5. read-only account
/// - other data is not compared
pub fn instruction_data_eq<'a>(
    left: &impl InstructionData<'a>,
    right: &impl InstructionData<'a>,
) -> bool {
    // Compare collections using our helper functions
    new_addresses_eq(left.new_addresses(), right.new_addresses()) &&
    input_accounts_eq(left.input_accounts(), right.input_accounts()) &&
    output_accounts_eq(left.output_accounts(), right.output_accounts()) &&
    // Compare read-only data
    left.read_only_addresses() == right.read_only_addresses() &&
    left.read_only_accounts() == right.read_only_accounts()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountOptions {
    pub sol_pool_pda: bool,
    pub decompression_recipient: bool,
    pub cpi_context_account: bool,
    pub write_to_cpi_context: bool,
}

impl AccountOptions {
    pub fn get_num_expected_accounts(&self) -> usize {
        let mut num = 3;
        if !self.write_to_cpi_context {
            num += 1;
        }
        if self.sol_pool_pda {
            num += 1;
        }
        if self.decompression_recipient {
            num += 1;
        }
        if self.cpi_context_account {
            num += 1;
        }
        num
    }
}
