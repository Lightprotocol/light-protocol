use aligned_sized::aligned_sized;
use anchor_lang::prelude::borsh::{self, BorshDeserialize, BorshSerialize};
use light_compressed_account::instruction_data::{
    invoke_cpi::InstructionDataInvokeCpi, zero_copy::ZInstructionDataInvokeCpi,
};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use std::slice;
use zerocopy::{little_endian::U32, Ref};

/// Collects instruction data without executing a compressed transaction.
/// Signer checks are performed on instruction data.
/// Collected instruction data is combined with the instruction data of the executing cpi,
/// and executed as a single transaction.
/// This enables to use input compressed accounts that are owned by multiple programs,
/// with one zero-knowledge proof.
#[aligned_sized(anchor)]
#[derive(Debug, PartialEq, Default, BorshDeserialize, BorshSerialize)]
// #[account]
#[repr(C)]
pub struct CpiContextAccount {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
    // Offset 72
    pub context: Vec<InstructionDataInvokeCpi>,
}

impl CpiContextAccount {
    pub fn init(&mut self, associated_merkle_tree: Pubkey) {
        self.associated_merkle_tree = associated_merkle_tree;
        self.context = Vec::new();
    }
}

pub struct ZCpiContextAccount<'a> {
    pub fee_payer: Ref<&'a mut [u8], light_compressed_account::pubkey::Pubkey>,
    pub associated_merkle_tree: Ref<&'a mut [u8], light_compressed_account::pubkey::Pubkey>,
    pub context: Vec<ZInstructionDataInvokeCpi<'a>>,
}

pub fn deserialize_cpi_context_account<'a>(
    account_info: &AccountInfo,
) -> std::result::Result<ZCpiContextAccount<'a>, ZeroCopyError> {
    let mut account_data = account_info.try_borrow_mut_data().unwrap();
    let data = unsafe { slice::from_raw_parts_mut(account_data.as_mut_ptr(), account_data.len()) };
    let (fee_payer, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(&mut data[8..])?;
    let (associated_merkle_tree, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(data)?;
    let (len, data) = Ref::<&'a mut [u8], U32>::from_prefix(data)?;
    let mut data = &*data;
    let mut context = Vec::new();
    for _ in 0..(u64::from(*len)) as usize {
        let (context_item, new_data) = ZInstructionDataInvokeCpi::zero_copy_at(data)?;
        context.push(context_item);
        data = new_data;
    }
    Ok(ZCpiContextAccount {
        fee_payer,
        associated_merkle_tree,
        context,
    })
}
