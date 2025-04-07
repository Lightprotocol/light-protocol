use light_compressed_account::instruction_data::zero_copy::ZPackedReadOnlyAddress;
use pinocchio::program_error::ProgramError;

use crate::{context::AcpAccount, errors::SystemProgramError, Result};

#[inline(always)]
pub fn verify_read_only_address_queue_non_inclusion(
    remaining_accounts: &mut [AcpAccount<'_>],
    read_only_addresses: &[ZPackedReadOnlyAddress],
) -> Result<()> {
    if read_only_addresses.is_empty() {
        return Ok(());
    }
    for read_only_address in read_only_addresses.iter() {
        let merkle_tree = if let AcpAccount::BatchedAddressTree(tree) =
            &mut remaining_accounts[read_only_address.address_merkle_tree_account_index as usize]
        {
            tree
        } else {
            // msg!(
            //     "Read only address account is not a BatchedAddressTree {:?}",
            //     read_only_address
            // );
            return Err(SystemProgramError::InvalidAccount.into());
        };
        merkle_tree
            .check_input_queue_non_inclusion(&read_only_address.address)
            .map_err(|_| ProgramError::from(SystemProgramError::ReadOnlyAddressAlreadyExists))?;
    }
    Ok(())
}
