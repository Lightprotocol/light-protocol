use std::slice;

use aligned_sized::aligned_sized;
use borsh::{BorshDeserialize, BorshSerialize};
use light_account_checks::discriminator::Discriminator;
use light_compressed_account::instruction_data::{
    invoke_cpi::InstructionDataInvokeCpi,
    zero_copy::{
        ZInstructionDataInvokeCpi, ZNewAddressParamsPacked,
        ZOutputCompressedAccountWithPackedContext, ZPackedCompressedAccountWithMerkleContext,
    },
};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use zerocopy::{little_endian::U32, Ref};

use crate::CPI_CONTEXT_ACCOUNT_DISCRIMINATOR;

/// Collects instruction data without executing a compressed transaction.
/// Signer checks are performed on instruction data.
/// Collected instruction data is combined with the instruction data of the executing cpi,
/// and executed as a single transaction.
/// This enables to use input compressed accounts that are owned by multiple programs,
/// with one zero-knowledge proof.
#[aligned_sized(anchor)]
#[derive(Debug, PartialEq, Default, BorshSerialize, BorshDeserialize, Clone)]
#[repr(C)]
pub struct CpiContextAccount {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
    // Offset 72
    pub context: Vec<InstructionDataInvokeCpi>,
}

impl Discriminator for CpiContextAccount {
    const DISCRIMINATOR: [u8; 8] = CPI_CONTEXT_ACCOUNT_DISCRIMINATOR;
    const DISCRIMINATOR_SLICE: &'static [u8] = &Self::DISCRIMINATOR;
}

#[derive(Debug)]
pub struct ZCpiContextAccount<'a> {
    pub fee_payer: Ref<&'a mut [u8], light_compressed_account::pubkey::Pubkey>,
    pub associated_merkle_tree: Ref<&'a mut [u8], light_compressed_account::pubkey::Pubkey>,
    pub context: Vec<ZInstructionDataInvokeCpi<'a>>,
}

pub fn deserialize_cpi_context_account<'a>(
    account_info: &AccountInfo,
) -> std::result::Result<(ZCpiContextAccount<'a>, (usize, usize)), ZeroCopyError> {
    let mut account_data = account_info.try_borrow_mut_data().unwrap();
    let data = unsafe { slice::from_raw_parts_mut(account_data.as_mut_ptr(), account_data.len()) };

    let data_len = data.len();
    let (fee_payer, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(&mut data[8..])?;

    let (associated_merkle_tree, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(data)?;

    let (len, data) = Ref::<&'a mut [u8], U32>::from_prefix(data)?;

    let (context, offsets) = if *len > U32::from(1) {
        return Err(ZeroCopyError::InvalidCapacity);
    } else if *len == 1 {
        // Skip proof option byte.
        let bytes: &[u8] = &data[1..];
        let (new_address_params, bytes) =
            ZeroCopySliceBorsh::<ZNewAddressParamsPacked>::from_bytes_at(bytes)?;
        let (input_compressed_accounts_with_merkle_context, bytes) =
            Vec::<ZPackedCompressedAccountWithMerkleContext>::zero_copy_at(bytes)?;

        let output_accounts_start_offset = data_len - bytes.len() + 4;
        let (output_compressed_accounts, bytes) =
            Vec::<ZOutputCompressedAccountWithPackedContext>::zero_copy_at(bytes)?;
        let output_accounts_end_offset = data_len - bytes.len();

        let context = vec![ZInstructionDataInvokeCpi {
            new_address_params,
            input_compressed_accounts_with_merkle_context,
            output_compressed_accounts,
            // Parameters are not used in cpi context.
            proof: None,
            relay_fee: None,
            compress_or_decompress_lamports: None,
            is_compress: false,
            cpi_context: None,
        }];
        (
            context,
            (output_accounts_start_offset, output_accounts_end_offset),
        )
    } else {
        (vec![], (0, 0))
    };

    Ok((
        ZCpiContextAccount {
            fee_payer,
            associated_merkle_tree,
            context,
        },
        offsets,
    ))
}
